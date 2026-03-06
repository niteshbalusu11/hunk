impl DiffViewer {
    fn render_jj_workspace_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let is_dark = cx.theme().mode.is_dark();
        let show_workflow_skeleton = self.workflow_loading && !self.jj_workflow_ready_for_panel();
        let panel_body = if show_workflow_skeleton {
            self.render_jj_workspace_panel_loading_skeleton(cx)
        } else {
            self.render_jj_workspace_operations_panel(cx)
        };

        v_flex()
            .size_full()
            .min_h_0()
            .min_w_0()
            .gap_2()
            .p_2()
            .rounded(px(8.0))
            .border_1()
            .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.74 }))
            .bg(cx.theme().background.blend(cx.theme().muted.opacity(if is_dark {
                0.16
            } else {
                0.24
            })))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
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
                                    .child(
                                        "Bookmarks, working-copy changes, revisions, and review actions.",
                                    ),
                            ),
                    )
                    .child({
                        let view = cx.entity();
                        let mut button = Button::new("jj-workspace-terms-toggle")
                            .outline()
                            .compact()
                            .with_size(gpui_component::Size::Small)
                            .rounded(px(7.0))
                            .label("JJ Terms")
                            .tooltip("Show a quick glossary of JJ terms used in this workspace.")
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.toggle_jj_terms_glossary(cx);
                                });
                            });
                        if self.show_jj_terms_glossary {
                            button = button.primary();
                        }
                        button
                    }),
            )
            .when(self.show_jj_terms_glossary, |this| {
                this.child(self.render_jj_terms_glossary_card(cx))
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .child(
                        div()
                            .id("jj-workspace-scroll-area")
                            .size_full()
                            .track_scroll(&self.jj_workspace_scroll_handle)
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
                                Scrollbar::vertical(&self.jj_workspace_scroll_handle)
                                    .scrollbar_show(ScrollbarShow::Always),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_jj_terms_glossary_card(&self, cx: &mut Context<Self>) -> AnyElement {
        let is_dark = cx.theme().mode.is_dark();
        v_flex()
            .w_full()
            .gap_0p5()
            .px_2()
            .py_1()
            .rounded(px(8.0))
            .border_1()
            .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.74 }))
            .bg(cx.theme().background.blend(cx.theme().muted.opacity(if is_dark {
                0.22
            } else {
                0.30
            })))
            .child(
                div()
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child("JJ Terms"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .whitespace_normal()
                    .child("Working copy (`@`): your mutable local changes."),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .whitespace_normal()
                    .child("Revision: an immutable committed change."),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .whitespace_normal()
                    .child("Bookmark: a movable pointer to a revision."),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .whitespace_normal()
                    .child("Publish: create remote tracking for a local bookmark."),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .whitespace_normal()
                    .child("Sync: fetch remote bookmark updates into local history."),
            )
            .into_any_element()
    }
}
