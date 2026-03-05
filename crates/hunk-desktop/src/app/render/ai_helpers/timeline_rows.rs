#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AiTimelineItemRole {
    User,
    Assistant,
    Tool,
}

fn ai_timeline_item_role(kind: &str) -> AiTimelineItemRole {
    match kind {
        "userMessage" => AiTimelineItemRole::User,
        "agentMessage" | "plan" => AiTimelineItemRole::Assistant,
        _ => AiTimelineItemRole::Tool,
    }
}

fn ai_timeline_item_is_renderable(item: &hunk_codex::state::ItemSummary) -> bool {
    if matches!(item.kind.as_str(), "reasoning" | "webSearch") {
        let has_content = !item.content.trim().is_empty();
        let has_metadata = item.display_metadata.as_ref().is_some_and(|metadata| {
            metadata
                .summary
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
                || metadata
                    .details_json
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
        });
        return has_content || has_metadata;
    }

    true
}

fn ai_timeline_row_is_renderable(this: &DiffViewer, row: &AiTimelineRow) -> bool {
    match &row.source {
        AiTimelineRowSource::Item { item_key } => this
            .ai_state_snapshot
            .items
            .get(item_key.as_str())
            .is_some_and(ai_timeline_item_is_renderable),
        AiTimelineRowSource::TurnDiff { turn_key } => this
            .ai_state_snapshot
            .turn_diffs
            .get(turn_key.as_str())
            .is_some_and(|diff| !diff.trim().is_empty()),
    }
}

fn render_ai_chat_timeline_row_for_view(
    this: &DiffViewer,
    row_id: &str,
    view: Entity<DiffViewer>,
    is_dark: bool,
    cx: &mut Context<DiffViewer>,
) -> AnyElement {
    let Some(row) = this.ai_timeline_row(row_id) else {
        return div().w_full().h(px(0.0)).into_any_element();
    };
    if !ai_timeline_row_is_renderable(this, row) {
        return div().w_full().h(px(0.0)).into_any_element();
    }

    match &row.source {
        AiTimelineRowSource::Item { item_key } => {
            let Some(item) = this.ai_state_snapshot.items.get(item_key.as_str()) else {
                return div().w_full().h(px(0.0)).into_any_element();
            };
            let role = ai_timeline_item_role(item.kind.as_str());
            match role {
                AiTimelineItemRole::User | AiTimelineItemRole::Assistant => {
                    let is_user = role == AiTimelineItemRole::User;
                    let bubble_bg = if is_user {
                        cx.theme().accent.opacity(if is_dark { 0.18 } else { 0.12 })
                    } else {
                        cx.theme().background.blend(cx.theme().muted.opacity(if is_dark {
                            0.18
                        } else {
                            0.26
                        }))
                    };
                    let bubble_border = if is_user {
                        cx.theme().accent.opacity(if is_dark { 0.72 } else { 0.48 })
                    } else {
                        cx.theme().border.opacity(if is_dark { 0.9 } else { 0.72 })
                    };
                    let role_label = if is_user {
                        "You"
                    } else if item.kind == "plan" {
                        "Plan"
                    } else {
                        "Assistant"
                    };
                    let status = ai_item_status_label(item.status);
                    let status_color = ai_item_status_color(item.status, cx);
                    let text_content = item.content.trim();
                    let fallback_summary = item
                        .display_metadata
                        .as_ref()
                        .and_then(|metadata| metadata.summary.as_deref())
                        .unwrap_or_default();
                    let bubble_text = if text_content.is_empty() {
                        fallback_summary
                    } else {
                        text_content
                    };

                    let row_element = h_flex()
                        .w_full()
                        .when(is_user, |this| this.justify_end())
                        .when(!is_user, |this| this.justify_start())
                        .child(
                            v_flex()
                                .max_w(px(760.0))
                                .gap_1()
                                .px_3()
                                .py_2()
                                .rounded(px(12.0))
                                .border_1()
                                .border_color(bubble_border)
                                .bg(bubble_bg)
                                .child(
                                    h_flex()
                                        .w_full()
                                        .items_center()
                                        .justify_between()
                                        .gap_2()
                                        .child(
                                            div().text_xs().font_semibold().child(role_label),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(status_color)
                                                .child(status),
                                        ),
                                )
                                .when(!bubble_text.is_empty(), |this| {
                                    this.child(
                                        div()
                                            .text_sm()
                                            .whitespace_normal()
                                            .child(bubble_text.to_string()),
                                    )
                                }),
                        );
                    ai_timeline_row_with_animation(this, row.id.as_str(), row_element)
                }
                AiTimelineItemRole::Tool => {
                    let label = item
                        .display_metadata
                        .as_ref()
                        .and_then(|metadata| metadata.summary.as_deref())
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| ai_item_display_label(item.kind.as_str()));
                    let status = ai_item_status_label(item.status);
                    let status_color = ai_item_status_color(item.status, cx);
                    let content_text = item.content.trim();
                    let details_json = item
                        .display_metadata
                        .as_ref()
                        .and_then(|metadata| metadata.details_json.as_deref())
                        .map(str::trim)
                        .filter(|value| !value.is_empty());
                    let details_text = details_json.unwrap_or(content_text);
                    let has_details = !details_text.is_empty();
                    let expanded = has_details && this.ai_expanded_timeline_row_ids.contains(row.id.as_str());
                    let (preview, _preview_truncated) = if !content_text.is_empty() {
                        ai_truncate_multiline_content(content_text, 3)
                    } else {
                        (String::new(), false)
                    };
                    let show_preview = !preview.is_empty() && !expanded;
                    let show_toggle = has_details;
                    let toggle_id =
                        format!("ai-toggle-timeline-row-{}", row.id.replace('\u{1f}', "--"));

                    let row_element = h_flex()
                        .w_full()
                        .justify_start()
                        .child(
                            v_flex()
                                .max_w(px(900.0))
                                .w_full()
                                .gap_1()
                                .px_2p5()
                                .py_2()
                                .rounded(px(10.0))
                                .border_1()
                                .border_color(cx.theme().border.opacity(if is_dark {
                                    0.88
                                } else {
                                    0.72
                                }))
                                .bg(cx.theme().background.blend(cx.theme().muted.opacity(if is_dark {
                                    0.14
                                } else {
                                    0.20
                                })))
                                .child(
                                    h_flex()
                                        .w_full()
                                        .items_center()
                                        .justify_between()
                                        .gap_2()
                                        .child(
                                            h_flex()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_semibold()
                                                        .child(label.to_string()),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(status_color)
                                                        .child(status),
                                                ),
                                        )
                                        .when(show_toggle, |this| {
                                            let row_id = row.id.clone();
                                            let view = view.clone();
                                            this.child(
                                                Button::new(toggle_id)
                                                    .compact()
                                                    .outline()
                                                    .with_size(gpui_component::Size::Small)
                                                    .icon(
                                                        Icon::new(if expanded {
                                                            IconName::ChevronDown
                                                        } else {
                                                            IconName::ChevronRight
                                                        })
                                                        .size(px(12.0)),
                                                    )
                                                    .tooltip(if expanded {
                                                        "Hide details"
                                                    } else {
                                                        "Show details"
                                                    })
                                                    .on_click(move |_, _, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.ai_toggle_timeline_row_expansion_action(
                                                                row_id.clone(),
                                                                cx,
                                                            );
                                                        });
                                                    }),
                                            )
                                        }),
                                )
                                .when(show_preview, |this| {
                                    this.child(
                                        div()
                                            .text_xs()
                                            .font_family(cx.theme().mono_font_family.clone())
                                            .text_color(cx.theme().muted_foreground)
                                            .whitespace_normal()
                                            .child(preview.clone()),
                                    )
                                })
                                .when(expanded, |this| {
                                    this.child(
                                        div()
                                            .w_full()
                                            .rounded(px(8.0))
                                            .border_1()
                                            .border_color(cx.theme().border.opacity(if is_dark {
                                                0.85
                                            } else {
                                                0.68
                                            }))
                                            .bg(cx.theme().background.blend(
                                                cx.theme().muted.opacity(if is_dark {
                                                    0.10
                                                } else {
                                                    0.14
                                                }),
                                            ))
                                            .px_2()
                                            .py_1p5()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_family(cx.theme().mono_font_family.clone())
                                                    .text_color(cx.theme().muted_foreground)
                                                    .whitespace_normal()
                                                    .child(details_text.to_string()),
                                            ),
                                    )
                                }),
                        );
                    ai_timeline_row_with_animation(this, row.id.as_str(), row_element)
                }
            }
        }
        AiTimelineRowSource::TurnDiff { turn_key } => {
            let Some(diff) = this.ai_state_snapshot.turn_diffs.get(turn_key.as_str()) else {
                return div().w_full().h(px(0.0)).into_any_element();
            };
            let diff_text = diff.trim();
            if diff_text.is_empty() {
                return div().w_full().h(px(0.0)).into_any_element();
            }
            let diff_line_count = diff_text.lines().count();
            let expanded = this.ai_expanded_timeline_row_ids.contains(row.id.as_str());
            let (preview, preview_truncated) = if expanded {
                (diff_text.to_string(), false)
            } else {
                ai_truncate_multiline_content(diff_text, 10)
            };
            let show_toggle = preview_truncated || expanded;
            let view_diff_button_id =
                format!("ai-open-review-tab-{}", row.turn_id.replace('\u{1f}', "--"));
            let toggle_id = format!("ai-toggle-diff-row-{}", row.id.replace('\u{1f}', "--"));

            let row_element = h_flex()
                .w_full()
                .justify_start()
                .child(
                    v_flex()
                        .max_w(px(920.0))
                        .w_full()
                        .gap_1()
                        .px_2p5()
                        .py_2()
                        .rounded(px(10.0))
                        .border_1()
                        .border_color(cx.theme().border.opacity(if is_dark { 0.9 } else { 0.74 }))
                        .bg(cx.theme().background.blend(cx.theme().muted.opacity(if is_dark {
                            0.16
                        } else {
                            0.22
                        })))
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
                                        .child(format!("Code Diff ({diff_line_count} lines)")),
                                )
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_1()
                                        .when(show_toggle, |this| {
                                            let row_id = row.id.clone();
                                            let view = view.clone();
                                            this.child(
                                                Button::new(toggle_id)
                                                    .compact()
                                                    .outline()
                                                    .with_size(gpui_component::Size::Small)
                                                    .icon(
                                                        Icon::new(if expanded {
                                                            IconName::ChevronDown
                                                        } else {
                                                            IconName::ChevronRight
                                                        })
                                                        .size(px(12.0)),
                                                    )
                                                    .tooltip(if expanded {
                                                        "Collapse diff preview"
                                                    } else {
                                                        "Expand diff preview"
                                                    })
                                                    .on_click(move |_, _, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.ai_toggle_timeline_row_expansion_action(
                                                                row_id.clone(),
                                                                cx,
                                                            );
                                                        });
                                                    }),
                                            )
                                        })
                                        .child({
                                            let view = view.clone();
                                            Button::new(view_diff_button_id)
                                                .compact()
                                                .outline()
                                                .with_size(gpui_component::Size::Small)
                                                .label("View Diff")
                                                .on_click(move |_, _, cx| {
                                                    view.update(cx, |this, cx| {
                                                        this.ai_open_review_tab(cx);
                                                    });
                                                })
                                        }),
                                ),
                        )
                        .when(!preview.is_empty(), |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .text_color(cx.theme().muted_foreground)
                                    .whitespace_normal()
                                    .child(preview),
                            )
                        }),
                );
            ai_timeline_row_with_animation(this, row.id.as_str(), row_element)
        }
    }
}

fn ai_timeline_row_with_animation(
    this: &DiffViewer,
    row_id: &str,
    row: gpui::Div,
) -> AnyElement {
    if this.reduced_motion_enabled() {
        row.into_any_element()
    } else {
        row.with_animation(
            row_id.to_string(),
            Animation::new(this.animation_duration_ms(170))
                .with_easing(cubic_bezier(0.32, 0.72, 0.0, 1.0)),
            |this, delta| {
                let entering = 1.0 - delta;
                this.top(px(entering * 7.0)).opacity(0.76 + (0.24 * delta))
            },
        )
        .into_any_element()
    }
}
