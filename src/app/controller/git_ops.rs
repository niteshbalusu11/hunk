impl DiffViewer {
    fn next_git_action_epoch(&mut self) -> usize {
        self.git_action_epoch = self.git_action_epoch.saturating_add(1);
        self.git_action_epoch
    }

    fn run_git_action<F>(&mut self, cx: &mut Context<Self>, action: F)
    where
        F: FnOnce(std::path::PathBuf) -> anyhow::Result<String> + Send + 'static,
    {
        if self.git_action_loading {
            return;
        }

        let Some(repo_root) = self.repo_root.clone() else {
            self.git_status_message = Some("No JJ repository available.".to_string());
            cx.notify();
            return;
        };

        let epoch = self.next_git_action_epoch();
        self.git_action_loading = true;
        self.git_status_message = None;
        cx.notify();

        self.git_action_task = cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { action(repo_root) }).await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.git_action_epoch {
                        return;
                    }

                    this.git_action_loading = false;
                    match result {
                        Ok(message) => {
                            this.git_status_message = if message.is_empty() {
                                None
                            } else {
                                Some(message)
                            };
                            this.request_snapshot_refresh_internal(true, cx);
                        }
                        Err(err) => {
                            this.git_status_message = Some(format!("JJ error: {err:#}"));
                        }
                    }

                    cx.notify();
                })
                .ok();
            }
        });
    }

    pub(super) fn checkout_branch(&mut self, branch_name: String, cx: &mut Context<Self>) {
        self.run_git_action(cx, move |repo_root| {
            checkout_or_create_branch(&repo_root, &branch_name)?;
            Ok(format!("Switched to {}", branch_name))
        });
    }

    pub(super) fn toggle_commit_file_included(
        &mut self,
        file_path: String,
        include: bool,
        cx: &mut Context<Self>,
    ) {
        if include {
            self.commit_excluded_files.remove(file_path.as_str());
        } else {
            self.commit_excluded_files.insert(file_path);
        }
        cx.notify();
    }

    pub(super) fn include_all_files_for_commit(&mut self, cx: &mut Context<Self>) {
        if self.commit_excluded_files.is_empty() {
            return;
        }
        self.commit_excluded_files.clear();
        cx.notify();
    }

    pub(super) fn included_commit_file_count(&self) -> usize {
        self.files
            .iter()
            .filter(|file| !self.commit_excluded_files.contains(file.path.as_str()))
            .count()
    }

    fn selected_commit_paths(&self) -> Vec<String> {
        self.files
            .iter()
            .filter(|file| !self.commit_excluded_files.contains(file.path.as_str()))
            .map(|file| file.path.clone())
            .collect()
    }

    pub(super) fn create_or_switch_branch_from_input(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let raw_name = self.branch_input_state.read(cx).value().to_string();
        if raw_name.trim().is_empty() {
            self.git_status_message = Some("Branch name is required.".to_string());
            cx.notify();
            return;
        }

        let sanitized = sanitize_branch_name(&raw_name);
        self.branch_input_state.update(cx, |state, cx| {
            state.set_value(sanitized.clone(), window, cx);
        });

        self.run_git_action(cx, move |repo_root| {
            checkout_or_create_branch(&repo_root, &sanitized)?;
            Ok(format!("Switched to {}", sanitized))
        });
    }

    pub(super) fn push_or_publish_current_branch(&mut self, cx: &mut Context<Self>) {
        let branch_name = self.branch_name.clone();
        let has_upstream = self.branch_has_upstream;
        if branch_name.is_empty() || branch_name == "unknown" || branch_name.starts_with("detached") {
            self.git_status_message = Some("Cannot push a detached or unknown branch.".to_string());
            cx.notify();
            return;
        }

        self.run_git_action(cx, move |repo_root| {
            push_current_branch(&repo_root, &branch_name, has_upstream)?;
            if has_upstream {
                Ok(format!("Pushed {}", branch_name))
            } else {
                Ok(format!("Published {}", branch_name))
            }
        });
    }

    pub(super) fn commit_from_input(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if self.git_action_loading {
            return;
        }

        let message = self.commit_input_state.read(cx).value().to_string();
        if message.trim().is_empty() {
            self.git_status_message = Some("Commit message cannot be empty.".to_string());
            cx.notify();
            return;
        }

        let Some(repo_root) = self.repo_root.clone() else {
            self.git_status_message = Some("No JJ repository available.".to_string());
            cx.notify();
            return;
        };
        let selected_paths = self.selected_commit_paths();
        if selected_paths.is_empty() {
            self.git_status_message =
                Some("Select at least one file to include in commit.".to_string());
            cx.notify();
            return;
        }
        let partial_commit = selected_paths.len() != self.files.len();

        let epoch = self.next_git_action_epoch();
        self.git_action_loading = true;
        self.git_status_message = None;
        cx.notify();

        self.git_action_task = cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move {
                if partial_commit {
                    commit_selected_paths(&repo_root, &message, &selected_paths)?;
                } else {
                    commit_staged(&repo_root, &message)?;
                }
                Ok::<String, anyhow::Error>(message.trim_end().to_string())
            });
            let result = result.await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.git_action_epoch {
                        return;
                    }

                    this.git_action_loading = false;
                    match result {
                        Ok(subject) => {
                            this.commit_excluded_files.clear();
                            this.git_status_message = Some("Created commit".to_string());
                            this.last_commit_subject = Some(subject);

                            let commit_input_state = this.commit_input_state.clone();
                            if let Some(window_handle) = cx.windows().into_iter().next()
                                && let Err(err) = cx.update_window(window_handle, |_, window, cx| {
                                    commit_input_state.update(cx, |state, cx| {
                                        state.set_value("", window, cx);
                                    });
                                })
                            {
                                error!("failed to clear commit input after commit: {err:#}");
                            }

                            this.request_snapshot_refresh(cx);
                        }
                        Err(err) => {
                            this.git_status_message = Some(format!("JJ error: {err:#}"));
                        }
                    }

                    cx.notify();
                })
                .ok();
            }
        });
    }

    pub(super) fn toggle_branch_picker(&mut self, cx: &mut Context<Self>) {
        self.branch_picker_open = !self.branch_picker_open;
        cx.notify();
    }
}
