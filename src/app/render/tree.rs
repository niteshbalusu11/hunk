impl DiffViewer {
    fn render_tree(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark = cx.theme().mode.is_dark();
        let tree_summary = format!(
            "{} files • {} folders",
            self.repo_tree_file_count, self.repo_tree_folder_count
        );

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_2()
                    .py_1p5()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().sidebar.blend(cx.theme().muted.opacity(if is_dark {
                        0.18
                    } else {
                        0.30
                    })))
                    .child(
                        div()
                            .text_xs()
                            .font_medium()
                            .text_color(cx.theme().muted_foreground)
                            .child(tree_summary),
                    ),
            )
            .child(div().flex_1().min_h_0().child(self.render_repo_tree_content(cx)))
    }

    fn render_repo_tree_content(&mut self, cx: &mut Context<Self>) -> AnyElement {
        if self.repo_tree_loading {
            return v_flex()
                .w_full()
                .px_2()
                .py_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Loading repository tree..."),
                )
                .into_any_element();
        }

        if let Some(error) = self.repo_tree_error.as_ref() {
            return v_flex()
                .w_full()
                .px_2()
                .py_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .whitespace_normal()
                        .child(error.clone()),
                )
                .into_any_element();
        }

        if self.repo_tree_rows.is_empty() {
            return v_flex()
                .w_full()
                .px_2()
                .py_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("No files found."),
                )
                .into_any_element();
        }

        self.sync_sidebar_repo_list_state(self.repo_tree_rows.len());
        let list_state = self.sidebar_repo_list_state.clone();

        let list = list(list_state.clone(), {
            cx.processor(move |this, ix: usize, _window, cx| {
                this.repo_tree_rows
                    .get(ix)
                    .map(|row| this.render_repo_tree_row(row, cx))
                    .unwrap_or_else(|| div().into_any_element())
            })
        })
        .size_full()
        .map(|mut list| {
            list.style().restrict_scroll_to_axis = Some(true);
            list
        })
        .with_sizing_behavior(ListSizingBehavior::Auto);

        div()
            .size_full()
            .overflow_y_scrollbar()
            .px_1()
            .py_1()
            .child(list)
            .into_any_element()
    }

    fn sync_sidebar_repo_list_state(&mut self, row_count: usize) {
        if self.sidebar_repo_row_count == row_count {
            return;
        }
        self.sidebar_repo_row_count = row_count;
        Self::sync_sidebar_list_state(&self.sidebar_repo_list_state, row_count);
    }

    fn sync_sidebar_list_state(list_state: &ListState, row_count: usize) {
        let previous_top = list_state.logical_scroll_top();
        list_state.reset(row_count);
        let clamped_item_ix = if row_count == 0 {
            0
        } else {
            previous_top.item_ix.min(row_count.saturating_sub(1))
        };
        list_state.scroll_to(ListOffset {
            item_ix: clamped_item_ix,
            offset_in_item: px(0.),
        });
    }

    fn render_repo_tree_row(
        &self,
        row: &super::data::RepoTreeRow,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let is_selected =
            row.kind == RepoTreeNodeKind::File && self.selected_path.as_deref() == Some(row.path.as_str());
        let row_bg = if is_selected {
            cx.theme().accent.opacity(if is_dark { 0.30 } else { 0.14 })
        } else if row.ignored {
            cx.theme().muted.opacity(if is_dark { 0.16 } else { 0.22 })
        } else {
            cx.theme().background.opacity(0.0)
        };
        let text_color = if row.ignored {
            cx.theme().muted_foreground.opacity(if is_dark { 0.88 } else { 0.95 })
        } else {
            cx.theme().foreground
        };
        let icon_color = cx.theme().muted_foreground;
        let chevron_icon = if row.kind == RepoTreeNodeKind::Directory {
            Some(if row.expanded {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            })
        } else {
            None
        };
        let icon = match row.kind {
            RepoTreeNodeKind::Directory => {
                if row.expanded {
                    IconName::FolderOpen
                } else {
                    IconName::FolderClosed
                }
            }
            RepoTreeNodeKind::File => file_icon_for_path(row.path.as_str()),
        };
        let row_id = stable_row_id_for_path(row.path.as_str());
        let file_status = row.file_status;

        h_flex()
            .id(("repo-tree-row", row_id))
            .w_full()
            .items_center()
            .gap_1()
            .px_1()
            .py_0p5()
            .rounded_sm()
            .bg(row_bg)
            .child(div().w(px(row.depth as f32 * 14.0)))
            .child(div().w(px(14.0)).when_some(chevron_icon, |this, icon_name| {
                this.child(
                    Icon::new(icon_name)
                        .size(px(12.0))
                        .text_color(cx.theme().muted_foreground),
                )
            }))
            .child(
                div().w(px(18.0)).child(
                    Icon::new(icon)
                        .size(px(14.0))
                        .text_color(icon_color),
                ),
            )
            .when_some(file_status, |this, status| {
                let (status_label, status_color) = change_status_label_color(status, cx);
                this.child(
                    div()
                        .px_1()
                        .py_0p5()
                        .rounded(px(4.0))
                        .text_xs()
                        .font_semibold()
                        .bg(status_color.opacity(if is_dark { 0.24 } else { 0.16 }))
                        .text_color(cx.theme().foreground)
                        .child(status_label),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    .truncate()
                    .text_color(text_color)
                    .child(row.name.clone()),
            )
            .on_click({
                let view = view.clone();
                let path = row.path.clone();
                let kind = row.kind;
                move |_, _, cx| {
                    view.update(cx, |this, cx| match kind {
                        RepoTreeNodeKind::Directory => {
                            this.toggle_repo_tree_directory(path.clone(), cx);
                        }
                        RepoTreeNodeKind::File => {
                            this.select_repo_tree_file(path.clone(), cx);
                        }
                    });
                }
            })
            .into_any_element()
    }
}

fn stable_row_id_for_path(path: &str) -> u64 {
    use std::hash::{Hash as _, Hasher as _};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

fn path_extension(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
}

fn file_icon_for_path(path: &str) -> IconName {
    match path_extension(path).as_deref() {
        Some("toml") | Some("yaml") | Some("yml") | Some("json") | Some("lock") => {
            IconName::Settings
        }
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") => {
            IconName::GalleryVerticalEnd
        }
        Some("md") => IconName::BookOpen,
        _ => IconName::File,
    }
}
