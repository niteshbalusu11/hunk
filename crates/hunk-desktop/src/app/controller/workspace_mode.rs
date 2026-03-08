impl DiffViewer {
    pub(super) fn request_activate_or_create_branch_with_dirty_guard(
        &mut self,
        branch_name: String,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) -> bool {
        let target_branch = branch_name.trim().to_string();
        let source_branch = self
            .checked_out_branch_name()
            .unwrap_or(self.branch_name.as_str())
            .to_string();

        if let Some(message) = branch_activation::branch_activation_block_message(
            target_branch.as_str(),
            source_branch.as_str(),
            self.git_action_loading,
            self.files.len(),
        ) {
            self.set_git_warning_message(message, window, cx);
            self.sync_branch_picker_state(cx);
            return false;
        }

        self.activate_or_create_branch(target_branch, cx)
    }

    pub(super) fn active_review_action_blocker(&self) -> Option<String> {
        if self.git_action_loading {
            return Some("Another workspace action is in progress.".to_string());
        }
        if !self.can_run_active_branch_actions() {
            return Some("Activate a branch before opening PR/MR.".to_string());
        }
        if !self.branch_has_upstream {
            return Some("Publish this branch before opening PR/MR.".to_string());
        }
        None
    }
}
