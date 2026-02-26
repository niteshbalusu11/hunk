impl DiffViewer {
    fn render_commit_footer(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let show_publish = !self.branch_has_upstream;
        let show_push = self.branch_has_upstream && self.branch_ahead_count > 0;
        let action_label = if show_publish { "Publish" } else { "Push" };
        let last_commit_text = self
            .last_commit_subject
            .as_deref()
            .map(str::trim_end)
            .filter(|text| !text.is_empty())
            .unwrap_or("No commits yet");
        let included_count = self.included_commit_file_count();
        let total_count = self.files.len();

        v_flex()
            .w_full()
            .gap_2()
            .px_2()
            .pt_2()
            .pb_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar.blend(cx.theme().muted.opacity(if is_dark {
                0.16
            } else {
                0.24
            })))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_1()
                    .child({
                        let view = view.clone();
                        Button::new("branch-picker-toggle")
                            .outline()
                            .compact()
                            .rounded(px(7.0))
                            .bg(cx.theme().secondary.opacity(if is_dark { 0.50 } else { 0.70 }))
                            .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.74 }))
                            .dropdown_caret(true)
                            .label(self.branch_name.clone())
                            .disabled(self.git_action_loading)
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.toggle_branch_picker(cx);
                                });
                            })
                    })
                    .when(show_publish || show_push, |this| {
                        this.child({
                            let view = view.clone();
                            Button::new("publish-or-push")
                                .primary()
                                .compact()
                                .rounded(px(7.0))
                                .label(action_label)
                                .disabled(self.git_action_loading)
                                .on_click(move |_, _, cx| {
                                    view.update(cx, |this, cx| {
                                        this.push_or_publish_current_branch(cx);
                                    });
                                })
                            })
                    }),
            )
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Commit includes {included_count}/{total_count} files")),
                    )
                    .when(included_count < total_count, |this| {
                        this.child({
                            let view = view.clone();
                            Button::new("commit-include-all")
                                .outline()
                                .compact()
                                .rounded(px(7.0))
                                .label("Include All")
                                .disabled(self.git_action_loading)
                                .on_click(move |_, _, cx| {
                                    view.update(cx, |this, cx| {
                                        this.include_all_files_for_commit(cx);
                                    });
                                })
                        })
                    }),
            )
            .when(self.branch_picker_open, |this| {
                this.child(self.render_branch_picker_panel(cx))
            })
            .child(
                Input::new(&self.commit_input_state)
                    .h(px(82.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(cx.theme().border.opacity(if is_dark { 0.92 } else { 0.78 }))
                    .bg(cx.theme().background.blend(cx.theme().muted.opacity(if is_dark {
                        0.24
                    } else {
                        0.12
                    })))
                    .disabled(self.git_action_loading),
            )
            .child({
                let view = view.clone();
                Button::new("commit-staged")
                    .primary()
                    .rounded(px(7.0))
                    .label("Commit")
                    .disabled(self.git_action_loading)
                    .on_click(move |_, window, cx| {
                        view.update(cx, |this, cx| {
                            this.commit_from_input(window, cx);
                        });
                    })
            })
            .child(
                div()
                    .w_full()
                    .min_h(px(28.0))
                    .px_2()
                    .py_1()
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(cx.theme().border.opacity(if is_dark { 0.92 } else { 0.76 }))
                    .bg(cx.theme().secondary.opacity(if is_dark { 0.42 } else { 0.56 }))
                    .text_xs()
                    .font_medium()
                    .text_color(cx.theme().foreground.opacity(0.90))
                    .whitespace_normal()
                    .child(last_commit_text.to_string()),
            )
            .into_any_element()
    }

    fn render_branch_picker_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();

        v_flex()
            .w_full()
            .gap_1()
            .p_2()
            .rounded(px(8.0))
            .border_1()
            .border_color(cx.theme().border.opacity(if is_dark { 0.94 } else { 0.74 }))
            .bg(cx.theme().background.blend(cx.theme().secondary.opacity(if is_dark {
                0.32
            } else {
                0.20
            })))
            .child(
                div()
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().muted_foreground)
                    .child("Branches"),
            )
            .child(
                div()
                    .max_h(px(144.0))
                    .overflow_y_scrollbar()
                    .child(
                        v_flex().w_full().gap_1().children(
                            self.branches
                                .iter()
                                .enumerate()
                                .map(|(ix, branch)| {
                                    let view = view.clone();
                                    let branch_name = branch.name.clone();

                                    h_flex()
                                        .id(("branch-row", ix))
                                        .w_full()
                                        .min_w_0()
                                        .items_center()
                                        .gap_1()
                                        .px_2()
                                        .py_0p5()
                                        .rounded(px(6.0))
                                        .bg(if branch.is_current {
                                            cx.theme().accent.opacity(if is_dark { 0.28 } else { 0.18 })
                                        } else {
                                            cx.theme().background.opacity(0.0)
                                        })
                                        .on_click(move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                this.checkout_branch(branch_name.clone(), cx);
                                            });
                                        })
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .truncate()
                                                .text_xs()
                                                .font_medium()
                                                .text_color(cx.theme().foreground)
                                                .child(branch.name.clone()),
                                        )
                                        .child(
                                            div()
                                                .flex_none()
                                                .pl_2()
                                                .whitespace_nowrap()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(relative_time_label(branch.tip_unix_time)),
                                        )
                                        .into_any_element()
                                }),
                        ),
                    ),
            )
            .child(
                Input::new(&self.branch_input_state)
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(cx.theme().border.opacity(if is_dark { 0.92 } else { 0.76 }))
                    .bg(cx.theme().background.blend(cx.theme().muted.opacity(if is_dark {
                        0.22
                    } else {
                        0.14
                    })))
                    .disabled(self.git_action_loading),
            )
            .child({
                let view = view.clone();
                Button::new("create-or-switch-branch")
                    .primary()
                    .rounded(px(7.0))
                    .label("Create / Switch")
                    .disabled(self.git_action_loading)
                    .on_click(move |_, window, cx| {
                        view.update(cx, |this, cx| {
                            this.create_or_switch_branch_from_input(window, cx);
                        });
                    })
            })
            .into_any_element()
    }

}
