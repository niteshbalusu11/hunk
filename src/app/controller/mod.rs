use anyhow::Context as _;
use tracing::{error, info};

use super::data::{
    DiffStreamRowKind, RepoTreeNodeKind, RightPaneMode, SidebarTreeMode, build_repo_tree,
    build_tree_items, decimal_digits, display_width, line_number_column_width, load_diff_stream,
    load_file_editor_document, message_row, save_file_editor_document,
};
use super::*;
use hunk::git::{
    RepoSnapshot, checkout_or_create_branch, commit_staged, load_repo_tree, load_snapshot,
    load_snapshot_fingerprint, push_current_branch, sanitize_branch_name, stage_all, stage_file,
    unstage_all, unstage_file,
};

include!("core.rs");
include!("git_ops.rs");
include!("file_tree.rs");
include!("editor.rs");
include!("selection.rs");
include!("scroll.rs");
include!("fps.rs");
