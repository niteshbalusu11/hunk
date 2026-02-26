impl DiffViewer {
    fn render_settings_popup(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(settings) = self.settings_draft.as_ref() else {
            return div().into_any_element();
        };

        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let backdrop_bg = cx.theme().background.opacity(if is_dark { 0.24 } else { 0.12 });
        let panel_bg = cx.theme().popover.blend(
            cx.theme()
                .background
                .opacity(if is_dark { 0.16 } else { 0.05 }),
        );
        let nav_bg = cx.theme().sidebar.blend(
            cx.theme()
                .muted
                .opacity(if is_dark { 0.24 } else { 0.16 }),
        );

        div()
            .id("settings-popup-overlay")
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .left_0()
            .bg(backdrop_bg)
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_mouse_down(MouseButton::Middle, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_mouse_down(MouseButton::Right, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_scroll_wheel(|_, _, cx| {
                cx.stop_propagation();
            })
            .child(
                div()
                    .id("settings-popup-anchor")
                    .absolute()
                    .top(px(88.0))
                    .right(px(24.0))
                    .w(px(860.0))
                    .h(px(620.0))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .on_mouse_down(MouseButton::Middle, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .on_mouse_down(MouseButton::Right, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .on_scroll_wheel(|_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(
                        v_flex()
                            .id("settings-popup")
                            .w_full()
                            .h_full()
                            .rounded(px(12.0))
                            .border_1()
                            .border_color(cx.theme().border.opacity(if is_dark { 0.92 } else { 0.72 }))
                            .bg(panel_bg)
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .px_4()
                                    .py_3()
                                    .border_b_1()
                                    .border_color(
                                        cx.theme().border.opacity(if is_dark { 0.92 } else { 0.74 }),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_0p5()
                                            .child(
                                                div()
                                                    .text_lg()
                                                    .font_semibold()
                                                    .text_color(cx.theme().foreground)
                                                    .child("Settings"),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("Changes are saved to ~/.hunkdiff/config.toml"),
                                            ),
                                    )
                                    .child({
                                        let view = view.clone();
                                        Button::new("settings-close")
                                            .ghost()
                                            .compact()
                                            .rounded(px(8.0))
                                            .label("Close")
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.close_settings(cx);
                                                });
                                            })
                                    }),
                            )
                            .child(
                                h_flex()
                                    .flex_1()
                                    .min_h_0()
                                    .items_start()
                                    .child(
                                        v_flex()
                                            .w(px(220.0))
                                            .h_full()
                                            .justify_start()
                                            .p_3()
                                            .gap_2()
                                            .border_r_1()
                                            .border_color(cx.theme().border.opacity(if is_dark {
                                                0.90
                                            } else {
                                                0.70
                                            }))
                                            .bg(nav_bg)
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_semibold()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("Categories"),
                                            )
                                            .children(SettingsCategory::ALL.into_iter().map(
                                                |category| {
                                                    let is_selected = settings.category == category;
                                                    let view = view.clone();
                                                    let label = category.title();
                                                    let id = match category {
                                                        SettingsCategory::Ui => "settings-nav-ui",
                                                        SettingsCategory::KeyboardShortcuts => {
                                                            "settings-nav-keyboard-shortcuts"
                                                        }
                                                    };

                                                    Button::new(id)
                                                        .outline()
                                                        .rounded(px(8.0))
                                                        .label(label)
                                                        .bg(if is_selected {
                                                            cx.theme()
                                                                .accent
                                                                .opacity(if is_dark { 0.28 } else { 0.16 })
                                                        } else {
                                                            cx.theme()
                                                                .secondary
                                                                .opacity(if is_dark { 0.36 } else { 0.48 })
                                                        })
                                                        .border_color(
                                                            cx.theme().border.opacity(if is_selected {
                                                                if is_dark { 0.92 } else { 0.78 }
                                                            } else if is_dark {
                                                                0.82
                                                            } else {
                                                                0.62
                                                            }),
                                                        )
                                                        .on_click(move |_, _, cx| {
                                                            view.update(cx, |this, cx| {
                                                                this.select_settings_category(
                                                                    category, cx,
                                                                );
                                                            });
                                                        })
                                                        .into_any_element()
                                                },
                                            )),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .h_full()
                                            .min_w_0()
                                            .min_h_0()
                                            .p_4()
                                            .overflow_y_scrollbar()
                                            .child(match settings.category {
                                                SettingsCategory::Ui => {
                                                    self.render_settings_ui_category(settings, cx)
                                                }
                                                SettingsCategory::KeyboardShortcuts => {
                                                    self.render_settings_shortcuts_category(
                                                        settings, cx,
                                                    )
                                                }
                                            }),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_4()
                                    .py_3()
                                    .border_t_1()
                                    .border_color(
                                        cx.theme().border.opacity(if is_dark { 0.92 } else { 0.74 }),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(if settings.error_message.is_some() {
                                                cx.theme().danger
                                            } else {
                                                cx.theme().muted_foreground
                                            })
                                            .child(
                                                settings.error_message.clone().unwrap_or_else(|| {
                                                    "Shortcut updates are saved to config.toml."
                                                        .to_string()
                                                }),
                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_2()
                                            .child({
                                                let view = view.clone();
                                                Button::new("settings-cancel")
                                                    .outline()
                                                    .rounded(px(8.0))
                                                    .label("Cancel")
                                                    .on_click(move |_, _, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.close_settings(cx);
                                                        });
                                                    })
                                            })
                                            .child({
                                                let view = view.clone();
                                                Button::new("settings-save")
                                                    .primary()
                                                    .rounded(px(8.0))
                                                    .label("Save")
                                                    .on_click(move |_, window, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.save_settings(window, cx);
                                                        });
                                                    })
                                            }),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_settings_ui_category(
        &self,
        settings: &SettingsDraft,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let card_bg = cx.theme().background.blend(
            cx.theme()
                .muted
                .opacity(if is_dark { 0.24 } else { 0.12 }),
        );

        v_flex()
            .w_full()
            .gap_3()
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .child(
                        div()
                            .text_base()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("UI"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Theme and diff marker preferences."),
                    ),
            )
            .child(
                v_flex()
                    .w_full()
                    .gap_3()
                    .p_3()
                    .rounded(px(10.0))
                    .border_1()
                    .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.72 }))
                    .bg(card_bg)
                    .child(
                        v_flex()
                            .w_full()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("Theme"),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_2()
                                    .child({
                                        let view = view.clone();
                                        Button::new("settings-theme-system")
                                            .outline()
                                            .rounded(px(8.0))
                                            .label("System")
                                            .bg(if settings.theme == ThemePreference::System {
                                                cx.theme().accent.opacity(if is_dark { 0.30 } else { 0.18 })
                                            } else {
                                                cx.theme()
                                                    .secondary
                                                    .opacity(if is_dark { 0.36 } else { 0.52 })
                                            })
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.set_settings_theme(ThemePreference::System, cx);
                                                });
                                            })
                                    })
                                    .child({
                                        let view = view.clone();
                                        Button::new("settings-theme-light")
                                            .outline()
                                            .rounded(px(8.0))
                                            .label("Light")
                                            .bg(if settings.theme == ThemePreference::Light {
                                                cx.theme().accent.opacity(if is_dark { 0.30 } else { 0.18 })
                                            } else {
                                                cx.theme()
                                                    .secondary
                                                    .opacity(if is_dark { 0.36 } else { 0.52 })
                                            })
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.set_settings_theme(ThemePreference::Light, cx);
                                                });
                                            })
                                    })
                                    .child({
                                        let view = view.clone();
                                        Button::new("settings-theme-dark")
                                            .outline()
                                            .rounded(px(8.0))
                                            .label("Dark")
                                            .bg(if settings.theme == ThemePreference::Dark {
                                                cx.theme().accent.opacity(if is_dark { 0.30 } else { 0.18 })
                                            } else {
                                                cx.theme()
                                                    .secondary
                                                    .opacity(if is_dark { 0.36 } else { 0.52 })
                                            })
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.set_settings_theme(ThemePreference::Dark, cx);
                                                });
                                            })
                                    }),
                            ),
                    )
                    .child(
                        v_flex()
                            .w_full()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("Whitespace Markers"),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_2()
                                    .child({
                                        let view = view.clone();
                                        Button::new("settings-whitespace-on")
                                            .outline()
                                            .rounded(px(8.0))
                                            .label("On")
                                            .bg(if settings.show_whitespace {
                                                cx.theme().accent.opacity(if is_dark { 0.30 } else { 0.18 })
                                            } else {
                                                cx.theme()
                                                    .secondary
                                                    .opacity(if is_dark { 0.36 } else { 0.52 })
                                            })
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.set_settings_show_whitespace(true, cx);
                                                });
                                            })
                                    })
                                    .child({
                                        let view = view.clone();
                                        Button::new("settings-whitespace-off")
                                            .outline()
                                            .rounded(px(8.0))
                                            .label("Off")
                                            .bg(if !settings.show_whitespace {
                                                cx.theme().accent.opacity(if is_dark { 0.30 } else { 0.18 })
                                            } else {
                                                cx.theme()
                                                    .secondary
                                                    .opacity(if is_dark { 0.36 } else { 0.52 })
                                            })
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.set_settings_show_whitespace(false, cx);
                                                });
                                            })
                                    }),
                            ),
                    )
                    .child(
                        v_flex()
                            .w_full()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("End-Of-Line Markers"),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_2()
                                    .child({
                                        let view = view.clone();
                                        Button::new("settings-eol-on")
                                            .outline()
                                            .rounded(px(8.0))
                                            .label("On")
                                            .bg(if settings.show_eol_markers {
                                                cx.theme().accent.opacity(if is_dark { 0.30 } else { 0.18 })
                                            } else {
                                                cx.theme()
                                                    .secondary
                                                    .opacity(if is_dark { 0.36 } else { 0.52 })
                                            })
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.set_settings_show_eol_markers(true, cx);
                                                });
                                            })
                                    })
                                    .child({
                                        let view = view.clone();
                                        Button::new("settings-eol-off")
                                            .outline()
                                            .rounded(px(8.0))
                                            .label("Off")
                                            .bg(if !settings.show_eol_markers {
                                                cx.theme().accent.opacity(if is_dark { 0.30 } else { 0.18 })
                                            } else {
                                                cx.theme()
                                                    .secondary
                                                    .opacity(if is_dark { 0.36 } else { 0.52 })
                                            })
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.set_settings_show_eol_markers(false, cx);
                                                });
                                            })
                                    }),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_settings_shortcuts_category(
        &self,
        settings: &SettingsDraft,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_dark = cx.theme().mode.is_dark();

        v_flex()
            .w_full()
            .gap_3()
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .child(
                        div()
                            .text_base()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Keyboard Shortcuts"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Edit comma-separated shortcut strings for each action."),
                    ),
            )
            .children(
                settings
                    .shortcuts
                    .rows()
                    .into_iter()
                    .map(|row| self.render_settings_shortcut_row(row, cx)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground.opacity(if is_dark { 0.94 } else { 1.0 }))
                    .child(
                        "Use commas to add alternatives. For comma key, use cmd-, literally.",
                    ),
            )
            .into_any_element()
    }

    fn render_settings_shortcut_row(
        &self,
        row: SettingsShortcutRow,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_dark = cx.theme().mode.is_dark();

        v_flex()
            .id(row.id)
            .w_full()
            .gap_1()
            .p_3()
            .rounded(px(10.0))
            .border_1()
            .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.72 }))
            .bg(cx.theme().background.blend(
                cx.theme()
                    .muted
                    .opacity(if is_dark { 0.24 } else { 0.12 }),
            ))
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child(row.label),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(row.hint),
            )
            .child(
                Input::new(&row.input_state)
                    .h(px(36.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.72 }))
                    .bg(cx.theme().background.blend(
                        cx.theme()
                            .muted
                            .opacity(if is_dark { 0.20 } else { 0.09 }),
                    ))
                    .disabled(false),
            )
            .into_any_element()
    }
}
