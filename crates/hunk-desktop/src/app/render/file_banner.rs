impl DiffViewer {
    fn review_view_file_shortcut_label(&self) -> Option<String> {
        let shortcuts = self.config.keyboard_shortcuts.view_current_review_file.as_slice();
        let preferred = if cfg!(target_os = "macos") {
            shortcuts
                .iter()
                .find(|shortcut| shortcut.to_ascii_lowercase().contains("cmd"))
        } else {
            shortcuts
                .iter()
                .find(|shortcut| shortcut.to_ascii_lowercase().contains("ctrl"))
        }
        .or_else(|| shortcuts.first())?;
        Some(format_shortcut_label(preferred.as_str()))
    }

    fn render_review_view_file_button(
        &self,
        button_id: (&'static str, u64),
        path: &str,
        status: FileStatus,
        view: Entity<DiffViewer>,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let path = path.to_string();
        let disabled = !self.can_open_file_in_files_workspace(path.as_str(), status);
        let tooltip = self
            .review_view_file_shortcut_label()
            .map_or_else(|| "View file".to_string(), |shortcut| {
                format!("View file ({shortcut})")
            });

        Button::new(button_id)
            .outline()
            .compact()
            .rounded(px(7.0))
            .label("View File")
            .disabled(disabled)
            .tooltip(tooltip)
            .on_click(move |_, window, cx| {
                view.update(cx, |this, cx| {
                    this.open_file_in_files_workspace(path.clone(), status, window, cx);
                });
            })
            .into_any_element()
    }

    fn render_file_status_banner_row(
        &self,
        row_ix: usize,
        path: &str,
        status: FileStatus,
        stats: LineStats,
        is_selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let view = cx.entity();
        let stable_row_id = self.diff_row_stable_id(row_ix);
        let is_dark = cx.theme().mode.is_dark();
        let chrome = hunk_diff_chrome(cx.theme(), is_dark);
        let path = path.to_string();
        let is_collapsed = self.collapsed_files.contains(path.as_str());
        let colors = hunk_file_status_banner(cx.theme(), status, is_dark, is_selected);

        h_flex()
            .id(("diff-file-header-row", stable_row_id))
            .relative()
            .overflow_x_hidden()
            .on_mouse_down(MouseButton::Left, {
                cx.listener(move |this, event, window, cx| {
                    this.on_diff_row_mouse_down(row_ix, event, window, cx);
                })
            })
            .on_mouse_down(MouseButton::Middle, {
                cx.listener(move |this, event, window, cx| {
                    this.on_diff_row_mouse_down(row_ix, event, window, cx);
                })
            })
            .on_mouse_move({
                cx.listener(move |this, event, window, cx| {
                    this.on_diff_row_mouse_move(row_ix, event, window, cx);
                })
            })
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_diff_row_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_diff_row_mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::on_diff_row_mouse_up))
            .on_mouse_up_out(MouseButton::Middle, cx.listener(Self::on_diff_row_mouse_up))
            .w_full()
            .items_center()
            .gap_2()
            .px_3()
            .py_0p5()
            .border_b_1()
            .border_color(chrome.row_divider)
            .bg(colors.row_background)
            .w_full()
            .child({
                let view = view.clone();
                let path = path.clone();
                Button::new(("toggle-file-collapse", stable_row_id))
                    .ghost()
                    .compact()
                    .icon(
                        Icon::new(if is_collapsed {
                            IconName::ChevronRight
                        } else {
                            IconName::ChevronDown
                        })
                        .size(px(14.0)),
                    )
                    .min_w(px(22.0))
                    .h(px(22.0))
                    .text_color(colors.arrow)
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        view.update(cx, |this, cx| {
                            this.toggle_file_collapsed(path.clone(), cx);
                        });
                    })
            })
            .child(
                div()
                    .px_1p5()
                    .py_0p5()
                    .text_xs()
                    .font_semibold()
                    .bg(colors.badge_background)
                    .border_1()
                    .border_color(colors.badge_border)
                    .text_color(cx.theme().foreground)
                    .child(colors.label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    .truncate()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(cx.theme().foreground)
                    .child(path.clone()),
            )
            .child(self.render_line_stats("file", stats, cx))
            .child(self.render_review_view_file_button(
                ("diff-file-view", stable_row_id),
                path.as_str(),
                status,
                view.clone(),
                cx,
            ))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w(px(2.0))
                    .bg(colors.accent_strip),
            )
            .into_any_element()
    }
}
