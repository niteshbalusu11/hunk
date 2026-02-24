use std::collections::BTreeMap;
use std::ops::Range;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use gpui::{
    AnyElement, AppContext as _, Application, Context, Entity, InteractiveElement as _,
    IntoElement, ListSizingBehavior, ParentElement as _, Render, ScrollWheelEvent, SharedString,
    Styled as _, Task, Timer, UniformListScrollHandle, Window, WindowOptions, div, point,
    prelude::FluentBuilder as _, px, uniform_list,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FileJumpAnchor {
    Top,
    Bottom,
}

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
    selected_path: Option<String>,
    selected_status: Option<FileStatus>,
    diff_rows: Vec<SideBySideRow>,
    diff_scroll_handle: UniformListScrollHandle,
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
    auto_next_armed: bool,
    auto_prev_armed: bool,
    pending_jump_anchor: Option<FileJumpAnchor>,
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
            selected_path: None,
            selected_status: None,
            diff_rows: Vec::new(),
            diff_scroll_handle: UniformListScrollHandle::new(),
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
            auto_next_armed: true,
            auto_prev_armed: true,
            pending_jump_anchor: None,
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

    fn select_file(
        &mut self,
        path: String,
        jump_anchor: Option<FileJumpAnchor>,
        cx: &mut Context<Self>,
    ) {
        if self.selected_path.as_deref() == Some(path.as_str()) {
            return;
        }

        self.selected_path = Some(path.clone());
        self.selected_status = self
            .files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.status);
        self.auto_next_armed = true;
        self.auto_prev_armed = true;
        self.pending_jump_anchor = jump_anchor;
        if jump_anchor != Some(FileJumpAnchor::Bottom) {
            self.reset_diff_scroll_to_top();
        }
        self.request_selected_diff_reload(cx);
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

        let current_selection = self
            .selected_path
            .as_ref()
            .filter(|selected| self.files.iter().any(|file| &file.path == *selected))
            .cloned();
        self.selected_path =
            current_selection.or_else(|| self.files.first().map(|f| f.path.clone()));
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
            self.request_selected_diff_reload(cx);
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
        self.diff_rows = vec![message_row(
            DiffRowKind::Empty,
            "Open this app from a Git repository to load diffs.",
        )];
        self.error_message = Some(err.to_string());
        self.rebuild_tree(cx);
        cx.notify();
    }

    fn request_selected_diff_reload(&mut self, cx: &mut Context<Self>) {
        let Some(repo_root) = self.repo_root.clone() else {
            self.diff_rows.clear();
            self.selected_line_stats = LineStats::default();
            self.patch_loading = false;
            return;
        };

        let Some(path) = self.selected_path.clone() else {
            self.diff_rows = vec![message_row(
                DiffRowKind::Empty,
                "Select a file to view its diff.",
            )];
            self.selected_line_stats = LineStats::default();
            self.patch_loading = false;
            return;
        };

        let status = self.selected_status.unwrap_or(FileStatus::Unknown);
        let path_for_error = path.clone();
        let epoch = self.next_patch_epoch();
        self.patch_loading = true;

        self.patch_task = cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let patch = load_patch(&repo_root, &path, status)?;
                    let rows = parse_patch_side_by_side(&patch);
                    let stats = line_stats_from_rows(&rows);
                    Ok::<(Vec<SideBySideRow>, LineStats), anyhow::Error>((rows, stats))
                })
                .await;

            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    if epoch != this.patch_epoch {
                        return;
                    }

                    this.patch_loading = false;
                    match result {
                        Ok((rows, stats)) => {
                            this.diff_rows = rows;
                            this.selected_line_stats = stats;
                            this.apply_pending_jump_anchor();
                        }
                        Err(err) => {
                            this.diff_rows = vec![message_row(
                                DiffRowKind::Meta,
                                format!("Failed to load patch for {path_for_error}: {err:#}"),
                            )];
                            this.selected_line_stats = LineStats::default();
                            this.pending_jump_anchor = None;
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

    fn reset_diff_scroll_to_top(&mut self) {
        self.diff_scroll_handle
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.), px(0.)));
        self.last_diff_scroll_offset = None;
        self.last_scroll_activity_at = Instant::now();
    }

    fn apply_pending_jump_anchor(&mut self) {
        match self.pending_jump_anchor.take() {
            Some(FileJumpAnchor::Bottom) => {
                if self.diff_rows.is_empty() {
                    return;
                }

                self.diff_scroll_handle.scroll_to_item_strict(
                    self.diff_rows.len().saturating_sub(1),
                    gpui::ScrollStrategy::Bottom,
                );
                self.last_diff_scroll_offset = None;
                self.last_scroll_activity_at = Instant::now();
            }
            Some(FileJumpAnchor::Top) => {
                self.reset_diff_scroll_to_top();
            }
            None => {}
        }
    }

    fn selected_file_index(&self) -> Option<usize> {
        let selected = self.selected_path.as_ref()?;
        self.files.iter().position(|file| &file.path == selected)
    }

    fn maybe_auto_advance_on_scroll(&mut self, delta_y: gpui::Pixels, cx: &mut Context<Self>) {
        if self.patch_loading || self.files.is_empty() {
            return;
        }

        let scroll_state = self.diff_scroll_handle.0.borrow();
        let base_handle = &scroll_state.base_handle;
        let offset_y = base_handle.offset().y;
        let max_y = base_handle.max_offset().height;
        drop(scroll_state);

        if max_y <= px(0.) {
            if delta_y < -px(0.5) {
                self.auto_prev_armed = true;
                let Some(current_ix) = self.selected_file_index() else {
                    return;
                };
                let Some(next_file) = self.files.get(current_ix.saturating_add(1)) else {
                    self.auto_next_armed = false;
                    return;
                };

                self.auto_next_armed = false;
                let next_path = next_file.path.clone();
                self.select_file(next_path, Some(FileJumpAnchor::Top), cx);
            } else if delta_y > px(0.5) {
                self.auto_next_armed = true;
                let Some(current_ix) = self.selected_file_index() else {
                    return;
                };
                if current_ix == 0 {
                    self.auto_prev_armed = false;
                    return;
                }

                self.auto_prev_armed = false;
                let previous_path = self.files[current_ix - 1].path.clone();
                self.select_file(previous_path, Some(FileJumpAnchor::Bottom), cx);
            }
            return;
        }

        let distance_to_bottom = (max_y + offset_y).abs();
        if distance_to_bottom > px(140.) {
            self.auto_next_armed = true;
        }
        if offset_y < -px(140.) {
            self.auto_prev_armed = true;
        }

        if delta_y < -px(0.5) && self.auto_next_armed && distance_to_bottom <= px(36.) {
            let Some(current_ix) = self.selected_file_index() else {
                return;
            };
            let Some(next_file) = self.files.get(current_ix.saturating_add(1)) else {
                self.auto_next_armed = false;
                return;
            };

            let next_path = next_file.path.clone();
            self.auto_next_armed = false;
            self.auto_prev_armed = true;
            self.select_file(next_path, Some(FileJumpAnchor::Top), cx);
            return;
        }

        if delta_y <= px(0.5) || !self.auto_prev_armed || offset_y < -px(36.) {
            return;
        }

        let Some(current_ix) = self.selected_file_index() else {
            return;
        };
        if current_ix == 0 {
            self.auto_prev_armed = false;
            return;
        }

        let previous_path = self.files[current_ix - 1].path.clone();
        self.auto_prev_armed = false;
        self.auto_next_armed = true;
        self.select_file(previous_path, Some(FileJumpAnchor::Bottom), cx);
    }

    fn on_diff_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let delta = event.delta.pixel_delta(window.line_height()).y;
        self.last_scroll_activity_at = Instant::now();
        self.maybe_auto_advance_on_scroll(delta, cx);
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

    for (name, _) in &folder.files {
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
