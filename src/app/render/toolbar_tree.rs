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
                    .child({
                        let view = view.clone();
                        Button::new("toggle-diff-fit")
                            .ghost()
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
                            .ghost()
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
                            .ghost()
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
        let staged_count = self.files.iter().filter(|file| file.staged).count();
        let view = cx.entity();

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_1()
                    .px_1()
                    .py_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_xs()
                            .font_family(cx.theme().mono_font_family.clone())
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
                                    .compact()
                                    .ghost()
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
                                    .compact()
                                    .ghost()
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
                        .py_0p5()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .text_xs()
                        .font_family(cx.theme().mono_font_family.clone())
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
                    .child(self.render_commit_footer(cx))
                    .child(self.render_last_commit_footer(cx)),
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
                            .font_family(cx.theme().mono_font_family.clone())
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
                        .text_color(cx.theme().muted_foreground)
                        .child("No files"),
                )
            })
            .children(
                files
                    .iter()
                    .enumerate()
                    .map(|(ix, file)| self.render_change_row(ix, file, cx)),
            )
            .into_any_element()
    }

    fn render_change_row(&self, ix: usize, file: &ChangedFile, cx: &mut Context<Self>) -> AnyElement {
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
            .id(("change-row", ix))
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
                    .font_family(cx.theme().mono_font_family.clone())
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
                                .font_family(cx.theme().mono_font_family.clone())
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
        let push_label = if self.branch_has_upstream { "Push" } else { "Publish" };

        v_flex()
            .w_full()
            .gap_1()
            .px_1()
            .py_1()
            .border_t_1()
            .border_color(cx.theme().border)
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
                            .dropdown_caret(true)
                            .label(self.branch_name.clone())
                            .disabled(self.git_action_loading)
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.toggle_branch_picker(cx);
                                });
                            })
                    })
                    .child({
                        let view = view.clone();
                        Button::new("publish-or-push")
                            .outline()
                            .compact()
                            .label(push_label)
                            .disabled(self.git_action_loading)
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.push_or_publish_current_branch(cx);
                                });
                            })
                    }),
            )
            .when(self.branch_picker_open, |this| {
                this.child(self.render_branch_picker_panel(cx))
            })
            .child(
                Input::new(&self.commit_input_state)
                    .h(px(88.0))
                    .disabled(self.git_action_loading),
            )
            .child({
                let view = view.clone();
                Button::new("commit-staged")
                    .outline()
                    .compact()
                    .label("Commit")
                    .disabled(self.git_action_loading)
                    .on_click(move |_, _, cx| {
                        view.update(cx, |this, cx| {
                            this.commit_from_input(cx);
                        });
                    })
            })
            .into_any_element()
    }

    fn render_last_commit_footer(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .w_full()
            .min_h(px(52.0))
            .gap_0p5()
            .px_2()
            .py_1()
            .pb_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                div()
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().muted_foreground)
                    .child("Last commit"),
            )
            .child(
                div()
                    .text_xs()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(cx.theme().foreground)
                    .child(
                        self.last_commit_subject
                            .clone()
                            .unwrap_or_else(|| "No commits yet".to_string()),
                    ),
            )
            .into_any_element()
    }

    fn render_branch_picker_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();

        v_flex()
            .w_full()
            .gap_1()
            .p_1()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
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
                                        .items_center()
                                        .justify_between()
                                        .gap_1()
                                        .px_1()
                                        .py_0p5()
                                        .rounded_sm()
                                        .bg(if branch.is_current {
                                            cx.theme().accent.opacity(0.20)
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
                                                .text_xs()
                                                .font_family(cx.theme().mono_font_family.clone())
                                                .text_color(cx.theme().foreground)
                                                .child(branch.name.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(relative_time_label(branch.tip_unix_time)),
                                        )
                                        .into_any_element()
                                }),
                        ),
                    ),
            )
            .child(Input::new(&self.branch_input_state).disabled(self.git_action_loading))
            .child({
                let view = view.clone();
                Button::new("create-or-switch-branch")
                    .compact()
                    .outline()
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
                    .blend(cx.theme().success.opacity(if is_dark { 0.20 } else { 0.10 })),
                cx.theme().success.opacity(if is_dark { 0.50 } else { 0.24 }),
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
                    .blend(cx.theme().warning.opacity(if is_dark { 0.20 } else { 0.10 })),
                cx.theme().warning.opacity(if is_dark { 0.45 } else { 0.24 }),
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

fn relative_time_label(unix_time: Option<i64>) -> String {
    let Some(unix_time) = unix_time else {
        return "unknown".to_string();
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(unix_time);

    let elapsed = now.saturating_sub(unix_time).max(0);

    if elapsed < 60 {
        format!("{}s ago", elapsed)
    } else if elapsed < 60 * 60 {
        format!("{}m ago", elapsed / 60)
    } else if elapsed < 60 * 60 * 24 {
        format!("{}h ago", elapsed / (60 * 60))
    } else {
        format!("{}d ago", elapsed / (60 * 60 * 24))
    }
}
