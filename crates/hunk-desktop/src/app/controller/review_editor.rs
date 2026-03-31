const REVIEW_EDITOR_SAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(250);
const REVIEW_EDITOR_PRESENTATION_DEBOUNCE: std::time::Duration =
    std::time::Duration::from_millis(90);
const REVIEW_EDITOR_CONTEXT_LINES: usize = 3;

impl DiffViewer {
    fn next_review_editor_epoch(&mut self) -> usize {
        self.review_editor_session.load_epoch =
            self.review_editor_session.load_epoch.saturating_add(1);
        self.review_editor_session.load_epoch
    }

    fn next_review_editor_save_epoch(&mut self) -> usize {
        self.review_editor_session.save_epoch =
            self.review_editor_session.save_epoch.saturating_add(1);
        self.review_editor_session.save_epoch
    }

    fn next_review_editor_presentation_epoch(&mut self) -> usize {
        self.review_editor_session.presentation_epoch = self
            .review_editor_session
            .presentation_epoch
            .saturating_add(1);
        self.review_editor_session.presentation_epoch
    }

    fn cancel_review_editor_save_task(&mut self) {
        let previous_task =
            std::mem::replace(&mut self.review_editor_session.save_task, Task::ready(()));
        drop(previous_task);
    }

    fn cancel_review_editor_presentation_task(&mut self) {
        let previous_task = std::mem::replace(
            &mut self.review_editor_session.presentation_task,
            Task::ready(()),
        );
        drop(previous_task);
    }

    fn clear_review_editor_session(&mut self) {
        self.next_review_editor_epoch();
        self.next_review_editor_save_epoch();
        self.next_review_editor_presentation_epoch();
        self.cancel_review_editor_presentation_task();
        self.review_editor_session.loading = false;
        self.review_editor_session.presentation_loading = false;
        self.review_editor_session.save_loading = false;
        self.review_editor_session.error = None;
        self.review_editor_session.path = None;
        self.review_editor_session.left_source_id = None;
        self.review_editor_session.right_source_id = None;
        self.review_editor_session.left_present = false;
        self.review_editor_session.right_present = false;
        self.review_editor_session.load_task = Task::ready(());
        self.review_editor_session.save_task = Task::ready(());
        self.review_editor_session.last_saved_text = None;
        self.review_editor_session.right_hunk_lines.clear();
        self.review_editor_session.right_to_left_line_map.clear();
        self.review_editor_session.pending_target_right_line = None;
        self.review_editor_session.left_editor.borrow_mut().clear();
        self.review_editor_session.right_editor.borrow_mut().clear();
    }

    fn apply_review_editor_presentation(
        &mut self,
        presentation: crate::app::review_editor_model::ReviewEditorPresentation,
    ) {
        self.review_editor_session
            .left_editor
            .borrow_mut()
            .set_manual_overlays(presentation.left_overlays);
        self.review_editor_session
            .right_editor
            .borrow_mut()
            .set_manual_overlays(presentation.right_overlays);
        self.review_editor_session
            .left_editor
            .borrow_mut()
            .set_folded_regions(presentation.left_folds);
        self.review_editor_session
            .right_editor
            .borrow_mut()
            .set_folded_regions(presentation.right_folds);
        self.review_editor_session.right_hunk_lines = presentation.right_hunk_lines;
        self.review_editor_session.right_to_left_line_map = presentation.right_to_left_line_map;
        self.apply_pending_review_editor_navigation_target();
    }

    fn request_review_editor_presentation_refresh(
        &mut self,
        debounce: Option<std::time::Duration>,
        cx: &mut Context<Self>,
    ) {
        let Some(left_text) = self.review_editor_session.left_editor.borrow().current_text() else {
            return;
        };
        let Some(right_text) = self.review_editor_session.right_editor.borrow().current_text() else {
            return;
        };
        let path = self.review_editor_session.path.clone();
        let left_source_id = self.review_editor_session.left_source_id.clone();
        let right_source_id = self.review_editor_session.right_source_id.clone();
        let pinned_right_line = self
            .review_editor_session
            .right_editor
            .borrow()
            .selection()
            .map(|selection| selection.head.line);
        let started_at = std::time::Instant::now();
        let presentation_epoch = self.next_review_editor_presentation_epoch();
        self.cancel_review_editor_presentation_task();
        self.review_editor_session.presentation_loading = debounce.is_some();
        self.review_editor_session.presentation_task = cx.spawn(async move |this, cx| {
            if let Some(delay) = debounce {
                cx.background_executor().timer(delay).await;
            }
            let presentation = cx.background_executor().spawn(async move {
                build_review_editor_presentation_from_texts(
                    left_text.as_str(),
                    right_text.as_str(),
                    REVIEW_EDITOR_CONTEXT_LINES,
                    pinned_right_line,
                )
            });
            let presentation = presentation.await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if presentation_epoch != this.review_editor_session.presentation_epoch
                        || this.review_editor_session.path != path
                        || this.review_editor_session.left_source_id != left_source_id
                        || this.review_editor_session.right_source_id != right_source_id
                    {
                        return;
                    }

                    this.review_editor_session.presentation_loading = false;
                    this.apply_review_editor_presentation(presentation);
                    this.sync_review_editor_viewports_from_right();
                    debug!(
                        path = path.as_deref().unwrap_or("unknown"),
                        left = left_source_id.as_deref().unwrap_or("unknown"),
                        right = right_source_id.as_deref().unwrap_or("unknown"),
                        hunks = this.review_editor_session.right_hunk_lines.len(),
                        elapsed_ms = started_at.elapsed().as_millis(),
                        "review editor presentation refreshed"
                    );
                    cx.notify();
                });
            }
        });
    }

    fn sync_review_editor_viewports_from_right(&mut self) {
        let right_first_visible_source_line = self
            .review_editor_session
            .right_editor
            .borrow()
            .first_visible_source_line()
            .unwrap_or(0);
        let mapped_left_source_line = nearest_mapped_review_editor_left_line(
            &self.review_editor_session.right_to_left_line_map,
            right_first_visible_source_line,
        )
        .unwrap_or(0);
        self.review_editor_session
            .left_editor
            .borrow_mut()
            .set_first_visible_source_line(mapped_left_source_line);
    }

    fn apply_pending_review_editor_navigation_target(&mut self) {
        let Some(target_line) = self.review_editor_session.pending_target_right_line.take() else {
            return;
        };
        self.review_editor_session
            .right_editor
            .borrow_mut()
            .set_caret_line(target_line);
    }

    fn jump_review_editor_to_line(&mut self, target_line: usize, cx: &mut Context<Self>) -> bool {
        if !self
            .review_editor_session
            .right_editor
            .borrow_mut()
            .set_caret_line(target_line)
        {
            return false;
        }
        self.sync_review_editor_viewports_from_right();
        cx.notify();
        true
    }

    fn navigate_review_editor_hunk_relative(
        &mut self,
        direction: isize,
        cx: &mut Context<Self>,
    ) -> bool {
        let current_line = self
            .review_editor_session
            .right_editor
            .borrow()
            .selection()
            .map(|selection| selection.head.line)
            .or_else(|| {
                self.review_editor_session
                    .right_editor
                    .borrow()
                    .first_visible_source_line()
            })
            .unwrap_or(0);
        let Some(target_line) = find_wrapped_review_editor_hunk_line(
            &self.review_editor_session.right_hunk_lines,
            current_line,
            direction,
        ) else {
            return false;
        };
        self.jump_review_editor_to_line(target_line, cx)
    }

    fn current_review_editor_text(&self) -> anyhow::Result<String> {
        self.review_editor_session
            .right_editor
            .borrow()
            .current_text()
            .ok_or_else(|| anyhow::anyhow!("no active review editor buffer"))
    }

    fn schedule_review_editor_save(&mut self, cx: &mut Context<Self>) {
        let Some(repo_root) = self.project_path.clone() else {
            return;
        };
        let Some(path) = self.review_editor_session.path.clone() else {
            return;
        };
        let Ok(current_text) = self.current_review_editor_text() else {
            return;
        };
        if self
            .review_editor_session
            .last_saved_text
            .as_deref()
            .is_some_and(|saved| saved == current_text.as_str())
        {
            return;
        }

        let text_to_write = current_text.clone();
        let saved_text = current_text;
        let path_for_write = path.clone();
        let status_path = path.clone();
        let save_epoch = self.next_review_editor_save_epoch();
        self.cancel_review_editor_save_task();
        self.review_editor_session.save_loading = true;
        self.review_editor_session.save_task = cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(REVIEW_EDITOR_SAVE_DEBOUNCE)
                .await;
            let result = cx.background_executor().spawn(async move {
                save_file_editor_document(&repo_root, path_for_write.as_str(), text_to_write.as_str())
            });
            let result = result.await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if save_epoch != this.review_editor_session.save_epoch {
                        return;
                    }

                    this.review_editor_session.save_loading = false;
                    match result {
                        Ok(()) => {
                            this.review_editor_session.last_saved_text = Some(saved_text.clone());
                            this.review_editor_session.right_editor.borrow_mut().mark_saved();
                            this.git_status_message = Some(format!("Saved {}", status_path));
                            this.request_snapshot_refresh_workflow_only(false, cx);
                        }
                        Err(err) => {
                            this.git_status_message =
                                Some(format!("Save failed for {}: {err:#}", status_path));
                        }
                    }

                    cx.notify();
                });
            }
        });
    }

    pub(super) fn review_editor_copy_selection(&self, cx: &mut Context<Self>) -> bool {
        let Some(text) = self.review_editor_session.right_editor.borrow().copy_selection_text() else {
            return false;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        true
    }

    pub(super) fn review_editor_cut_selection(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(text) = self.review_editor_session.right_editor.borrow_mut().cut_selection_text() else {
            return false;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.sync_review_editor_viewports_from_right();
        self.request_review_editor_presentation_refresh(
            Some(REVIEW_EDITOR_PRESENTATION_DEBOUNCE),
            cx,
        );
        self.schedule_review_editor_save(cx);
        cx.notify();
        true
    }

    pub(super) fn review_editor_paste_from_clipboard(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return false;
        };
        if !self
            .review_editor_session
            .right_editor
            .borrow_mut()
            .paste_text(text.as_str())
        {
            return false;
        }
        self.sync_review_editor_viewports_from_right();
        self.request_review_editor_presentation_refresh(
            Some(REVIEW_EDITOR_PRESENTATION_DEBOUNCE),
            cx,
        );
        self.schedule_review_editor_save(cx);
        cx.notify();
        true
    }

    pub(super) fn review_editor_handle_keystroke(
        &mut self,
        keystroke: &gpui::Keystroke,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self
            .review_editor_session
            .right_editor
            .borrow_mut()
            .handle_keystroke(keystroke)
        {
            return false;
        }
        self.sync_review_editor_viewports_from_right();
        self.request_review_editor_presentation_refresh(
            Some(REVIEW_EDITOR_PRESENTATION_DEBOUNCE),
            cx,
        );
        self.schedule_review_editor_save(cx);
        cx.notify();
        true
    }

    pub(super) fn review_editor_scroll_lines(
        &mut self,
        line_count: usize,
        direction: crate::app::native_files_editor::ScrollDirection,
        cx: &mut Context<Self>,
    ) -> bool {
        self.review_editor_session
            .right_editor
            .borrow_mut()
            .scroll_lines(line_count, direction);
        self.sync_review_editor_viewports_from_right();
        cx.notify();
        true
    }

    fn request_review_editor_reload(&mut self, force: bool, cx: &mut Context<Self>) {
        if self.workspace_view_mode != WorkspaceViewMode::Diff {
            self.clear_review_editor_session();
            return;
        }

        let Some(path) = self.selected_path.clone() else {
            self.clear_review_editor_session();
            return;
        };
        if self.review_editor_session.path.as_deref() != Some(path.as_str()) {
            self.active_review_editor_comment_line = None;
        }
        if !self.active_diff_contains_path(path.as_str()) {
            self.clear_review_editor_session();
            return;
        }

        let Some(project_root) = self.project_path.clone() else {
            self.clear_review_editor_session();
            return;
        };
        let Some((left_source, right_source)) = self.selected_review_compare_sources() else {
            self.clear_review_editor_session();
            return;
        };
        let previous_path = self.review_editor_session.path.clone();
        let previous_left_source_id = self.review_editor_session.left_source_id.clone();
        let previous_right_source_id = self.review_editor_session.right_source_id.clone();
        let left_source_id = self.review_left_source_id.clone();
        let right_source_id = self.review_right_source_id.clone();

        if !force
            && self.review_editor_session.path.as_deref() == Some(path.as_str())
            && self.review_editor_session.left_source_id == left_source_id
            && self.review_editor_session.right_source_id == right_source_id
            && !self.review_editor_session.loading
            && self.review_editor_session.error.is_none()
        {
            return;
        }

        let epoch = self.next_review_editor_epoch();
        self.next_review_editor_presentation_epoch();
        self.cancel_review_editor_presentation_task();
        self.review_editor_session.loading = true;
        self.review_editor_session.presentation_loading = false;
        self.review_editor_session.error = None;
        self.review_editor_session.path = Some(path.clone());
        self.review_editor_session.left_source_id = left_source_id.clone();
        self.review_editor_session.right_source_id = right_source_id.clone();
        self.review_editor_session.load_task = cx.spawn(async move |this, cx| {
            let project_root_for_load = project_root.clone();
            let result = cx.background_executor().spawn(async move {
                load_compare_file_document(
                    &project_root_for_load,
                    &left_source,
                    &right_source,
                    path.as_str(),
                )
            });
            let result = result.await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.review_editor_session.load_epoch {
                        return;
                    }

                    this.review_editor_session.loading = false;
                    match result {
                        Ok(document) => {
                            let path = document.path.clone();
                            let absolute_path = project_root.join(path.as_str());
                            let preserve_dirty_right = should_preserve_dirty_review_editor_right(
                                previous_path.as_deref(),
                                previous_left_source_id.as_deref(),
                                previous_right_source_id.as_deref(),
                                path.as_str(),
                                left_source_id.as_deref(),
                                right_source_id.as_deref(),
                                this.review_editor_session.right_editor.borrow().is_dirty(),
                            );
                            let left_result = this
                                .review_editor_session
                                .left_editor
                                .borrow_mut()
                                .sync_document(&absolute_path, document.left_text.as_str(), true);
                            let right_result = if preserve_dirty_right {
                                Ok(())
                            } else {
                                this.review_editor_session
                                    .right_editor
                                    .borrow_mut()
                                    .sync_document(&absolute_path, document.right_text.as_str(), true)
                            };

                            match left_result.and(right_result) {
                                Ok(()) => {
                                    this.review_editor_session.left_present = document.left_present;
                                    this.review_editor_session.right_present =
                                        document.right_present || preserve_dirty_right;
                                    if !preserve_dirty_right {
                                        this.review_editor_session.save_loading = false;
                                    }
                                    this.review_editor_session.error = None;
                                    if !preserve_dirty_right {
                                        this.review_editor_session.last_saved_text =
                                            Some(document.right_text.clone());
                                    }
                                    this.request_review_editor_presentation_refresh(None, cx);
                                }
                                Err(err) => {
                                    this.review_editor_session.error = Some(format!(
                                        "Review editor preview unavailable: {err:#}"
                                    ));
                                    this.review_editor_session.left_present = false;
                                    this.review_editor_session.right_present = false;
                                    this.review_editor_session.presentation_loading = false;
                                    this.review_editor_session.save_loading = false;
                                    this.review_editor_session.last_saved_text = None;
                                    this.review_editor_session.right_hunk_lines.clear();
                                    this.review_editor_session.right_to_left_line_map.clear();
                                    this.review_editor_session.left_editor.borrow_mut().clear();
                                    this.review_editor_session.right_editor.borrow_mut().clear();
                                }
                            }
                        }
                        Err(err) => {
                            this.review_editor_session.error =
                                Some(format!("Review editor preview unavailable: {err:#}"));
                            this.review_editor_session.left_present = false;
                            this.review_editor_session.right_present = false;
                            this.review_editor_session.presentation_loading = false;
                            this.review_editor_session.save_loading = false;
                            this.review_editor_session.last_saved_text = None;
                            this.review_editor_session.right_hunk_lines.clear();
                            this.review_editor_session.right_to_left_line_map.clear();
                            this.review_editor_session.left_editor.borrow_mut().clear();
                            this.review_editor_session.right_editor.borrow_mut().clear();
                        }
                    }

                    cx.notify();
                });
            }
        });
        cx.notify();
    }
}
