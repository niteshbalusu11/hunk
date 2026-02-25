use anyhow::Context as _;
use tracing::{error, info};

use super::data::{
    DiffStreamRowKind, build_tree_items, decimal_digits, display_width, line_number_column_width,
    load_diff_stream, message_row,
};
use super::*;
use hunk::git::{RepoSnapshot, load_snapshot};

include!("core.rs");
include!("selection.rs");
include!("scroll.rs");
include!("fps.rs");
