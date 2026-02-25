use super::data::{
    DiffStreamRowKind, RepoTreeNodeKind, RightPaneMode, SidebarTreeMode, flatten_repo_tree_rows,
};
use super::highlight::{SyntaxTokenKind, build_line_segments, build_plain_line_segments};
use super::*;
use gpui_component::Disableable as _;
use gpui_component::animation::cubic_bezier;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::Input;
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::scroll::{Scrollbar, ScrollbarShow};

include!("toolbar.rs");
include!("tree.rs");
include!("commit.rs");
include!("file_banner.rs");
include!("file_status.rs");
include!("diff.rs");
include!("file_preview.rs");
include!("root.rs");
