impl DiffViewer {
    pub(super) fn save_current_file_action(
        &mut self,
        _: &SaveCurrentFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.save_current_editor_file(window, cx);
    }

    pub(super) fn reload_current_editor_file(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.editor_path.clone() else {
            return;
        };

        self.request_file_editor_reload(path, cx);
    }

    pub(super) fn request_file_editor_reload(&mut self, path: String, cx: &mut Context<Self>) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.editor_loading = false;
            self.editor_error = Some("No repository is open.".to_string());
            self.editor_path = None;
            self.editor_last_saved_text = None;
            self.editor_dirty = false;
            self.reset_editor_input(cx);
            cx.notify();
            return;
        };

        let epoch = self.next_editor_epoch();
        self.editor_loading = true;
        self.editor_error = None;
        self.editor_path = Some(path.clone());
        cx.notify();

        self.editor_task = cx.spawn(async move |this, cx| {
            let target_path = path.clone();
            let result = cx.background_executor().spawn(async move {
                load_file_editor_document(&repo_root, target_path.as_str(), FILE_EDITOR_MAX_BYTES)
            });
            let result = result.await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.editor_epoch {
                        return;
                    }

                    this.editor_loading = false;
                    match result {
                        Ok(document) => {
                            let language = document.language;
                            let text = document.text;
                            this.editor_last_saved_text = Some(text.clone());
                            this.editor_dirty = false;
                            this.editor_error = None;
                            this.apply_editor_document(language, text, cx);
                        }
                        Err(err) => {
                            this.editor_last_saved_text = None;
                            this.editor_dirty = false;
                            this.editor_error = Some(format!("Editor unavailable: {err}"));
                            this.reset_editor_input(cx);
                        }
                    }

                    cx.notify();
                })
                .ok();
            }
        });
    }

    pub(super) fn save_current_editor_file(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.right_pane_mode != RightPaneMode::FileEditor
            || self.editor_loading
            || self.editor_save_loading
        {
            return;
        }

        let Some(repo_root) = self.repo_root.clone() else {
            self.git_status_message = Some("No git repository available.".to_string());
            cx.notify();
            return;
        };
        let Some(path) = self.editor_path.clone() else {
            self.git_status_message = Some("No file is open in editor.".to_string());
            cx.notify();
            return;
        };

        self.sync_editor_dirty_from_input(cx);
        if !self.editor_dirty {
            self.git_status_message = Some("No unsaved changes.".to_string());
            cx.notify();
            return;
        }

        let current_text = self.editor_input_state.read(cx).value().to_string();
        let text_to_write = current_text.clone();
        let saved_text = current_text;
        let path_for_write = path.clone();
        let status_path = path.clone();
        let epoch = self.next_editor_save_epoch();
        self.editor_save_loading = true;
        self.editor_error = None;
        self.git_status_message = None;
        cx.notify();

        self.editor_save_task = cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move {
                save_file_editor_document(&repo_root, path_for_write.as_str(), text_to_write.as_str())
            });
            let result = result.await;

            if let Some(this) = this.upgrade() {
                this.update(cx, move |this, cx| {
                    if epoch != this.editor_save_epoch {
                        return;
                    }

                    this.editor_save_loading = false;
                    match result {
                        Ok(()) => {
                            this.editor_last_saved_text = Some(saved_text.clone());
                            this.sync_editor_dirty_from_input(cx);
                            this.git_status_message = Some(format!("Saved {}", status_path));
                            this.request_snapshot_refresh(cx);
                        }
                        Err(err) => {
                            this.git_status_message =
                                Some(format!("Save failed for {}: {err:#}", status_path));
                        }
                    }

                    cx.notify();
                })
                .ok();
            }
        });
    }

    pub(super) fn sync_editor_dirty_from_input(&mut self, cx: &mut Context<Self>) {
        if self.editor_loading || self.editor_path.is_none() {
            return;
        }

        let current_text = self.editor_input_state.read(cx).value();
        let saved_text = self.editor_last_saved_text.as_deref().unwrap_or_default();
        let dirty = current_text.as_ref() != saved_text;
        if self.editor_dirty != dirty {
            self.editor_dirty = dirty;
            cx.notify();
        }
    }

    fn apply_editor_document(&mut self, language: String, text: String, cx: &mut Context<Self>) {
        let editor_input_state = self.editor_input_state.clone();
        let Some(window_handle) = cx.windows().into_iter().next() else {
            self.editor_error = Some("Cannot open editor without an active window.".to_string());
            return;
        };

        if let Err(err) = cx.update_window(window_handle, |_, window, cx| {
            editor_input_state.update(cx, |input, cx| {
                input.set_highlighter(language.clone(), cx);
                input.set_value(text.clone(), window, cx);
                input.focus(window, cx);
            });
        }) {
            error!("failed to apply editor document: {err:#}");
            self.editor_error = Some("Failed to initialize editor view.".to_string());
        }
    }

    pub(super) fn clear_editor_state(&mut self, cx: &mut Context<Self>) {
        self.editor_path = None;
        self.editor_loading = false;
        self.editor_error = None;
        self.editor_dirty = false;
        self.editor_last_saved_text = None;
        self.editor_save_loading = false;
        self.reset_editor_input(cx);
    }

    fn reset_editor_input(&mut self, cx: &mut Context<Self>) {
        let editor_input_state = self.editor_input_state.clone();
        let Some(window_handle) = cx.windows().into_iter().next() else {
            return;
        };

        if let Err(err) = cx.update_window(window_handle, |_, window, cx| {
            editor_input_state.update(cx, |input, cx| {
                input.set_highlighter("text", cx);
                input.set_value("", window, cx);
            });
        }) {
            error!("failed to reset editor input: {err:#}");
        }
    }

    fn next_editor_epoch(&mut self) -> usize {
        self.editor_epoch = self.editor_epoch.saturating_add(1);
        self.editor_epoch
    }

    fn next_editor_save_epoch(&mut self) -> usize {
        self.editor_save_epoch = self.editor_save_epoch.saturating_add(1);
        self.editor_save_epoch
    }
}
