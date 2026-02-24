use super::*;
use gpui_component::button::{Button, ButtonVariants as _};

impl DiffViewer {
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
                        Button::new("toggle-diff-fit")
                            .ghost()
                            .label(if self.diff_fit_to_width { "Pan" } else { "Fit" })
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.toggle_diff_fit_to_width(cx);
                                });
                            }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} files", self.files.len())),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_color(if self.fps >= 110.0 {
                                cx.theme().success
                            } else if self.fps >= 60.0 {
                                cx.theme().warning
                            } else {
                                cx.theme().danger
                            })
                            .child(format!("{:>3.0} fps", self.fps.round())),
                    ),
            )
    }

    fn render_tree(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let selected_path = self.selected_path.clone();
        let status_by_path = self
            .files
            .iter()
            .map(|file| (file.path.clone(), file.status))
            .collect::<BTreeMap<_, _>>();
        let collapsed_by_path = self.collapsed_files.clone();
        let is_dark = cx.theme().mode.is_dark();

        v_flex().size_full().overflow_y_scrollbar().child(tree(
            &self.tree_state,
            move |ix, entry, _selected, _window, cx| {
                let item = entry.item();
                let item_id = item.id.to_string();
                let item_label = item.label.clone();
                let is_folder = entry.is_folder();
                let is_selected = selected_path.as_deref() == Some(item_id.as_str());
                let click_path = item_id.clone();
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
                                this.select_file(click_path.clone(), cx);
                            });
                        }
                    })
                    .child(if is_folder {
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_2()
                            .child(div().text_sm().child(icon))
                            .child(div().text_sm().child(item_label))
                            .into_any_element()
                    } else {
                        let status = status_by_path
                            .get(item_id.as_str())
                            .copied()
                            .unwrap_or(FileStatus::Unknown);
                        let is_collapsed = collapsed_by_path.contains(item_id.as_str());

                        let (status_label, accent) = match status {
                            FileStatus::Added => ("ADD", cx.theme().success),
                            FileStatus::Modified => ("MOD", cx.theme().warning),
                            FileStatus::Deleted => ("DEL", cx.theme().danger),
                            FileStatus::Renamed => ("REN", cx.theme().accent),
                            FileStatus::Untracked => ("NEW", cx.theme().success),
                            FileStatus::TypeChange => ("TYP", cx.theme().warning),
                            FileStatus::Conflicted => ("CON", cx.theme().danger),
                            FileStatus::Unknown => ("---", cx.theme().muted_foreground),
                        };

                        let badge_bg = if is_selected {
                            accent.opacity(if is_dark { 0.40 } else { 0.30 })
                        } else {
                            accent.opacity(if is_dark { 0.30 } else { 0.18 })
                        };

                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w_4()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(if is_collapsed { "▸" } else { "▾" }),
                            )
                            .child(
                                div()
                                    .min_w_10()
                                    .px_1p5()
                                    .py_0p5()
                                    .text_xs()
                                    .font_semibold()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .text_color(cx.theme().foreground)
                                    .bg(badge_bg)
                                    .border_1()
                                    .border_color(accent.opacity(if is_dark { 0.88 } else { 0.50 }))
                                    .rounded_sm()
                                    .child(status_label),
                            )
                            .child(div().text_sm().child(item_label))
                            .into_any_element()
                    })
            },
        ))
    }

    fn render_diff(&mut self, cx: &mut Context<Self>) -> AnyElement {
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
        let diff_scroll_handle = self.diff_scroll_handle.clone();
        let list = uniform_list("diff-rows", self.diff_rows.len(), {
            cx.processor(move |this, visible_range: Range<usize>, _window, cx| {
                if visible_range.start < visible_range.end {
                    this.sync_selected_file_from_visible_row(visible_range.start, cx);
                }

                let mut items = Vec::with_capacity(visible_range.len());
                for ix in visible_range {
                    let Some(row) = this.diff_rows.get(ix) else {
                        continue;
                    };

                    let row_element = match row.kind {
                        DiffRowKind::Code => this.render_code_row(ix, row, cx),
                        DiffRowKind::HunkHeader | DiffRowKind::Meta | DiffRowKind::Empty => {
                            this.render_meta_row(ix, row, cx)
                        }
                    };
                    items.push(row_element);
                }
                items
            })
        })
        .flex_grow()
        .size_full()
        .track_scroll(diff_scroll_handle.clone())
        .with_sizing_behavior(ListSizingBehavior::Auto);

        if self.diff_fit_to_width {
            return v_flex()
                .size_full()
                .child(self.render_file_status_banner(cx))
                .child(
                    v_flex()
                        .flex_1()
                        .min_h_0()
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
                        .child(
                            div()
                                .flex_1()
                                .min_h_0()
                                .on_scroll_wheel(cx.listener(Self::on_diff_scroll_wheel))
                                .child(list)
                                .vertical_scrollbar(&diff_scroll_handle),
                        ),
                )
                .into_any_element();
        }

        let horizontal_scroll_handle = self.diff_horizontal_scroll_handle.clone();

        v_flex()
            .size_full()
            .child(self.render_file_status_banner(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_hidden()
                    .child(
                        h_flex()
                            .id("diff-horizontal-scroll-area")
                            .size_full()
                            .overflow_x_scroll()
                            .overflow_y_hidden()
                            .track_scroll(&horizontal_scroll_handle)
                            .on_scroll_wheel(cx.listener(Self::on_diff_horizontal_scroll_wheel))
                            .child(
                                v_flex()
                                    .h_full()
                                    .min_w(px(DIFF_MIN_CONTENT_WIDTH))
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
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_h_0()
                                            .on_scroll_wheel(
                                                cx.listener(Self::on_diff_scroll_wheel),
                                            )
                                            .child(list)
                                            .vertical_scrollbar(&diff_scroll_handle),
                                    ),
                            ),
                    )
                    .horizontal_scrollbar(&horizontal_scroll_handle),
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
            .when(self.diff_fit_to_width, |this| this.truncate())
            .when(!self.diff_fit_to_width, |this| this.whitespace_nowrap())
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
                    .whitespace_nowrap()
                    .child(line_number),
            )
            .child(
                div()
                    .w_4()
                    .text_sm()
                    .text_color(marker_color)
                    .font_family(cx.theme().mono_font_family.clone())
                    .whitespace_nowrap()
                    .child(marker),
            )
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(text_color)
                    .font_family(cx.theme().mono_font_family.clone())
                    .when(self.diff_fit_to_width, |this| this.truncate())
                    .when(!self.diff_fit_to_width, |this| this.whitespace_nowrap())
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
        let view = cx.entity();
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
        let hint_text = if self.selected_file_is_collapsed() {
            "Collapsed in stream. Expand to render this file inline."
        } else {
            hint
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
                    .child(hint_text),
            )
            .child(self.render_line_stats("file", self.selected_line_stats, cx))
            .child(
                Button::new("toggle-file-collapse")
                    .ghost()
                    .label(if self.selected_file_is_collapsed() {
                        "Expand"
                    } else {
                        "Collapse"
                    })
                    .on_click(move |_, _, cx| {
                        view.update(cx, |this, cx| {
                            this.toggle_selected_file_collapsed(cx);
                        });
                    }),
            )
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
        self.clamp_diff_scroll_offset();
        self.clamp_diff_horizontal_scroll_offset();
        let current_scroll_offset = self.diff_scroll_handle.0.borrow().base_handle.offset();
        if self.last_diff_scroll_offset != Some(current_scroll_offset) {
            self.last_diff_scroll_offset = Some(current_scroll_offset);
            self.last_scroll_activity_at = Instant::now();
        }
        self.frame_sample_count = self.frame_sample_count.saturating_add(1);

        div()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.render_toolbar(cx))
            .child(
                h_resizable("hunk-main")
                    .child(
                        resizable_panel()
                            .size(px(280.0))
                            .size_range(px(160.0)..px(520.0))
                            .child(self.render_tree(cx)),
                    )
                    .child(resizable_panel().child(self.render_diff(cx))),
            )
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
