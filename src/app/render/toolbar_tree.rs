impl DiffViewer {
    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let repo_label = self
            .repo_root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "No git repository found".to_string());
        let selected_theme = self.config.theme;
        let theme_label = match self.config.theme {
            ThemePreference::System => "System",
            ThemePreference::Light => "Light",
            ThemePreference::Dark => "Dark",
        };
        let theme_button_label = format!("Theme ({theme_label})");
        let is_dark = cx.theme().mode.is_dark();
        let chip_bg = cx.theme().muted.opacity(if is_dark { 0.26 } else { 0.52 });
        let chip_border = cx.theme().border.opacity(if is_dark { 0.88 } else { 0.70 });
        let brand_bg = cx
            .theme()
            .accent
            .opacity(if is_dark { 0.26 } else { 0.14 });

        h_flex()
            .w_full()
            .h_11()
            .items_center()
            .justify_between()
            .gap_2()
            .px_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap_2()
                    .overflow_x_hidden()
                    .child(
                        h_flex()
                            .items_center()
                            .px_2()
                            .py_0p5()
                            .rounded_md()
                            .bg(brand_bg)
                            .border_1()
                            .border_color(cx.theme().accent.opacity(if is_dark { 0.62 } else { 0.42 }))
                            .child(div().text_sm().font_semibold().child("Hunk")),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .px_2()
                            .py_0p5()
                            .rounded_md()
                            .bg(chip_bg)
                            .border_1()
                            .border_color(chip_border)
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("branch"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child(self.branch_name.clone()),
                            ),
                    )
                    .child(
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .items_center()
                            .gap_1()
                            .overflow_x_hidden()
                            .px_2()
                            .py_0p5()
                            .rounded_md()
                            .bg(chip_bg)
                            .border_1()
                            .border_color(chip_border)
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("repo"),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_x_hidden()
                                    .whitespace_nowrap()
                                    .text_sm()
                                    .text_color(cx.theme().foreground.opacity(0.82))
                                    .child(repo_label),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .flex_none()
                    .items_center()
                    .gap_2()
                    .child(
                        h_flex().items_center().gap_1().child(
                            Button::new("theme-dropdown")
                                .outline()
                                .compact()
                                .rounded(px(7.0))
                                .bg(cx.theme().secondary.opacity(if is_dark { 0.52 } else { 0.70 }))
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
                    .child({
                        let view = view.clone();
                        Button::new("toggle-diff-fit")
                            .outline()
                            .compact()
                            .rounded(px(7.0))
                            .bg(cx.theme().secondary.opacity(if is_dark { 0.44 } else { 0.64 }))
                            .label(if self.diff_fit_to_width { "Pan" } else { "Fit" })
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.toggle_diff_fit_to_width(cx);
                                });
                            })
                    })
                    .child({
                        let view = view.clone();
                        Button::new("toggle-diff-whitespace")
                            .outline()
                            .compact()
                            .rounded(px(7.0))
                            .bg(cx.theme().secondary.opacity(if is_dark { 0.44 } else { 0.64 }))
                            .label(if self.diff_show_whitespace {
                                "Whitespace: On"
                            } else {
                                "Whitespace: Off"
                            })
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.toggle_diff_show_whitespace(cx);
                                });
                            })
                    })
                    .child({
                        let view = view.clone();
                        Button::new("toggle-diff-eol")
                            .outline()
                            .compact()
                            .rounded(px(7.0))
                            .bg(cx.theme().secondary.opacity(if is_dark { 0.44 } else { 0.64 }))
                            .label(if self.diff_show_eol_markers {
                                "EOL: On"
                            } else {
                                "EOL: Off"
                            })
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.toggle_diff_show_eol_markers(cx);
                                });
                            })
                    })
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

    fn render_commit_footer(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let show_publish = !self.branch_has_upstream;
        let show_push = self.branch_has_upstream && self.branch_ahead_count > 0;
        let action_label = if show_publish { "Publish" } else { "Push" };
        let last_commit_text = self
            .last_commit_subject
            .as_deref()
            .map(str::trim_end)
            .filter(|text| !text.is_empty())
            .unwrap_or("No commits yet");

        v_flex()
            .w_full()
            .gap_2()
            .px_2()
            .pt_2()
            .pb_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar.blend(cx.theme().muted.opacity(if is_dark {
                0.16
            } else {
                0.24
            })))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_1()
                    .child({
                        let view = view.clone();
                        Button::new("branch-picker-toggle")
                            .outline()
                            .compact()
                            .rounded(px(7.0))
                            .bg(cx.theme().secondary.opacity(if is_dark { 0.50 } else { 0.70 }))
                            .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.74 }))
                            .dropdown_caret(true)
                            .label(self.branch_name.clone())
                            .disabled(self.git_action_loading)
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.toggle_branch_picker(cx);
                                });
                            })
                    })
                    .when(show_publish || show_push, |this| {
                        this.child({
                            let view = view.clone();
                            Button::new("publish-or-push")
                                .primary()
                                .compact()
                                .rounded(px(7.0))
                                .label(action_label)
                                .disabled(self.git_action_loading)
                                .on_click(move |_, _, cx| {
                                    view.update(cx, |this, cx| {
                                        this.push_or_publish_current_branch(cx);
                                    });
                                })
                        })
                    }),
            )
            .when(self.branch_picker_open, |this| {
                this.child(self.render_branch_picker_panel(cx))
            })
            .child(
                Input::new(&self.commit_input_state)
                    .h(px(82.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(cx.theme().border.opacity(if is_dark { 0.92 } else { 0.78 }))
                    .bg(cx.theme().background.blend(cx.theme().muted.opacity(if is_dark {
                        0.24
                    } else {
                        0.12
                    })))
                    .disabled(self.git_action_loading),
            )
            .child({
                let view = view.clone();
                Button::new("commit-staged")
                    .primary()
                    .rounded(px(7.0))
                    .label("Commit")
                    .disabled(self.git_action_loading)
                    .on_click(move |_, window, cx| {
                        view.update(cx, |this, cx| {
                            this.commit_from_input(window, cx);
                        });
                    })
            })
            .child(
                div()
                    .w_full()
                    .min_h(px(28.0))
                    .px_2()
                    .py_1()
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(cx.theme().border.opacity(if is_dark { 0.92 } else { 0.76 }))
                    .bg(cx.theme().secondary.opacity(if is_dark { 0.42 } else { 0.56 }))
                    .text_xs()
                    .font_medium()
                    .text_color(cx.theme().foreground.opacity(0.90))
                    .whitespace_normal()
                    .child(last_commit_text.to_string()),
            )
            .into_any_element()
    }

    fn render_branch_picker_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();

        v_flex()
            .w_full()
            .gap_1()
            .p_2()
            .rounded(px(8.0))
            .border_1()
            .border_color(cx.theme().border.opacity(if is_dark { 0.94 } else { 0.74 }))
            .bg(cx.theme().background.blend(cx.theme().secondary.opacity(if is_dark {
                0.32
            } else {
                0.20
            })))
            .child(
                div()
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().muted_foreground)
                    .child("Branches"),
            )
            .child(
                div()
                    .max_h(px(144.0))
                    .overflow_y_scrollbar()
                    .child(
                        v_flex().w_full().gap_1().children(
                            self.branches
                                .iter()
                                .enumerate()
                                .map(|(ix, branch)| {
                                    let view = view.clone();
                                    let branch_name = branch.name.clone();

                                    h_flex()
                                        .id(("branch-row", ix))
                                        .w_full()
                                        .min_w_0()
                                        .items_center()
                                        .gap_1()
                                        .px_2()
                                        .py_0p5()
                                        .rounded(px(6.0))
                                        .bg(if branch.is_current {
                                            cx.theme().accent.opacity(if is_dark { 0.28 } else { 0.18 })
                                        } else {
                                            cx.theme().background.opacity(0.0)
                                        })
                                        .on_click(move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                this.checkout_branch(branch_name.clone(), cx);
                                            });
                                        })
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .truncate()
                                                .text_xs()
                                                .font_medium()
                                                .text_color(cx.theme().foreground)
                                                .child(branch.name.clone()),
                                        )
                                        .child(
                                            div()
                                                .flex_none()
                                                .pl_2()
                                                .whitespace_nowrap()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(relative_time_label(branch.tip_unix_time)),
                                        )
                                        .into_any_element()
                                }),
                        ),
                    ),
            )
            .child(
                Input::new(&self.branch_input_state)
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(cx.theme().border.opacity(if is_dark { 0.92 } else { 0.76 }))
                    .bg(cx.theme().background.blend(cx.theme().muted.opacity(if is_dark {
                        0.22
                    } else {
                        0.14
                    })))
                    .disabled(self.git_action_loading),
            )
            .child({
                let view = view.clone();
                Button::new("create-or-switch-branch")
                    .primary()
                    .rounded(px(7.0))
                    .label("Create / Switch")
                    .disabled(self.git_action_loading)
                    .on_click(move |_, window, cx| {
                        view.update(cx, |this, cx| {
                            this.create_or_switch_branch_from_input(window, cx);
                        });
                    })
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
        let path = path.to_string();
        let is_collapsed = self.collapsed_files.contains(path.as_str());

        let (label, accent) = match status {
            FileStatus::Added | FileStatus::Untracked => ("NEW FILE", cx.theme().success),
            FileStatus::Deleted => ("DELETED FILE", cx.theme().danger),
            FileStatus::Renamed => ("RENAMED", cx.theme().accent),
            FileStatus::Modified => ("MODIFIED", cx.theme().warning),
            FileStatus::TypeChange => ("TYPE CHANGED", cx.theme().warning),
            FileStatus::Conflicted => ("CONFLICTED", cx.theme().danger),
            FileStatus::Unknown => ("MODIFIED", cx.theme().muted_foreground),
        };
        let background = cx
            .theme()
            .background
            .blend(accent.opacity(if is_dark { 0.34 } else { 0.16 }));
        let row_background = if is_selected {
            background.blend(
                cx.theme()
                    .primary
                    .opacity(if is_dark { 0.28 } else { 0.16 }),
            )
        } else {
            background
        };
        let border_color = accent.opacity(if is_dark { 0.78 } else { 0.52 });
        let badge_background = accent.opacity(if is_dark { 0.50 } else { 0.27 });
        let accent_strip = if is_dark {
            accent.lighten(0.18)
        } else {
            accent.darken(0.06)
        };
        let arrow_color = if is_dark {
            accent.lighten(0.34)
        } else {
            accent.darken(0.18)
        };

        h_flex()
            .id(("diff-file-header-row", stable_row_id))
            .relative()
            .overflow_x_hidden()
            .on_mouse_down(MouseButton::Left, {
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
            .w_full()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .border_1()
            .border_color(border_color)
            .bg(row_background)
            .when(self.diff_fit_to_width, |this| this.w_full())
            .when(!self.diff_fit_to_width, |this| {
                this.w(px(self.diff_pan_content_width))
                    .min_w(px(self.diff_pan_content_width))
            })
            .child({
                let view = view.clone();
                let path = path.clone();
                Button::new(("toggle-file-collapse", stable_row_id))
                    .ghost()
                    .compact()
                    .label(if is_collapsed {
                        "▶"
                    } else {
                        "▼"
                    })
                    .min_w(px(24.0))
                    .h(px(24.0))
                    .text_sm()
                    .text_color(arrow_color)
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        view.update(cx, |this, cx| {
                            this.toggle_file_collapsed(path.clone(), cx);
                        });
                    })
            })
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
                    .child(path.clone()),
            )
            .child(self.render_line_stats("file", stats, cx))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w(px(3.0))
                    .bg(accent_strip),
            )
            .into_any_element()
    }

}
