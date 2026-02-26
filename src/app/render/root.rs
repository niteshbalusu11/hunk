impl DiffViewer {
    fn render_in_app_menu_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(menu_bar) = self.in_app_menu_bar.clone() else {
            return div().into_any_element();
        };
        let is_dark = cx.theme().mode.is_dark();
        h_flex()
            .w_full()
            .h_8()
            .items_center()
            .px_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().title_bar.blend(
                cx.theme()
                    .muted
                    .opacity(if is_dark { 0.16 } else { 0.24 }),
            ))
            .child(div().flex_1().min_w_0().h_full().child(menu_bar))
            .into_any_element()
    }
}

impl Render for DiffViewer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_scroll_offset = self.diff_list_state.scroll_px_offset_for_scrollbar();
        if self.last_diff_scroll_offset != Some(current_scroll_offset) {
            self.last_diff_scroll_offset = Some(current_scroll_offset);
            self.last_scroll_activity_at = Instant::now();
        }
        self.frame_sample_count = self.frame_sample_count.saturating_add(1);

        v_flex()
            .size_full()
            .relative()
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
            .on_action(cx.listener(Self::open_project_action))
            .on_action(cx.listener(Self::save_current_file_action))
            .on_action(cx.listener(Self::open_settings_action))
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .when(!cfg!(target_os = "macos"), |this| {
                this.child(self.render_in_app_menu_bar(cx))
            })
            .child(self.render_toolbar(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .pb(px(APP_BOTTOM_SAFE_INSET))
                    .child(
                        h_resizable("hunk-main")
                            .child(
                                resizable_panel()
                                    .size(px(280.0))
                                    .size_range(px(160.0)..px(520.0))
                                    .child(self.render_tree(cx)),
                            )
                            .child(resizable_panel().child(self.render_diff(cx))),
                    ),
            )
            .when(self.settings_draft.is_some(), |this| {
                this.child(self.render_settings_popup(cx))
            })
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
