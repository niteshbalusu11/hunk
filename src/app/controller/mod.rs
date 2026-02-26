use anyhow::Context as _;
use tracing::{error, info};

use super::data::{
    DiffStream, DiffStreamRowKind, RepoTreeNodeKind, RightPaneMode, SidebarTreeMode,
    build_diff_row_segment_cache, build_repo_tree, build_tree_items, decimal_digits,
    line_number_column_width, load_diff_stream, load_file_editor_document, message_row,
    save_file_editor_document, use_detailed_segments_for_file,
};
use super::*;
use hunk::jj::{
    RepoSnapshot, checkout_or_create_branch, commit_selected_paths, commit_staged, load_repo_tree,
    load_snapshot, load_snapshot_fingerprint, push_current_branch, sanitize_branch_name,
    sync_current_branch,
};

include!("core.rs");
include!("git_ops.rs");
include!("file_tree.rs");
include!("editor.rs");
include!("selection.rs");
include!("scroll.rs");
include!("fps.rs");
include!("settings.rs");
