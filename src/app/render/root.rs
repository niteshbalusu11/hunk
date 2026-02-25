impl Render for DiffViewer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.clamp_diff_horizontal_scroll_offset();
        let current_scroll_offset = self.diff_list_state.scroll_px_offset_for_scrollbar();
        if self.last_diff_scroll_offset != Some(current_scroll_offset) {
            self.last_diff_scroll_offset = Some(current_scroll_offset);
            self.last_scroll_activity_at = Instant::now();
        }
        self.frame_sample_count = self.frame_sample_count.saturating_add(1);

        div()
            .size_full()
            .key_context("DiffViewer")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::select_next_line_action))
            .on_action(cx.listener(Self::select_previous_line_action))
            .on_action(cx.listener(Self::extend_selection_next_line_action))
            .on_action(cx.listener(Self::extend_selection_previous_line_action))
            .on_action(cx.listener(Self::copy_selection_action))
            .on_action(cx.listener(Self::select_all_rows_action))
            .on_action(cx.listener(Self::next_hunk_action))
            .on_action(cx.listener(Self::previous_hunk_action))
            .on_action(cx.listener(Self::next_file_action))
            .on_action(cx.listener(Self::previous_file_action))
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
