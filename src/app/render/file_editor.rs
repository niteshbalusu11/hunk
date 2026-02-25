impl DiffViewer {
    fn render_file_editor(&mut self, cx: &mut Context<Self>) -> AnyElement {
        if self.editor_loading {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Loading file editor..."),
                )
                .into_any_element();
        }

        if let Some(error) = self.editor_error.as_ref() {
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

        let Some(file_path) = self.editor_path.clone() else {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Select a file from Files tree to edit it."),
                )
                .into_any_element();
        };

        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let status_color = if self.editor_save_loading {
            cx.theme().warning
        } else if self.editor_dirty {
            cx.theme().danger
        } else {
            cx.theme().success
        };
        let status_label = if self.editor_save_loading {
            "Saving..."
        } else if self.editor_dirty {
            "Unsaved changes"
        } else {
            "Saved"
        };
        let save_disabled = self.editor_save_loading || !self.editor_dirty;
        let reload_disabled = self.editor_save_loading;

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .child(
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .truncate()
                                    .text_xs()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .text_color(cx.theme().muted_foreground)
                                    .child(file_path),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(status_color)
                                    .child(status_label),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .child({
                                let view = view.clone();
                                Button::new("editor-reload")
                                    .outline()
                                    .compact()
                                    .rounded(px(7.0))
                                    .bg(
                                        cx.theme().secondary.opacity(if is_dark { 0.46 } else { 0.68 }),
                                    )
                                    .border_color(
                                        cx.theme().border.opacity(if is_dark { 0.86 } else { 0.70 }),
                                    )
                                    .label("Reload")
                                    .disabled(reload_disabled)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.reload_current_editor_file(cx);
                                        });
                                    })
                            })
                            .child({
                                let view = view.clone();
                                Button::new("editor-save")
                                    .primary()
                                    .compact()
                                    .rounded(px(7.0))
                                    .label("Save")
                                    .disabled(save_disabled)
                                    .on_click(move |_, window, cx| {
                                        view.update(cx, |this, cx| {
                                            this.save_current_editor_file(window, cx);
                                        });
                                    })
                            }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .child(
                        Input::new(&self.editor_input_state)
                            .h_full()
                            .disabled(self.editor_loading || self.editor_save_loading)
                            .rounded(px(8.0))
                            .border_1()
                            .border_color(cx.theme().border.opacity(if is_dark { 0.92 } else { 0.78 }))
                            .bg(cx.theme().background.blend(cx.theme().muted.opacity(if is_dark {
                                0.22
                            } else {
                                0.10
                            }))),
                    ),
            )
            .into_any_element()
    }
}
