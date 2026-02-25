impl DiffViewer {
    fn render_tree(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tracked_files = self
            .files
            .iter()
            .filter(|file| file.is_tracked())
            .cloned()
            .collect::<Vec<_>>();
        let untracked_files = self
            .files
            .iter()
            .filter(|file| !file.is_tracked())
            .cloned()
            .collect::<Vec<_>>();
        let is_dark = cx.theme().mode.is_dark();
        let staged_count = self.files.iter().filter(|file| file.staged).count();
        let view = cx.entity();

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
                            .child(format!("{} changes • {} staged", self.files.len(), staged_count)),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .child(if staged_count == 0 {
                                let view = view.clone();
                                Button::new("stage-all")
                                    .outline()
                                    .compact()
                                    .rounded(px(7.0))
                                    .bg(cx.theme().secondary.opacity(if is_dark { 0.46 } else { 0.68 }))
                                    .border_color(cx.theme().border.opacity(if is_dark { 0.86 } else { 0.70 }))
                                    .disabled(self.git_action_loading || self.files.is_empty())
                                    .label("Stage All")
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.stage_all_files(cx);
                                        });
                                    })
                                    .into_any_element()
                            } else {
                                let view = view.clone();
                                Button::new("unstage-all")
                                    .outline()
                                    .compact()
                                    .rounded(px(7.0))
                                    .bg(cx.theme().secondary.opacity(if is_dark { 0.46 } else { 0.68 }))
                                    .border_color(cx.theme().border.opacity(if is_dark { 0.86 } else { 0.70 }))
                                    .disabled(self.git_action_loading || self.files.is_empty())
                                    .label("Unstage All")
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.unstage_all_files(cx);
                                        });
                                    })
                                    .into_any_element()
                            }),
                    ),
            )
            .when_some(self.git_status_message.as_ref(), |this, message| {
                this.child(
                    div()
                        .w_full()
                        .px_2()
                        .py_1()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().muted_foreground)
                        .child(message.clone()),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_1()
                            .px_1()
                            .py_1()
                            .child(self.render_changes_section("Tracked", &tracked_files, cx))
                            .child(self.render_changes_section("Untracked", &untracked_files, cx)),
                    ),
            )
            .child(
                v_flex()
                    .w_full()
                    .child(self.render_commit_footer(cx)),
            )
    }

    fn render_changes_section(
        &self,
        title: &'static str,
        files: &[ChangedFile],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_dark = cx.theme().mode.is_dark();

        v_flex()
            .w_full()
            .gap_1()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .px_1()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{}", files.len())),
                    ),
            )
            .when(files.is_empty(), |this| {
                this.child(
                    div()
                        .w_full()
                        .px_1()
                        .py_1()
                        .rounded_md()
                        .bg(cx.theme().muted.opacity(if is_dark { 0.24 } else { 0.36 }))
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().muted_foreground)
                        .child("No files"),
                )
            })
            .children(files.iter().map(|file| self.render_change_row(file, cx)))
            .into_any_element()
    }

    fn render_change_row(&self, file: &ChangedFile, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let is_selected = self.selected_path.as_deref() == Some(file.path.as_str());
        let is_dark = cx.theme().mode.is_dark();
        let is_collapsed = self.collapsed_files.contains(file.path.as_str());
        let git_action_loading = self.git_action_loading;
        let currently_staged = file.staged;
        let stage_checkbox_id = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::hash::Hash::hash(&file.path, &mut hasher);
            std::hash::Hasher::finish(&hasher)
        };

        let (status_label, accent) = match file.status {
            FileStatus::Added => ("ADD", cx.theme().success),
            FileStatus::Modified => ("MOD", cx.theme().warning),
            FileStatus::Deleted => ("DEL", cx.theme().danger),
            FileStatus::Renamed => ("REN", cx.theme().accent),
            FileStatus::Untracked => ("NEW", cx.theme().success),
            FileStatus::TypeChange => ("TYP", cx.theme().warning),
            FileStatus::Conflicted => ("CON", cx.theme().danger),
            FileStatus::Unknown => ("---", cx.theme().muted_foreground),
        };

        let row_bg = if is_selected {
            cx.theme().accent.opacity(if is_dark { 0.30 } else { 0.14 })
        } else {
            cx.theme().background.opacity(0.0)
        };

        let badge_bg = if is_selected {
            accent.opacity(if is_dark { 0.42 } else { 0.30 })
        } else {
            accent.opacity(if is_dark { 0.28 } else { 0.17 })
        };

        let (dir, file_name) = file.path.rsplit_once('/').map_or(("", file.path.as_str()), |parts| parts);

        h_flex()
            .id(("change-row", stage_checkbox_id))
            .w_full()
            .items_center()
            .gap_0p5()
            .px_1()
            .py_0p5()
            .rounded_sm()
            .bg(row_bg)
            .child({
                let path = file.path.clone();
                let view = view.clone();
                let check_color = if currently_staged {
                    if is_dark {
                        cx.theme().success.lighten(0.52)
                    } else {
                        cx.theme().success.darken(0.12)
                    }
                } else {
                    cx.theme().muted_foreground.opacity(0.58)
                };
                Button::new(("stage-file", stage_checkbox_id))
                    .compact()
                    .outline()
                    .rounded(px(5.0))
                    .label(if currently_staged { "✔" } else { " " })
                    .min_w(px(16.0))
                    .h(px(16.0))
                    .bg(if currently_staged {
                        cx.theme().success.opacity(if is_dark { 0.18 } else { 0.10 })
                    } else {
                        cx.theme().background.opacity(0.0)
                    })
                    .text_color(check_color)
                    .disabled(git_action_loading)
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        view.update(cx, |this, cx| {
                            this.toggle_stage_for_file(path.clone(), !currently_staged, cx);
                        });
                    })
            })
            .child(
                div()
                    .w_3()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(if is_collapsed { "▸" } else { "▾" }),
            )
            .child(
                div()
                    .min_w_8()
                    .px_1()
                    .py_0p5()
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .bg(badge_bg)
                    .border_1()
                    .border_color(accent.opacity(if is_dark { 0.88 } else { 0.50 }))
                    .rounded_sm()
                    .child(status_label),
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_0p5()
                    .child(div().text_xs().child(file_name.to_string()))
                    .when(!dir.is_empty(), |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(dir.to_string()),
                        )
                    }),
            )
            .on_click({
                let view = view.clone();
                let path = file.path.clone();
                move |_, _, cx| {
                    view.update(cx, |this, cx| {
                        this.select_file(path.clone(), cx);
                    });
                }
            })
            .into_any_element()
    }

}
