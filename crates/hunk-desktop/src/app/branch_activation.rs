pub(crate) fn branch_activation_preflight_message(
    target_branch: &str,
    source_branch: &str,
    workspace_action_loading: bool,
) -> Option<String> {
    let target_branch = target_branch.trim();
    if target_branch.is_empty() {
        return Some("Branch name is required.".to_string());
    }

    if workspace_action_loading {
        return Some("Wait for the current workspace action to finish.".to_string());
    }

    if source_branch == target_branch {
        return Some(format!("Branch {target_branch} is already active."));
    }

    None
}

pub(crate) fn branch_activation_block_message(
    target_branch: &str,
    source_branch: &str,
    workspace_action_loading: bool,
    dirty_file_count: usize,
) -> Option<String> {
    if let Some(message) =
        branch_activation_preflight_message(target_branch, source_branch, workspace_action_loading)
    {
        return Some(message);
    }

    if dirty_file_count > 0 {
        return Some(format!(
            "Commit or discard {dirty_file_count} local files before switching {source_branch} -> {target_branch}."
        ));
    }

    None
}
