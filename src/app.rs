use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context as _, Result};
use gpui::{
    AnyElement, AppContext as _, Application, Context, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, SharedString, Styled as _, Window, WindowOptions, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Colorize as _, Root, StyledExt as _, Theme, ThemeMode,
    button::Button,
    h_flex,
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    scroll::ScrollableElement,
    switch::Switch,
    tree::{TreeItem, TreeState, tree},
    v_flex,
};
use tracing::{error, info};

use hunk::diff::{DiffCell, DiffCellKind, DiffRowKind, SideBySideRow, parse_patch_side_by_side};
use hunk::git::{ChangedFile, FileStatus, LineStats, load_patch, load_snapshot};

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
    overall_line_stats: LineStats,
    selected_line_stats: LineStats,
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
            overall_line_stats: LineStats::default(),
            selected_line_stats: LineStats::default(),
            error_message: None,
            tree_state,
        };
        view.refresh(cx);
        view
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let snapshot = std::env::current_dir()
            .context("failed to resolve current directory")
            .and_then(|cwd| load_snapshot(&cwd));

        match snapshot {
            Ok(snapshot) => {
                info!(
                    "loaded repository snapshot from {}",
                    snapshot.root.display()
                );
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

                self.rebuild_tree(cx);
                self.reload_selected_diff();
            }
            Err(err) => {
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
            }
        }

        cx.notify();
    }

    fn select_file(&mut self, path: String, cx: &mut Context<Self>) {
        self.selected_path = Some(path.clone());
        self.selected_status = self
            .files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.status);
        self.reload_selected_diff();
        cx.notify();
    }

    fn reload_selected_diff(&mut self) {
        let Some(repo_root) = self.repo_root.as_ref() else {
            self.diff_rows.clear();
            self.selected_line_stats = LineStats::default();
            return;
        };

        let Some(path) = self.selected_path.as_ref() else {
            self.diff_rows = vec![message_row(
                DiffRowKind::Empty,
                "Select a file to view its diff.",
            )];
            self.selected_line_stats = LineStats::default();
            return;
        };

        let status = self.selected_status.unwrap_or(FileStatus::Unknown);
        match load_patch(repo_root, path, status) {
            Ok(patch) => {
                self.diff_rows = parse_patch_side_by_side(&patch);
                self.selected_line_stats = line_stats_from_rows(&self.diff_rows);
            }
            Err(err) => {
                self.diff_rows = vec![message_row(
                    DiffRowKind::Meta,
                    format!("Failed to load patch for {path}: {err:#}"),
                )];
                self.selected_line_stats = LineStats::default();
            }
        }
    }

    fn rebuild_tree(&mut self, cx: &mut Context<Self>) {
        let items = build_tree_items(&self.files);
        self.tree_state
            .update(cx, |state, cx| state.set_items(items, cx));
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let repo_label = self
            .repo_root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "No git repository found".to_string());
        let branch_label = format!("branch: {}", self.branch_name);

        h_flex()
            .w_full()
            .h_11()
            .items_center()
            .justify_between()
            .px_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_sm().font_semibold().child("hunk"))
                    .child(
                        div()
                            .text_xs()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_color(cx.theme().muted_foreground)
                            .child(branch_label),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(repo_label),
                    ),
            )
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("refresh")
                            .label("Refresh")
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| this.refresh(cx));
                            }),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_sm().child("Dark"))
                            .child(
                                Switch::new("theme-mode")
                                    .checked(cx.theme().mode.is_dark())
                                    .on_click(|checked, window, cx| {
                                        let mode = if *checked {
                                            ThemeMode::Dark
                                        } else {
                                            ThemeMode::Light
                                        };
                                        Theme::change(mode, Some(window), cx);
                                    }),
                            ),
                    )
                    .child(self.render_line_stats("overall", self.overall_line_stats, cx))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} files", self.files.len())),
                    ),
            )
    }

    fn render_tree(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let selected_path = self.selected_path.clone();

        v_flex().size_full().overflow_y_scrollbar().child(tree(
            &self.tree_state,
            move |ix, entry, _selected, _window, _cx| {
                let item = entry.item();
                let item_id = item.id.to_string();
                let item_label = item.label.clone();
                let is_folder = entry.is_folder();
                let is_selected = selected_path.as_deref() == Some(item_id.as_str());
                let icon = if is_folder {
                    if entry.is_expanded() { "▾" } else { "▸" }
                } else {
                    "•"
                };
                let indent = px(10.0 + (entry.depth() as f32 * 16.0));

                ListItem::new(ix)
                    .selected(is_selected)
                    .pl(indent)
                    .on_click({
                        let view = view.clone();
                        move |_, _, cx| {
                            if is_folder {
                                return;
                            }

                            view.update(cx, |this, cx| {
                                this.select_file(item_id.clone(), cx);
                            });
                        }
                    })
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_2()
                            .child(div().text_sm().child(icon))
                            .child(div().text_sm().child(item_label)),
                    )
            },
        ))
    }

    fn render_diff(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(error_message) = &self.error_message {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .p_4()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(error_message.clone()),
                )
                .into_any_element();
        }

        let (old_label, new_label) = self.diff_column_labels();

        v_flex()
            .size_full()
            .overflow_y_scrollbar()
            .child(self.render_file_status_banner(cx))
            .child(
                h_flex()
                    .w_full()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex_1()
                            .px_2()
                            .py_1()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(old_label),
                    )
                    .child(
                        div()
                            .flex_1()
                            .px_2()
                            .py_1()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(new_label),
                    ),
            )
            .children(
                self.diff_rows
                    .iter()
                    .enumerate()
                    .map(|(ix, row)| match row.kind {
                        DiffRowKind::Code => self.render_code_row(ix, row, cx),
                        DiffRowKind::HunkHeader | DiffRowKind::Meta | DiffRowKind::Empty => {
                            self.render_meta_row(ix, row, cx)
                        }
                    }),
            )
            .into_any_element()
    }

    fn render_meta_row(
        &self,
        ix: usize,
        row: &SideBySideRow,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_dark = cx.theme().mode.is_dark();

        let (background, foreground, accent) = match row.kind {
            DiffRowKind::HunkHeader => (
                cx.theme().primary_hover,
                cx.theme().primary_foreground,
                cx.theme().primary,
            ),
            DiffRowKind::Meta => {
                let line = row.text.as_str();
                if line.starts_with("new file mode") || line.starts_with("+++ b/") {
                    (
                        cx.theme()
                            .background
                            .blend(
                                cx.theme()
                                    .success
                                    .opacity(if is_dark { 0.22 } else { 0.12 }),
                            ),
                        if is_dark {
                            cx.theme().success.lighten(0.45)
                        } else {
                            cx.theme().success.darken(0.10)
                        },
                        cx.theme().success,
                    )
                } else if line.starts_with("deleted file mode") || line.starts_with("--- a/") {
                    (
                        cx.theme()
                            .background
                            .blend(cx.theme().danger.opacity(if is_dark { 0.22 } else { 0.12 })),
                        if is_dark {
                            cx.theme().danger.lighten(0.45)
                        } else {
                            cx.theme().danger.darken(0.10)
                        },
                        cx.theme().danger,
                    )
                } else if line.starts_with("diff --git") {
                    (
                        cx.theme()
                            .background
                            .blend(cx.theme().accent.opacity(if is_dark { 0.18 } else { 0.10 })),
                        cx.theme().foreground,
                        cx.theme().accent,
                    )
                } else {
                    (
                        cx.theme().muted,
                        cx.theme().muted_foreground,
                        cx.theme().border,
                    )
                }
            }
            DiffRowKind::Empty => (
                cx.theme().background,
                cx.theme().muted_foreground,
                cx.theme().border,
            ),
            DiffRowKind::Code => (
                cx.theme().background,
                cx.theme().foreground,
                cx.theme().border,
            ),
        };

        div()
            .id(("diff-meta-row", ix))
            .relative()
            .w_full()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(background)
            .text_sm()
            .text_color(foreground)
            .font_family(cx.theme().mono_font_family.clone())
            .child(row.text.clone())
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w(px(2.))
                    .bg(accent),
            )
            .into_any_element()
    }

    fn render_code_row(
        &self,
        ix: usize,
        row: &SideBySideRow,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        h_flex()
            .id(("diff-code-row", ix))
            .w_full()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(self.render_diff_cell(ix, "left", &row.left, row.right.kind, cx))
            .child(self.render_diff_cell(ix, "right", &row.right, row.left.kind, cx))
            .into_any_element()
    }

    fn render_diff_cell(
        &self,
        row_ix: usize,
        side: &'static str,
        cell: &DiffCell,
        peer_kind: DiffCellKind,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let cell_id = if side == "left" {
            ("diff-cell-left", row_ix)
        } else {
            ("diff-cell-right", row_ix)
        };

        let is_dark = cx.theme().mode.is_dark();
        let add_alpha = if is_dark { 0.42 } else { 0.18 };
        let remove_alpha = if is_dark { 0.42 } else { 0.18 };
        let ghost_alpha = if is_dark { 0.24 } else { 0.11 };

        let (background, marker_color, line_color, text_color, marker) =
            match (cell.kind, peer_kind) {
                (DiffCellKind::Added, _) => (
                    cx.theme()
                        .background
                        .blend(cx.theme().success.opacity(add_alpha)),
                    if is_dark {
                        cx.theme().success.lighten(0.55)
                    } else {
                        cx.theme().success.darken(0.18)
                    },
                    if is_dark {
                        cx.theme().success.lighten(0.52)
                    } else {
                        cx.theme().success.darken(0.16)
                    },
                    cx.theme().foreground,
                    "+",
                ),
                (DiffCellKind::Removed, _) => (
                    cx.theme()
                        .background
                        .blend(cx.theme().danger.opacity(remove_alpha)),
                    if is_dark {
                        cx.theme().danger.lighten(0.55)
                    } else {
                        cx.theme().danger.darken(0.18)
                    },
                    if is_dark {
                        cx.theme().danger.lighten(0.52)
                    } else {
                        cx.theme().danger.darken(0.16)
                    },
                    cx.theme().foreground,
                    "-",
                ),
                (DiffCellKind::None, DiffCellKind::Added) => (
                    cx.theme()
                        .background
                        .blend(cx.theme().success.opacity(ghost_alpha)),
                    if is_dark {
                        cx.theme().muted_foreground.lighten(0.22)
                    } else {
                        cx.theme().muted_foreground.darken(0.08)
                    },
                    if is_dark {
                        cx.theme().muted_foreground.lighten(0.16)
                    } else {
                        cx.theme().muted_foreground.darken(0.06)
                    },
                    if is_dark {
                        cx.theme().muted_foreground.lighten(0.18)
                    } else {
                        cx.theme().muted_foreground.darken(0.08)
                    },
                    "∅",
                ),
                (DiffCellKind::None, DiffCellKind::Removed) => (
                    cx.theme()
                        .background
                        .blend(cx.theme().danger.opacity(ghost_alpha)),
                    if is_dark {
                        cx.theme().muted_foreground.lighten(0.22)
                    } else {
                        cx.theme().muted_foreground.darken(0.08)
                    },
                    if is_dark {
                        cx.theme().muted_foreground.lighten(0.16)
                    } else {
                        cx.theme().muted_foreground.darken(0.06)
                    },
                    if is_dark {
                        cx.theme().muted_foreground.lighten(0.18)
                    } else {
                        cx.theme().muted_foreground.darken(0.08)
                    },
                    "∅",
                ),
                (DiffCellKind::Context, _) => (
                    cx.theme().background,
                    if is_dark {
                        cx.theme().muted_foreground.lighten(0.08)
                    } else {
                        cx.theme().muted_foreground.darken(0.10)
                    },
                    if is_dark {
                        cx.theme().muted_foreground.lighten(0.16)
                    } else {
                        cx.theme().muted_foreground.darken(0.12)
                    },
                    cx.theme().foreground,
                    " ",
                ),
                (DiffCellKind::None, _) => (
                    cx.theme().background,
                    if is_dark {
                        cx.theme().muted_foreground.lighten(0.08)
                    } else {
                        cx.theme().muted_foreground.darken(0.10)
                    },
                    if is_dark {
                        cx.theme().muted_foreground.lighten(0.16)
                    } else {
                        cx.theme().muted_foreground.darken(0.12)
                    },
                    if is_dark {
                        cx.theme().muted_foreground.lighten(0.04)
                    } else {
                        cx.theme().muted_foreground.darken(0.06)
                    },
                    "",
                ),
            };

        let line_number = cell.line.map(|line| line.to_string()).unwrap_or_default();
        let content = if cell.text.is_empty() && marker == "∅" {
            "no line".to_string()
        } else {
            cell.text.clone()
        };

        h_flex()
            .id(cell_id)
            .flex_1()
            .min_w_0()
            .px_2()
            .py_1()
            .gap_2()
            .items_start()
            .bg(background)
            .when(side == "left", |this| {
                this.border_r_1().border_color(cx.theme().border)
            })
            .child(
                div()
                    .w_10()
                    .text_xs()
                    .text_color(line_color)
                    .font_family(cx.theme().mono_font_family.clone())
                    .child(line_number),
            )
            .child(
                div()
                    .w_4()
                    .text_sm()
                    .text_color(marker_color)
                    .font_family(cx.theme().mono_font_family.clone())
                    .child(marker),
            )
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(text_color)
                    .font_family(cx.theme().mono_font_family.clone())
                    .child(content),
            )
            .into_any_element()
    }

    fn diff_column_labels(&self) -> (String, String) {
        let selected = self
            .selected_path
            .clone()
            .unwrap_or_else(|| "file".to_string());
        match self.selected_status.unwrap_or(FileStatus::Unknown) {
            FileStatus::Added | FileStatus::Untracked => ("/dev/null".to_string(), selected),
            FileStatus::Deleted => (selected, "/dev/null".to_string()),
            _ => ("Old".to_string(), "New".to_string()),
        }
    }

    fn render_file_status_banner(&self, cx: &mut Context<Self>) -> AnyElement {
        let path = self
            .selected_path
            .clone()
            .unwrap_or_else(|| "No file selected".to_string());

        let status = self.selected_status.unwrap_or(FileStatus::Unknown);
        let is_dark = cx.theme().mode.is_dark();

        let (label, hint, accent, background, badge_background) = match status {
            FileStatus::Added | FileStatus::Untracked => (
                "NEW FILE",
                "Content exists only on the right side.",
                cx.theme().success,
                cx.theme()
                    .background
                    .blend(
                        cx.theme()
                            .success
                            .opacity(if is_dark { 0.20 } else { 0.10 }),
                    ),
                cx.theme()
                    .success
                    .opacity(if is_dark { 0.50 } else { 0.24 }),
            ),
            FileStatus::Deleted => (
                "DELETED FILE",
                "Content exists only on the left side.",
                cx.theme().danger,
                cx.theme()
                    .background
                    .blend(cx.theme().danger.opacity(if is_dark { 0.20 } else { 0.10 })),
                cx.theme().danger.opacity(if is_dark { 0.50 } else { 0.24 }),
            ),
            FileStatus::Renamed => (
                "RENAMED",
                "Showing textual changes for this path.",
                cx.theme().warning,
                cx.theme()
                    .background
                    .blend(
                        cx.theme()
                            .warning
                            .opacity(if is_dark { 0.20 } else { 0.10 }),
                    ),
                cx.theme()
                    .warning
                    .opacity(if is_dark { 0.45 } else { 0.24 }),
            ),
            _ => (
                "MODIFIED",
                "Side-by-side diff view.",
                cx.theme().accent,
                cx.theme()
                    .background
                    .blend(cx.theme().accent.opacity(if is_dark { 0.14 } else { 0.08 })),
                cx.theme().accent.opacity(if is_dark { 0.50 } else { 0.24 }),
            ),
        };

        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(background)
            .child(
                div()
                    .px_2()
                    .py_0p5()
                    .text_xs()
                    .font_semibold()
                    .bg(badge_background)
                    .border_1()
                    .border_color(accent.opacity(if is_dark { 0.88 } else { 0.44 }))
                    .text_color(cx.theme().foreground)
                    .child(label),
            )
            .child(
                div()
                    .text_sm()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(cx.theme().foreground)
                    .child(path),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(hint),
            )
            .child(self.render_line_stats("file", self.selected_line_stats, cx))
            .into_any_element()
    }

    fn render_line_stats(
        &self,
        label: &'static str,
        stats: LineStats,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        h_flex()
            .items_center()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(label),
            )
            .child(
                div()
                    .text_xs()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(if cx.theme().mode.is_dark() {
                        cx.theme().success.lighten(0.42)
                    } else {
                        cx.theme().success.darken(0.05)
                    })
                    .child(format!("+{}", stats.added)),
            )
            .child(
                div()
                    .text_xs()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(if cx.theme().mode.is_dark() {
                        cx.theme().danger.lighten(0.42)
                    } else {
                        cx.theme().danger.darken(0.05)
                    })
                    .child(format!("-{}", stats.removed)),
            )
            .child(
                div()
                    .text_xs()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("chg {}", stats.changed())),
            )
            .into_any_element()
    }
}

impl Render for DiffViewer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.render_toolbar(cx))
            .child(
                h_resizable("hunk-main")
                    .child(
                        resizable_panel()
                            .size(px(320.0))
                            .size_range(px(220.0)..px(560.0))
                            .child(self.render_tree(cx)),
                    )
                    .child(resizable_panel().child(self.render_diff(cx))),
            )
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
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

    for (name, status) in &folder.files {
        let id = join_path(prefix, name);
        let label = format!("[{}] {}", status.tag(), name);
        items.push(TreeItem::new(
            SharedString::from(id),
            SharedString::from(label),
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
