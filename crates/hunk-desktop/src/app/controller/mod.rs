use anyhow::Context as _;
use futures::StreamExt;
use futures::channel::{mpsc, oneshot};
use notify::Watcher;
use tracing::{debug, error, warn};

use super::data::{
    DiffSegmentQuality, DiffStream, DiffStreamRowKind, RepoTreeNodeKind,
    base_segment_quality_for_file, build_changed_files_tree,
    build_diff_row_segment_cache_from_cells, build_diff_stream_from_patch_map, build_repo_tree,
    count_repo_tree_kind, decimal_digits, effective_segment_quality, flatten_repo_tree_rows,
    is_markdown_path, line_number_column_width, load_file_editor_document, message_row,
    save_file_editor_document,
};
use super::*;
use hunk_git::branch::{
    rename_branch, review_url_for_branch_with_provider_map, sanitize_branch_name,
};
use hunk_git::git::{
    WorkflowSnapshot, count_non_ignored_repo_tree_entries, invalidate_repo_metadata_caches,
    load_patches_for_files_from_session, load_repo_file_line_stats_for_paths_without_refresh,
    load_repo_file_line_stats_without_refresh, load_repo_tree, load_snapshot_fingerprint,
    load_workflow_snapshot_if_changed, load_workflow_snapshot_if_changed_without_refresh,
    load_workflow_snapshot_with_fingerprint,
    load_workflow_snapshot_with_fingerprint_without_refresh, open_patch_session,
};
use hunk_git::mutation::{
    activate_or_create_branch as checkout_or_create_branch_with_change_transfer,
    commit_all as commit_staged, commit_selected_paths, restore_working_copy_paths,
};
use hunk_git::network::{push_current_branch, sync_current_branch};

include!("core.rs");
include!("core_runtime.rs");
include!("git_ops.rs");
include!("workspace_mode.rs");
include!("ai.rs");
include!("file_tree.rs");
include!("editor.rs");
include!("comments.rs");
include!("comments_match.rs");
include!("selection.rs");
include!("scroll.rs");
include!("fps.rs");
include!("settings.rs");
