struct DiffCellRenderSpec<'a> {
    row_ix: usize,
    side: &'static str,
    cell: &'a DiffCell,
    peer_text: &'a str,
    peer_kind: DiffCellKind,
    column_width: Option<f32>,
}

impl DiffViewer {
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
        let diff_list_state = self.diff_list_state.clone();
        let visible_row = diff_list_state.logical_scroll_top().item_ix;
        if visible_row < self.diff_rows.len() {
            self.sync_selected_file_from_visible_row(visible_row, cx);
        }
        let sticky_hunk_banner = self.render_visible_hunk_banner(visible_row, cx);

        let list = list(diff_list_state.clone(), {
            cx.processor(move |this, ix: usize, _window, cx| {
                let Some(row) = this.diff_rows.get(ix) else {
                    return div().into_any_element();
                };
                let is_selected = this.is_row_selected(ix);

                match row.kind {
                    DiffRowKind::Code => this.render_code_row(ix, row, is_selected, cx),
                    DiffRowKind::HunkHeader | DiffRowKind::Meta | DiffRowKind::Empty => {
                        this.render_meta_row(ix, row, is_selected, cx)
                    }
                }
            })
        })
        .flex_grow()
        .size_full()
        .map(|mut this| {
            this.style().restrict_scroll_to_axis = Some(true);
            this
        })
        .with_sizing_behavior(ListSizingBehavior::Auto);

        let scrollbar_size = px(DIFF_SCROLLBAR_SIZE);
        let edge_inset = px(DIFF_BOTTOM_SAFE_INSET);
        let right_inset = px(DIFF_SCROLLBAR_RIGHT_INSET);
        let vertical_bar_bottom = edge_inset + px(DIFF_VERTICAL_SCROLLBAR_EXTRA_BOTTOM_INSET);

        if self.diff_fit_to_width {
            return v_flex()
                .size_full()
                .child(self.render_file_status_banner(cx))
                .child(sticky_hunk_banner)
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
                                .relative()
                                .child(
                                    div()
                                        .size_full()
                                        .on_scroll_wheel(
                                            cx.listener(Self::on_diff_list_scroll_wheel),
                                        )
                                        .child(list),
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .top_0()
                                        .right(right_inset)
                                        .bottom(vertical_bar_bottom)
                                        .w(scrollbar_size)
                                        .child(
                                            Scrollbar::vertical(&diff_list_state)
                                                .scrollbar_show(ScrollbarShow::Always),
                                        ),
                                ),
                        ),
                )
                .into_any_element();
        }

        let horizontal_scroll_handle = self.diff_horizontal_scroll_handle.clone();

        v_flex()
            .size_full()
            .child(self.render_file_status_banner(cx))
            .child(sticky_hunk_banner)
            .child(
                v_flex().flex_1().min_h_0().child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .relative()
                        .child(
                            div().size_full().overflow_y_hidden().child(
                                h_flex()
                                    .id("diff-horizontal-scroll-area")
                                    .size_full()
                                    .overflow_x_scroll()
                                    .overflow_y_hidden()
                                    .map(|mut this| {
                                        this.style().restrict_scroll_to_axis = Some(true);
                                        this
                                    })
                                    .track_scroll(&horizontal_scroll_handle)
                                    .on_scroll_wheel(
                                        cx.listener(Self::on_diff_horizontal_scroll_wheel),
                                    )
                                    .child(
                                        v_flex()
                                            .h_full()
                                            .w(px(self.diff_pan_content_width))
                                            .min_w(px(self.diff_pan_content_width))
                                            .child(
                                                h_flex()
                                                    .w(px(self.diff_pan_content_width))
                                                    .min_w(px(self.diff_pan_content_width))
                                                    .border_b_1()
                                                    .border_color(cx.theme().border)
                                                    .child(
                                                        div()
                                                            .w(px(self.diff_left_column_width))
                                                            .min_w(px(self.diff_left_column_width))
                                                            .max_w(px(self.diff_left_column_width))
                                                            .px_2()
                                                            .py_1()
                                                            .text_xs()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(old_label),
                                                    )
                                                    .child(
                                                        div()
                                                            .w(px(self.diff_right_column_width))
                                                            .min_w(px(self.diff_right_column_width))
                                                            .max_w(px(self.diff_right_column_width))
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
                                                        cx.listener(
                                                            Self::on_diff_list_scroll_wheel,
                                                        ),
                                                    )
                                                    .child(list),
                                            ),
                                    ),
                            ),
                        )
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .right(right_inset)
                                .bottom(vertical_bar_bottom)
                                .w(scrollbar_size)
                                .child(
                                    Scrollbar::vertical(&diff_list_state)
                                        .scrollbar_show(ScrollbarShow::Always),
                                ),
                        ),
                ),
            )
            .into_any_element()
    }

    fn render_visible_hunk_banner(&self, visible_row: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some((path, header)) = self.visible_hunk_header(visible_row) else {
            return div().w_full().h(px(0.)).into_any_element();
        };

        let is_dark = cx.theme().mode.is_dark();
        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx
                .theme()
                .background
                .blend(
                    cx.theme()
                        .primary
                        .opacity(if is_dark { 0.24 } else { 0.10 }),
                ))
            .child(
                div()
                    .px_2()
                    .py_0p5()
                    .text_xs()
                    .font_semibold()
                    .font_family(cx.theme().mono_font_family.clone())
                    .bg(cx
                        .theme()
                        .primary
                        .opacity(if is_dark { 0.42 } else { 0.24 }))
                    .text_color(cx.theme().primary_foreground)
                    .child("HUNK"),
            )
            .child(
                div()
                    .text_xs()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(cx.theme().muted_foreground)
                    .child(path),
            )
            .child(
                div()
                    .text_xs()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(if is_dark {
                        cx.theme().primary.lighten(0.48)
                    } else {
                        cx.theme().primary.darken(0.12)
                    })
                    .child(header),
            )
            .into_any_element()
    }

    fn visible_hunk_header(&self, visible_row: usize) -> Option<(String, String)> {
        if self.diff_rows.is_empty() {
            return None;
        }

        let capped = visible_row.min(self.diff_rows.len().saturating_sub(1));

        if self.diff_row_metadata.len() == self.diff_rows.len() {
            let current_file = self
                .diff_row_metadata
                .get(capped)
                .and_then(|row| row.file_path.clone());

            for ix in (0..=capped).rev() {
                let meta = self.diff_row_metadata.get(ix)?;
                if current_file.is_some() && meta.file_path != current_file {
                    break;
                }

                if meta.kind == DiffStreamRowKind::CoreHunkHeader {
                    let path = meta
                        .file_path
                        .clone()
                        .or_else(|| self.selected_path.clone())
                        .unwrap_or_else(|| "file".to_string());
                    let header = self.diff_rows.get(ix)?.text.clone();
                    return Some((path, header));
                }

                if matches!(meta.kind, DiffStreamRowKind::FileHeader) {
                    break;
                }
            }
        }

        for ix in (0..=capped).rev() {
            let row = self.diff_rows.get(ix)?;
            if row.kind == DiffRowKind::HunkHeader {
                let path = self
                    .selected_path
                    .clone()
                    .unwrap_or_else(|| "file".to_string());
                return Some((path, row.text.clone()));
            }
        }

        None
    }

    fn render_meta_row(
        &self,
        ix: usize,
        row: &SideBySideRow,
        is_selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let stable_row_id = self.diff_row_stable_id(ix);
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
            .id(("diff-meta-row", stable_row_id))
            .relative()
            .overflow_x_hidden()
            .on_mouse_down(MouseButton::Left, {
                let row_ix = ix;
                cx.listener(move |this, event, window, cx| {
                    this.on_diff_row_mouse_down(row_ix, event, window, cx);
                })
            })
            .on_mouse_move({
                let row_ix = ix;
                cx.listener(move |this, event, window, cx| {
                    this.on_diff_row_mouse_move(row_ix, event, window, cx);
                })
            })
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_diff_row_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_diff_row_mouse_up))
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(if is_selected {
                background.blend(
                    cx.theme()
                        .primary
                        .opacity(if is_dark { 0.32 } else { 0.18 }),
                )
            } else {
                background
            })
            .text_sm()
            .text_color(foreground)
            .font_family(cx.theme().mono_font_family.clone())
            .when(self.diff_fit_to_width, |this| this.w_full())
            .when(!self.diff_fit_to_width, |this| {
                this.w(px(self.diff_pan_content_width))
                    .min_w(px(self.diff_pan_content_width))
            })
            .when(self.diff_fit_to_width, |this| this.whitespace_normal())
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
        row_data: &SideBySideRow,
        is_selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let stable_row_id = self.diff_row_stable_id(ix);
        let row = h_flex()
            .id(("diff-code-row", stable_row_id))
            .overflow_x_hidden()
            .on_mouse_down(MouseButton::Left, {
                let row_ix = ix;
                cx.listener(move |this, event, window, cx| {
                    this.on_diff_row_mouse_down(row_ix, event, window, cx);
                })
            })
            .on_mouse_move({
                let row_ix = ix;
                cx.listener(move |this, event, window, cx| {
                    this.on_diff_row_mouse_move(row_ix, event, window, cx);
                })
            })
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_diff_row_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_diff_row_mouse_up))
            .border_b_1()
            .border_color(cx.theme().border)
            .when(self.diff_fit_to_width, |this| this.w_full())
            .when(!self.diff_fit_to_width, |this| {
                this.w(px(self.diff_pan_content_width))
                    .min_w(px(self.diff_pan_content_width))
            });

        if self.diff_fit_to_width {
            return row
                .child(self.render_diff_cell(
                    stable_row_id,
                    is_selected,
                    DiffCellRenderSpec {
                        row_ix: ix,
                        side: "left",
                        cell: &row_data.left,
                        peer_text: &row_data.right.text,
                        peer_kind: row_data.right.kind,
                        column_width: None,
                    },
                    cx,
                ))
                .child(self.render_diff_cell(
                    stable_row_id,
                    is_selected,
                    DiffCellRenderSpec {
                        row_ix: ix,
                        side: "right",
                        cell: &row_data.right,
                        peer_text: &row_data.left.text,
                        peer_kind: row_data.left.kind,
                        column_width: None,
                    },
                    cx,
                ))
                .into_any_element();
        }

        row.child(self.render_diff_cell(
            stable_row_id,
            is_selected,
            DiffCellRenderSpec {
                row_ix: ix,
                side: "left",
                cell: &row_data.left,
                peer_text: &row_data.right.text,
                peer_kind: row_data.right.kind,
                column_width: Some(self.diff_left_column_width),
            },
            cx,
        ))
        .child(self.render_diff_cell(
            stable_row_id,
            is_selected,
            DiffCellRenderSpec {
                row_ix: ix,
                side: "right",
                cell: &row_data.right,
                peer_text: &row_data.left.text,
                peer_kind: row_data.left.kind,
                column_width: Some(self.diff_right_column_width),
            },
            cx,
        ))
        .into_any_element()
    }

    fn render_diff_cell(
        &self,
        row_stable_id: u64,
        row_is_selected: bool,
        spec: DiffCellRenderSpec<'_>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let side = spec.side;
        let cell = spec.cell;
        let peer_text = spec.peer_text;
        let peer_kind = spec.peer_kind;
        let column_width = spec.column_width;
        let file_path = self
            .diff_row_metadata
            .get(spec.row_ix)
            .and_then(|meta| meta.file_path.as_deref());
        let cell_id = if side == "left" {
            ("diff-cell-left", row_stable_id)
        } else {
            ("diff-cell-right", row_stable_id)
        };

        let is_dark = cx.theme().mode.is_dark();
        let add_alpha = if is_dark { 0.22 } else { 0.12 };
        let remove_alpha = if is_dark { 0.22 } else { 0.12 };
        let ghost_alpha = if is_dark { 0.10 } else { 0.06 };

        let (mut background, marker_color, line_color, text_color, marker) =
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
                    "",
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
                    "",
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
        if row_is_selected {
            background =
                background.blend(
                    cx.theme()
                        .primary
                        .opacity(if is_dark { 0.25 } else { 0.15 }),
                );
        }

        let line_number = cell.line.map(|line| line.to_string()).unwrap_or_default();
        let styled_segments =
            build_line_segments(file_path, &cell.text, cell.kind, peer_text, peer_kind);
        let line_number_width = if side == "left" {
            self.diff_left_line_number_width
        } else {
            self.diff_right_line_number_width
        };

        let should_draw_right_divider = side == "left";
        let gutter_background = cx
            .theme()
            .background
            .blend(cx.theme().muted.opacity(if is_dark { 0.26 } else { 0.52 }));
        let gutter_width = line_number_width + DIFF_MARKER_GUTTER_WIDTH + 12.0;

        let base = h_flex()
            .id(cell_id)
            .overflow_x_hidden()
            .px_2()
            .py_1()
            .gap_2()
            .items_start()
            .bg(background)
            .when(should_draw_right_divider, |this| {
                this.border_r_1().border_color(cx.theme().border)
            })
            .child(
                h_flex()
                    .items_start()
                    .gap_2()
                    .w(px(gutter_width))
                    .min_w(px(gutter_width))
                    .px_1p5()
                    .py_0p5()
                    .rounded_sm()
                    .bg(gutter_background)
                    .child(
                        div()
                            .w(px(line_number_width))
                            .text_xs()
                            .text_color(line_color)
                            .font_family(cx.theme().mono_font_family.clone())
                            .whitespace_nowrap()
                            .child(line_number),
                    )
                    .child(
                        div()
                            .w(px(DIFF_MARKER_GUTTER_WIDTH))
                            .text_xs()
                            .text_color(marker_color)
                            .font_family(cx.theme().mono_font_family.clone())
                            .whitespace_nowrap()
                            .child(marker),
                    ),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .items_start()
                    .gap_0()
                    .text_sm()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(text_color)
                    .when(self.diff_fit_to_width, |this| {
                        this.flex_wrap().whitespace_normal()
                    })
                    .when(!self.diff_fit_to_width, |this| {
                        this.flex_nowrap().whitespace_nowrap()
                    })
                    .children(styled_segments.into_iter().map(|segment| {
                        let segment_color =
                            self.syntax_color_for_segment(text_color, segment.syntax, cx);
                        div()
                            .flex_none()
                            .whitespace_nowrap()
                            .text_color(segment_color)
                            .when(segment.changed, |this| {
                                this.bg(marker_color.opacity(if is_dark { 0.16 } else { 0.12 }))
                            })
                            .child(segment.text)
                    })),
            );

        if let Some(width) = column_width {
            return base
                .w(px(width))
                .min_w(px(width))
                .max_w(px(width))
                .into_any_element();
        }

        base.flex_1().min_w_0().into_any_element()
    }

    fn syntax_color_for_segment(
        &self,
        default_color: gpui::Hsla,
        token: SyntaxTokenKind,
        cx: &mut Context<Self>,
    ) -> gpui::Hsla {
        let is_dark = cx.theme().mode.is_dark();
        let github = |dark: u32, light: u32| -> gpui::Hsla {
            if is_dark {
                gpui::rgb(dark).into()
            } else {
                gpui::rgb(light).into()
            }
        };
        match token {
            SyntaxTokenKind::Plain => default_color,
            SyntaxTokenKind::Keyword => github(0xff7b72, 0xcf222e),
            SyntaxTokenKind::String => github(0xa5d6ff, 0x0a3069),
            SyntaxTokenKind::Number => github(0x79c0ff, 0x0550ae),
            SyntaxTokenKind::Comment => github(0x8b949e, 0x57606a),
            SyntaxTokenKind::Function => github(0xd2a8ff, 0x8250df),
            SyntaxTokenKind::TypeName => github(0xffa657, 0x953800),
            SyntaxTokenKind::Constant => github(0x79c0ff, 0x0550ae),
            SyntaxTokenKind::Variable => github(0xffa657, 0x953800),
            SyntaxTokenKind::Operator => github(0xff7b72, 0xcf222e),
        }
    }

    fn diff_row_stable_id(&self, row_ix: usize) -> u64 {
        self.diff_row_metadata
            .get(row_ix)
            .map(|row| row.stable_id)
            .unwrap_or(row_ix as u64)
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
}
