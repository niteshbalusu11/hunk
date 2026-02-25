use anyhow::Context as _;
use tracing::{error, info};

use super::data::{
    DiffStreamRowKind, build_tree_items, decimal_digits, display_width, line_number_column_width,
    load_diff_stream, message_row,
};
use super::*;
use hunk::git::{
    RepoSnapshot, checkout_or_create_branch, commit_staged, load_snapshot, push_current_branch,
    sanitize_branch_name, stage_all, stage_file, unstage_all, unstage_file,
};

include!("core.rs");
include!("git_ops.rs");
include!("selection.rs");
include!("scroll.rs");
include!("fps.rs");
