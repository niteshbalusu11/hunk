impl DiffViewer {
    fn render_jj_workspace_graph_shell(&self, cx: &mut Context<Self>) -> AnyElement {
        h_resizable("hunk-jj-graph-workspace")
            .child(
                resizable_panel()
                    .size(px(700.0))
                    .size_range(px(360.0)..px(1200.0))
                    .child(self.render_jj_graph_canvas(cx)),
            )
            .child(
                resizable_panel()
                    .size(px(440.0))
                    .size_range(px(320.0)..px(760.0))
                    .child(self.render_jj_graph_right_panel(cx)),
            )
            .into_any_element()
    }

    fn render_jj_graph_right_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let is_dark = cx.theme().mode.is_dark();
        let panel_body = match self.graph_right_panel_mode {
            GraphRightPanelMode::ActiveWorkflow => self.render_jj_graph_active_workflow_panel(cx),
            GraphRightPanelMode::SelectedBookmark => self.render_jj_graph_selected_bookmark_panel(cx),
        };

        v_flex()
            .size_full()
            .min_h_0()
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
            .child(self.render_jj_graph_right_panel_mode_switch(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(panel_body),
            )
            .into_any_element()
    }

    fn render_jj_graph_right_panel_mode_switch(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let selected_available = self.graph_selected_bookmark.is_some();
        let active_selected = self.graph_right_panel_mode == GraphRightPanelMode::ActiveWorkflow;
        let bookmark_selected = self.graph_right_panel_mode == GraphRightPanelMode::SelectedBookmark;

        v_flex()
            .w_full()
            .gap_1()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .child("Right Panel Mode"),
                    )
                    .child({
                        let view = view.clone();
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
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_1()
                    .flex_wrap()
                    .child({
                        let view = view.clone();
                        let button = Button::new("jj-graph-right-mode-active")
                            .compact()
                            .with_size(gpui_component::Size::Small)
                            .rounded(px(7.0))
                            .label("Active Workflow")
                            .tooltip("Show working-copy, commit, and active-bookmark actions.")
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.set_graph_right_panel_mode_active(cx);
                                });
                            });
                        if active_selected {
                            button.primary().into_any_element()
                        } else {
                            button.outline().into_any_element()
                        }
                    })
                    .child({
                        let view = view.clone();
                        let button = Button::new("jj-graph-right-mode-selected")
                            .compact()
                            .with_size(gpui_component::Size::Small)
                            .rounded(px(7.0))
                            .label("Selected Bookmark")
                            .tooltip("Show bookmark-focused history and actions for the selected graph bookmark.")
                            .disabled(!selected_available)
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.set_graph_right_panel_mode_selected(cx);
                                });
                            });
                        if bookmark_selected {
                            button.primary().into_any_element()
                        } else {
                            button.outline().into_any_element()
                        }
                    })
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(if selected_available {
                                "Select bookmark chips in graph to populate bookmark mode."
                            } else {
                                "No bookmark selected in graph."
                            }),
                    ),
            )
            .when(self.show_jj_terms_glossary, |this| {
                this.child(self.render_jj_terms_glossary_card(cx))
            })
            .into_any_element()
    }

    fn render_jj_graph_active_workflow_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .w_full()
            .gap_2()
            .child(
                v_flex()
                    .w_full()
                    .gap_0p5()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Active Workflow Mode"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Use this mode for working-copy changes, commit actions, and active bookmark operations."),
                    ),
            )
            .child(self.render_jj_graph_operations_panel(cx))
            .into_any_element()
    }

    fn render_jj_graph_selected_bookmark_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.graph_selected_bookmark.is_none() {
            return v_flex()
                .w_full()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("No bookmark selected. Click a bookmark chip in the graph."),
                )
                .child({
                    let view = cx.entity();
                    Button::new("jj-graph-right-mode-fallback")
                        .outline()
                        .compact()
                        .with_size(gpui_component::Size::Small)
                        .rounded(px(7.0))
                        .label("Back to Active Workflow")
                        .tooltip("Return to active workflow actions.")
                        .on_click(move |_, _, cx| {
                            view.update(cx, |this, cx| {
                                this.set_graph_right_panel_mode_active(cx);
                            });
                        })
                })
                .into_any_element();
        }

        v_flex()
            .w_full()
            .gap_2()
            .child(
                v_flex()
                    .w_full()
                    .gap_0p5()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Selected Bookmark Mode"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Inspect a bookmark chain here, then activate it only when you want to make it your active working context."),
                    ),
            )
            .child({
                let view = cx.entity();
                let selected_local = self
                    .graph_selected_bookmark
                    .as_ref()
                    .is_some_and(|bookmark| bookmark.scope == GraphBookmarkScope::Local);
                Button::new("jj-graph-activate-selected-bookmark")
                    .primary()
                    .compact()
                    .with_size(gpui_component::Size::Small)
                    .rounded(px(7.0))
                    .label("Activate This Bookmark")
                    .tooltip("Switch active work to the selected local bookmark. If there are local changes, you will be asked how to switch.")
                    .disabled(self.git_action_loading || !selected_local)
                    .on_click(move |_, _, cx| {
                        view.update(cx, |this, cx| {
                            this.request_activate_selected_graph_bookmark(cx);
                        });
                    })
            })
            .child(self.render_jj_graph_inspector(cx))
            .child(self.render_jj_graph_focus_strip(cx))
            .into_any_element()
    }

    fn render_jj_graph_canvas(&self, cx: &mut Context<Self>) -> AnyElement {
        let graph_list_state = self.graph_list_state.clone();
        let is_dark = cx.theme().mode.is_dark();
        let nodes_len = self.graph_nodes.len();
        let view = cx.entity();

        v_flex()
            .size_full()
            .min_h_0()
            .gap_1()
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
                                    .child("Revision Graph"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Read-only graph window (single-select)"),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "{} nodes{}",
                                nodes_len,
                                if self.graph_has_more { " (windowed)" } else { "" }
                            )),
                    ),
            )
            .child({
                if self.graph_nodes.is_empty() {
                    return div()
                        .flex_1()
                        .min_h_0()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("No revisions available."),
                        )
                        .into_any_element();
                }

                let list = list(graph_list_state.clone(), {
                    cx.processor(move |this, ix: usize, _window, cx| {
                        let Some(node) = this.graph_nodes.get(ix) else {
                            return div().into_any_element();
                        };
                        this.render_jj_graph_row(ix, node, cx)
                    })
                })
                .flex_grow()
                .size_full()
                .with_sizing_behavior(ListSizingBehavior::Auto);

                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .child(list)
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .right_0()
                            .bottom_0()
                            .w(px(16.0))
                            .child(
                                Scrollbar::vertical(&graph_list_state)
                                    .scrollbar_show(ScrollbarShow::Always),
                            ),
                    )
                    .into_any_element()
            })
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} edges", self.graph_edges.len())),
                    )
                    .when(self.graph_has_more, |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("More history available in backend windowing."),
                        )
                    })
                    .child({
                        let view = view.clone();
                        Button::new("jj-graph-focus-active")
                            .outline()
                            .compact()
                            .with_size(gpui_component::Size::Small)
                            .rounded(px(7.0))
                            .label("Focus Active Bookmark")
                            .tooltip("Select and focus the currently active bookmark in the graph.")
                            .disabled(self.graph_active_bookmark.is_none())
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.select_active_graph_bookmark(cx);
                                });
                            })
                    }),
            )
            .into_any_element()
    }

    fn render_jj_graph_row(&self, row_ix: usize, node: &GraphNode, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let node_id = node.id.clone();
        let is_dark = cx.theme().mode.is_dark();
        let row_bg = if self.graph_node_is_selected(node.id.as_str()) {
            cx.theme().accent.opacity(if is_dark { 0.22 } else { 0.14 })
        } else {
            cx.theme().background.opacity(0.0)
        };
        let row_border = cx.theme().border.opacity(0.0);
        let marker = if node.is_working_copy_parent {
            "@"
        } else if node.is_active_bookmark_target {
            "*"
        } else {
            "o"
        };
        let parent_count = self
            .graph_edges
            .iter()
            .filter(|edge| edge.from == node.id)
            .count();
        let short_id = node.id.chars().take(12).collect::<String>();

        let row = h_flex()
            .id(("jj-graph-row", row_ix))
            .w_full()
            .items_start()
            .gap_2()
            .px_2()
            .py_1()
            .rounded(px(6.0))
            .border_1()
            .border_color(row_border)
            .bg(row_bg)
            .on_click({
                let view = view.clone();
                move |_, _, cx| {
                    view.update(cx, |this, cx| {
                        this.select_graph_node(node_id.clone(), cx);
                    });
                }
            })
            .child(
                div()
                    .w(px(18.0))
                    .pt_0p5()
                    .text_xs()
                    .font_semibold()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(cx.theme().muted_foreground)
                    .child(marker),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .text_color(cx.theme().muted_foreground)
                                    .child(short_id),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(relative_time_label(Some(node.unix_time))),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("parents:{parent_count}")),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .truncate()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(node.subject.clone()),
                    )
                    .child(
                        h_flex().w_full().items_center().gap_1().flex_wrap().children(
                            node.bookmarks.iter().enumerate().map(|(bookmark_ix, bookmark)| {
                                self.render_jj_graph_bookmark_chip(
                                    node.id.as_str(),
                                    row_ix,
                                    bookmark_ix,
                                    bookmark,
                                    cx,
                                )
                            }),
                        ),
                    ),
            );
        row.into_any_element()
    }

    fn render_jj_graph_bookmark_chip(
        &self,
        node_id: &str,
        row_ix: usize,
        bookmark_ix: usize,
        bookmark: &GraphBookmarkRef,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let node_id = node_id.to_string();
        let name = bookmark.name.clone();
        let remote = bookmark.remote.clone();
        let scope = bookmark.scope;
        let activate_node_id = node_id.clone();
        let activate_name = bookmark.name.clone();
        let activate_remote = bookmark.remote.clone();
        let activate_scope = bookmark.scope;
        let selected = self.graph_selected_bookmark.as_ref().is_some_and(|selected| {
            selected.name == bookmark.name
                && selected.remote == bookmark.remote
                && selected.scope == bookmark.scope
        });

        let status_token = match bookmark.scope {
            GraphBookmarkScope::Local if bookmark.conflicted => "conflict",
            GraphBookmarkScope::Local if bookmark.tracked && bookmark.needs_push => "ahead",
            GraphBookmarkScope::Local if bookmark.tracked => "synced",
            GraphBookmarkScope::Local => "local",
            GraphBookmarkScope::Remote if bookmark.conflicted && bookmark.tracked => "track-conflict",
            GraphBookmarkScope::Remote if bookmark.conflicted => "conflict",
            GraphBookmarkScope::Remote if bookmark.tracked => "tracked",
            GraphBookmarkScope::Remote => "remote",
        };

        let mut label = match bookmark.scope {
            GraphBookmarkScope::Local => format!("L {} [{status_token}]", bookmark.name),
            GraphBookmarkScope::Remote => format!(
                "R {}@{} [{status_token}]",
                bookmark.name,
                bookmark.remote.as_deref().unwrap_or("remote")
            ),
        };
        if bookmark.is_active {
            label = format!("* {label}");
        }

        let tooltip = match (bookmark.scope, bookmark.tracked, bookmark.conflicted) {
            (GraphBookmarkScope::Local, _, true) => "Local bookmark (conflicted)".to_string(),
            (GraphBookmarkScope::Local, true, false) => "Local bookmark (published)".to_string(),
            (GraphBookmarkScope::Local, false, false) => "Local bookmark (not published)".to_string(),
            (GraphBookmarkScope::Remote, true, true) => {
                "Remote bookmark (tracked, conflicted)".to_string()
            }
            (GraphBookmarkScope::Remote, true, false) => "Remote bookmark (tracked)".to_string(),
            (GraphBookmarkScope::Remote, false, true) => {
                "Remote bookmark (untracked, conflicted)".to_string()
            }
            (GraphBookmarkScope::Remote, false, false) => "Remote bookmark (untracked)".to_string(),
        };

        let button_id = row_ix.saturating_mul(1_024).saturating_add(bookmark_ix);
        let mut button = Button::new(("jj-graph-bookmark-chip", button_id))
            .compact()
            .with_size(gpui_component::Size::Small)
            .rounded(px(6.0))
            .label(label)
            .tooltip(tooltip)
            .on_click(move |_, _, cx| {
                cx.stop_propagation();
                view.update(cx, |this, cx| {
                    this.select_graph_bookmark(
                        node_id.clone(),
                        name.clone(),
                        remote.clone(),
                        scope,
                        cx,
                    );
                });
            });

        if selected {
            button = button.primary();
        } else {
            let chip_bg = match bookmark.scope {
                GraphBookmarkScope::Local if bookmark.conflicted => {
                    cx.theme().danger.opacity(if is_dark { 0.28 } else { 0.18 })
                }
                GraphBookmarkScope::Local if bookmark.needs_push => {
                    cx.theme().warning.opacity(if is_dark { 0.30 } else { 0.18 })
                }
                GraphBookmarkScope::Local if bookmark.tracked => {
                    cx.theme().success.opacity(if is_dark { 0.28 } else { 0.16 })
                }
                GraphBookmarkScope::Local => {
                    cx.theme().accent.opacity(if is_dark { 0.16 } else { 0.10 })
                }
                GraphBookmarkScope::Remote => {
                    cx.theme().secondary.opacity(if is_dark { 0.34 } else { 0.54 })
                }
            };
            button = button.outline().bg(chip_bg);
        }

        let chip_wrapper = |child: AnyElement| {
            let view = cx.entity();
            div()
                .on_mouse_down(MouseButton::Left, {
                    let activate_node_id = activate_node_id.clone();
                    let activate_name = activate_name.clone();
                    let activate_remote = activate_remote.clone();
                    move |event, _, cx| {
                        cx.stop_propagation();
                        view.update(cx, |this, cx| {
                            if event.click_count >= 2 {
                                this.activate_graph_bookmark(
                                    activate_node_id.clone(),
                                    activate_name.clone(),
                                    activate_remote.clone(),
                                    activate_scope,
                                    cx,
                                );
                            }
                        });
                    }
                })
                .child(child)
                .into_any_element()
        };

        if selected && bookmark.scope == GraphBookmarkScope::Local {
            let view = cx.entity();
            return h_flex()
                .items_center()
                .gap_1()
                .child(chip_wrapper(button.into_any_element()))
                .child(
                    Input::new(&self.graph_action_input_state)
                        .h(px(22.0))
                        .w(px(164.0))
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.74 }))
                        .bg(cx.theme().background.opacity(if is_dark { 0.30 } else { 0.18 }))
                        .disabled(self.git_action_loading),
                )
                .child(
                    Button::new(("jj-graph-bookmark-inline-rename", button_id))
                        .outline()
                        .compact()
                        .with_size(gpui_component::Size::Small)
                        .rounded(px(6.0))
                        .label("Rename")
                        .tooltip("Rename the selected local bookmark.")
                        .disabled(self.git_action_loading)
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            view.update(cx, |this, cx| {
                                this.rename_selected_graph_bookmark_from_input(cx);
                            });
                        }),
                )
                .into_any_element();
        }

        chip_wrapper(button.into_any_element())
    }

    fn reduced_motion_enabled(&self) -> bool {
        self.config.reduce_motion
    }

    fn animation_duration_ms(&self, millis: u64) -> Duration {
        if self.reduced_motion_enabled() {
            Duration::from_millis(1)
        } else {
            Duration::from_millis(millis)
        }
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
                    .child("Revision: an immutable committed node in the graph."),
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
