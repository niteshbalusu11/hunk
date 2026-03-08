#[path = "../src/app/branch_activation.rs"]
mod branch_activation;

use branch_activation::branch_activation_block_message;

#[test]
fn branch_activation_blocks_empty_branch_name() {
    assert_eq!(
        branch_activation_block_message("", "main", false, 0),
        Some("Branch name is required.".to_string())
    );
}

#[test]
fn branch_activation_blocks_while_another_git_action_is_running() {
    assert_eq!(
        branch_activation_block_message("feature/auth", "main", true, 0),
        Some("Wait for the current workspace action to finish.".to_string())
    );
}

#[test]
fn branch_activation_blocks_when_target_branch_is_already_active() {
    assert_eq!(
        branch_activation_block_message("main", "main", false, 0),
        Some("Branch main is already active.".to_string())
    );
}

#[test]
fn branch_activation_blocks_dirty_worktrees_with_source_target_context() {
    assert_eq!(
        branch_activation_block_message("feature/auth", "main", false, 3),
        Some("Commit or discard 3 local files before switching main -> feature/auth.".to_string())
    );
}

#[test]
fn branch_activation_allows_clean_switches() {
    assert_eq!(
        branch_activation_block_message("feature/auth", "main", false, 0),
        None
    );
}
