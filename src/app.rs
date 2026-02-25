use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use gpui::{
    AnyElement, AppContext as _, Application, Context, Entity, InteractiveElement as _,
    IntoElement, IsZero as _, ListAlignment, ListOffset, ListSizingBehavior, ListState,
    ParentElement as _, Render, ScrollHandle, ScrollWheelEvent, SharedString,
    StatefulInteractiveElement as _, Styled as _, Task, Timer, Window, WindowOptions, div, list,
    point, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Colorize as _, Root, StyledExt as _, Theme, ThemeMode, h_flex,
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    scroll::ScrollableElement,
    switch::Switch,
    tree::{TreeItem, TreeState, tree},
    v_flex,
};
use tracing::{error, info};

use hunk::diff::{DiffCell, DiffCellKind, DiffRowKind, SideBySideRow, parse_patch_side_by_side};
use hunk::git::{ChangedFile, FileStatus, LineStats, RepoSnapshot, load_patch, load_snapshot};

const AUTO_REFRESH_INTERVAL: Duration = Duration::from_millis(900);
const FPS_SAMPLE_INTERVAL: Duration = Duration::from_millis(250);
const AUTO_REFRESH_SCROLL_DEBOUNCE: Duration = Duration::from_millis(500);
const DIFF_MIN_CONTENT_WIDTH: f32 = 960.0;
const DIFF_MIN_COLUMN_WIDTH: f32 = DIFF_MIN_CONTENT_WIDTH / 2.0;
const DIFF_CELL_GUTTER_WIDTH: f32 = 80.0;
const DIFF_MONO_CHAR_WIDTH: f32 = 8.0;
const DIFF_PAN_COLUMN_PADDING: f32 = 28.0;
const DIFF_BOTTOM_SAFE_INSET: f32 = 24.0;
const DIFF_SCROLLBAR_RIGHT_INSET: f32 = 2.0;
const DIFF_SCROLLBAR_SIZE: f32 = 16.0;
const DIFF_VERTICAL_SCROLLBAR_EXTRA_BOTTOM_INSET: f32 = 20.0;
const DIFF_FOOTER_SPACER_ROWS: usize = 2;

mod render;

pub fn run() -> Result<()> {
    let app = Application::new();
    app.run(|cx| {
        gpui_component::init(cx);

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
    repo_root: Option<PathBuf>,
    branch_name: String,
    files: Vec<ChangedFile>,
    collapsed_files: BTreeSet<String>,
    selected_path: Option<String>,
    selected_status: Option<FileStatus>,
    diff_rows: Vec<SideBySideRow>,
    file_row_ranges: Vec<FileRowRange>,
    file_line_stats: BTreeMap<String, LineStats>,
    diff_list_state: ListState,
    diff_horizontal_scroll_handle: ScrollHandle,
    diff_fit_to_width: bool,
    diff_left_column_width: f32,
    diff_right_column_width: f32,
    diff_pan_content_width: f32,
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

impl DiffViewer {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|cx| TreeState::new(cx));

        let mut view = Self {
            repo_root: None,
            branch_name: "unknown".to_string(),
            files: Vec::new(),
            collapsed_files: BTreeSet::new(),
            selected_path: None,
            selected_status: None,
            diff_rows: Vec::new(),
            file_row_ranges: Vec::new(),
            file_line_stats: BTreeMap::new(),
            diff_list_state: ListState::new(0, ListAlignment::Top, px(360.0)),
            diff_horizontal_scroll_handle: ScrollHandle::new(),
            diff_fit_to_width: false,
            diff_left_column_width: DIFF_MIN_COLUMN_WIDTH,
            diff_right_column_width: DIFF_MIN_COLUMN_WIDTH,
            diff_pan_content_width: DIFF_MIN_CONTENT_WIDTH,
            overall_line_stats: LineStats::default(),
            selected_line_stats: LineStats::default(),
            refresh_epoch: 0,
            auto_refresh_task: Task::ready(()),
            snapshot_epoch: 0,
            snapshot_task: Task::ready(()),
            snapshot_loading: false,
            patch_epoch: 0,
            patch_task: Task::ready(()),
            patch_loading: false,
            scroll_selected_after_reload: true,
            last_visible_row_start: None,
            last_diff_scroll_offset: None,
            last_scroll_activity_at: Instant::now(),
            fps: 0.0,
            frame_sample_count: 0,
            frame_sample_started_at: Instant::now(),
            fps_epoch: 0,
            fps_task: Task::ready(()),
            error_message: None,
            tree_state,
        };
        view.request_snapshot_refresh(cx);
        view.start_auto_refresh(cx);
        view.start_fps_monitor(cx);
        view
    }

    fn select_file(&mut self, path: String, cx: &mut Context<Self>) {
        self.selected_path = Some(path.clone());
        self.selected_status = self
            .files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.status);
        self.sync_selected_line_stats();
        self.scroll_to_file_start(&path);
        self.last_visible_row_start = None;
        self.last_diff_scroll_offset = None;
        self.last_scroll_activity_at = Instant::now();
        cx.notify();
    }

    fn request_snapshot_refresh(&mut self, cx: &mut Context<Self>) {
        if self.snapshot_loading {
            return;
        }

        let cwd_result = std::env::current_dir().context("failed to resolve current directory");
        let epoch = self.next_snapshot_epoch();
        self.snapshot_loading = true;

        self.snapshot_task = cx.spawn(async move |this, cx| {
            let result = match cwd_result {
                Ok(cwd) => {
                    cx.background_executor()
                        .spawn(async move { load_snapshot(&cwd) })
                        .await
                }
                Err(err) => Err(err),
            };

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.snapshot_epoch {
                        return;
                    }

                    this.snapshot_loading = false;
                    match result {
                        Ok(snapshot) => this.apply_snapshot(snapshot, cx),
                        Err(err) => this.apply_snapshot_error(err, cx),
                    }
                })
                .ok();
            }
        });
    }

    fn apply_snapshot(&mut self, snapshot: RepoSnapshot, cx: &mut Context<Self>) {
        info!(
            "loaded repository snapshot from {}",
            snapshot.root.display()
        );

        let files_changed = self.files != snapshot.files;
        let overall_changed = self.overall_line_stats != snapshot.line_stats;
        let previous_selected_path = self.selected_path.clone();
        let previous_selected_status = self.selected_status;

        self.repo_root = Some(snapshot.root);
        self.branch_name = snapshot.branch_name;
        self.files = snapshot.files;
        self.overall_line_stats = snapshot.line_stats;
        self.error_message = None;
        self.collapsed_files
            .retain(|path| self.files.iter().any(|file| file.path == *path));

        let current_selection = self
            .selected_path
            .as_ref()
            .filter(|selected| self.files.iter().any(|file| &file.path == *selected))
            .cloned();
        self.selected_path =
            current_selection.or_else(|| self.files.first().map(|file| file.path.clone()));
        self.selected_status = self.selected_path.as_ref().and_then(|selected| {
            self.files
                .iter()
                .find(|file| &file.path == selected)
                .map(|file| file.status)
        });

        let selected_changed = self.selected_path != previous_selected_path
            || self.selected_status != previous_selected_status;

        if files_changed {
            self.rebuild_tree(cx);
        }

        if files_changed || overall_changed || selected_changed || self.diff_rows.is_empty() {
            self.scroll_selected_after_reload = selected_changed || self.diff_rows.is_empty();
            self.request_selected_diff_reload(cx);
        } else {
            self.sync_selected_line_stats();
        }

        cx.notify();
    }

    fn apply_snapshot_error(&mut self, err: anyhow::Error, cx: &mut Context<Self>) {
        self.repo_root = None;
        self.branch_name = "unknown".to_string();
        self.files.clear();
        self.selected_path = None;
        self.selected_status = None;
        self.overall_line_stats = LineStats::default();
        self.selected_line_stats = LineStats::default();
        self.file_row_ranges.clear();
        self.file_line_stats.clear();
        self.diff_rows = vec![message_row(
            DiffRowKind::Empty,
            "Open this app from a Git repository to load diffs.",
        )];
        self.sync_diff_list_state();
        self.recompute_diff_pan_layout();
        self.error_message = Some(err.to_string());
        self.rebuild_tree(cx);
        cx.notify();
    }

    fn request_selected_diff_reload(&mut self, cx: &mut Context<Self>) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.diff_rows.clear();
            self.sync_diff_list_state();
            self.file_row_ranges.clear();
            self.file_line_stats.clear();
            self.selected_line_stats = LineStats::default();
            self.recompute_diff_pan_layout();
            self.patch_loading = false;
            return;
        };

        if self.files.is_empty() {
            self.diff_rows = vec![message_row(DiffRowKind::Empty, "No changed files.")];
            self.sync_diff_list_state();
            self.file_row_ranges.clear();
            self.file_line_stats.clear();
            self.selected_line_stats = LineStats::default();
            self.recompute_diff_pan_layout();
            self.patch_loading = false;
            return;
        }

        let files = self.files.clone();
        let collapsed_files = self.collapsed_files.clone();
        let epoch = self.next_patch_epoch();
        self.patch_loading = true;

        self.patch_task = cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { load_diff_stream(&repo_root, &files, &collapsed_files) })
                .await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.patch_epoch {
                        return;
                    }

                    this.patch_loading = false;
                    match result {
                        Ok(stream) => {
                            this.diff_rows = stream.rows;
                            this.sync_diff_list_state();
                            this.file_row_ranges = stream.file_ranges;
                            this.file_line_stats = stream.file_line_stats;
                            this.recompute_diff_pan_layout();

                            let has_selection = this.selected_path.as_ref().is_some_and(|path| {
                                this.files.iter().any(|file| file.path == *path)
                            });
                            if !has_selection {
                                this.selected_path =
                                    this.files.first().map(|file| file.path.clone());
                            }

                            this.selected_status =
                                this.selected_path.as_ref().and_then(|selected| {
                                    this.files
                                        .iter()
                                        .find(|file| &file.path == selected)
                                        .map(|file| file.status)
                                });
                            this.sync_selected_line_stats();
                            this.last_visible_row_start = None;

                            if this.scroll_selected_after_reload {
                                this.scroll_selected_after_reload = false;
                                this.scroll_selected_file_to_top();
                            }
                        }
                        Err(err) => {
                            this.diff_rows = vec![message_row(
                                DiffRowKind::Meta,
                                format!("Failed to load diff stream: {err:#}"),
                            )];
                            this.sync_diff_list_state();
                            this.file_row_ranges.clear();
                            this.file_line_stats.clear();
                            this.selected_line_stats = LineStats::default();
                            this.recompute_diff_pan_layout();
                            this.scroll_selected_after_reload = false;
                        }
                    }

                    cx.notify();
                })
                .ok();
            }
        });
    }

    fn next_snapshot_epoch(&mut self) -> usize {
        self.snapshot_epoch = self.snapshot_epoch.saturating_add(1);
        self.snapshot_epoch
    }

    fn next_patch_epoch(&mut self) -> usize {
        self.patch_epoch = self.patch_epoch.saturating_add(1);
        self.patch_epoch
    }

    fn rebuild_tree(&mut self, cx: &mut Context<Self>) {
        let items = build_tree_items(&self.files);
        self.tree_state
            .update(cx, |state, cx| state.set_items(items, cx));
    }

    fn start_auto_refresh(&mut self, cx: &mut Context<Self>) {
        let epoch = self.next_refresh_epoch();
        self.schedule_auto_refresh(epoch, cx);
    }

    fn next_refresh_epoch(&mut self) -> usize {
        self.refresh_epoch = self.refresh_epoch.saturating_add(1);
        self.refresh_epoch
    }

    fn schedule_auto_refresh(&mut self, epoch: usize, cx: &mut Context<Self>) {
        if epoch != self.refresh_epoch {
            return;
        }

        self.auto_refresh_task = cx.spawn(async move |this, cx| {
            Timer::after(AUTO_REFRESH_INTERVAL).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if this.recently_scrolling() {
                        let next_epoch = this.next_refresh_epoch();
                        this.schedule_auto_refresh(next_epoch, cx);
                        return;
                    }

                    this.request_snapshot_refresh(cx);
                    let next_epoch = this.next_refresh_epoch();
                    this.schedule_auto_refresh(next_epoch, cx);
                })
                .ok();
            }
        });
    }

    fn recently_scrolling(&self) -> bool {
        self.last_scroll_activity_at.elapsed() < AUTO_REFRESH_SCROLL_DEBOUNCE
    }

    fn selected_file_is_collapsed(&self) -> bool {
        self.selected_path
            .as_ref()
            .is_some_and(|path| self.collapsed_files.contains(path))
    }

    fn toggle_selected_file_collapsed(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.selected_path.clone() else {
            return;
        };

        if self.collapsed_files.contains(path.as_str()) {
            self.collapsed_files.remove(path.as_str());
        } else {
            self.collapsed_files.insert(path);
        }

        self.scroll_selected_after_reload = true;
        self.last_diff_scroll_offset = None;
        self.last_scroll_activity_at = Instant::now();
        self.request_selected_diff_reload(cx);
        cx.notify();
    }

    fn sync_selected_line_stats(&mut self) {
        self.selected_line_stats = self
            .selected_path
            .as_ref()
            .and_then(|path| self.file_line_stats.get(path))
            .copied()
            .unwrap_or_default();
    }

    fn scroll_selected_file_to_top(&mut self) {
        let Some(path) = self.selected_path.clone() else {
            return;
        };
        self.scroll_to_file_start(&path);
    }

    fn scroll_to_file_start(&mut self, path: &str) {
        let Some(start_row) = self
            .file_row_ranges
            .iter()
            .find(|range| range.path == path)
            .map(|range| range.start_row)
        else {
            return;
        };

        self.diff_list_state.scroll_to(ListOffset {
            item_ix: start_row,
            offset_in_item: px(0.),
        });
        self.last_diff_scroll_offset = None;
        self.last_scroll_activity_at = Instant::now();
    }

    fn sync_selected_file_from_visible_row(&mut self, row_ix: usize, cx: &mut Context<Self>) {
        if self.last_visible_row_start == Some(row_ix) {
            return;
        }
        self.last_visible_row_start = Some(row_ix);

        let range = self
            .file_row_ranges
            .iter()
            .find(|range| row_ix < range.end_row)
            .or_else(|| self.file_row_ranges.last());
        let Some(range) = range else {
            return;
        };

        if self.selected_path.as_deref() == Some(range.path.as_str()) {
            return;
        }

        self.selected_path = Some(range.path.clone());
        self.selected_status = Some(range.status);
        self.sync_selected_line_stats();
        cx.notify();
    }

    fn on_diff_horizontal_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.diff_fit_to_width {
            return;
        }

        let mut delta = event.delta.pixel_delta(window.line_height());
        if delta.x.is_zero() && event.modifiers.shift && !delta.y.is_zero() {
            delta.x = delta.y;
            delta.y = px(0.);
        }
        let horizontal_intent =
            !delta.x.is_zero() && (delta.y.is_zero() || delta.x.abs() >= delta.y.abs());
        if !horizontal_intent {
            // Prevent GPUI's default overflow-x wheel remapping (y -> x) on this container.
            cx.stop_propagation();
            return;
        }

        let changed = self.scroll_diff_horizontal_by(delta.x);
        self.last_scroll_activity_at = Instant::now();
        if changed {
            cx.notify();
        }
        cx.stop_propagation();
    }

    fn on_diff_list_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.diff_fit_to_width {
            self.last_scroll_activity_at = Instant::now();
            return;
        }

        let mut delta = event.delta.pixel_delta(window.line_height());
        if delta.x.is_zero() && event.modifiers.shift && !delta.y.is_zero() {
            delta.x = delta.y;
            delta.y = px(0.);
        }

        let horizontal_intent =
            !delta.x.is_zero() && (delta.y.is_zero() || delta.x.abs() >= delta.y.abs());
        if horizontal_intent {
            let changed = self.scroll_diff_horizontal_by(delta.x);
            self.last_scroll_activity_at = Instant::now();
            if changed {
                cx.notify();
            }
            cx.stop_propagation();
            return;
        }

        if !delta.y.is_zero() {
            self.last_scroll_activity_at = Instant::now();
        }
    }

    fn scroll_diff_horizontal_by(&mut self, delta_x: gpui::Pixels) -> bool {
        if delta_x.is_zero() {
            return false;
        }

        let mut offset = self.diff_horizontal_scroll_handle.offset();
        offset.x += delta_x;
        offset.y = px(0.);

        let max_x = self
            .diff_horizontal_scroll_handle
            .max_offset()
            .width
            .max(px(0.));
        offset.x = offset.x.clamp(-max_x, px(0.));

        if offset != self.diff_horizontal_scroll_handle.offset() {
            self.diff_horizontal_scroll_handle.set_offset(offset);
            return true;
        }

        false
    }

    fn toggle_diff_fit_to_width(&mut self, cx: &mut Context<Self>) {
        self.diff_fit_to_width = !self.diff_fit_to_width;
        self.diff_horizontal_scroll_handle
            .set_offset(point(px(0.), px(0.)));
        self.last_scroll_activity_at = Instant::now();
        cx.notify();
    }

    fn recompute_diff_pan_layout(&mut self) {
        let mut max_left_chars = 0usize;
        let mut max_right_chars = 0usize;

        for row in &self.diff_rows {
            match row.kind {
                DiffRowKind::Code => {
                    max_left_chars = max_left_chars.max(display_width(&row.left.text));
                    max_right_chars = max_right_chars.max(display_width(&row.right.text));
                }
                DiffRowKind::HunkHeader | DiffRowKind::Meta | DiffRowKind::Empty => {}
            }
        }

        let left_width = (max_left_chars as f32 * DIFF_MONO_CHAR_WIDTH
            + DIFF_CELL_GUTTER_WIDTH
            + DIFF_PAN_COLUMN_PADDING)
            .max(DIFF_MIN_COLUMN_WIDTH);
        let right_width = (max_right_chars as f32 * DIFF_MONO_CHAR_WIDTH
            + DIFF_CELL_GUTTER_WIDTH
            + DIFF_PAN_COLUMN_PADDING)
            .max(DIFF_MIN_COLUMN_WIDTH);

        self.diff_left_column_width = left_width;
        self.diff_right_column_width = right_width;
        self.diff_pan_content_width = (left_width + right_width).max(DIFF_MIN_CONTENT_WIDTH);
    }

    fn clamp_diff_horizontal_scroll_offset(&mut self) {
        if self.diff_fit_to_width {
            self.diff_horizontal_scroll_handle
                .set_offset(point(px(0.), px(0.)));
            return;
        }

        let offset = self.diff_horizontal_scroll_handle.offset();
        let max_x = self
            .diff_horizontal_scroll_handle
            .max_offset()
            .width
            .max(px(0.));
        let clamped_x = offset.x.clamp(-max_x, px(0.));

        if clamped_x != offset.x || !offset.y.is_zero() {
            self.diff_horizontal_scroll_handle
                .set_offset(point(clamped_x, px(0.)));
        }
    }

    fn sync_diff_list_state(&self) {
        let previous_top = self.diff_list_state.logical_scroll_top();
        self.diff_list_state.reset(self.diff_rows.len());
        let clamped_item_ix = if self.diff_rows.is_empty() {
            0
        } else {
            previous_top
                .item_ix
                .min(self.diff_rows.len().saturating_sub(1))
        };
        self.diff_list_state.scroll_to(ListOffset {
            item_ix: clamped_item_ix,
            offset_in_item: px(0.),
        });
    }

    fn start_fps_monitor(&mut self, cx: &mut Context<Self>) {
        let epoch = self.next_fps_epoch();
        self.schedule_fps_sample(epoch, cx);
    }

    fn next_fps_epoch(&mut self) -> usize {
        self.fps_epoch = self.fps_epoch.saturating_add(1);
        self.fps_epoch
    }

    fn schedule_fps_sample(&mut self, epoch: usize, cx: &mut Context<Self>) {
        if epoch != self.fps_epoch {
            return;
        }

        self.fps_task = cx.spawn(async move |this, cx| {
            Timer::after(FPS_SAMPLE_INTERVAL).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    let elapsed = this.frame_sample_started_at.elapsed().as_secs_f32();
                    if elapsed > 0.0 {
                        this.fps = this.frame_sample_count as f32 / elapsed;
                    } else {
                        this.fps = 0.0;
                    }
                    this.frame_sample_count = 0;
                    this.frame_sample_started_at = Instant::now();

                    let next_epoch = this.next_fps_epoch();
                    this.schedule_fps_sample(next_epoch, cx);
                    cx.notify();
                })
                .ok();
            }
        });
    }
}

#[derive(Default)]
struct TreeFolder {
    folders: BTreeMap<String, TreeFolder>,
    files: BTreeMap<String, FileStatus>,
}

#[derive(Debug, Clone)]
struct FileRowRange {
    path: String,
    status: FileStatus,
    start_row: usize,
    end_row: usize,
}

struct DiffStream {
    rows: Vec<SideBySideRow>,
    file_ranges: Vec<FileRowRange>,
    file_line_stats: BTreeMap<String, LineStats>,
}

fn build_tree_items(files: &[ChangedFile]) -> Vec<TreeItem> {
    let mut root = TreeFolder::default();

    for file in files {
        let mut cursor = &mut root;
        let mut parts = file.path.split('/').peekable();
        while let Some(part) = parts.next() {
            if parts.peek().is_some() {
                cursor = cursor.folders.entry(part.to_string()).or_default();
            } else {
                cursor.files.insert(part.to_string(), file.status);
            }
        }
    }

    build_folder_items(&root, "")
}

fn build_folder_items(folder: &TreeFolder, prefix: &str) -> Vec<TreeItem> {
    let mut items = Vec::new();

    for (name, child_folder) in &folder.folders {
        let id = join_path(prefix, name);
        let children = build_folder_items(child_folder, &id);
        items.push(
            TreeItem::new(
                SharedString::from(id.clone()),
                SharedString::from(name.clone()),
            )
            .expanded(true)
            .children(children),
        );
    }

    for name in folder.files.keys() {
        let id = join_path(prefix, name);
        items.push(TreeItem::new(
            SharedString::from(id),
            SharedString::from(name.clone()),
        ));
    }

    items
}

fn join_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}/{name}")
    }
}

fn message_row(kind: DiffRowKind, text: impl Into<String>) -> SideBySideRow {
    SideBySideRow {
        kind,
        left: DiffCell {
            line: None,
            text: String::new(),
            kind: DiffCellKind::None,
        },
        right: DiffCell {
            line: None,
            text: String::new(),
            kind: DiffCellKind::None,
        },
        text: text.into(),
    }
}

fn load_diff_stream(
    repo_root: &Path,
    files: &[ChangedFile],
    collapsed_files: &BTreeSet<String>,
) -> Result<DiffStream> {
    let mut rows = Vec::new();
    let mut file_ranges = Vec::with_capacity(files.len());
    let mut file_line_stats = BTreeMap::new();

    for file in files {
        let start_row = rows.len();
        rows.push(message_row(
            DiffRowKind::Meta,
            format!("── {} [{}] ──", file.path, file.status.tag()),
        ));

        let (parsed_rows, stats) = match load_patch(repo_root, &file.path, file.status) {
            Ok(patch) => {
                let parsed_rows = parse_patch_side_by_side(&patch);
                let stats = line_stats_from_rows(&parsed_rows);
                (parsed_rows, stats)
            }
            Err(err) => (
                vec![message_row(
                    DiffRowKind::Meta,
                    format!("Failed to load patch for {}: {err:#}", file.path),
                )],
                LineStats::default(),
            ),
        };

        file_line_stats.insert(file.path.clone(), stats);

        if collapsed_files.contains(file.path.as_str()) {
            rows.push(message_row(
                DiffRowKind::Empty,
                format!("File collapsed ({} changed lines hidden).", stats.changed()),
            ));
        } else {
            rows.extend(parsed_rows);
        }

        rows.push(message_row(
            DiffRowKind::Meta,
            format!("── End of {} ──", file.path),
        ));

        let end_row = rows.len();
        file_ranges.push(FileRowRange {
            path: file.path.clone(),
            status: file.status,
            start_row,
            end_row,
        });
    }

    if rows.is_empty() {
        rows.push(message_row(DiffRowKind::Empty, "No changed files."));
    } else {
        rows.push(message_row(DiffRowKind::Meta, "── End of change set ──"));
        rows.push(message_row(
            DiffRowKind::Empty,
            "You are at the bottom of the diff stream.",
        ));
        for _ in 0..DIFF_FOOTER_SPACER_ROWS {
            rows.push(message_row(DiffRowKind::Empty, ""));
        }
    }

    Ok(DiffStream {
        rows,
        file_ranges,
        file_line_stats,
    })
}

fn line_stats_from_rows(rows: &[SideBySideRow]) -> LineStats {
    let mut stats = LineStats::default();

    for row in rows {
        if row.kind != DiffRowKind::Code {
            continue;
        }

        if row.left.kind == DiffCellKind::Removed {
            stats.removed = stats.removed.saturating_add(1);
        }
        if row.right.kind == DiffCellKind::Added {
            stats.added = stats.added.saturating_add(1);
        }
    }

    stats
}

fn display_width(text: &str) -> usize {
    text.chars().fold(0, |acc, ch| {
        acc + match ch {
            '\t' => 4,
            ch if ch.is_control() => 0,
            _ => 1,
        }
    })
}
