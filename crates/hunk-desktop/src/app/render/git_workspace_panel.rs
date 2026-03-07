impl DiffViewer {
    fn render_git_workspace_operations_panel_v2(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let activate_branch_loading = self.git_action_loading_named("Activate branch");
        let sync_loading = self.git_action_loading_named("Sync branch");
        let publish_loading = self.git_action_loading_named("Publish branch");
        let rename_loading = self.git_action_loading_named("Rename branch");
        let open_review_loading = self.git_action_loading_named("Open PR/MR");
        let copy_review_loading = self.git_action_loading_named("Copy PR/MR URL");
        let create_commit_loading = self.git_action_loading_named("Create commit");
        let push_loading = self.git_action_loading_named("Push branch");
        let branch_syncable = self.can_run_active_branch_actions();
        let sync_disabled = !self.can_sync_current_branch();
        let publish_disabled = !self.can_publish_current_branch();
        let push_available = self.can_push_current_branch() || push_loading;
        let push_disabled = !push_available || (self.git_action_loading && !push_loading);
        let sync_tooltip = if !branch_syncable {
            "Activate a branch before syncing."
        } else if !self.branch_has_upstream {
            "Publish this branch before syncing."
        } else if !self.files.is_empty() {
            "Commit or discard working tree changes before syncing."
        } else {
            "Fetch and fast-forward this branch from its upstream remote."
        };
        let publish_state_tooltip = if self.branch_has_upstream {
            "This branch already tracks upstream. Use Push below."
        } else if !branch_syncable {
            "Activate a branch before publishing."
        } else if !self.files.is_empty() {
            "Commit or discard working tree changes before publishing."
        } else {
            "Publish this branch to upstream and start tracking it."
        };
        let active_review_blocker = self.active_review_action_blocker();
        let review_url_disabled = active_review_blocker.is_some();
        let push_tooltip = if !branch_syncable {
            "Activate a branch before pushing."
        } else if !self.branch_has_upstream {
            "Publish this branch before pushing."
        } else if self.branch_ahead_count == 0 {
            "No local commits to push."
        } else if !self.files.is_empty() {
            "Commit or discard working tree changes before pushing."
        } else {
            "Push all local commits on this branch."
        };

        let active_branch_label = self
            .checked_out_branch_name()
            .map_or_else(|| "detached".to_string(), ToOwned::to_owned);
        let active_branch_chip_label = active_branch_label.clone();
        let sync_state_label = if !branch_syncable {
            "Detached".to_string()
        } else if self.branch_has_upstream {
            if self.branch_ahead_count > 0 {
                format!("{} to push", self.branch_ahead_count)
            } else {
                "Up to date".to_string()
            }
        } else {
            "Not published".to_string()
        };

        let branch_menu_entries = self
            .branches
            .iter()
            .map(|branch| {
                (
                    branch.name.clone(),
                    branch.is_current,
                    relative_time_label(branch.tip_unix_time),
                )
            })
            .collect::<Vec<_>>();

        let last_commit_text = self
            .last_commit_subject
            .as_deref()
            .map(str::trim_end)
            .filter(|text| !text.is_empty())
            .unwrap_or("No commits yet")
            .to_string();

        let included_count = self.included_commit_file_count();
        let total_count = self.files.len();
        let commit_message_present = !self.commit_input_state.read(cx).value().trim().is_empty();
        let commit_disabled = !commit_message_present
            || included_count == 0
            || (self.git_action_loading && !create_commit_loading);

        let branch_input_empty = self.branch_input_state.read(cx).value().trim().is_empty();
        let rename_disabled =
            self.git_action_loading || branch_input_empty || !self.can_run_active_branch_actions();
        let create_or_activate_disabled = self.git_action_loading || branch_input_empty;

        v_flex()
            .w_full()
            .gap_2()
            .px_3()
            .pt_2()
            .pb_2()
            .bg(hunk_blend(
                cx.theme().sidebar,
                cx.theme().muted,
                is_dark,
                0.16,
                0.24,
            ))
            .child(self.render_git_action_status_banner(cx))
            .child(self.render_workspace_changes_panel(cx))
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .p_2()
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(hunk_opacity(cx.theme().border, is_dark, 0.90, 0.74))
                    .bg(hunk_blend(
                        cx.theme().background,
                        cx.theme().muted,
                        is_dark,
                        0.20,
                        0.26,
                    ))
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Branches"),
                            )
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_1()
                            .flex_wrap()
                            .child(
                                Button::new("branch-selector-v2")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .loading(activate_branch_loading)
                                    .min_w(px(150.0))
                                    .bg(hunk_opacity(cx.theme().secondary, is_dark, 0.50, 0.70))
                                    .border_color(hunk_opacity(
                                        cx.theme().border,
                                        is_dark,
                                        0.90,
                                        0.74,
                                    ))
                                    .label(active_branch_chip_label)
                                    .dropdown_caret(true)
                                    .tooltip("Select a branch to activate it.")
                                    .disabled(self.git_action_loading)
                                    .dropdown_menu({
                                        let view = view.clone();
                                        move |menu, _, _| {
                                            branch_menu_entries.iter().fold(menu, |menu, entry| {
                                                let view = view.clone();
                                                let branch_name = entry.0.clone();
                                                let branch_label = format!("{} · {}", entry.0, entry.2);

                                                menu.item(
                                                    PopupMenuItem::new(branch_label)
                                                        .checked(entry.1)
                                                        .on_click(move |_, window, cx| {
                                                                view.update(cx, |this, cx| {
                                                                this.checkout_branch(
                                                                    branch_name.clone(),
                                                                    window,
                                                                    cx,
                                                                );
                                                            });
                                                        }),
                                                )
                                            })
                                        }
                                    }),
                            )
                            .child({
                                let view = view.clone();
                                Button::new("sync-branch-v2")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .loading(sync_loading)
                                    .min_w(px(92.0))
                                    .label("Sync")
                                    .tooltip(sync_tooltip)
                                    .disabled(sync_disabled)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.sync_current_branch_from_remote(cx);
                                        });
                                    })
                            })
                            .child({
                                let view = view.clone();
                                let mut button = Button::new("branch-publish-state-v2")
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .loading(publish_loading)
                                    .min_w(px(104.0))
                                    .label(if self.branch_has_upstream {
                                        "Published"
                                    } else {
                                        "Publish"
                                    })
                                    .tooltip(publish_state_tooltip)
                                    .disabled(self.branch_has_upstream || publish_disabled)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.publish_current_branch(cx);
                                        });
                                    });
                                if self.branch_has_upstream {
                                    button = button.outline();
                                } else {
                                    button = button.primary();
                                }
                                button.into_any_element()
                            })
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("State: {sync_state_label}")),
                    )
                    .child(
                        Input::new(&self.branch_input_state)
                            .rounded(px(8.0))
                            .border_1()
                            .border_color(hunk_opacity(cx.theme().border, is_dark, 0.92, 0.76))
                            .bg(hunk_blend(
                                cx.theme().background,
                                cx.theme().muted,
                                is_dark,
                                0.22,
                                0.14,
                            ))
                            .disabled(self.git_action_loading),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_1()
                            .flex_wrap()
                            .child({
                                let view = view.clone();
                                Button::new("create-or-switch-branch-v2")
                                    .primary()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .loading(activate_branch_loading)
                                    .label("Create / Activate")
                                    .tooltip("Create a branch from the entered name or activate it if it already exists.")
                                    .disabled(create_or_activate_disabled)
                                    .on_click(move |_, window, cx| {
                                        view.update(cx, |this, cx| {
                                            this.create_or_switch_branch_from_input(window, cx);
                                        });
                                    })
                            })
                            .child({
                                let view = view.clone();
                                Button::new("rename-active-branch-v2")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .loading(rename_loading)
                                    .label("Rename Active")
                                    .tooltip("Rename the currently active branch to the entered name.")
                                    .disabled(rename_disabled)
                                    .on_click(move |_, window, cx| {
                                        view.update(cx, |this, cx| {
                                            this.rename_current_branch_from_input(window, cx);
                                        });
                                    })
                            })
                            .child({
                                let view = view.clone();
                                let blocker = active_review_blocker.clone();
                                Button::new("open-review-url-v2")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .loading(open_review_loading)
                                    .label("Open PR/MR")
                                    .tooltip(blocker.clone().unwrap_or_else(|| {
                                        "Open a prefilled pull/merge request page for the active branch.".to_string()
                                    }))
                                    .disabled(review_url_disabled)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.open_current_branch_review_url(cx);
                                        });
                                    })
                            })
                            .child({
                                let view = view.clone();
                                let blocker = active_review_blocker.clone();
                                Button::new("copy-review-url-v2")
                                    .outline()
                                    .compact()
                                    .with_size(gpui_component::Size::Small)
                                    .rounded(px(7.0))
                                    .loading(copy_review_loading)
                                    .label("Copy Review URL")
                                    .tooltip(blocker.unwrap_or_else(|| {
                                        "Copy a prefilled pull/merge request URL for the active branch.".to_string()
                                    }))
                                    .disabled(review_url_disabled)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.copy_current_branch_review_url(cx);
                                        });
                                    })
                            }),
                    ),
            )
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .p_2()
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(hunk_opacity(cx.theme().border, is_dark, 0.90, 0.74))
                    .bg(hunk_blend(
                        cx.theme().background,
                        cx.theme().muted,
                        is_dark,
                        0.24,
                        0.26,
                    ))
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
                                    Button::new("commit-include-all-v2")
                                        .outline()
                                        .compact()
                                        .rounded(px(7.0))
                                        .label("Include All")
                                        .tooltip("Include every changed file in the next commit.")
                                        .disabled(self.git_action_loading)
                                        .on_click(move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                this.include_all_files_for_commit(cx);
                                            });
                                        })
                                })
                            }),
                    )
                    .child(
                        Input::new(&self.commit_input_state)
                            .h(px(82.0))
                            .rounded(px(8.0))
                            .border_1()
                            .border_color(hunk_opacity(cx.theme().border, is_dark, 0.92, 0.78))
                            .bg(hunk_blend(
                                cx.theme().background,
                                cx.theme().muted,
                                is_dark,
                                0.24,
                                0.12,
                            ))
                            .disabled(self.git_action_loading),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_1()
                            .flex_wrap()
                            .child({
                                let view = view.clone();
                                Button::new("commit-staged-v2")
                                    .primary()
                                    .rounded(px(7.0))
                                    .loading(create_commit_loading)
                                    .label(if create_commit_loading {
                                        "Creating Commit..."
                                    } else {
                                        "Create Commit"
                                    })
                                    .tooltip("Create a new commit from included files using the message above.")
                                    .disabled(commit_disabled)
                                    .on_click(move |_, window, cx| {
                                        view.update(cx, |this, cx| {
                                            this.commit_from_input(window, cx);
                                        });
                                    })
                            })
                            .child({
                                let view = view.clone();
                                Button::new("push-branch-v2")
                                    .outline()
                                    .rounded(px(7.0))
                                    .loading(push_loading)
                                    .label(if push_loading {
                                        "Pushing..."
                                    } else {
                                        "Push"
                                    })
                                    .tooltip(push_tooltip)
                                    .disabled(push_disabled)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.push_current_branch(cx);
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
                            .border_color(hunk_opacity(cx.theme().border, is_dark, 0.92, 0.76))
                            .bg(hunk_opacity(cx.theme().secondary, is_dark, 0.42, 0.56))
                            .text_xs()
                            .font_medium()
                            .text_color(cx.theme().foreground.opacity(0.90))
                            .whitespace_normal()
                            .child(last_commit_text),
                    ),
            )
            .into_any_element()
    }
}
