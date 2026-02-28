impl DiffViewer {
    fn render_jj_graph_operations_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let branch_syncable = self.can_run_active_bookmark_actions();
        let show_sync = branch_syncable && self.branch_has_upstream;
        let show_publish = branch_syncable && !self.branch_has_upstream;
        let show_push = branch_syncable && self.branch_has_upstream;
        let sync_disabled = !self.can_sync_current_bookmark();
        let push_or_publish_disabled = !self.can_push_or_publish_current_bookmark();
        let review_url_disabled = self.git_action_loading || !branch_syncable;
        let action_label = if show_publish {
            "Publish Bookmark"
        } else {
            "Push Bookmark"
        };
        let active_bookmark_label = self
            .checked_out_bookmark_name()
            .map_or_else(|| "detached".to_string(), ToOwned::to_owned);
        let sync_state_label = if !branch_syncable {
            "Detached".to_string()
        } else if self.branch_has_upstream {
            if self.branch_ahead_count > 0 {
                "Needs push".to_string()
            } else {
                "Up to date".to_string()
            }
        } else {
            "Not published".to_string()
        };
        let last_commit_text = self
            .last_commit_subject
            .as_deref()
            .map(str::trim_end)
            .filter(|text| !text.is_empty())
            .unwrap_or("No commits yet");
        let included_count = self.included_commit_file_count();
        let total_count = self.files.len();
        let commit_message_present = !self.commit_input_state.read(cx).value().trim().is_empty();
        let commit_disabled =
            self.git_action_loading || !commit_message_present || included_count == 0;
        let describe_tip_disabled = self.git_action_loading
            || !commit_message_present
            || !branch_syncable
            || self.bookmark_revisions.is_empty();

        v_flex()
            .size_full()
            .gap_2()
            .px_3()
            .pt_2()
            .pb_2()
            .bg(cx.theme().sidebar.blend(cx.theme().muted.opacity(if is_dark {
                0.16
                } else {
                0.24
            })))
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Bookmarks & Revisions"),
                    )
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
                                    .child(format!("Active bookmark: {active_bookmark_label}")),
                            )
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded(px(6.0))
                                    .text_xs()
                                    .font_semibold()
                                    .bg(cx.theme().secondary.opacity(if is_dark { 0.54 } else { 0.70 }))
                                    .text_color(cx.theme().foreground)
                                    .child(sync_state_label),
                            ),
                    ),
            )
            .when_some(self.git_status_message.as_ref(), |this, message| {
                this.child(
                    div()
                        .w_full()
                        .px_2()
                        .py_1()
                        .rounded(px(8.0))
                        .border_1()
                        .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.70 }))
                        .bg(cx.theme().background.blend(cx.theme().muted.opacity(if is_dark {
                            0.24
                        } else {
                            0.32
                        })))
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().muted_foreground)
                        .whitespace_normal()
                        .child(message.clone()),
                )
            })
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_1()
                    .child({
                        let view = view.clone();
                        Button::new("branch-picker-label")
                            .outline()
                            .compact()
                            .with_size(gpui_component::Size::Small)
                            .rounded(px(7.0))
                            .bg(cx.theme().secondary.opacity(if is_dark { 0.50 } else { 0.70 }))
                            .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.74 }))
                            .label(active_bookmark_label.clone())
                            .disabled(self.git_action_loading)
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.toggle_bookmark_picker(cx);
                                });
                            })
                    })
                    .child({
                        let view = view.clone();
                        let mut button = Button::new("branch-picker-toggle")
                            .outline()
                            .compact()
                            .with_size(gpui_component::Size::Small)
                            .rounded(px(7.0))
                            .min_w(px(24.0))
                            .h(px(24.0))
                            .icon(
                                Icon::new(if self.branch_picker_open {
                                    IconName::ChevronUp
                                } else {
                                    IconName::ChevronDown
                                })
                                .size(px(12.0)),
                            )
                            .tooltip(if self.branch_picker_open {
                                "Hide bookmark menu"
                            } else {
                                "Show bookmark menu"
                            })
                            .disabled(self.git_action_loading)
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.toggle_bookmark_picker(cx);
                                });
                            });

                        if self.branch_picker_open {
                            button = button.primary();
                        }

                        button.into_any_element()
                    })
                    .when(show_sync, |this| {
                        this.child({
                            let view = view.clone();
                            Button::new("sync-branch")
                                .outline()
                                .compact()
                                .with_size(gpui_component::Size::Small)
                                .rounded(px(7.0))
                                .label("Sync Bookmark")
                                .disabled(sync_disabled)
                                .on_click(move |_, _, cx| {
                                    view.update(cx, |this, cx| {
                                        this.sync_current_bookmark_from_remote(cx);
                                    });
                                })
                        })
                    })
                    .when(show_publish || show_push, |this| {
                        this.child({
                            let view = view.clone();
                            Button::new("publish-or-push")
                                .primary()
                                .compact()
                                .with_size(gpui_component::Size::Small)
                                .rounded(px(7.0))
                                .label(action_label)
                                .disabled(push_or_publish_disabled)
                                .on_click(move |_, _, cx| {
                                    view.update(cx, |this, cx| {
                                        this.push_or_publish_current_bookmark(cx);
                                    });
                                })
                            })
                    })
                    .child({
                        let view = view.clone();
                        Button::new("open-review-url")
                            .primary()
                            .compact()
                            .with_size(gpui_component::Size::Small)
                            .rounded(px(7.0))
                            .label("Open PR/MR")
                            .disabled(review_url_disabled)
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.open_current_bookmark_review_url(cx);
                                });
                            })
                    })
                    .child({
                        let view = view.clone();
                        Button::new("copy-review-url")
                            .outline()
                            .compact()
                            .with_size(gpui_component::Size::Small)
                            .rounded(px(7.0))
                            .label("Copy Review URL")
                            .disabled(review_url_disabled)
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.copy_current_bookmark_review_url(cx);
                                });
                            })
                    }),
            )
            .child(self.render_workspace_changes_panel(cx))
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
                            .child(format!("Commit includes {included_count}/{total_count} files")),
                    )
                    .when(included_count < total_count, |this| {
                        this.child({
                            let view = view.clone();
                            Button::new("commit-include-all")
                                .outline()
                                .compact()
                                .rounded(px(7.0))
                                .label("Include All")
                                .disabled(self.git_action_loading)
                                .on_click(move |_, _, cx| {
                                    view.update(cx, |this, cx| {
                                        this.include_all_files_for_commit(cx);
                                    });
                                })
                        })
                    }),
            )
            .when(self.branch_picker_open, |this| {
                this.child(self.render_branch_picker_panel(cx))
            })
            .child(self.render_revision_stack_panel(cx))
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
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_1()
                    .child({
                        let view = view.clone();
                        Button::new("commit-staged")
                            .primary()
                            .rounded(px(7.0))
                            .label("Create Revision")
                            .disabled(commit_disabled)
                            .on_click(move |_, window, cx| {
                                view.update(cx, |this, cx| {
                                    this.commit_from_input(window, cx);
                                });
                            })
                    })
                    .child({
                        let view = view.clone();
                        Button::new("describe-tip-revision")
                            .outline()
                            .rounded(px(7.0))
                            .label("Edit Tip Revision")
                            .disabled(describe_tip_disabled)
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.describe_current_bookmark_from_input(cx);
                                });
                            })
                    }),
            )
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

    fn render_revision_stack_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let revisions = &self.bookmark_revisions;
        let actions_enabled = self.can_run_active_bookmark_actions();
        let can_abandon_tip =
            !self.git_action_loading && actions_enabled && !revisions.is_empty();
        let can_squash_tip =
            !self.git_action_loading && actions_enabled && revisions.len() >= 2;
        let can_reorder_tip =
            !self.git_action_loading && actions_enabled && revisions.len() >= 2;

        v_flex()
            .w_full()
            .gap_1()
            .p_2()
            .rounded(px(8.0))
            .border_1()
            .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.74 }))
            .bg(cx.theme().background.blend(cx.theme().muted.opacity(if is_dark {
                0.20
            } else {
                0.26
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
                            .child("Revision Stack"),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{}", revisions.len())),
                            )
                            .child({
                                let view = view.clone();
                                Button::new("reorder-top-two-revisions")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .label("Swap Top 2")
                                    .disabled(!can_reorder_tip)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.reorder_current_bookmark_tip_older(cx);
                                        });
                                    })
                            })
                            .child({
                                let view = view.clone();
                                Button::new("squash-tip-revision")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .label("Squash Tip")
                                    .disabled(!can_squash_tip)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.squash_current_bookmark_tip_into_parent(cx);
                                        });
                                    })
                            })
                            .child({
                                let view = view.clone();
                                Button::new("abandon-tip-revision")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .label("Abandon Tip")
                                    .disabled(!can_abandon_tip)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.abandon_current_bookmark_tip(cx);
                                        });
                                    })
                            }),
                    ),
            )
            .child({
                if revisions.is_empty() {
                    return div()
                        .w_full()
                        .px_1()
                        .py_0p5()
                        .rounded(px(6.0))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("No revisions for this bookmark.")
                        .into_any_element();
                }

                v_flex()
                    .w_full()
                    .max_h(px(180.0))
                    .overflow_y_scrollbar()
                    .gap_0p5()
                    .children(revisions.iter().enumerate().map(|(ix, revision)| {
                        let short_id = revision.id.chars().take(12).collect::<String>();
                        let row_bg = if ix == 0 {
                            cx.theme().accent.opacity(if is_dark { 0.18 } else { 0.10 })
                        } else {
                            cx.theme().background.opacity(0.0)
                        };

                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_1()
                            .px_1()
                            .py_0p5()
                            .rounded(px(6.0))
                            .bg(row_bg)
                            .child(
                                div()
                                    .px_1()
                                    .py_0p5()
                                    .rounded(px(4.0))
                                    .text_xs()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .text_color(cx.theme().muted_foreground)
                                    .bg(cx.theme().muted.opacity(if is_dark { 0.32 } else { 0.42 }))
                                    .child(short_id),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_xs()
                                    .text_color(cx.theme().foreground)
                                    .child(revision.subject.clone()),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .whitespace_nowrap()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(relative_time_label(Some(revision.unix_time))),
                            )
                            .into_any_element()
                    }))
                    .into_any_element()
            })
            .into_any_element()
    }

    fn render_workspace_changes_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut tracked = Vec::with_capacity(self.files.len());
        let mut untracked = Vec::new();
        for file in &self.files {
            if file.is_tracked() {
                tracked.push(file);
            } else {
                untracked.push(file);
            }
        }
        let is_dark = cx.theme().mode.is_dark();

        v_flex()
            .w_full()
            .gap_1()
            .p_2()
            .rounded(px(8.0))
            .border_1()
            .border_color(cx.theme().border.opacity(if is_dark { 0.90 } else { 0.74 }))
            .bg(cx.theme().background.blend(cx.theme().muted.opacity(if is_dark {
                0.20
            } else {
                0.26
            })))
            .child(
                div()
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().muted_foreground)
                    .child("Working Copy"),
            )
            .child(self.render_workspace_change_section("Tracked", &tracked, cx))
            .child(self.render_workspace_change_section("Untracked", &untracked, cx))
            .into_any_element()
    }

    fn render_workspace_change_section(
        &self,
        title: &'static str,
        files: &[&ChangedFile],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        v_flex()
            .w_full()
            .gap_1()
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
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{}", files.len())),
                    ),
            )
            .child({
                if files.is_empty() {
                    return div()
                        .w_full()
                        .px_1()
                        .py_0p5()
                        .rounded(px(6.0))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("No files")
                        .into_any_element();
                }

                v_flex()
                    .w_full()
                    .max_h(px(120.0))
                    .overflow_y_scrollbar()
                    .gap_0p5()
                    .children(files.iter().enumerate().map(|(row_ix, file)| {
                        let section_offset: usize = if title == "Tracked" { 0 } else { 1_000_000 };
                        self.render_workspace_change_row(section_offset + row_ix, file, cx)
                    }))
                    .into_any_element()
            })
            .into_any_element()
    }

    fn render_workspace_change_row(
        &self,
        row_id: usize,
        file: &ChangedFile,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let view = cx.entity();
        let included_in_commit = !self.commit_excluded_files.contains(file.path.as_str());
        let is_selected = self.selected_path.as_deref() == Some(file.path.as_str());
        let is_dark = cx.theme().mode.is_dark();
        let (status_label, status_color) = change_status_label_color(file.status, cx);
        let row_bg = if is_selected {
            cx.theme().accent.opacity(if is_dark { 0.22 } else { 0.14 })
        } else {
            cx.theme().background.opacity(0.0)
        };
        let path = file.path.clone();

        h_flex()
            .id(("workspace-change-row", row_id))
            .w_full()
            .items_center()
            .gap_1()
            .px_1()
            .py_0p5()
            .rounded(px(6.0))
            .bg(row_bg)
            .child({
                let view = view.clone();
                let path = path.clone();
                let include = included_in_commit;
                Button::new(("workspace-commit-include-toggle", row_id))
                    .outline()
                    .compact()
                    .rounded(px(5.0))
                    .min_w(px(22.0))
                    .h(px(20.0))
                    .label(if include { "x" } else { "" })
                    .tooltip(if include {
                        "Included in next revision"
                    } else {
                        "Excluded from next revision"
                    })
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        view.update(cx, |this, cx| {
                            this.toggle_commit_file_included(path.clone(), !include, cx);
                        });
                    })
            })
            .child(
                div()
                    .px_1()
                    .py_0p5()
                    .rounded(px(4.0))
                    .text_xs()
                    .font_semibold()
                    .bg(status_color.opacity(if is_dark { 0.24 } else { 0.16 }))
                    .text_color(cx.theme().foreground)
                    .child(status_label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_xs()
                    .text_color(cx.theme().foreground)
                    .child(path.clone()),
            )
            .on_click(move |_, _, cx| {
                view.update(cx, |this, cx| {
                    this.select_file(path.clone(), cx);
                });
            })
            .into_any_element()
    }

    fn render_branch_picker_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let bookmark_input_empty = self.branch_input_state.read(cx).value().trim().is_empty();
        let rename_disabled =
            self.git_action_loading || bookmark_input_empty || !self.can_run_active_bookmark_actions();
        let create_or_activate_disabled = self.git_action_loading || bookmark_input_empty;

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
                    .child("Bookmarks"),
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
                                    let activate_view = view.clone();
                                    let activate_branch_name = branch_name.clone();
                                    let move_disabled = self.git_action_loading
                                        || self.files.is_empty()
                                        || branch.is_current;

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
                                        .on_click(move |_, window, cx| {
                                            activate_view.update(cx, |this, cx| {
                                                this.checkout_bookmark(
                                                    activate_branch_name.clone(),
                                                    window,
                                                    cx,
                                                );
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
                                        .child({
                                            let move_view = view.clone();
                                            let move_branch_name = branch_name.clone();
                                            Button::new(("bookmark-row-move", ix))
                                                .outline()
                                                .compact()
                                                .rounded(px(6.0))
                                                .label("Move")
                                                .disabled(move_disabled)
                                                .tooltip("Move local changes and activate bookmark")
                                                .on_click(move |_, _, cx| {
                                                    cx.stop_propagation();
                                                    move_view.update(cx, |this, cx| {
                                                        this.checkout_bookmark_with_change_transfer(
                                                            move_branch_name.clone(),
                                                            cx,
                                                        );
                                                    });
                                                })
                                        })
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
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_1()
                    .child(
                        Button::new("create-or-switch-bookmark")
                            .primary()
                            .rounded(px(7.0))
                            .label("Create / Activate")
                            .disabled(create_or_activate_disabled)
                            .on_click({
                                let view = view.clone();
                                move |_, window, cx| {
                                    view.update(cx, |this, cx| {
                                        this.create_or_switch_bookmark_from_input(window, cx);
                                    });
                                }
                            }),
                    )
                    .child(
                        Button::new("rename-active-bookmark")
                            .outline()
                            .rounded(px(7.0))
                            .label("Rename Active")
                            .disabled(rename_disabled)
                            .on_click(move |_, window, cx| {
                                view.update(cx, |this, cx| {
                                    this.rename_current_bookmark_from_input(window, cx);
                                });
                            }),
                    )
            })
            .into_any_element()
    }

}
