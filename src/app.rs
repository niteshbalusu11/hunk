use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context as _, Result};
use gpui::{
    AnyElement, AppContext as _, Application, Context, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, SharedString, Styled as _, Window, WindowOptions, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Root, StyledExt as _, Theme, ThemeMode,
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

use crate::diff::{DiffCell, DiffCellKind, DiffRowKind, SideBySideRow, parse_patch_side_by_side};
use crate::git::{ChangedFile, FileStatus, load_patch, load_snapshot};

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
    files: Vec<ChangedFile>,
    selected_path: Option<String>,
    selected_status: Option<FileStatus>,
    diff_rows: Vec<SideBySideRow>,
    error_message: Option<String>,
    tree_state: Entity<TreeState>,
}

impl DiffViewer {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|cx| TreeState::new(cx));

        let mut view = Self {
            repo_root: None,
            files: Vec::new(),
            selected_path: None,
            selected_status: None,
            diff_rows: Vec::new(),
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
                self.files = snapshot.files;
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
                self.files.clear();
                self.selected_path = None;
                self.selected_status = None;
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
            return;
        };

        let Some(path) = self.selected_path.as_ref() else {
            self.diff_rows = vec![message_row(
                DiffRowKind::Empty,
                "Select a file to view its diff.",
            )];
            return;
        };

        let status = self.selected_status.unwrap_or(FileStatus::Unknown);
        match load_patch(repo_root, path, status) {
            Ok(patch) => {
                self.diff_rows = parse_patch_side_by_side(&patch);
            }
            Err(err) => {
                self.diff_rows = vec![message_row(
                    DiffRowKind::Meta,
                    format!("Failed to load patch for {path}: {err:#}"),
                )];
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

        v_flex()
            .size_full()
            .overflow_y_scrollbar()
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
                            .child("Old"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .px_2()
                            .py_1()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("New"),
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
        let (background, foreground) = match row.kind {
            DiffRowKind::HunkHeader => (cx.theme().primary_hover, cx.theme().primary_foreground),
            DiffRowKind::Meta => (cx.theme().muted, cx.theme().muted_foreground),
            DiffRowKind::Empty => (cx.theme().background, cx.theme().muted_foreground),
            DiffRowKind::Code => (cx.theme().background, cx.theme().foreground),
        };

        div()
            .id(("diff-meta-row", ix))
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
            .child(self.render_diff_cell(ix, "left", &row.left, cx))
            .child(self.render_diff_cell(ix, "right", &row.right, cx))
            .into_any_element()
    }

    fn render_diff_cell(
        &self,
        row_ix: usize,
        side: &'static str,
        cell: &DiffCell,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let cell_id = if side == "left" {
            ("diff-cell-left", row_ix)
        } else {
            ("diff-cell-right", row_ix)
        };

        let (background, text_color, marker) = match cell.kind {
            DiffCellKind::Added => (cx.theme().success_hover, cx.theme().success, "+"),
            DiffCellKind::Removed => (cx.theme().danger_hover, cx.theme().danger, "-"),
            DiffCellKind::Context => (cx.theme().background, cx.theme().foreground, " "),
            DiffCellKind::None => (cx.theme().background, cx.theme().muted_foreground, ""),
        };

        let line_number = cell.line.map(|line| line.to_string()).unwrap_or_default();
        let content = if marker.is_empty() {
            String::new()
        } else {
            format!("{marker}{}", cell.text)
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
                    .text_color(cx.theme().muted_foreground)
                    .font_family(cx.theme().mono_font_family.clone())
                    .child(line_number),
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
