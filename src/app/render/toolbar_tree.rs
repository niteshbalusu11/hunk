impl DiffViewer {
    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let repo_label = self
            .repo_root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "No git repository found".to_string());
        let branch_label = format!("branch: {}", self.branch_name);
        let selected_theme = self.config.theme;
        let theme_label = match self.config.theme {
            ThemePreference::System => "System",
            ThemePreference::Light => "Light",
            ThemePreference::Dark => "Dark",
        };
        let theme_button_label = format!("Theme ({theme_label})");

        h_flex()
            .w_full()
            .h_11()
            .items_center()
            .justify_between()
            .px_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_sm().font_semibold().child("hunk"))
                    .child(
                        div()
                            .text_xs()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_color(cx.theme().muted_foreground)
                            .child(branch_label),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(repo_label),
                    ),
            )
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        h_flex().items_center().gap_1().child(
                            Button::new("theme-dropdown")
                                .outline()
                                .compact()
                                .dropdown_caret(true)
                                .label(theme_button_label)
                                .dropdown_menu({
                                    let view = view.clone();
                                    move |menu, _, _| {
                                        menu.item(
                                            PopupMenuItem::new("System")
                                                .checked(selected_theme == ThemePreference::System)
                                                .on_click({
                                                    let view = view.clone();
                                                    move |_, window, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.set_theme_preference(
                                                                ThemePreference::System,
                                                                window,
                                                                cx,
                                                            );
                                                        });
                                                    }
                                                }),
                                        )
                                        .item(
                                            PopupMenuItem::new("Light")
                                                .checked(selected_theme == ThemePreference::Light)
                                                .on_click({
                                                    let view = view.clone();
                                                    move |_, window, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.set_theme_preference(
                                                                ThemePreference::Light,
                                                                window,
                                                                cx,
                                                            );
                                                        });
                                                    }
                                                }),
                                        )
                                        .item(
                                            PopupMenuItem::new("Dark")
                                                .checked(selected_theme == ThemePreference::Dark)
                                                .on_click({
                                                    let view = view.clone();
                                                    move |_, window, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.set_theme_preference(
                                                                ThemePreference::Dark,
                                                                window,
                                                                cx,
                                                            );
                                                        });
                                                    }
                                                }),
                                        )
                                    }
                                }),
                        ),
                    )
                    .child(self.render_line_stats("overall", self.overall_line_stats, cx))
                    .child(
                        Button::new("toggle-diff-fit")
                            .ghost()
                            .label(if self.diff_fit_to_width { "Pan" } else { "Fit" })
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.toggle_diff_fit_to_width(cx);
                                });
                            }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} files", self.files.len())),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_color(if self.fps >= 110.0 {
                                cx.theme().success
                            } else if self.fps >= 60.0 {
                                cx.theme().warning
                            } else {
                                cx.theme().danger
                            })
                            .child(format!("{:>3.0} fps", self.fps.round())),
                    ),
            )
    }

    fn render_tree(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let selected_path = self.selected_path.clone();
        let status_by_path = self
            .files
            .iter()
            .map(|file| (file.path.clone(), file.status))
            .collect::<BTreeMap<_, _>>();
        let collapsed_by_path = self.collapsed_files.clone();
        let is_dark = cx.theme().mode.is_dark();

        v_flex().size_full().overflow_y_scrollbar().child(tree(
            &self.tree_state,
            move |ix, entry, _selected, _window, cx| {
                let item = entry.item();
                let item_id = item.id.to_string();
                let item_label = item.label.clone();
                let is_folder = entry.is_folder();
                let is_selected = selected_path.as_deref() == Some(item_id.as_str());
                let click_path = item_id.clone();
                let icon = if is_folder {
                    if entry.is_expanded() { "▾" } else { "▸" }
                } else {
                    "•"
                };
                let indent = px(10.0 + (entry.depth() as f32 * 16.0));

                ListItem::new(ix)
                    .selected(is_selected)
                    .pl(indent)
                    .on_click({
                        let view = view.clone();
                        move |_, _, cx| {
                            if is_folder {
                                return;
                            }

                            view.update(cx, |this, cx| {
                                this.select_file(click_path.clone(), cx);
                            });
                        }
                    })
                    .child(if is_folder {
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_2()
                            .child(div().text_sm().child(icon))
                            .child(div().text_sm().child(item_label))
                            .into_any_element()
                    } else {
                        let status = status_by_path
                            .get(item_id.as_str())
                            .copied()
                            .unwrap_or(FileStatus::Unknown);
                        let is_collapsed = collapsed_by_path.contains(item_id.as_str());

                        let (status_label, accent) = match status {
                            FileStatus::Added => ("ADD", cx.theme().success),
                            FileStatus::Modified => ("MOD", cx.theme().warning),
                            FileStatus::Deleted => ("DEL", cx.theme().danger),
                            FileStatus::Renamed => ("REN", cx.theme().accent),
                            FileStatus::Untracked => ("NEW", cx.theme().success),
                            FileStatus::TypeChange => ("TYP", cx.theme().warning),
                            FileStatus::Conflicted => ("CON", cx.theme().danger),
                            FileStatus::Unknown => ("---", cx.theme().muted_foreground),
                        };

                        let badge_bg = if is_selected {
                            accent.opacity(if is_dark { 0.40 } else { 0.30 })
                        } else {
                            accent.opacity(if is_dark { 0.30 } else { 0.18 })
                        };

                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w_4()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(if is_collapsed { "▸" } else { "▾" }),
                            )
                            .child(
                                div()
                                    .min_w_10()
                                    .px_1p5()
                                    .py_0p5()
                                    .text_xs()
                                    .font_semibold()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .text_color(cx.theme().foreground)
                                    .bg(badge_bg)
                                    .border_1()
                                    .border_color(accent.opacity(if is_dark { 0.88 } else { 0.50 }))
                                    .rounded_sm()
                                    .child(status_label),
                            )
                            .child(div().text_sm().child(item_label))
                            .into_any_element()
                    })
            },
        ))
    }
    fn render_file_status_banner(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let path = self
            .selected_path
            .clone()
            .unwrap_or_else(|| "No file selected".to_string());

        let status = self.selected_status.unwrap_or(FileStatus::Unknown);
        let is_dark = cx.theme().mode.is_dark();

        let (label, hint, accent, background, badge_background) = match status {
            FileStatus::Added | FileStatus::Untracked => (
                "NEW FILE",
                "Content exists only on the right side.",
                cx.theme().success,
                cx.theme()
                    .background
                    .blend(
                        cx.theme()
                            .success
                            .opacity(if is_dark { 0.20 } else { 0.10 }),
                    ),
                cx.theme()
                    .success
                    .opacity(if is_dark { 0.50 } else { 0.24 }),
            ),
            FileStatus::Deleted => (
                "DELETED FILE",
                "Content exists only on the left side.",
                cx.theme().danger,
                cx.theme()
                    .background
                    .blend(cx.theme().danger.opacity(if is_dark { 0.20 } else { 0.10 })),
                cx.theme().danger.opacity(if is_dark { 0.50 } else { 0.24 }),
            ),
            FileStatus::Renamed => (
                "RENAMED",
                "Showing textual changes for this path.",
                cx.theme().warning,
                cx.theme()
                    .background
                    .blend(
                        cx.theme()
                            .warning
                            .opacity(if is_dark { 0.20 } else { 0.10 }),
                    ),
                cx.theme()
                    .warning
                    .opacity(if is_dark { 0.45 } else { 0.24 }),
            ),
            _ => (
                "MODIFIED",
                "Side-by-side diff view.",
                cx.theme().accent,
                cx.theme()
                    .background
                    .blend(cx.theme().accent.opacity(if is_dark { 0.14 } else { 0.08 })),
                cx.theme().accent.opacity(if is_dark { 0.50 } else { 0.24 }),
            ),
        };
        let hint_text = if self.selected_file_is_collapsed() {
            "Collapsed in stream. Expand to render this file inline."
        } else {
            hint
        };

        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(background)
            .child(
                div()
                    .px_2()
                    .py_0p5()
                    .text_xs()
                    .font_semibold()
                    .bg(badge_background)
                    .border_1()
                    .border_color(accent.opacity(if is_dark { 0.88 } else { 0.44 }))
                    .text_color(cx.theme().foreground)
                    .child(label),
            )
            .child(
                div()
                    .text_sm()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(cx.theme().foreground)
                    .child(path),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(hint_text),
            )
            .child(self.render_line_stats("file", self.selected_line_stats, cx))
            .child(
                Button::new("toggle-file-collapse")
                    .ghost()
                    .label(if self.selected_file_is_collapsed() {
                        "Expand"
                    } else {
                        "Collapse"
                    })
                    .on_click(move |_, _, cx| {
                        view.update(cx, |this, cx| {
                            this.toggle_selected_file_collapsed(cx);
                        });
                    }),
            )
            .into_any_element()
    }

    fn render_line_stats(
        &self,
        label: &'static str,
        stats: LineStats,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        h_flex()
            .items_center()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(label),
            )
            .child(
                div()
                    .text_xs()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(if cx.theme().mode.is_dark() {
                        cx.theme().success.lighten(0.42)
                    } else {
                        cx.theme().success.darken(0.05)
                    })
                    .child(format!("+{}", stats.added)),
            )
            .child(
                div()
                    .text_xs()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(if cx.theme().mode.is_dark() {
                        cx.theme().danger.lighten(0.42)
                    } else {
                        cx.theme().danger.darken(0.05)
                    })
                    .child(format!("-{}", stats.removed)),
            )
            .child(
                div()
                    .text_xs()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("chg {}", stats.changed())),
            )
            .into_any_element()
    }
}
