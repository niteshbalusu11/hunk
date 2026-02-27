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

    fn render_diff_workspace_screen(&mut self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .size_full()
            .child(if self.sidebar_collapsed {
                self.render_diff(cx).into_any_element()
            } else {
                h_resizable("hunk-diff-workspace")
                    .child(
                        resizable_panel()
                            .size(px(300.0))
                            .size_range(px(240.0)..px(520.0))
                            .child(self.render_tree(cx)),
                    )
                    .child(resizable_panel().child(self.render_diff(cx)))
                    .into_any_element()
            })
            .into_any_element()
    }

    fn render_file_workspace_screen(&mut self, cx: &mut Context<Self>) -> AnyElement {
        if self.repo_discovery_failed {
            return self.render_open_project_empty_state(cx);
        }

        if let Some(error_message) = &self.error_message {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .p_4()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(error_message.clone()),
                )
                .into_any_element();
        }

        div()
            .size_full()
            .child(if self.sidebar_collapsed {
                self.render_file_editor(cx).into_any_element()
            } else {
                h_resizable("hunk-file-workspace")
                    .child(
                        resizable_panel()
                            .size(px(300.0))
                            .size_range(px(240.0)..px(520.0))
                            .child(self.render_tree(cx)),
                    )
                    .child(resizable_panel().child(self.render_file_editor(cx)))
                    .into_any_element()
            })
            .into_any_element()
    }

    fn render_jj_workspace_screen(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let is_dark = cx.theme().mode.is_dark();
        let active_bookmark = if self.branch_syncable() {
            self.branch_name.clone()
        } else {
            "detached".to_string()
        };

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .border_b_1()
                    .border_color(cx.theme().border.opacity(if is_dark { 0.88 } else { 0.70 }))
                    .bg(cx.theme().sidebar.blend(cx.theme().muted.opacity(if is_dark {
                        0.20
                    } else {
                        0.30
                    })))
                    .child(
                        v_flex()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("JJ Workspace"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Working copy changes and bookmark operations"),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("Active bookmark: {active_bookmark}")),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{} changed files", self.files.len())),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .pb(px(APP_BOTTOM_SAFE_INSET))
                    .child(self.render_jj_workspace_main_surface(cx)),
            )
            .into_any_element()
    }

    fn render_jj_workspace_main_surface(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let is_dark = cx.theme().mode.is_dark();
        div()
            .size_full()
            .bg(cx.theme().sidebar.blend(cx.theme().muted.opacity(if is_dark {
                0.18
            } else {
                0.24
            })))
            .overflow_y_scrollbar()
            .child(self.render_jj_workspace(cx))
            .into_any_element()
    }

    fn render_app_footer(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let files_selected = self.workspace_view_mode == WorkspaceViewMode::Files;
        let diff_selected = self.workspace_view_mode == WorkspaceViewMode::Diff;
        let jj_selected = self.workspace_view_mode == WorkspaceViewMode::JjWorkspace;

        h_flex()
            .w_full()
            .h_10()
            .items_center()
            .justify_between()
            .gap_2()
            .px_2()
            .border_t_1()
            .border_color(cx.theme().border.opacity(if is_dark { 0.88 } else { 0.68 }))
            .bg(cx.theme().sidebar.blend(cx.theme().muted.opacity(if is_dark {
                0.18
            } else {
                0.22
            })))
            .child(
                h_flex()
                    .items_center()
                    .gap_1()
                    .when(!jj_selected, |this| {
                        this.child({
                            let view = view.clone();
                            let mut button = Button::new("footer-toggle-sidebar")
                                .compact()
                                .rounded(px(7.0))
                                .icon(
                                    Icon::new(if self.sidebar_collapsed {
                                        IconName::ChevronRight
                                    } else {
                                        IconName::ChevronLeft
                                    })
                                    .size(px(14.0)),
                                )
                                .min_w(px(30.0))
                                .h(px(28.0))
                                .tooltip(if self.sidebar_collapsed {
                                    "Show file tree (Cmd/Ctrl+B)"
                                } else {
                                    "Hide file tree (Cmd/Ctrl+B)"
                                })
                                .on_click(move |_, _, cx| {
                                    view.update(cx, |this, cx| {
                                        this.toggle_sidebar_tree(cx);
                                    });
                                });
                            if self.sidebar_collapsed {
                                button = button.outline();
                            } else {
                                button = button.primary();
                            }
                            button.into_any_element()
                        })
                    })
                    .child({
                        let view = view.clone();
                        let mut button = Button::new("footer-workspace-files")
                            .compact()
                            .rounded(px(7.0))
                            .icon(Icon::new(IconName::FolderClosed).size(px(14.0)))
                            .min_w(px(30.0))
                            .h(px(28.0))
                            .tooltip("Switch to file view")
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.set_workspace_view_mode(WorkspaceViewMode::Files, cx);
                                });
                            });
                        if files_selected {
                            button = button.primary();
                        } else {
                            button = button.outline();
                        }
                        button.into_any_element()
                    })
                    .child({
                        let view = view.clone();
                        let mut button = Button::new("footer-workspace-diff")
                            .compact()
                            .rounded(px(7.0))
                            .icon(Icon::new(IconName::File).size(px(14.0)))
                            .min_w(px(30.0))
                            .h(px(28.0))
                            .tooltip("Switch to diff view")
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.set_workspace_view_mode(WorkspaceViewMode::Diff, cx);
                                });
                            });
                        if diff_selected {
                            button = button.primary();
                        } else {
                            button = button.outline();
                        }
                        button.into_any_element()
                    })
                    .child({
                        let view = view.clone();
                        let mut button = Button::new("footer-workspace-jj")
                            .compact()
                            .rounded(px(7.0))
                            .icon(Icon::new(IconName::BookOpen).size(px(14.0)))
                            .min_w(px(30.0))
                            .h(px(28.0))
                            .tooltip("Switch to JJ workspace")
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.set_workspace_view_mode(WorkspaceViewMode::JjWorkspace, cx);
                                });
                            });
                        if jj_selected {
                            button = button.primary();
                        } else {
                            button = button.outline();
                        }
                        button.into_any_element()
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("{} changed files", self.files.len())),
            )
            .into_any_element()
    }
}

impl Render for DiffViewer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let jj_fullscreen = self.workspace_view_mode == WorkspaceViewMode::JjWorkspace;
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
            .on_action(cx.listener(Self::toggle_sidebar_tree_action))
            .on_action(cx.listener(Self::open_project_action))
            .on_action(cx.listener(Self::save_current_file_action))
            .on_action(cx.listener(Self::open_settings_action))
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .when(!cfg!(target_os = "macos") && !jj_fullscreen, |this| {
                this.child(self.render_in_app_menu_bar(cx))
            })
            .when(!jj_fullscreen, |this| this.child(self.render_toolbar(cx)))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(match self.workspace_view_mode {
                        WorkspaceViewMode::Files => self.render_file_workspace_screen(cx),
                        WorkspaceViewMode::Diff => self.render_diff_workspace_screen(cx),
                        WorkspaceViewMode::JjWorkspace => self.render_jj_workspace_screen(cx),
                    }),
            )
            .child(self.render_app_footer(cx))
            .when(self.settings_draft.is_some(), |this| {
                this.child(self.render_settings_popup(cx))
            })
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
