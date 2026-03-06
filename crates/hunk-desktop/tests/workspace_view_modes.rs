#[path = "../src/app/workspace_view.rs"]
mod workspace_view;

use workspace_view::{WorkspaceSwitchAction, WorkspaceViewMode};

#[test]
fn mode_switching_keeps_existing_tabs_and_adds_ai_as_fourth_tab() {
    let tabs = [
        WorkspaceViewMode::Files,
        WorkspaceViewMode::Diff,
        WorkspaceViewMode::JjWorkspace,
        WorkspaceViewMode::Ai,
    ];
    assert_eq!(tabs[0], WorkspaceViewMode::Files);
    assert_eq!(tabs[1], WorkspaceViewMode::Diff);
    assert_eq!(tabs[2], WorkspaceViewMode::JjWorkspace);
    assert_eq!(tabs[3], WorkspaceViewMode::Ai);
}

#[test]
fn ai_controller_switch_action_targets_ai_mode() {
    assert_eq!(
        WorkspaceSwitchAction::Ai.target_mode(),
        WorkspaceViewMode::Ai
    );
    assert_eq!(
        WorkspaceSwitchAction::Files.target_mode(),
        WorkspaceViewMode::Files
    );
    assert_eq!(
        WorkspaceSwitchAction::Review.target_mode(),
        WorkspaceViewMode::Diff
    );
    assert_eq!(
        WorkspaceSwitchAction::Git.target_mode(),
        WorkspaceViewMode::JjWorkspace
    );
}

#[test]
fn ai_mode_does_not_enable_sidebar_or_diff_stream() {
    assert!(!WorkspaceViewMode::Ai.supports_sidebar_tree());
    assert!(!WorkspaceViewMode::Ai.supports_diff_stream());
    assert!(WorkspaceViewMode::Files.supports_sidebar_tree());
    assert!(WorkspaceViewMode::Files.supports_diff_stream());
    assert!(WorkspaceViewMode::Diff.supports_sidebar_tree());
    assert!(WorkspaceViewMode::Diff.supports_diff_stream());
    assert!(!WorkspaceViewMode::JjWorkspace.supports_sidebar_tree());
    assert!(!WorkspaceViewMode::JjWorkspace.supports_diff_stream());
}
