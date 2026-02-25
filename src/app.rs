use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use anyhow::Result;
use gpui::{
    AnyElement, App, AppContext as _, Application, ClipboardItem, Context, Entity, FocusHandle,
    InteractiveElement as _, IntoElement, IsZero as _, KeyBinding, ListAlignment, ListOffset,
    ListSizingBehavior, ListState, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement as _, Render, ScrollHandle, ScrollWheelEvent, SharedString,
    StatefulInteractiveElement as _, Styled as _, Task, Timer, Window, WindowOptions, actions, div,
    list, point, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Colorize as _, Root, StyledExt as _, Theme, ThemeMode, h_flex,
    input::InputState,
    resizable::{h_resizable, resizable_panel},
    scroll::ScrollableElement,
    tree::{TreeItem, TreeState},
    v_flex,
};
use tracing::error;

use hunk::config::{AppConfig, ConfigStore, DiffViewMode, ThemePreference};
use hunk::diff::{DiffCell, DiffCellKind, DiffRowKind, SideBySideRow};
use hunk::git::{ChangedFile, FileStatus, LineStats, LocalBranch};

use data::{DiffStreamRowMeta, FileRowRange};

const AUTO_REFRESH_INTERVAL: Duration = Duration::from_millis(900);
const FPS_SAMPLE_INTERVAL: Duration = Duration::from_millis(250);
const AUTO_REFRESH_SCROLL_DEBOUNCE: Duration = Duration::from_millis(500);
const DIFF_MIN_CONTENT_WIDTH: f32 = 960.0;
const DIFF_MIN_COLUMN_WIDTH: f32 = DIFF_MIN_CONTENT_WIDTH / 2.0;
const DIFF_MONO_CHAR_WIDTH: f32 = 8.0;
const DIFF_LINE_NUMBER_MIN_DIGITS: u32 = 3;
const DIFF_LINE_NUMBER_EXTRA_PADDING: f32 = 6.0;
const DIFF_MARKER_GUTTER_WIDTH: f32 = 10.0;
const DIFF_CELL_SIDE_PADDING_WIDTH: f32 = 20.0;
const DIFF_PAN_COLUMN_PADDING: f32 = 28.0;
const DIFF_BOTTOM_SAFE_INSET: f32 = 24.0;
const DIFF_SCROLLBAR_RIGHT_INSET: f32 = 2.0;
const DIFF_SCROLLBAR_SIZE: f32 = 16.0;
const DIFF_VERTICAL_SCROLLBAR_EXTRA_BOTTOM_INSET: f32 = 20.0;
const DIFF_FOOTER_SPACER_ROWS: usize = 2;

mod controller;
mod data;
mod highlight;
mod render;

actions!(
    diff_viewer,
    [
        SelectNextLine,
        SelectPreviousLine,
        ExtendSelectionNextLine,
        ExtendSelectionPreviousLine,
        CopySelection,
        SelectAllDiffRows,
        NextHunk,
        PreviousHunk,
        NextFile,
        PreviousFile,
    ]
);

fn apply_soft_light_theme(cx: &mut App) {
    let mut light_theme = (*Theme::global(cx).light_theme).clone();

    // Reduce eye strain in light mode by shifting from pure white to a soft off-white palette.
    light_theme.colors.background = Some("#f5f6f8".into());
    light_theme.colors.list = Some("#f5f6f8".into());
    light_theme.colors.popover = Some("#f5f6f8".into());
    light_theme.colors.table = Some("#f5f6f8".into());
    light_theme.colors.sidebar = Some("#f5f6f8".into());
    light_theme.colors.title_bar = Some("#f5f6f8".into());
    light_theme.colors.list_even = Some("#f1f2f5".into());
    light_theme.colors.list_head = Some("#eef0f4".into());
    light_theme.colors.secondary = Some("#eceef3".into());
    light_theme.colors.secondary_hover = Some("#e4e7ee".into());
    light_theme.colors.secondary_active = Some("#dce1ea".into());
    light_theme.colors.muted = Some("#e9ecf2".into());
    light_theme.colors.muted_foreground = Some("#616977".into());
    light_theme.colors.border = Some("#d2d8e3".into());

    Theme::global_mut(cx).light_theme = Rc::new(light_theme);

    if !Theme::global(cx).mode.is_dark() {
        Theme::change(ThemeMode::Light, None, cx);
    }
}

pub fn run() -> Result<()> {
    let app = Application::new();
    app.run(|cx| {
        gpui_component::init(cx);
        apply_soft_light_theme(cx);
        cx.bind_keys([
            KeyBinding::new("down", SelectNextLine, Some("DiffViewer")),
            KeyBinding::new("up", SelectPreviousLine, Some("DiffViewer")),
            KeyBinding::new("shift-down", ExtendSelectionNextLine, Some("DiffViewer")),
            KeyBinding::new("shift-up", ExtendSelectionPreviousLine, Some("DiffViewer")),
            KeyBinding::new("cmd-c", CopySelection, Some("DiffViewer")),
            KeyBinding::new("ctrl-c", CopySelection, Some("DiffViewer")),
            KeyBinding::new("cmd-a", SelectAllDiffRows, Some("DiffViewer")),
            KeyBinding::new("ctrl-a", SelectAllDiffRows, Some("DiffViewer")),
            KeyBinding::new("f7", NextHunk, Some("DiffViewer")),
            KeyBinding::new("shift-f7", PreviousHunk, Some("DiffViewer")),
            KeyBinding::new("alt-down", NextFile, Some("DiffViewer")),
            KeyBinding::new("alt-up", PreviousFile, Some("DiffViewer")),
        ]);

        if let Err(err) = cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(|cx| DiffViewer::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        }) {
            error!("failed to open window: {err:#}");
        }
    });

    Ok(())
}

struct DiffViewer {
    config_store: Option<ConfigStore>,
    config: AppConfig,
    repo_root: Option<PathBuf>,
    branch_name: String,
    branch_has_upstream: bool,
    branches: Vec<LocalBranch>,
    files: Vec<ChangedFile>,
    branch_picker_open: bool,
    branch_input_state: Entity<InputState>,
    commit_input_state: Entity<InputState>,
    last_commit_subject: Option<String>,
    git_action_epoch: usize,
    git_action_task: Task<()>,
    git_action_loading: bool,
    git_status_message: Option<String>,
    collapsed_files: BTreeSet<String>,
    selected_path: Option<String>,
    selected_status: Option<FileStatus>,
    diff_rows: Vec<SideBySideRow>,
    diff_row_metadata: Vec<DiffStreamRowMeta>,
    file_row_ranges: Vec<FileRowRange>,
    file_line_stats: BTreeMap<String, LineStats>,
    diff_list_state: ListState,
    diff_horizontal_scroll_handle: ScrollHandle,
    diff_fit_to_width: bool,
    diff_show_whitespace: bool,
    diff_show_eol_markers: bool,
    diff_left_column_width: f32,
    diff_right_column_width: f32,
    diff_pan_content_width: f32,
    diff_left_line_number_width: f32,
    diff_right_line_number_width: f32,
    overall_line_stats: LineStats,
    selected_line_stats: LineStats,
    refresh_epoch: usize,
    auto_refresh_task: Task<()>,
    snapshot_epoch: usize,
    snapshot_task: Task<()>,
    snapshot_loading: bool,
    patch_epoch: usize,
    patch_task: Task<()>,
    patch_loading: bool,
    focus_handle: FocusHandle,
    selection_anchor_row: Option<usize>,
    selection_head_row: Option<usize>,
    drag_selecting_rows: bool,
    scroll_selected_after_reload: bool,
    last_visible_row_start: Option<usize>,
    last_diff_scroll_offset: Option<gpui::Point<gpui::Pixels>>,
    last_scroll_activity_at: Instant,
    fps: f32,
    frame_sample_count: u32,
    frame_sample_started_at: Instant,
    fps_epoch: usize,
    fps_task: Task<()>,
    error_message: Option<String>,
    tree_state: Entity<TreeState>,
}
