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
            self.git_status_message = Some("No git repository available.".to_string());
            cx.notify();
            return;
        };

        let epoch = self.next_git_action_epoch();
        self.git_action_loading = true;
        self.git_status_message = None;

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
                            this.request_snapshot_refresh(cx);
                        }
                        Err(err) => {
                            this.git_status_message = Some(format!("Git error: {err:#}"));
                        }
                    }

                    cx.notify();
                })
                .ok();
            }
        });
    }

    pub(super) fn toggle_stage_for_file(
        &mut self,
        file_path: String,
        should_stage: bool,
        cx: &mut Context<Self>,
    ) {
        self.run_git_action(cx, move |repo_root| {
            if should_stage {
                stage_file(&repo_root, &file_path)?;
                Ok(format!("Staged {}", file_path))
            } else {
                unstage_file(&repo_root, &file_path)?;
                Ok(format!("Unstaged {}", file_path))
            }
        });
    }

    pub(super) fn stage_all_files(&mut self, cx: &mut Context<Self>) {
        self.run_git_action(cx, move |repo_root| {
            stage_all(&repo_root)?;
            Ok("Staged all changes".to_string())
        });
    }

    pub(super) fn unstage_all_files(&mut self, cx: &mut Context<Self>) {
        self.run_git_action(cx, move |repo_root| {
            unstage_all(&repo_root)?;
            Ok("Unstaged all changes".to_string())
        });
    }

    pub(super) fn checkout_branch(&mut self, branch_name: String, cx: &mut Context<Self>) {
        self.run_git_action(cx, move |repo_root| {
            checkout_or_create_branch(&repo_root, &branch_name)?;
            Ok(format!("Switched to {}", branch_name))
        });
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

    pub(super) fn commit_from_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
            self.git_status_message = Some("No git repository available.".to_string());
            cx.notify();
            return;
        };

        self.git_action_loading = true;
        self.git_status_message = None;

        match commit_staged(&repo_root, &message) {
            Ok(()) => {
                self.git_action_loading = false;
                self.git_status_message = Some("Created commit".to_string());
                self.last_commit_subject = Some(message.trim_end().to_string());
                self.commit_input_state.update(cx, |state, cx| {
                    state.set_value("", window, cx);
                });
                self.request_snapshot_refresh(cx);
            }
            Err(err) => {
                self.git_action_loading = false;
                self.git_status_message = Some(format!("Git error: {err:#}"));
            }
        }

        cx.notify();
    }

    pub(super) fn toggle_branch_picker(&mut self, cx: &mut Context<Self>) {
        self.branch_picker_open = !self.branch_picker_open;
        cx.notify();
    }
}
