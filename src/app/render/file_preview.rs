impl DiffViewer {
    fn render_file_preview(&mut self, cx: &mut Context<Self>) -> AnyElement {
        if self.file_preview_loading {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Loading file preview..."),
                )
                .into_any_element();
        }

        if let Some(error) = self.file_preview_error.as_ref() {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .p_6()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .whitespace_normal()
                        .child(error.clone()),
                )
                .into_any_element();
        }

        let Some(document) = self.file_preview_document.as_ref() else {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Select a file from Files tree to preview it."),
                )
                .into_any_element();
        };

        let file_path = self.file_preview_path.clone().unwrap_or_default();
        let line_count = document.lines.len().max(1);
        let line_digits = line_count.to_string().len() as f32;
        let line_number_width = (line_digits * DIFF_MONO_CHAR_WIDTH + 10.0).max(28.0);
        let list_state = self.file_preview_list_state.clone();
        let header_label = if document.truncated {
            format!(
                "{} • {} lines shown (truncated at {})",
                file_path,
                document.lines.len(),
                FILE_PREVIEW_MAX_LINES
            )
        } else {
            format!("{} • {} bytes", file_path, document.byte_len)
        };

        let list = list(list_state.clone(), {
            cx.processor(move |this, ix: usize, _window, cx| {
                let Some(document) = this.file_preview_document.as_ref() else {
                    return div().into_any_element();
                };
                let Some(_line) = document.lines.get(ix) else {
                    return div().into_any_element();
                };
                let Some(segments) = document.line_segments.get(ix) else {
                    return div().into_any_element();
                };
                this.render_file_preview_line(ix + 1, segments, line_number_width, cx)
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
        let right_inset = px(DIFF_SCROLLBAR_RIGHT_INSET);

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .child(
                        div()
                            .text_xs()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_color(cx.theme().muted_foreground)
                            .child(header_label),
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
                            .on_scroll_wheel(cx.listener(Self::on_file_preview_scroll_wheel))
                            .child(list),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .right(right_inset)
                            .bottom_0()
                            .w(scrollbar_size)
                            .child(
                                Scrollbar::vertical(&list_state)
                                    .scrollbar_show(ScrollbarShow::Always),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_file_preview_line(
        &self,
        line_number: usize,
        segments: &[CachedStyledSegment],
        line_number_width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_dark = cx.theme().mode.is_dark();
        let default_text = cx.theme().foreground;
        let gutter_bg = cx
            .theme()
            .background
            .blend(cx.theme().muted.opacity(if is_dark { 0.34 } else { 0.52 }));

        h_flex()
            .w_full()
            .items_start()
            .gap_2()
            .px_2()
            .py_0p5()
            .child(
                h_flex()
                    .items_start()
                    .gap_2()
                    .px_1p5()
                    .py_0p5()
                    .rounded_sm()
                    .bg(gutter_bg)
                    .border_1()
                    .border_color(cx.theme().border.opacity(if is_dark { 0.74 } else { 0.56 }))
                    .child(
                        div()
                            .w(px(line_number_width))
                            .text_xs()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_color(cx.theme().muted_foreground)
                            .whitespace_nowrap()
                            .child(format!("{line_number}")),
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
                    .text_color(default_text)
                    .when(self.diff_fit_to_width, |this| {
                        this.flex_wrap().whitespace_normal()
                    })
                    .when(!self.diff_fit_to_width, |this| {
                        this.flex_nowrap().whitespace_nowrap()
                    })
                    .children(segments.iter().map(|segment| {
                        let text = if self.diff_show_whitespace {
                            segment.whitespace_text.clone()
                        } else {
                            segment.plain_text.clone()
                        };
                        let color = self.syntax_color_for_segment(default_text, segment.syntax, cx);
                        div().flex_none().whitespace_nowrap().text_color(color).child(text)
                    }))
                    .when(self.diff_show_eol_markers, |this| {
                        this.child(
                            div()
                                .flex_none()
                                .whitespace_nowrap()
                                .text_color(
                                    cx.theme()
                                        .muted_foreground
                                        .opacity(if is_dark { 0.90 } else { 0.95 }),
                                )
                                .child("↵"),
                        )
                    }),
            )
            .into_any_element()
    }
}
