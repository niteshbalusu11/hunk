impl DiffViewer {
    fn push_error_notification(message: String, cx: &mut Context<Self>) {
        let window_handles = cx.windows().into_iter().collect::<Vec<_>>();
        if window_handles.is_empty() {
            error!("cannot show git action error notification: no windows available");
            return;
        }

        for window_handle in window_handles {
            if let Err(err) = cx.update_window(window_handle, |_, window, cx| {
                gpui_component::WindowExt::push_notification(
                    window,
                    gpui_component::notification::Notification::error(message.clone()),
                    cx,
                );
            }) {
                error!("failed to show git action error notification: {err:#}");
            }
        }
    }

    fn push_warning_notification(message: String, cx: &mut Context<Self>) {
        let window_handles = cx.windows().into_iter().collect::<Vec<_>>();
        if window_handles.is_empty() {
            error!("cannot show git action warning notification: no windows available");
            return;
        }

        for window_handle in window_handles {
            if let Err(err) = cx.update_window(window_handle, |_, window, cx| {
                gpui_component::WindowExt::push_notification(
                    window,
                    gpui_component::notification::Notification::warning(message.clone()),
                    cx,
                );
            }) {
                error!("failed to show git action warning notification: {err:#}");
            }
        }
    }

    fn next_git_action_epoch(&mut self) -> usize {
        self.git_action_epoch = self.git_action_epoch.saturating_add(1);
        self.git_action_epoch
    }

    fn run_git_action<F>(&mut self, action_name: &'static str, cx: &mut Context<Self>, action: F)
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
                            error!("{action_name} failed: {err:#}");
                            let summary = err.to_string();
                            this.git_status_message = Some(format!("JJ error: {err:#}"));
                            Self::push_error_notification(
                                format!("{action_name} failed: {summary}"),
                                cx,
                            );
                        }
                    }

                    cx.notify();
                })
                .ok();
            }
        });
    }

    fn checkout_or_create_branch_with_options(
        &mut self,
        branch_name: String,
        move_changes_to_new_bookmark: bool,
        cx: &mut Context<Self>,
    ) {
        self.run_git_action("Switch branch", cx, move |repo_root| {
            checkout_or_create_branch_with_change_transfer(
                &repo_root,
                &branch_name,
                move_changes_to_new_bookmark,
            )?;
            if move_changes_to_new_bookmark {
                Ok(format!("Switched to {} and moved changes", branch_name))
            } else {
                Ok(format!("Switched to {}", branch_name))
            }
        });
    }

    pub(super) fn checkout_branch(&mut self, branch_name: String, cx: &mut Context<Self>) {
        self.checkout_or_create_branch_with_options(branch_name, false, cx);
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

    pub(super) fn branch_syncable(&self) -> bool {
        !self.branch_name.is_empty()
            && self.branch_name != "unknown"
            && !self.branch_name.starts_with("detached")
    }

    fn tracking_area_clean(&self) -> bool {
        self.files.is_empty()
    }

    pub(super) fn can_sync_current_branch(&self) -> bool {
        self.branch_syncable()
            && self.branch_has_upstream
            && self.tracking_area_clean()
            && !self.git_action_loading
    }

    pub(super) fn can_push_or_publish_current_branch(&self) -> bool {
        if !self.branch_syncable() || !self.tracking_area_clean() || self.git_action_loading {
            return false;
        }

        if self.branch_has_upstream {
            self.branch_ahead_count > 0
        } else {
            true
        }
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

        let creating_new_bookmark = !self.branches.iter().any(|branch| branch.name == sanitized);
        let has_local_changes = !self.files.is_empty();

        if creating_new_bookmark && has_local_changes {
            let tracked_count = self.files.iter().filter(|file| !file.untracked).count();
            let untracked_count = self.files.iter().filter(|file| file.untracked).count();
            let change_summary = match (tracked_count, untracked_count) {
                (tracked, 0) => format!("{tracked} tracked change{}", if tracked == 1 { "" } else { "s" }),
                (0, untracked) => format!(
                    "{untracked} untracked file{}",
                    if untracked == 1 { "" } else { "s" }
                ),
                (tracked, untracked) => format!(
                    "{tracked} tracked change{} and {untracked} untracked file{}",
                    if tracked == 1 { "" } else { "s" },
                    if untracked == 1 { "" } else { "s" }
                ),
            };

            let view = cx.entity();
            let move_branch_name = sanitized.clone();
            let keep_branch_name = sanitized.clone();
            let dialog_branch_name = sanitized.clone();

            gpui_component::WindowExt::open_dialog(window, cx, move |dialog, _, _| {
                let view_for_move = view.clone();
                let view_for_keep = view.clone();
                let move_branch_name_for_ok = move_branch_name.clone();
                let keep_branch_name_for_cancel = keep_branch_name.clone();
                dialog
                    .confirm()
                    .title("Move local changes to new bookmark?")
                    .button_props(
                        gpui_component::dialog::DialogButtonProps::default()
                            .ok_text("Move changes")
                            .cancel_text("Keep on current"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child(format!(
                                "Detected {change_summary}. Move them to '{dialog_branch_name}'?"
                            )),
                    )
                    .on_ok(move |_, _, cx| {
                        view_for_move.update(cx, |this, cx| {
                            this.checkout_or_create_branch_with_options(
                                move_branch_name_for_ok.clone(),
                                true,
                                cx,
                            );
                        });
                        true
                    })
                    .on_cancel(move |_, _, cx| {
                        view_for_keep.update(cx, |this, cx| {
                            this.checkout_or_create_branch_with_options(
                                keep_branch_name_for_cancel.clone(),
                                false,
                                cx,
                            );
                        });
                        true
                    })
            });
            return;
        }

        self.checkout_or_create_branch_with_options(sanitized, false, cx);
    }

    pub(super) fn push_or_publish_current_branch(&mut self, cx: &mut Context<Self>) {
        if !self.branch_syncable() {
            let message = "Cannot push a detached or unknown branch.".to_string();
            self.git_status_message = Some(message.clone());
            Self::push_warning_notification(message, cx);
            cx.notify();
            return;
        }
        if !self.tracking_area_clean() {
            let message = "Commit or discard tracked changes before pushing.".to_string();
            self.git_status_message = Some(message.clone());
            Self::push_warning_notification(message, cx);
            cx.notify();
            return;
        }
        if self.branch_has_upstream && self.branch_ahead_count == 0 {
            let message = "Nothing to push.".to_string();
            self.git_status_message = Some(message.clone());
            Self::push_warning_notification(message, cx);
            cx.notify();
            return;
        }
        if self.git_action_loading {
            return;
        }

        let branch_name = self.branch_name.clone();
        let has_upstream = self.branch_has_upstream;

        let action_name = if has_upstream {
            "Push branch"
        } else {
            "Publish branch"
        };
        self.run_git_action(action_name, cx, move |repo_root| {
            push_current_branch(&repo_root, &branch_name, has_upstream)?;
            if has_upstream {
                Ok(format!("Pushed {}", branch_name))
            } else {
                Ok(format!("Published {}", branch_name))
            }
        });
    }

    pub(super) fn sync_current_branch_from_remote(&mut self, cx: &mut Context<Self>) {
        if !self.branch_syncable() {
            let message = "Cannot sync a detached or unknown branch.".to_string();
            self.git_status_message = Some(message.clone());
            Self::push_warning_notification(message, cx);
            cx.notify();
            return;
        }
        if !self.branch_has_upstream {
            let message = "No upstream bookmark to sync from.".to_string();
            self.git_status_message = Some(message.clone());
            Self::push_warning_notification(message, cx);
            cx.notify();
            return;
        }
        if !self.tracking_area_clean() {
            let message = "Commit or discard tracked changes before syncing.".to_string();
            self.git_status_message = Some(message.clone());
            Self::push_warning_notification(message, cx);
            cx.notify();
            return;
        }
        if self.git_action_loading {
            return;
        }

        let branch_name = self.branch_name.clone();

        self.run_git_action("Sync branch", cx, move |repo_root| {
            sync_current_branch(&repo_root, &branch_name)?;
            Ok(format!("Synced {}", branch_name))
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
                            error!("Commit failed: {err:#}");
                            this.git_status_message = Some(format!("JJ error: {err:#}"));
                            Self::push_error_notification(
                                format!("Commit failed: {}", err),
                                cx,
                            );
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
