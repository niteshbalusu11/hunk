impl DiffViewer {
    fn git_action_loading_named(&self, action_label: &str) -> bool {
        self.git_action_loading
            && self
                .git_action_label
                .as_deref()
                .is_some_and(|label| label.eq_ignore_ascii_case(action_label))
    }

    fn render_git_action_status_banner(&self, cx: &mut Context<Self>) -> AnyElement {
        let is_dark = cx.theme().mode.is_dark();
        let loading = self.git_action_loading;
        let headline = if loading {
            match self.git_action_label.as_deref() {
                Some(label) => format!("{label}..."),
                None => "Running workspace action...".to_string(),
            }
        } else {
            self.git_status_message
                .clone()
                .unwrap_or_else(|| "Ready.".to_string())
        };
        let detail = if loading {
            self.git_status_message.clone()
        } else {
            self.git_action_label.clone()
        };
        let detail_text = detail.unwrap_or_else(|| {
            "Actions update this banner when operations complete.".to_string()
        });

        v_flex()
            .w_full()
            .h(px(52.0))
            .overflow_hidden()
            .px_2()
            .py_1()
            .gap_0p5()
            .rounded(px(8.0))
            .border_1()
            .border_color(if loading {
                hunk_opacity(cx.theme().accent, is_dark, 0.90, 0.72)
            } else {
                hunk_opacity(cx.theme().border, is_dark, 0.90, 0.70)
            })
            .bg(if loading {
                hunk_opacity(cx.theme().accent, is_dark, 0.22, 0.12)
            } else {
                hunk_blend(cx.theme().background, cx.theme().muted, is_dark, 0.24, 0.32)
            })
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .text_xs()
                    .font_medium()
                    .text_color(if loading {
                        cx.theme().foreground
                    } else {
                        cx.theme().muted_foreground
                    })
                    .whitespace_nowrap()
                    .truncate()
                    .child(headline),
            )
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground.opacity(0.9))
                    .whitespace_nowrap()
                    .truncate()
                    .child(detail_text),
            )
            .into_any_element()
    }

    fn render_git_workspace_operations_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        self.render_git_workspace_operations_panel_v2(cx)
    }

    fn render_workspace_changes_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let tracked_count = self.files.iter().filter(|file| file.is_tracked()).count();
        let untracked_count = self.files.len().saturating_sub(tracked_count);
        let is_dark = cx.theme().mode.is_dark();

        v_flex()
            .w_full()
            .gap_1()
            .p_2()
            .rounded(px(8.0))
            .border_1()
            .border_color(hunk_opacity(cx.theme().border, is_dark, 0.90, 0.74))
            .bg(hunk_blend(
                cx.theme().background,
                cx.theme().muted,
                is_dark,
                0.20,
                0.26,
            ))
            .child(
                div()
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().muted_foreground)
                    .child("Working Tree"),
            )
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_1()
                    .flex_wrap()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "{} files (tracked: {}, untracked: {})",
                                self.files.len(),
                                tracked_count,
                                untracked_count
                            )),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground.opacity(0.9))
                            .child("Single unified working tree list"),
                    ),
            )
            .child({
                let list_container = if self.files.is_empty() {
                    v_flex()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("No tracked or untracked changes."),
                        )
                        .into_any_element()
                } else {
                    v_flex()
                        .id("git-working-copy-scroll")
                        .size_full()
                        .overflow_y_scroll()
                        .occlude()
                        .gap_0p5()
                        .children(self.files.iter().enumerate().map(|(row_ix, file)| {
                            self.render_workspace_change_row(row_ix, file, cx)
                        }))
                        .into_any_element()
                };

                div()
                    .w_full()
                    .h(px(220.0))
                    .min_h(px(220.0))
                    .max_h(px(220.0))
                    .p_1()
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(hunk_opacity(cx.theme().border, is_dark, 0.88, 0.74))
                    .bg(hunk_blend(
                        cx.theme().background,
                        cx.theme().muted,
                        is_dark,
                        0.12,
                        0.18,
                    ))
                    .child(list_container)
                    .into_any_element()
            })
            .into_any_element()
    }

}
