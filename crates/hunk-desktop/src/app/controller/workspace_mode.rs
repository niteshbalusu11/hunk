impl DiffViewer {
    pub(super) fn toggle_jj_terms_glossary(&mut self, cx: &mut Context<Self>) {
        self.show_jj_terms_glossary = !self.show_jj_terms_glossary;
        cx.notify();
    }

    pub(super) fn pending_bookmark_switch(&self) -> Option<&PendingBookmarkSwitch> {
        self.pending_bookmark_switch.as_ref()
    }

    pub(super) fn pending_workspace_switch(&self) -> Option<&PendingWorkspaceSwitch> {
        self.pending_workspace_switch.as_ref()
    }

    pub(super) fn pending_workspace_forget(&self) -> Option<&PendingWorkspaceForget> {
        self.pending_workspace_forget.as_ref()
    }

    pub(super) fn request_activate_or_create_bookmark_with_dirty_guard(
        &mut self,
        bookmark_name: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(reason) = self.task_workspace_bookmark_mutation_blocker() {
            let reason = reason.to_string();
            self.git_status_message = Some(reason.clone());
            Self::push_warning_notification(reason, cx);
            cx.notify();
            return;
        }
        let target_bookmark = bookmark_name.trim().to_string();
        if target_bookmark.is_empty() {
            self.git_status_message = Some("Bookmark name is required.".to_string());
            cx.notify();
            return;
        }
        if self.git_action_loading {
            self.git_status_message = Some("Wait for the current workspace action to finish.".to_string());
            cx.notify();
            return;
        }
        self.pending_workspace_switch = None;
        self.pending_workspace_forget = None;
        self.graph_right_panel_mode = GraphRightPanelMode::ActiveWorkflow;

        let source_bookmark = self
            .checked_out_bookmark_name()
            .unwrap_or(self.branch_name.as_str())
            .to_string();
        let same_bookmark = source_bookmark == target_bookmark;
        if same_bookmark {
            self.pending_bookmark_switch = None;
            self.git_status_message =
                Some(format!("Bookmark {} is already active.", target_bookmark));
            cx.notify();
            return;
        }

        if !self.files.is_empty() {
            self.pending_bookmark_switch = Some(PendingBookmarkSwitch {
                source_bookmark: source_bookmark.clone(),
                target_bookmark: target_bookmark.clone(),
                changed_file_count: self.files.len(),
                unix_time: Self::now_unix_seconds(),
            });
            self.graph_right_panel_mode = GraphRightPanelMode::ActiveWorkflow;
            self.branch_picker_open = false;
            self.git_status_message = Some(format!(
                "Switching {} -> {} with {} local files. Choose move or snapshot before switching.",
                source_bookmark,
                target_bookmark,
                self.files.len()
            ));
            cx.notify();
            return;
        }

        self.pending_bookmark_switch = None;
        self.activate_or_create_bookmark(target_bookmark, false, cx);
    }

    pub(super) fn confirm_pending_bookmark_switch_move_changes(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_bookmark_switch.take() else {
            self.git_status_message = Some("No pending bookmark switch to confirm.".to_string());
            cx.notify();
            return;
        };
        self.graph_right_panel_mode = GraphRightPanelMode::ActiveWorkflow;
        self.activate_or_create_bookmark(pending.target_bookmark, true, cx);
    }

    pub(super) fn confirm_pending_bookmark_switch_snapshot(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_bookmark_switch.take() else {
            self.git_status_message = Some("No pending bookmark switch to confirm.".to_string());
            cx.notify();
            return;
        };
        self.graph_right_panel_mode = GraphRightPanelMode::ActiveWorkflow;
        self.activate_or_create_bookmark(pending.target_bookmark, false, cx);
    }

    pub(super) fn cancel_pending_bookmark_switch(&mut self, cx: &mut Context<Self>) {
        if self.pending_bookmark_switch.is_none() {
            return;
        }
        self.pending_bookmark_switch = None;
        self.git_status_message = Some("Canceled bookmark switch.".to_string());
        cx.notify();
    }

    pub(super) fn discard_latest_working_copy_recovery_candidate_for_active_bookmark(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(candidate) = self.latest_working_copy_recovery_candidate_for_active_bookmark() else {
            self.git_status_message =
                Some("No captured working-copy record to discard for this bookmark.".to_string());
            cx.notify();
            return;
        };

        let before_len = self.working_copy_recovery_candidates.len();
        self.working_copy_recovery_candidates
            .retain(|existing| existing.source_revision_id != candidate.source_revision_id);
        let removed = before_len.saturating_sub(self.working_copy_recovery_candidates.len());
        self.git_status_message = Some(format!(
            "Discarded {} captured working-copy record{}.",
            removed,
            if removed == 1 { "" } else { "s" }
        ));
        cx.notify();
    }

    pub(super) fn request_activate_selected_graph_bookmark(&mut self, cx: &mut Context<Self>) {
        let Some(bookmark_name) = self.selected_local_graph_bookmark_name() else {
            let message = "Select a local bookmark before activating it.".to_string();
            self.git_status_message = Some(message.clone());
            Self::push_warning_notification(message, cx);
            cx.notify();
            return;
        };

        self.request_activate_or_create_bookmark_with_dirty_guard(bookmark_name, cx);
    }

    pub(super) fn request_switch_selected_graph_workspace(&mut self, cx: &mut Context<Self>) {
        if let Some(reason) = self.selected_graph_workspace_switch_blocker() {
            self.git_status_message = Some(reason.clone());
            Self::push_warning_notification(reason, cx);
            cx.notify();
            return;
        }
        let Some(selected_workspace_name) = self
            .graph_selected_workspace_state()
            .map(|workspace| workspace.name.clone())
        else {
            let message = "Select a workspace chip before switching workspace.".to_string();
            self.git_status_message = Some(message.clone());
            Self::push_warning_notification(message, cx);
            cx.notify();
            return;
        };

        let source_workspace = self
            .graph_current_workspace_name
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        if source_workspace == selected_workspace_name {
            self.pending_workspace_switch = None;
            self.git_status_message = Some(format!(
                "Workspace {}@ is already active.",
                selected_workspace_name
            ));
            cx.notify();
            return;
        }

        let Some(repo_root) = self.repo_root.clone() else {
            self.git_status_message = Some("No JJ repository available.".to_string());
            cx.notify();
            return;
        };

        let epoch = self.begin_git_action("Switch workspace", cx);
        self.pending_bookmark_switch = None;
        self.pending_workspace_switch = None;
        self.pending_workspace_forget = None;
        self.graph_right_panel_mode = GraphRightPanelMode::ActiveWorkflow;

        self.git_action_task = cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move {
                resolve_workspace_switch_target(&repo_root, &selected_workspace_name)
            });
            let result = result.await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.git_action_epoch {
                        return;
                    }

                    this.finish_git_action();
                    match result {
                        Ok(target) => {
                            this.prepare_or_apply_workspace_switch(
                                source_workspace.clone(),
                                target,
                                cx,
                            );
                        }
                        Err(err) => {
                            error!("Switch workspace target resolution failed: {err:#}");
                            let summary = err.to_string();
                            this.git_status_message = Some(format!("JJ error: {err:#}"));
                            Self::push_error_notification(
                                format!("Switch workspace failed: {summary}"),
                                cx,
                            );
                        }
                    }

                    cx.notify();
                });
            }
        });
    }

    pub(super) fn confirm_pending_workspace_switch(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_workspace_switch.take() else {
            self.git_status_message = Some("No pending workspace switch to confirm.".to_string());
            cx.notify();
            return;
        };
        self.apply_workspace_switch_root(
            pending.target_workspace,
            pending.target_workspace_root,
            cx,
        );
    }

    pub(super) fn cancel_pending_workspace_switch(&mut self, cx: &mut Context<Self>) {
        if self.pending_workspace_switch.is_none() {
            return;
        }
        self.pending_workspace_switch = None;
        self.git_status_message = Some("Canceled workspace switch.".to_string());
        cx.notify();
    }

    pub(super) fn selected_graph_workspace_switch_blocker(&self) -> Option<String> {
        if self.git_action_loading {
            return Some("Another workspace action is in progress.".to_string());
        }
        if self.pending_workspace_switch.is_some() {
            return Some(
                "Confirm or cancel the pending workspace switch before starting another one."
                    .to_string(),
            );
        }
        if self.pending_workspace_forget.is_some() {
            return Some(
                "Confirm or cancel the pending workspace forget before starting another action."
                    .to_string(),
            );
        }
        let Some(selected_workspace) = self.graph_selected_workspace_state() else {
            return Some("Select a workspace chip in the graph first.".to_string());
        };
        if selected_workspace.is_current {
            return Some("Selected workspace is already active.".to_string());
        }
        None
    }

    pub(super) fn request_create_graph_workspace_at_selected_revision(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let workspace_name = self
            .graph_workspace_action_input_state
            .read(cx)
            .value()
            .trim()
            .to_string();
        if let Some(reason) = self.selected_graph_workspace_create_blocker(workspace_name.as_str()) {
            self.git_status_message = Some(reason.clone());
            Self::push_warning_notification(reason, cx);
            cx.notify();
            return;
        }

        let Some(base_revision_id) = self.active_bookmark_target_revision_id() else {
            let message =
                "Cannot resolve trunk bookmark target revision for task workspace creation."
                    .to_string();
            self.git_status_message = Some(message.clone());
            Self::push_warning_notification(message, cx);
            cx.notify();
            return;
        };
        let Some(repo_root) = self.repo_root.clone() else {
            self.git_status_message = Some("No JJ repository available.".to_string());
            cx.notify();
            return;
        };
        let destination_root =
            Self::workspace_create_destination_root(repo_root.as_path(), workspace_name.as_str());

        let epoch = self.begin_git_action("Create Task Workspace", cx);
        self.pending_workspace_switch = None;
        self.pending_workspace_forget = None;
        self.pending_bookmark_switch = None;
        self.graph_pending_confirmation = None;
        self.graph_right_panel_mode = GraphRightPanelMode::ActiveWorkflow;
        self.branch_picker_open = false;

        self.git_action_task = cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move {
                let created = create_workspace_at_revision(
                    &repo_root,
                    &workspace_name,
                    &base_revision_id,
                    destination_root.as_path(),
                )?;

                match checkout_or_create_bookmark_with_change_transfer(
                    &created.root,
                    &workspace_name,
                    false,
                ) {
                    Ok(()) => Ok(created),
                    Err(bookmark_err) => {
                        let rollback_result = forget_workspace(&repo_root, &workspace_name);
                        if let Err(rollback_err) = rollback_result {
                            return Err(bookmark_err).with_context(|| {
                                format!(
                                    "failed to create paired bookmark '{}' in workspace '{}' and rollback failed: {rollback_err:#}",
                                    workspace_name, workspace_name
                                )
                            });
                        }
                        Err(bookmark_err).with_context(|| {
                            format!(
                                "failed to create paired bookmark '{}' in workspace '{}'; partially created workspace was forgotten",
                                workspace_name, workspace_name
                            )
                        })
                    }
                }
            });
            let result = result.await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.git_action_epoch {
                        return;
                    }

                    this.finish_git_action();
                    match result {
                        Ok(created) => {
                            let workspace_name = created.name.clone();
                            let workspace_root = created.root.clone();
                            this.apply_workspace_switch_root(
                                workspace_name.clone(),
                                workspace_root,
                                cx,
                            );
                            this.git_status_message = Some(format!(
                                "Created task workspace {}@ with paired bookmark {}@ and switched context.",
                                workspace_name, workspace_name
                            ));
                        }
                        Err(err) => {
                            error!("Create workspace failed: {err:#}");
                            let summary = err.to_string();
                            this.git_status_message = Some(format!("JJ error: {err:#}"));
                            Self::push_error_notification(
                                format!("Create workspace failed: {summary}"),
                                cx,
                            );
                        }
                    }
                    cx.notify();
                });
            }
        });
    }

    pub(super) fn request_forget_selected_graph_workspace(&mut self, cx: &mut Context<Self>) {
        if let Some(reason) = self.selected_graph_workspace_forget_blocker() {
            self.git_status_message = Some(reason.clone());
            Self::push_warning_notification(reason, cx);
            cx.notify();
            return;
        }

        let Some(selected_workspace) = self.graph_selected_workspace_state().cloned() else {
            let message = "Select a workspace chip before forgetting workspace.".to_string();
            self.git_status_message = Some(message.clone());
            Self::push_warning_notification(message, cx);
            cx.notify();
            return;
        };

        self.pending_workspace_switch = None;
        self.pending_workspace_forget = Some(PendingWorkspaceForget {
            workspace_name: selected_workspace.name.clone(),
            workspace_commit_id: selected_workspace.commit_id,
            unix_time: Self::now_unix_seconds(),
        });
        self.branch_picker_open = true;
        self.git_status_message = Some(format!(
            "Forget workspace {}@ from repository metadata? Confirm to continue.",
            selected_workspace.name
        ));
        cx.notify();
    }

    pub(super) fn confirm_pending_workspace_forget(&mut self, cx: &mut Context<Self>) {
        if self.git_action_loading {
            self.git_status_message = Some("Wait for the current workspace action to finish.".to_string());
            cx.notify();
            return;
        }
        let Some(pending) = self.pending_workspace_forget.take() else {
            self.git_status_message = Some("No pending workspace forget to confirm.".to_string());
            cx.notify();
            return;
        };
        let Some(repo_root) = self.repo_root.clone() else {
            self.git_status_message = Some("No JJ repository available.".to_string());
            cx.notify();
            return;
        };
        let workspace_name = pending.workspace_name.clone();
        let workspace_name_for_task = workspace_name.clone();

        let epoch = self.begin_git_action("Forget workspace", cx);
        self.pending_workspace_switch = None;
        self.pending_bookmark_switch = None;
        self.graph_pending_confirmation = None;
        self.graph_right_panel_mode = GraphRightPanelMode::ActiveWorkflow;
        self.branch_picker_open = false;

        self.git_action_task = cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { forget_workspace(&repo_root, &workspace_name_for_task) });
            let result = result.await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.git_action_epoch {
                        return;
                    }

                    this.finish_git_action();
                    match result {
                        Ok(()) => {
                            this.graph_selected_workspace = None;
                            this.git_status_message =
                                Some(format!("Forgot workspace {}@.", workspace_name));
                            this.request_snapshot_refresh_internal(true, cx);
                        }
                        Err(err) => {
                            error!("Forget workspace failed: {err:#}");
                            let summary = err.to_string();
                            this.git_status_message = Some(format!("JJ error: {err:#}"));
                            Self::push_error_notification(
                                format!("Forget workspace failed: {summary}"),
                                cx,
                            );
                        }
                    }
                    cx.notify();
                });
            }
        });
    }

    pub(super) fn cancel_pending_workspace_forget(&mut self, cx: &mut Context<Self>) {
        if self.pending_workspace_forget.is_none() {
            return;
        }
        self.pending_workspace_forget = None;
        self.git_status_message = Some("Canceled workspace forget.".to_string());
        cx.notify();
    }

    pub(super) fn selected_graph_workspace_create_blocker(
        &self,
        workspace_name_input: &str,
    ) -> Option<String> {
        if self.git_action_loading {
            return Some("Another workspace action is in progress.".to_string());
        }
        if self.pending_workspace_switch.is_some() {
            return Some(
                "Confirm or cancel the pending workspace switch before creating a workspace."
                    .to_string(),
            );
        }
        if self.pending_workspace_forget.is_some() {
            return Some(
                "Confirm or cancel the pending workspace forget before creating a workspace."
                    .to_string(),
            );
        }
        if let Some(reason) = Self::task_workspace_create_context_blocker(
            self.graph_current_workspace_name.as_deref(),
            self.graph_active_bookmark.as_deref(),
            self.files.len(),
        ) {
            return Some(reason.to_string());
        }
        if self.active_bookmark_target_revision_id().is_none() {
            return Some(
                "Cannot resolve active trunk bookmark target revision from graph state."
                    .to_string(),
            );
        }
        let workspace_name = workspace_name_input.trim();
        if workspace_name.is_empty() {
            return Some("Enter a workspace name before creating it.".to_string());
        }
        if let Some(reason) = Self::workspace_name_validation_error(workspace_name) {
            return Some(reason);
        }
        if !is_valid_bookmark_name(workspace_name) {
            return Some(
                "Task workspace name must also be a valid bookmark name for pairing."
                    .to_string(),
            );
        }
        if self
            .graph_workspaces
            .iter()
            .any(|workspace| workspace.name == workspace_name)
        {
            return Some(format!("Workspace {}@ already exists.", workspace_name));
        }
        if self.repo_root.is_none() {
            return Some("No JJ repository available.".to_string());
        }
        None
    }

    pub(super) fn selected_graph_workspace_forget_blocker(&self) -> Option<String> {
        if self.git_action_loading {
            return Some("Another workspace action is in progress.".to_string());
        }
        if self.pending_workspace_switch.is_some() {
            return Some(
                "Confirm or cancel the pending workspace switch before forgetting a workspace."
                    .to_string(),
            );
        }
        if self.pending_workspace_forget.is_some() {
            return Some(
                "Confirm or cancel the pending workspace forget before starting another one."
                    .to_string(),
            );
        }
        let Some(selected_workspace) = self.graph_selected_workspace_state() else {
            return Some("Select a workspace chip in the graph first.".to_string());
        };
        if selected_workspace.is_current {
            return Some(
                "Current workspace cannot be forgotten from itself. Switch to another workspace first."
                    .to_string(),
            );
        }
        None
    }

    pub(super) fn active_review_action_blocker(&self) -> Option<String> {
        if self.git_action_loading {
            return Some("Another workspace action is in progress.".to_string());
        }
        if !self.can_run_active_bookmark_actions() {
            return Some("Activate a bookmark before opening PR/MR.".to_string());
        }
        if !self.branch_has_upstream {
            return Some("Publish this bookmark before opening PR/MR.".to_string());
        }
        None
    }

    pub(super) fn selected_graph_review_action_blocker(&self) -> Option<String> {
        if self.git_action_loading {
            return Some("Another workspace action is in progress.".to_string());
        }
        let Some(bookmark) = self.graph_selected_bookmark_ref() else {
            return Some("Select a bookmark in the graph first.".to_string());
        };
        if bookmark.scope != GraphBookmarkScope::Local {
            return Some("Select a local bookmark to open PR/MR.".to_string());
        }
        if bookmark.conflicted {
            return Some("Resolve bookmark conflicts before opening PR/MR.".to_string());
        }
        if !bookmark.tracked {
            return Some("Publish this bookmark before opening PR/MR.".to_string());
        }
        None
    }

    fn prepare_or_apply_workspace_switch(
        &mut self,
        source_workspace: String,
        target: WorkspaceSwitchTarget,
        cx: &mut Context<Self>,
    ) {
        if Self::requires_workspace_switch_confirmation(self.files.len()) {
            self.pending_workspace_switch = Some(PendingWorkspaceSwitch {
                source_workspace: source_workspace.clone(),
                target_workspace: target.name.clone(),
                target_workspace_root: target.root,
                changed_file_count: self.files.len(),
                unix_time: Self::now_unix_seconds(),
            });
            self.branch_picker_open = true;
            self.git_status_message = Some(Self::workspace_switch_confirmation_message(
                source_workspace.as_str(),
                target.name.as_str(),
                self.files.len(),
            ));
            return;
        }

        self.apply_workspace_switch_root(target.name, target.root, cx);
    }

    fn apply_workspace_switch_root(
        &mut self,
        target_workspace: String,
        target_workspace_root: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.pending_bookmark_switch = None;
        self.pending_workspace_switch = None;
        self.pending_workspace_forget = None;
        self.graph_pending_confirmation = None;
        self.graph_right_panel_mode = GraphRightPanelMode::ActiveWorkflow;
        self.branch_picker_open = false;
        self.project_path = Some(target_workspace_root.clone());
        self.set_last_project_path(Some(target_workspace_root.clone()));
        self.repo_root = Some(target_workspace_root.clone());
        self.workspace_execution_context = None;
        self.start_repo_watch(cx);
        self.request_snapshot_refresh_internal(true, cx);
        self.git_status_message = Some(format!(
            "Switched workspace to {}@ ({})",
            target_workspace,
            target_workspace_root.display()
        ));
    }

    fn workspace_create_destination_root(repo_root: &Path, workspace_name: &str) -> PathBuf {
        let workspace_name = workspace_name.trim();
        if let Some(internal_workspaces_dir) = Self::workspace_internal_workspaces_dir(repo_root) {
            return internal_workspaces_dir.join(workspace_name);
        }

        repo_root.join(".jj").join("workspaces").join(workspace_name)
    }

    fn workspace_internal_workspaces_dir(repo_root: &Path) -> Option<PathBuf> {
        let parent = repo_root.parent()?;
        if parent.file_name() != Some(std::ffi::OsStr::new("workspaces")) {
            return None;
        }
        if parent
            .parent()
            .and_then(Path::file_name)
            != Some(std::ffi::OsStr::new(".jj"))
        {
            return None;
        }
        Some(parent.to_path_buf())
    }

    fn active_bookmark_target_revision_id(&self) -> Option<String> {
        self.graph_nodes
            .iter()
            .find(|node| node.is_active_bookmark_target)
            .map(|node| node.id.clone())
    }

    fn is_trunk_bookmark_name(bookmark_name: &str) -> bool {
        matches!(bookmark_name, "main" | "master")
    }

    fn task_workspace_bookmark_mutation_blocker_for_workspace(
        current_workspace_name: Option<&str>,
    ) -> Option<&'static str> {
        match current_workspace_name {
            Some("default") | None => None,
            Some(_) => Some(
                "Bookmark mutations are disabled in task workspace mode. Switch to default workspace.",
            ),
        }
    }

    pub(super) fn task_workspace_bookmark_mutation_blocker(&self) -> Option<&'static str> {
        Self::task_workspace_bookmark_mutation_blocker_for_workspace(
            self.graph_current_workspace_name.as_deref(),
        )
    }

    fn task_workspace_create_context_blocker(
        current_workspace_name: Option<&str>,
        active_bookmark: Option<&str>,
        changed_file_count: usize,
    ) -> Option<&'static str> {
        if current_workspace_name != Some("default") {
            return Some(
                "Task workspace creation is only available from the default workspace.",
            );
        }
        let Some(active_bookmark) = active_bookmark else {
            return Some("Activate trunk bookmark (main/master) before creating a task workspace.");
        };
        if !Self::is_trunk_bookmark_name(active_bookmark) {
            return Some(
                "Task workspace creation is only available from trunk bookmark (main/master).",
            );
        }
        if changed_file_count > 0 {
            return Some(
                "Commit, restore, or discard local changes before creating a task workspace.",
            );
        }
        None
    }

    fn workspace_name_validation_error(workspace_name: &str) -> Option<String> {
        if workspace_name == "." || workspace_name == ".." {
            return Some("Workspace name cannot be '.' or '..'.".to_string());
        }
        if workspace_name.contains(std::path::is_separator) || workspace_name.contains('\\') {
            return Some(
                "Workspace name cannot contain path separators; use a plain name.".to_string(),
            );
        }
        None
    }

    fn requires_workspace_switch_confirmation(changed_file_count: usize) -> bool {
        changed_file_count > 0
    }

    fn workspace_switch_confirmation_message(
        source_workspace: &str,
        target_workspace: &str,
        changed_file_count: usize,
    ) -> String {
        format!(
            "Switch {}@ -> {}@ with {} local files. Confirm to keep local changes in {}@ before opening {}@.",
            source_workspace,
            target_workspace,
            changed_file_count,
            source_workspace,
            target_workspace
        )
    }
}

#[cfg(test)]
mod workspace_mode_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn workspace_switch_confirmation_required_when_dirty_files_present() {
        assert!(DiffViewer::requires_workspace_switch_confirmation(1));
        assert!(!DiffViewer::requires_workspace_switch_confirmation(0));
    }

    #[test]
    fn workspace_switch_confirmation_message_mentions_source_target_and_count() {
        let message = DiffViewer::workspace_switch_confirmation_message("default", "ws2", 3);
        assert!(message.contains("default@"));
        assert!(message.contains("ws2@"));
        assert!(message.contains("3"));
    }

    #[test]
    fn workspace_create_destination_uses_internal_jj_workspaces_directory() {
        let repo_root = Path::new("/tmp/repo-default");
        let destination =
            DiffViewer::workspace_create_destination_root(repo_root, "feature-workspace");
        assert_eq!(
            destination,
            Path::new("/tmp/repo-default/.jj/workspaces/feature-workspace")
        );
    }

    #[test]
    fn workspace_create_destination_reuses_internal_workspaces_parent_when_already_inside_it() {
        let repo_root = Path::new("/tmp/repo-default/.jj/workspaces/ws-a");
        let destination = DiffViewer::workspace_create_destination_root(repo_root, "ws-b");
        assert_eq!(destination, Path::new("/tmp/repo-default/.jj/workspaces/ws-b"));
    }

    #[test]
    fn workspace_name_validation_rejects_path_unsafe_inputs() {
        assert!(DiffViewer::workspace_name_validation_error(".").is_some());
        assert!(DiffViewer::workspace_name_validation_error("..").is_some());
        assert!(DiffViewer::workspace_name_validation_error("foo/bar").is_some());
        assert!(DiffViewer::workspace_name_validation_error("foo\\bar").is_some());
        assert!(DiffViewer::workspace_name_validation_error("ws2").is_none());
    }

    #[test]
    fn task_workspace_name_must_be_valid_bookmark_name() {
        assert!(!is_valid_bookmark_name("task workspace"));
        assert!(is_valid_bookmark_name("task-workspace"));
    }

    #[test]
    fn task_workspace_create_context_requires_default_workspace() {
        let reason = DiffViewer::task_workspace_create_context_blocker(Some("ws2"), Some("main"), 0);
        assert!(reason.is_some());
    }

    #[test]
    fn task_workspace_create_context_requires_trunk_bookmark() {
        let reason =
            DiffViewer::task_workspace_create_context_blocker(Some("default"), Some("feature"), 0);
        assert!(reason.is_some());
    }

    #[test]
    fn task_workspace_create_context_requires_clean_working_copy() {
        let reason = DiffViewer::task_workspace_create_context_blocker(
            Some("default"),
            Some("main"),
            1,
        );
        assert!(reason.is_some());
    }

    #[test]
    fn task_workspace_create_context_allows_default_on_trunk_when_clean() {
        let reason =
            DiffViewer::task_workspace_create_context_blocker(Some("default"), Some("master"), 0);
        assert!(reason.is_none());
    }

    #[test]
    fn task_workspace_bookmark_mutation_blocked_outside_default_workspace() {
        let reason = DiffViewer::task_workspace_bookmark_mutation_blocker_for_workspace(Some("ws2"));
        assert!(reason.is_some());
    }

    #[test]
    fn task_workspace_bookmark_mutation_allowed_in_default_workspace() {
        let reason =
            DiffViewer::task_workspace_bookmark_mutation_blocker_for_workspace(Some("default"));
        assert!(reason.is_none());
    }
}
