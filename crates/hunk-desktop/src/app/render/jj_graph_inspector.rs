impl DiffViewer {
    fn render_jj_graph_inspector(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let selected_node = self.graph_selected_node();
        let selected_bookmark = self.graph_selected_bookmark_ref();
        let selected_workspace = self.graph_selected_workspace_state();
        let selected_workspace_selection = self.graph_selected_workspace.as_ref();
        let selected_workspace_commit_visible = selected_workspace.is_some_and(|workspace| {
            self.graph_nodes
                .iter()
                .any(|node| node.id == workspace.commit_id)
        });
        let pending_workspace_switch = self.pending_workspace_switch();
        let pending_workspace_forget = self.pending_workspace_forget();
        let workspace_switch_blocker = self.selected_graph_workspace_switch_blocker();
        let workspace_switch_disabled = workspace_switch_blocker.is_some();
        let graph_workspace_action_input = self
            .graph_workspace_action_input_state
            .read(cx)
            .value()
            .trim()
            .to_string();
        let graph_workspace_action_input_empty = graph_workspace_action_input.is_empty();
        let workspace_create_blocker =
            self.selected_graph_workspace_create_blocker(graph_workspace_action_input.as_str());
        let workspace_create_disabled = workspace_create_blocker.is_some();
        let workspace_forget_blocker = self.selected_graph_workspace_forget_blocker();
        let workspace_forget_disabled = workspace_forget_blocker.is_some();
        let selected_bookmark_selection = self.graph_selected_bookmark.as_ref();
        let selected_bookmark_is_local = selected_bookmark_selection
            .is_some_and(|bookmark| bookmark.scope == GraphBookmarkScope::Local);
        let bookmark_mutation_blocker = self.graph_bookmark_mutation_blocker_reason();
        let bookmark_mutation_disabled = bookmark_mutation_blocker.is_some();
        let selected_review_blocker = self.selected_graph_review_action_blocker();
        let selected_review_disabled = selected_review_blocker.is_some();
        let has_selected_node = self.graph_selected_node_id.is_some();
        let selected_node_is_working_copy = selected_node
            .is_some_and(|node| self.graph_working_copy_commit_id.as_deref() == Some(node.id.as_str()));
        let graph_action_input_empty = self
            .graph_action_input_state
            .read(cx)
            .value()
            .trim()
            .is_empty();
        let move_confirmation = self.graph_move_confirmation();
        let selected_node_parent_count = selected_node
            .map(|node| {
                self.graph_edges
                    .iter()
                    .filter(|edge| edge.from == node.id)
                    .count()
            })
            .unwrap_or(0);
        let create_tooltip = bookmark_mutation_blocker.map(str::to_string).unwrap_or_else(|| {
            "Create a new bookmark at the selected revision.".to_string()
        });
        let fork_tooltip = bookmark_mutation_blocker.map(str::to_string).unwrap_or_else(|| {
            "Create another bookmark from the selected revision, using focus as naming context."
                .to_string()
        });
        let rename_tooltip = bookmark_mutation_blocker.map(str::to_string).unwrap_or_else(|| {
            "Rename the selected local bookmark to the input name.".to_string()
        });
        let move_tooltip = bookmark_mutation_blocker.map(str::to_string).unwrap_or_else(|| {
            "Retarget the selected local bookmark to the selected revision.".to_string()
        });
        let focus_workspace_tooltip = if let Some(workspace) = selected_workspace {
            if selected_workspace_commit_visible {
                format!("Focus workspace {}@ commit in the graph.", workspace.name)
            } else {
                format!(
                    "Workspace {}@ commit is outside the current graph window. Refresh or load more history.",
                    workspace.name
                )
            }
        } else {
            "Select a workspace chip to focus its commit.".to_string()
        };
        let copy_workspace_tooltip = if let Some(workspace) = selected_workspace {
            format!("Copy workspace name {}@ to clipboard.", workspace.name)
        } else {
            "Select a workspace chip before copying workspace name.".to_string()
        };
        let switch_workspace_tooltip = workspace_switch_blocker.clone().unwrap_or_else(|| {
            "Switch app context to the selected workspace root.".to_string()
        });
        let create_workspace_tooltip = workspace_create_blocker.clone().unwrap_or_else(|| {
            "Create a task workspace + same-name bookmark from trunk in .jj/workspaces."
                .to_string()
        });
        let forget_workspace_tooltip = workspace_forget_blocker.clone().unwrap_or_else(|| {
            "Forget the selected non-current workspace from repository metadata.".to_string()
        });

        v_flex()
            .w_full()
            .gap_1()
            .p_2()
            .rounded(px(8.0))
            .border_1()
            .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.74 }))
            .bg(cx.theme().background.blend(cx.theme().muted.opacity(if is_dark {
                0.18
            } else {
                0.28
            })))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .child("Graph Inspector"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("nodes: {}  edges: {}", self.graph_nodes.len(), self.graph_edges.len())),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!(
                        "Active bookmark: {}",
                        self.graph_active_bookmark.as_deref().unwrap_or("detached")
                    )),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!(
                        "Active workspace: {}@",
                        self.graph_current_workspace_name
                            .as_deref()
                            .unwrap_or("unknown")
                    )),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!(
                        "Working-copy parent: {}",
                        self.graph_working_copy_parent_commit_id
                            .as_deref()
                            .map(|id| id.chars().take(12).collect::<String>())
                            .unwrap_or_else(|| "none".to_string())
                    )),
            )
            .child({
                if let Some(node) = selected_node {
                    let short_id = node.id.chars().take(12).collect::<String>();
                    return v_flex()
                        .w_full()
                        .gap_0p5()
                        .px_1()
                        .py_1()
                        .rounded(px(6.0))
                        .bg(cx.theme().background.opacity(if is_dark { 0.36 } else { 0.48 }))
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child("Selected Revision"),
                        )
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
                                .text_color(cx.theme().foreground)
                                .whitespace_normal()
                                .child(node.subject.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!(
                                    "parents:{} bookmarks:{} workspaces:{}",
                                    selected_node_parent_count,
                                    node.bookmarks.len(),
                                    node.workspaces.len()
                                )),
                        )
                        .when(selected_node_is_working_copy, |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(cx.theme().warning)
                                    .child("Mutable working-copy revision"),
                            )
                        })
                        .into_any_element();
                }

                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Select a revision node to inspect details.")
                    .into_any_element()
            })
            .child({
                if let Some(bookmark) = selected_bookmark {
                    let label = match bookmark.scope {
                        GraphBookmarkScope::Local => format!("Local: {}", bookmark.name),
                        GraphBookmarkScope::Remote => format!(
                            "Remote: {}@{}",
                            bookmark.name,
                            bookmark.remote.as_deref().unwrap_or("remote")
                        ),
                    };
                    return v_flex()
                        .w_full()
                        .gap_0p5()
                        .px_1()
                        .py_1()
                        .rounded(px(6.0))
                        .bg(cx.theme().background.opacity(if is_dark { 0.36 } else { 0.48 }))
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child("Selected Bookmark"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().foreground)
                                .child(label),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!(
                                    "tracked:{} needs_push:{} conflicted:{}",
                                    bookmark.tracked, bookmark.needs_push, bookmark.conflicted
                                )),
                        )
                        .into_any_element();
                }

                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Select a bookmark chip to inspect tracking details.")
                    .into_any_element()
            })
            .child({
                if let Some(workspace) = selected_workspace {
                    let short_id = workspace.commit_id.chars().take(12).collect::<String>();
                    return v_flex()
                        .w_full()
                        .gap_0p5()
                        .px_1()
                        .py_1()
                        .rounded(px(6.0))
                        .bg(cx.theme().background.opacity(if is_dark { 0.36 } else { 0.48 }))
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child("Selected Workspace"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().foreground)
                                .child(format!("{}@", workspace.name)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("target: {}", short_id)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(if workspace.is_current {
                                    "current workspace".to_string()
                                } else {
                                    "non-current workspace".to_string()
                                }),
                        )
                        .into_any_element();
                }
                if let Some(selected) = selected_workspace_selection {
                    return v_flex()
                        .w_full()
                        .gap_0p5()
                        .px_1()
                        .py_1()
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(cx.theme().warning.opacity(if is_dark { 0.90 } else { 0.72 }))
                        .bg(cx.theme().warning.opacity(if is_dark { 0.16 } else { 0.10 }))
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child("Selected Workspace"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().foreground)
                                .child(format!("{}@", selected.name)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .whitespace_normal()
                                .child("Workspace is no longer present in the current snapshot."),
                        )
                        .into_any_element();
                }

                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Select a workspace chip to inspect workspace details.")
                    .into_any_element()
            })
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .px_1()
                    .py_1()
                    .rounded(px(6.0))
                    .bg(cx.theme().background.opacity(if is_dark { 0.36 } else { 0.48 }))
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Workspace Actions"),
                    )
                    .child(
                        Input::new(&self.graph_workspace_action_input_state)
                            .h(px(30.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.74 }))
                            .bg(cx.theme().background.opacity(if is_dark { 0.28 } else { 0.18 }))
                            .disabled(self.git_action_loading || pending_workspace_forget.is_some()),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_1()
                            .flex_wrap()
                            .child({
                                let view = view.clone();
                                Button::new("jj-graph-inspector-focus-workspace-commit")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .label("Focus Workspace Commit")
                                    .tooltip(focus_workspace_tooltip)
                                    .disabled(
                                        self.git_action_loading || !selected_workspace_commit_visible,
                                    )
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.focus_selected_graph_workspace_commit(cx);
                                        });
                                    })
                            })
                            .child({
                                let view = view.clone();
                                Button::new("jj-graph-inspector-copy-workspace-name")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .label("Copy Workspace Name")
                                    .tooltip(copy_workspace_tooltip)
                                    .disabled(self.git_action_loading || selected_workspace.is_none())
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.copy_selected_graph_workspace_name(cx);
                                        });
                                    })
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
                                Button::new("jj-graph-inspector-switch-workspace")
                                    .primary()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .label("Switch Workspace")
                                    .tooltip(switch_workspace_tooltip)
                                    .disabled(workspace_switch_disabled)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.request_switch_selected_graph_workspace(cx);
                                        });
                                    })
                            })
                            .child({
                                let view = view.clone();
                                Button::new("jj-graph-inspector-create-workspace")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .label("Create Workspace")
                                    .tooltip(create_workspace_tooltip)
                                    .disabled(workspace_create_disabled || graph_workspace_action_input_empty)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.request_create_graph_workspace_at_selected_revision(cx);
                                        });
                                    })
                            })
                            .child({
                                let view = view.clone();
                                Button::new("jj-graph-inspector-forget-workspace")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .label("Forget Workspace")
                                    .tooltip(forget_workspace_tooltip)
                                    .disabled(workspace_forget_disabled)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.request_forget_selected_graph_workspace(cx);
                                        });
                                    })
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .whitespace_normal()
                            .child(
                                "Workspace switch/create/forget are explicit and guarded. Task workspace create uses active trunk target and pairs workspace+bookmark names.",
                            ),
                    ),
            )
            .child({
                if let Some(pending) = pending_workspace_switch {
                    return v_flex()
                        .w_full()
                        .gap_1()
                        .px_1()
                        .py_1()
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(cx.theme().warning.opacity(if is_dark { 0.90 } else { 0.72 }))
                        .bg(cx.theme().warning.opacity(if is_dark { 0.18 } else { 0.10 }))
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child("Confirm Workspace Switch"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().foreground)
                                .whitespace_normal()
                                .child(format!(
                                    "Switch {}@ -> {}@ with {} local files?",
                                    pending.source_workspace,
                                    pending.target_workspace,
                                    pending.changed_file_count
                                )),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .whitespace_normal()
                                .child(format!(
                                    "Opening target workspace root: {}. Local changes stay in {}@.",
                                    pending.target_workspace_root.display(),
                                    pending.source_workspace,
                                )),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .gap_1()
                                .child({
                                    let view = view.clone();
                                    Button::new("jj-graph-inspector-confirm-workspace-switch")
                                        .primary()
                                        .compact()
                                        .with_size(gpui_component::Size::Small)
                                        .rounded(px(7.0))
                                        .label("Confirm Switch")
                                        .tooltip(
                                            "Switch app context to selected workspace and refresh repository state.",
                                        )
                                        .disabled(self.git_action_loading)
                                        .on_click(move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                this.confirm_pending_workspace_switch(cx);
                                            });
                                        })
                                })
                                .child({
                                    let view = view.clone();
                                    Button::new("jj-graph-inspector-cancel-workspace-switch")
                                        .outline()
                                        .compact()
                                        .with_size(gpui_component::Size::Small)
                                        .rounded(px(7.0))
                                        .label("Cancel")
                                        .tooltip("Cancel pending workspace switch.")
                                        .disabled(self.git_action_loading)
                                        .on_click(move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                this.cancel_pending_workspace_switch(cx);
                                            });
                                        })
                                }),
                        )
                        .into_any_element();
                }

                div().into_any_element()
            })
            .child({
                if let Some(pending) = pending_workspace_forget {
                    let short_id = pending
                        .workspace_commit_id
                        .chars()
                        .take(12)
                        .collect::<String>();
                    return v_flex()
                        .w_full()
                        .gap_1()
                        .px_1()
                        .py_1()
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(cx.theme().warning.opacity(if is_dark { 0.90 } else { 0.72 }))
                        .bg(cx.theme().warning.opacity(if is_dark { 0.18 } else { 0.10 }))
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child("Confirm Workspace Forget"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().foreground)
                                .whitespace_normal()
                                .child(format!(
                                    "Forget workspace {}@ (wc {}) from repository metadata?",
                                    pending.workspace_name, short_id
                                )),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .whitespace_normal()
                                .child(
                                    "This does not delete files on disk. You can remove the directory manually later.",
                                ),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .gap_1()
                                .child({
                                    let view = view.clone();
                                    Button::new("jj-graph-inspector-confirm-workspace-forget")
                                        .primary()
                                        .compact()
                                        .with_size(gpui_component::Size::Small)
                                        .rounded(px(7.0))
                                        .label("Confirm Forget")
                                        .tooltip("Forget selected workspace from repository metadata.")
                                        .disabled(self.git_action_loading)
                                        .on_click(move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                this.confirm_pending_workspace_forget(cx);
                                            });
                                        })
                                })
                                .child({
                                    let view = view.clone();
                                    Button::new("jj-graph-inspector-cancel-workspace-forget")
                                        .outline()
                                        .compact()
                                        .with_size(gpui_component::Size::Small)
                                        .rounded(px(7.0))
                                        .label("Cancel")
                                        .tooltip("Cancel pending workspace forget.")
                                        .disabled(self.git_action_loading)
                                        .on_click(move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                this.cancel_pending_workspace_forget(cx);
                                            });
                                        })
                                }),
                        )
                        .into_any_element();
                }

                div().into_any_element()
            })
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .px_1()
                    .py_1()
                    .rounded(px(6.0))
                    .bg(cx.theme().background.opacity(if is_dark { 0.36 } else { 0.48 }))
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Bookmark Actions"),
                    )
                    .child(
                        Input::new(&self.graph_action_input_state)
                            .h(px(30.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.74 }))
                            .bg(cx.theme().background.opacity(if is_dark { 0.28 } else { 0.18 }))
                            .disabled(self.git_action_loading || bookmark_mutation_disabled),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_1()
                            .flex_wrap()
                            .child({
                                let view = view.clone();
                                Button::new("jj-graph-inspector-create-bookmark")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .label("Create At Revision")
                                    .tooltip(create_tooltip)
                                    .disabled(
                                        self.git_action_loading
                                            || bookmark_mutation_disabled
                                            || !has_selected_node
                                            || graph_action_input_empty
                                            || selected_node_is_working_copy,
                                    )
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.create_graph_bookmark_from_selected_revision(cx);
                                        });
                                    })
                            })
                            .child({
                                let view = view.clone();
                                Button::new("jj-graph-inspector-fork-bookmark")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .label("Fork From Focus")
                                    .tooltip(fork_tooltip)
                                    .disabled(
                                        self.git_action_loading
                                            || bookmark_mutation_disabled
                                            || !has_selected_node
                                            || selected_bookmark_selection.is_none()
                                            || selected_node_is_working_copy,
                                    )
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.fork_graph_bookmark_from_selected_revision(cx);
                                        });
                                    })
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
                                Button::new("jj-graph-inspector-rename-bookmark")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .label("Rename Focused")
                                    .tooltip(rename_tooltip)
                                    .disabled(
                                        self.git_action_loading
                                            || bookmark_mutation_disabled
                                            || !selected_bookmark_is_local
                                            || graph_action_input_empty,
                                    )
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.rename_selected_graph_bookmark_from_input(cx);
                                        });
                                    })
                            })
                            .child({
                                let view = view.clone();
                                Button::new("jj-graph-inspector-move-bookmark")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .dropdown_caret(true)
                                    .label("Move Bookmark")
                                    .tooltip(move_tooltip)
                                    .disabled(
                                        self.git_action_loading
                                            || bookmark_mutation_disabled
                                            || !selected_bookmark_is_local
                                            || !has_selected_node,
                                    )
                                    .dropdown_menu(move |menu, _, _| {
                                        menu.item(
                                            PopupMenuItem::new("Retarget to Selected Revision")
                                                .on_click({
                                                    let view = view.clone();
                                                    move |_, _, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.arm_move_selected_graph_bookmark_to_selected_revision(cx);
                                                        });
                                                    }
                                                }),
                                        )
                                    })
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
                                let blocker = selected_review_blocker.clone();
                                Button::new("jj-graph-inspector-open-review-url")
                                    .primary()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .label("Open PR/MR")
                                    .tooltip(blocker.clone().unwrap_or_else(|| {
                                        "Open a prefilled pull/merge request for the selected local bookmark."
                                            .to_string()
                                    }))
                                    .disabled(selected_review_disabled)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.open_selected_graph_bookmark_review_url(cx);
                                        });
                                    })
                            })
                            .child({
                                let view = view.clone();
                                let blocker = selected_review_blocker.clone();
                                Button::new("jj-graph-inspector-copy-review-url")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .label("Copy Review URL")
                                    .tooltip(blocker.unwrap_or_else(|| {
                                        "Copy a prefilled pull/merge request URL for the selected local bookmark."
                                            .to_string()
                                    }))
                                    .disabled(selected_review_disabled)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.copy_selected_graph_bookmark_review_url(cx);
                                        });
                                    })
                            })
                            .when_some(selected_review_blocker, |this, reason| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .whitespace_normal()
                                        .child(format!("PR/MR unavailable: {reason}")),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .whitespace_normal()
                            .child(
                                "Create/fork can target any selected revision. Rename/move/review URL actions apply to selected local bookmarks.",
                            ),
                    )
                    .when_some(bookmark_mutation_blocker, |this, reason| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .whitespace_normal()
                                .child(format!("Bookmark mutations disabled: {reason}")),
                        )
                    }),
            )
            .child({
                if let Some((bookmark_name, target_node_id)) = move_confirmation {
                    let short_id = target_node_id.chars().take(12).collect::<String>();
                    return v_flex()
                        .w_full()
                        .gap_1()
                        .px_1()
                        .py_1()
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(cx.theme().warning.opacity(if is_dark { 0.90 } else { 0.72 }))
                        .bg(cx.theme().warning.opacity(if is_dark { 0.18 } else { 0.10 }))
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child("Confirm Destructive Move"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().foreground)
                                .whitespace_normal()
                                .child(format!(
                                    "Move bookmark {} to revision {}?",
                                    bookmark_name, short_id
                                )),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .gap_1()
                                .child({
                                    let view = view.clone();
                                    let tooltip = bookmark_mutation_blocker
                                        .map(str::to_string)
                                        .unwrap_or_else(|| {
                                            "Apply bookmark retargeting to the selected revision."
                                                .to_string()
                                        });
                                    Button::new("jj-graph-inspector-confirm-move")
                                        .primary()
                                        .compact()
                                        .with_size(gpui_component::Size::Small)
                                        .rounded(px(7.0))
                                        .label("Confirm Move")
                                        .tooltip(tooltip)
                                        .disabled(self.git_action_loading || bookmark_mutation_disabled)
                                        .on_click(move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                this.confirm_graph_pending_confirmation(cx);
                                            });
                                        })
                                })
                                .child({
                                    let view = view.clone();
                                    Button::new("jj-graph-inspector-cancel-move")
                                        .outline()
                                        .compact()
                                        .with_size(gpui_component::Size::Small)
                                        .rounded(px(7.0))
                                        .label("Cancel")
                                        .tooltip("Cancel the pending bookmark move.")
                                        .disabled(self.git_action_loading)
                                        .on_click(move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                this.cancel_graph_pending_confirmation(cx);
                                            });
                                        })
                                }),
                        )
                        .into_any_element();
                }

                div().into_any_element()
            })
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_1()
                    .flex_wrap()
                    .child({
                        let view = view.clone();
                        Button::new("jj-graph-inspector-focus-active")
                            .outline()
                            .compact()
                            .with_size(gpui_component::Size::Small)
                            .rounded(px(7.0))
                            .label("Focus Active")
                            .tooltip("Select and focus the currently active bookmark.")
                            .disabled(self.graph_active_bookmark.is_none())
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.select_active_graph_bookmark(cx);
                                });
                            })
                    })
                    .child({
                        let view = view.clone();
                        Button::new("jj-graph-inspector-clear-bookmark")
                            .outline()
                            .compact()
                            .with_size(gpui_component::Size::Small)
                            .rounded(px(7.0))
                            .label("Clear Bookmark Focus")
                            .tooltip("Exit bookmark focus and return to full graph context.")
                            .disabled(self.graph_selected_bookmark.is_none())
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.clear_graph_bookmark_selection(cx);
                                });
                            })
                    }),
            )
            .into_any_element()
    }
}
