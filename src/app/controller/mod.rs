use anyhow::Context as _;
use tracing::{error, info};

use super::data::{
    DiffSegmentQuality, DiffStream, DiffStreamRowKind, RepoTreeNodeKind, RightPaneMode,
    WorkspaceViewMode, base_segment_quality_for_file, build_diff_row_segment_cache,
    build_diff_stream_from_patch_map, build_repo_tree, decimal_digits, effective_segment_quality,
    flatten_repo_tree_rows, line_number_column_width, load_file_editor_document, message_row,
    save_file_editor_document,
};
use super::*;
use hunk::jj::{
    RepoSnapshot, abandon_bookmark_head, checkout_or_create_bookmark_with_change_transfer,
    commit_selected_paths, commit_staged, describe_bookmark_head, load_patches_for_files,
    load_repo_tree, load_snapshot, load_snapshot_fingerprint, push_current_bookmark,
    rename_bookmark, reorder_bookmark_tip_older, review_url_for_bookmark, sanitize_bookmark_name,
    squash_bookmark_head_into_parent, sync_current_bookmark,
};

include!("core.rs");
include!("git_ops.rs");
include!("file_tree.rs");
include!("editor.rs");
include!("selection.rs");
include!("scroll.rs");
include!("fps.rs");
include!("settings.rs");
