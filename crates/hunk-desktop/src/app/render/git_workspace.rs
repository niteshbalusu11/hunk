impl DiffViewer {
    fn render_git_workspace_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let is_dark = cx.theme().mode.is_dark();
        let show_workflow_skeleton = self.workflow_loading && !self.git_workflow_ready_for_panel();
        let panel_body = if show_workflow_skeleton {
            self.render_git_workspace_panel_loading_skeleton(cx)
        } else {
            self.render_git_workspace_operations_panel(cx)
        };

        v_flex()
            .size_full()
            .min_h_0()
            .min_w_0()
            .gap_2()
            .p_2()
            .rounded(px(8.0))
            .border_1()
            .border_color(hunk_opacity(cx.theme().border, is_dark, 0.90, 0.74))
            .bg(hunk_blend(cx.theme().background, cx.theme().muted, is_dark, 0.16, 0.24))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(
                        v_flex()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("Git Workflow"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Branches, working tree changes, commits, and review actions."),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .child(
                        div()
                            .id("git-workspace-scroll-area")
                            .size_full()
                            .track_scroll(&self.git_workspace_scroll_handle)
                            .overflow_y_scroll()
                            .child(v_flex().w_full().gap_2().pb_2().child(panel_body)),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .right_0()
                            .bottom_0()
                            .w(px(16.0))
                            .child(
                                Scrollbar::vertical(&self.git_workspace_scroll_handle)
                                    .scrollbar_show(ScrollbarShow::Always),
                            ),
                    ),
            )
            .into_any_element()
    }
}
