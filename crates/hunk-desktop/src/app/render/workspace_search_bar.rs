impl DiffViewer {
    fn render_workspace_search_bar(
        &self,
        view: Entity<Self>,
        editor_chrome: HunkEditorChromeColors,
        is_dark: bool,
        search_match_count: usize,
        show_replace_controls: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let search_surface = hunk_input_surface(cx.theme(), is_dark);
        let search_count_label = match search_match_count {
            0 => "No matches".to_string(),
            1 => "1 match".to_string(),
            count => format!("{count} matches"),
        };

        let search_controls = h_flex()
            .flex_1()
            .min_w_0()
            .items_center()
            .gap_2()
            .child(
                Input::new(&self.editor_search_input_state)
                    .flex_1()
                    .h(px(32.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(search_surface.border)
                    .bg(search_surface.background),
            )
            .child(
                div()
                    .min_w(px(72.0))
                    .text_xs()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(editor_chrome.line_number)
                    .child(search_count_label),
            )
            .child({
                let view = view.clone();
                Button::new("workspace-search-prev")
                    .outline()
                    .compact()
                    .rounded(px(7.0))
                    .icon(Icon::new(IconName::ChevronUp).size(px(12.0)))
                    .tooltip("Previous match")
                    .on_click(move |_, _, cx| {
                        view.update(cx, |this, cx| {
                            this.navigate_editor_search(false, cx);
                        });
                    })
            })
            .child({
                let view = view.clone();
                Button::new("workspace-search-next")
                    .outline()
                    .compact()
                    .rounded(px(7.0))
                    .icon(Icon::new(IconName::ChevronDown).size(px(12.0)))
                    .tooltip("Next match")
                    .on_click(move |_, _, cx| {
                        view.update(cx, |this, cx| {
                            this.navigate_editor_search(true, cx);
                        });
                    })
            });

        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(hunk_opacity(cx.theme().border, is_dark, 0.82, 0.70))
            .bg(hunk_blend(
                editor_chrome.background,
                cx.theme().muted,
                is_dark,
                0.10,
                0.18,
            ))
            .child(search_controls)
            .when(show_replace_controls, |this| {
                this.child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Input::new(&self.editor_replace_input_state)
                                .w(px(220.0))
                                .h(px(32.0))
                                .rounded(px(8.0))
                                .border_1()
                                .border_color(search_surface.border)
                                .bg(search_surface.background),
                        )
                        .child({
                            let view = view.clone();
                            Button::new("workspace-search-replace")
                                .outline()
                                .compact()
                                .rounded(px(7.0))
                                .icon(Icon::new(IconName::Replace).size(px(12.0)))
                                .label("Replace")
                                .tooltip("Replace current match")
                                .on_click(move |_, window, cx| {
                                    view.update(cx, |this, cx| {
                                        this.replace_current_editor_search_match(Some(window), cx);
                                    });
                                })
                        })
                        .child({
                            let view = view.clone();
                            Button::new("workspace-search-replace-all")
                                .outline()
                                .compact()
                                .rounded(px(7.0))
                                .label("Replace All")
                                .tooltip("Replace all matches in the active file")
                                .on_click(move |_, _, cx| {
                                    view.update(cx, |this, cx| {
                                        this.replace_all_editor_search_matches(cx);
                                    });
                                })
                        }),
                )
            })
            .child({
                let view = view.clone();
                Button::new("workspace-search-close")
                    .ghost()
                    .compact()
                    .rounded(px(7.0))
                    .icon(Icon::new(IconName::Close).size(px(12.0)))
                    .tooltip(if show_replace_controls {
                        "Close find and replace"
                    } else {
                        "Close find"
                    })
                    .on_click(move |_, window, cx| {
                        view.update(cx, |this, cx| {
                            this.toggle_editor_search(false, window, cx);
                        });
                    })
            })
            .into_any_element()
    }
}
