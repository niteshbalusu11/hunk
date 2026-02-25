use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use anyhow::Result;
use gpui::{
    Animation, AnimationExt as _, AnyElement, App, AppContext as _, Application, ClipboardItem,
    Context, Entity, FocusHandle, InteractiveElement as _, IntoElement, IsZero as _, KeyBinding,
    ListAlignment, ListOffset, ListSizingBehavior, ListState, Menu, MenuItem, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, OsAction, ParentElement as _, PathPromptOptions,
    Render, ScrollHandle, ScrollWheelEvent, SharedString, StatefulInteractiveElement as _,
    Styled as _, SystemMenuType, Task, Timer, TitlebarOptions, Window, WindowOptions, actions, div,
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
use gpui_component_assets::Assets;
use tracing::error;

use hunk::config::{AppConfig, ConfigStore, DiffViewMode, ThemePreference};
use hunk::diff::{DiffCell, DiffCellKind, DiffRowKind, SideBySideRow};
use hunk::git::{ChangedFile, FileStatus, LineStats, LocalBranch, RepoSnapshotFingerprint};
use hunk::state::{AppState, AppStateStore};

use data::{
    DiffRowSegmentCache, DiffStreamRowMeta, FileRowRange, RepoTreeNode, RightPaneMode,
    SidebarTreeMode,
};

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
const APP_BOTTOM_SAFE_INSET: f32 = 0.0;
const DIFF_BOTTOM_SAFE_INSET: f32 = APP_BOTTOM_SAFE_INSET;
const DIFF_SCROLLBAR_RIGHT_INSET: f32 = 0.0;
const DIFF_SCROLLBAR_SIZE: f32 = 16.0;
const FILE_EDITOR_MAX_BYTES: usize = 2_400_000;

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
        OpenProject,
        SaveCurrentFile,
        QuitApp,
    ]
);

fn preferred_ui_font_family() -> &'static str {
    if cfg!(target_os = "macos") {
        ".SystemUIFont"
    } else if cfg!(target_os = "windows") {
        "Segoe UI"
    } else {
        "Inter"
    }
}

fn preferred_mono_font_family() -> &'static str {
    if cfg!(target_os = "macos") {
        "Menlo"
    } else if cfg!(target_os = "windows") {
        "Consolas"
    } else {
        "DejaVu Sans Mono"
    }
}

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
    light_theme.font_family = Some(preferred_ui_font_family().into());
    light_theme.font_size = Some(14.0);
    light_theme.mono_font_family = Some(preferred_mono_font_family().into());
    light_theme.mono_font_size = Some(13.0);
    light_theme.radius = Some(8);
    light_theme.radius_lg = Some(10);
    light_theme.shadow = Some(false);

    Theme::global_mut(cx).light_theme = Rc::new(light_theme);

    if !Theme::global(cx).mode.is_dark() {
        Theme::change(ThemeMode::Light, None, cx);
    }
}

fn apply_soft_dark_theme(cx: &mut App) {
    let mut dark_theme = (*Theme::global(cx).dark_theme).clone();

    // Match a softer charcoal palette so colored diff cues stand out without eye strain.
    dark_theme.colors.background = Some("#1f2126".into());
    dark_theme.colors.list = Some("#1f2126".into());
    dark_theme.colors.popover = Some("#242831".into());
    dark_theme.colors.table = Some("#1f2126".into());
    dark_theme.colors.sidebar = Some("#1b1e24".into());
    dark_theme.colors.title_bar = Some("#1a1d22".into());
    dark_theme.colors.list_even = Some("#21242b".into());
    dark_theme.colors.list_head = Some("#292d36".into());
    dark_theme.colors.secondary = Some("#2a2f38".into());
    dark_theme.colors.secondary_hover = Some("#343b47".into());
    dark_theme.colors.secondary_active = Some("#3b4452".into());
    dark_theme.colors.muted = Some("#272c35".into());
    dark_theme.colors.muted_foreground = Some("#a3adbb".into());
    dark_theme.colors.border = Some("#3d4554".into());
    dark_theme.font_family = Some(preferred_ui_font_family().into());
    dark_theme.font_size = Some(14.0);
    dark_theme.mono_font_family = Some(preferred_mono_font_family().into());
    dark_theme.mono_font_size = Some(13.0);
    dark_theme.radius = Some(8);
    dark_theme.radius_lg = Some(10);
    dark_theme.shadow = Some(false);

    Theme::global_mut(cx).dark_theme = Rc::new(dark_theme);

    if Theme::global(cx).mode.is_dark() {
        Theme::change(ThemeMode::Dark, None, cx);
    }
}

pub fn run() -> Result<()> {
    let app = Application::new().with_assets(Assets);
    app.on_reopen(|cx| {
        if cx.windows().is_empty() {
            open_main_window(cx);
        }
        cx.activate(true);
    });

    app.run(|cx| {
        gpui_component::init(cx);
        apply_soft_light_theme(cx);
        apply_soft_dark_theme(cx);
        cx.on_action(quit_app);
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
            KeyBinding::new("cmd-shift-o", OpenProject, None),
            KeyBinding::new("ctrl-shift-o", OpenProject, None),
            KeyBinding::new("cmd-s", SaveCurrentFile, None),
            KeyBinding::new("ctrl-s", SaveCurrentFile, None),
            KeyBinding::new("cmd-q", QuitApp, None),
        ]);
        cx.set_menus(vec![
            Menu {
                name: "Hunk".into(),
                items: vec![
                    MenuItem::os_submenu("Services", SystemMenuType::Services),
                    MenuItem::separator(),
                    MenuItem::action("Quit Hunk", QuitApp),
                ],
            },
            Menu {
                name: "File".into(),
                items: vec![
                    MenuItem::action("Open Project...", OpenProject),
                    MenuItem::action("Save File", SaveCurrentFile),
                ],
            },
            Menu {
                name: "Edit".into(),
                items: vec![
                    MenuItem::os_action("Copy", CopySelection, OsAction::Copy),
                    MenuItem::os_action("Select All", SelectAllDiffRows, OsAction::SelectAll),
                ],
            },
        ]);
        cx.activate(true);
        open_main_window(cx);
    });

    Ok(())
}

fn open_main_window(cx: &mut App) {
    let window_options = WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some("Hunk".into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    if let Err(err) = cx.open_window(window_options, |window, cx| {
        let view = cx.new(|cx| DiffViewer::new(window, cx));
        cx.new(|cx| Root::new(view, window, cx))
    }) {
        error!("failed to open window: {err:#}");
    }
}

fn quit_app(_: &QuitApp, cx: &mut App) {
    cx.quit();
}

struct DiffViewer {
    config_store: Option<ConfigStore>,
    config: AppConfig,
    state_store: Option<AppStateStore>,
    state: AppState,
    project_path: Option<PathBuf>,
    repo_root: Option<PathBuf>,
    branch_name: String,
    branch_has_upstream: bool,
    branch_ahead_count: usize,
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
    diff_row_segment_cache: BTreeMap<u64, DiffRowSegmentCache>,
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
    refresh_epoch: usize,
    auto_refresh_task: Task<()>,
    snapshot_epoch: usize,
    snapshot_task: Task<()>,
    snapshot_loading: bool,
    last_snapshot_fingerprint: Option<RepoSnapshotFingerprint>,
    open_project_task: Task<()>,
    patch_epoch: usize,
    patch_task: Task<()>,
    patch_loading: bool,
    focus_handle: FocusHandle,
    selection_anchor_row: Option<usize>,
    selection_head_row: Option<usize>,
    drag_selecting_rows: bool,
    horizontal_pan_dragging: bool,
    horizontal_pan_last_x: Option<gpui::Pixels>,
    scroll_selected_after_reload: bool,
    last_visible_row_start: Option<usize>,
    last_diff_scroll_offset: Option<gpui::Point<gpui::Pixels>>,
    last_scroll_activity_at: Instant,
    fps: f32,
    frame_sample_count: u32,
    frame_sample_started_at: Instant,
    fps_epoch: usize,
    fps_task: Task<()>,
    repo_discovery_failed: bool,
    error_message: Option<String>,
    tree_state: Entity<TreeState>,
    sidebar_tree_mode: SidebarTreeMode,
    repo_tree_nodes: Vec<RepoTreeNode>,
    repo_tree_file_count: usize,
    repo_tree_folder_count: usize,
    repo_tree_expanded_dirs: BTreeSet<String>,
    repo_tree_epoch: usize,
    repo_tree_task: Task<()>,
    repo_tree_loading: bool,
    repo_tree_error: Option<String>,
    right_pane_mode: RightPaneMode,
    editor_input_state: Entity<InputState>,
    editor_path: Option<String>,
    editor_loading: bool,
    editor_error: Option<String>,
    editor_dirty: bool,
    editor_last_saved_text: Option<String>,
    editor_epoch: usize,
    editor_task: Task<()>,
    editor_save_loading: bool,
    editor_save_epoch: usize,
    editor_save_task: Task<()>,
}
