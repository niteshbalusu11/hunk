use anyhow::Context as _;
use tracing::{error, info};

use super::data::{
    DiffSegmentQuality, DiffStream, DiffStreamRowKind, RepoTreeNodeKind, RightPaneMode,
    SidebarTreeMode, base_segment_quality_for_file, build_diff_row_segment_cache,
    build_diff_stream_from_patch_map, build_repo_tree, build_tree_items, decimal_digits,
    effective_segment_quality, flatten_repo_tree_rows, line_number_column_width,
    load_file_editor_document, message_row, save_file_editor_document,
};
use super::*;
use hunk::jj::{
    RepoSnapshot, checkout_or_create_branch, commit_selected_paths, commit_staged,
    load_patches_for_files, load_repo_tree, load_snapshot, load_snapshot_fingerprint,
    push_current_branch, sanitize_branch_name, sync_current_branch,
};

include!("core.rs");
include!("git_ops.rs");
include!("file_tree.rs");
include!("editor.rs");
include!("selection.rs");
include!("scroll.rs");
include!("fps.rs");
include!("settings.rs");
