impl DiffViewer {
    fn render_diff(&mut self, cx: &mut Context<Self>) -> AnyElement {
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
        if self.repo_root.is_some()
            && self.workspace_view_mode != WorkspaceViewMode::Diff
            && self.files.is_empty()
        {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("No files changed"),
                )
                .into_any_element();
        }

        let (old_label, new_label) = self.diff_column_labels();
        let diff_list_state = self.diff_list_state.clone();
        let logical_top = diff_list_state.logical_scroll_top();
        let visible_row = logical_top.item_ix;
        let sticky_hunk_banner = self.render_visible_hunk_banner(visible_row, cx);
        let sticky_file_banner =
            self.render_visible_file_banner(visible_row, logical_top.offset_in_item, cx);

        let list = list(diff_list_state.clone(), {
            cx.processor(move |this, ix: usize, _window, cx| {
                let Some(row) = this.diff_rows.get(ix) else {
                    return div().into_any_element();
                };
                let is_selected = this.is_row_selected(ix);

                match row.kind {
                    DiffRowKind::Code => this.render_code_row(ix, row, is_selected, cx),
                    DiffRowKind::HunkHeader | DiffRowKind::Meta | DiffRowKind::Empty => {
                        this.render_meta_row(ix, row, is_selected, cx)
                    }
                }
            })
        })
        .flex_grow()
        .size_full()
        .map(|mut this| {
            this.style().restrict_scroll_to_axis = Some(true);
            this
        })
        .with_sizing_behavior(ListSizingBehavior::Auto);

        let scrollbar_size = px(DIFF_SCROLLBAR_SIZE);
        let edge_inset = px(DIFF_BOTTOM_SAFE_INSET);
        let right_inset = px(DIFF_SCROLLBAR_RIGHT_INSET);
        let vertical_bar_bottom = edge_inset;
        let is_dark = cx.theme().mode.is_dark();
        let chrome = hunk_diff_chrome(cx.theme(), is_dark);

        v_flex()
            .size_full()
            .child(sticky_hunk_banner)
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .when(self.workspace_view_mode == WorkspaceViewMode::Diff, |this| {
                        this.child(self.render_review_compare_controls(cx))
                    })
                    .child(
                        h_flex()
                            .w_full()
                            .border_b_1()
                            .border_color(chrome.row_divider)
                            .bg(chrome.column_header_background)
                            .child(
                                h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .py_1()
                                    .border_r_1()
                                    .border_color(hunk_opacity(
                                        cx.theme().border,
                                        is_dark,
                                        0.82,
                                        0.72,
                                    ))
                                    .child(
                                        div()
                                            .px_1p5()
                                            .py_0p5()
                                            .text_xs()
                                            .font_semibold()
                                            .font_family(cx.theme().mono_font_family.clone())
                                            .bg(chrome.column_header_badge_background)
                                            .text_color(cx.theme().muted_foreground)
                                            .child("OLD"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_family(cx.theme().mono_font_family.clone())
                                            .text_color(cx.theme().muted_foreground)
                                            .child(old_label),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .py_1()
                                    .child(
                                        div()
                                            .px_1p5()
                                            .py_0p5()
                                            .text_xs()
                                            .font_semibold()
                                            .font_family(cx.theme().mono_font_family.clone())
                                            .bg(chrome.column_header_badge_background)
                                            .text_color(cx.theme().muted_foreground)
                                            .child("NEW"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_family(cx.theme().mono_font_family.clone())
                                            .text_color(cx.theme().muted_foreground)
                                            .child(new_label),
                                    ),
                            ),
                    )
                    .child(sticky_file_banner)
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .relative()
                            .child(
                                div()
                                    .size_full()
                                    .on_scroll_wheel(cx.listener(Self::on_diff_list_scroll_wheel))
                                    .child(list),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .right(right_inset)
                                    .bottom(vertical_bar_bottom)
                                    .w(scrollbar_size)
                                    .child(
                                        Scrollbar::vertical(&diff_list_state)
                                            .scrollbar_show(ScrollbarShow::Always),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_open_project_empty_state(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();

        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .p_6()
            .child(
                v_flex()
                    .items_center()
                    .gap_3()
                    .max_w(px(520.0))
                    .px_8()
                    .py_6()
                    .rounded_lg()
                    .border_1()
                    .border_color(hunk_opacity(cx.theme().border, is_dark, 0.92, 0.74))
                    .bg(hunk_blend(cx.theme().sidebar, cx.theme().muted, is_dark, 0.22, 0.34))
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Open a project"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Choose a folder that contains a Git repository."),
                    )
                    .child(
                        Button::new("open-project-empty-state")
                            .primary()
                            .rounded(px(8.0))
                            .label("Open Project Folder")
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.open_project_picker(cx);
                                });
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Shortcut: Cmd/Ctrl+Shift+O"),
                    ),
            )
            .into_any_element()
    }

    fn render_visible_file_banner(
        &self,
        visible_row: usize,
        top_offset: gpui::Pixels,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some((header_row_ix, path, status)) = self.visible_file_header(visible_row) else {
            return div().w_full().h(px(0.)).into_any_element();
        };

        if visible_row == header_row_ix && top_offset.is_zero() {
            return div().w_full().h(px(0.)).into_any_element();
        }

        let stats = self
            .active_diff_file_line_stats()
            .get(path.as_str())
            .copied()
            .unwrap_or_default();
        self.render_sticky_file_status_banner_row(header_row_ix, path.as_str(), status, stats, cx)
    }

    fn render_visible_hunk_banner(&self, visible_row: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some((path, header)) = self.visible_hunk_header(visible_row) else {
            return div().w_full().h(px(0.)).into_any_element();
        };

        let is_dark = cx.theme().mode.is_dark();
        let chrome = hunk_diff_chrome(cx.theme(), is_dark);
        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .px_3()
            .py_0p5()
            .border_b_1()
            .border_color(chrome.row_divider)
            .bg(hunk_blend(
                chrome.column_header_background,
                cx.theme().primary,
                is_dark,
                0.10,
                0.05,
            ))
            .child(
                div()
                    .text_xs()
                    .font_semibold()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(hunk_tone(cx.theme().primary, is_dark, 0.34, 0.10))
                    .child("HUNK"),
            )
            .child(
                div()
                    .text_xs()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(cx.theme().muted_foreground)
                    .child(path),
            )
            .child(
                div()
                    .text_xs()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(hunk_tone(cx.theme().primary, is_dark, 0.42, 0.12))
                    .child(header),
            )
            .into_any_element()
    }

    fn visible_hunk_header(&self, visible_row: usize) -> Option<(String, String)> {
        if self.diff_rows.is_empty() {
            return None;
        }

        let capped = visible_row.min(self.diff_rows.len().saturating_sub(1));

        if self.diff_row_metadata.len() == self.diff_rows.len() {
            let hunk_ix = self
                .diff_visible_hunk_header_lookup
                .get(capped)
                .copied()
                .flatten()?;
            let meta = self.diff_row_metadata.get(hunk_ix)?;
            let path = meta
                .file_path
                .clone()
                .or_else(|| self.selected_path.clone())
                .unwrap_or_else(|| "file".to_string());
            let header = self.diff_rows.get(hunk_ix)?.text.clone();
            return Some((path, header));
        }

        let hunk_ix = self
            .diff_visible_hunk_header_lookup
            .get(capped)
            .copied()
            .flatten()?;
        let path = self
            .selected_path
            .clone()
            .unwrap_or_else(|| "file".to_string());
        Some((path, self.diff_rows.get(hunk_ix)?.text.clone()))
    }

    fn visible_file_header(&self, visible_row: usize) -> Option<(usize, String, FileStatus)> {
        if self.diff_rows.is_empty() {
            return None;
        }

        let capped = visible_row.min(self.diff_rows.len().saturating_sub(1));

        if self.diff_row_metadata.len() == self.diff_rows.len() {
            let header_ix = self
                .diff_visible_file_header_lookup
                .get(capped)
                .copied()
                .flatten()?;
            let meta = self.diff_row_metadata.get(header_ix)?;
            if meta.kind == DiffStreamRowKind::EmptyState {
                return None;
            }
            let path = meta.file_path.clone()?;
            let status = meta
                .file_status
                .or_else(|| self.status_for_path(path.as_str()))
                .unwrap_or(FileStatus::Unknown);
            return Some((header_ix, path, status));
        }

        let header_ix = self
            .diff_visible_file_header_lookup
            .get(capped)
            .copied()
            .flatten()?;
        self.file_row_ranges
            .iter()
            .find(|range| range.start_row == header_ix)
            .map(|range| (range.start_row, range.path.clone(), range.status))
    }

    fn diff_column_labels(&self) -> (String, String) {
        if self.workspace_view_mode == WorkspaceViewMode::Diff {
            return (
                self.review_compare_source_label(self.review_left_source_id.as_deref()),
                self.review_compare_source_label(self.review_right_source_id.as_deref()),
            );
        }

        let selected = self
            .selected_path
            .clone()
            .unwrap_or_else(|| "file".to_string());
        match self.selected_status.unwrap_or(FileStatus::Unknown) {
            FileStatus::Added | FileStatus::Untracked => ("/dev/null".to_string(), selected),
            FileStatus::Deleted => (selected, "/dev/null".to_string()),
            _ => ("Old".to_string(), "New".to_string()),
        }
    }

    fn render_review_compare_controls(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let is_dark = cx.theme().mode.is_dark();
        let left_label = self.review_compare_source_label(self.review_left_source_id.as_deref());
        let right_label = self.review_compare_source_label(self.review_right_source_id.as_deref());
        let reset_available = self.review_compare_reset_available();
        let picker_surface = hunk_blend(
            cx.theme().background,
            cx.theme().muted,
            is_dark,
            0.24,
            0.16,
        );
        let picker_border = hunk_opacity(cx.theme().border, is_dark, 0.96, 0.84);
        let picker_title = hunk_opacity(cx.theme().foreground, is_dark, 0.82, 0.90);
        let arrow_color = hunk_tone(cx.theme().accent, is_dark, 0.26, 0.42);
        let status_message = if let Some(error) = self.review_compare_error.as_ref() {
            error.clone()
        } else if self.review_compare_loading {
            "Loading comparison...".to_string()
        } else if !self.review_comments_enabled() {
            "Custom compare mode is read-only. Comments are disabled.".to_string()
        } else {
            self.review_compare_source_detail(self.review_left_source_id.as_deref())
                .zip(self.review_compare_source_detail(self.review_right_source_id.as_deref()))
                .map(|(left, right)| format!("{left} -> {right}"))
                .unwrap_or_else(|| "Choose a base source and a compare source.".to_string())
        };

        v_flex()
            .w_full()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(hunk_opacity(cx.theme().border, is_dark, 0.88, 0.72))
            .bg(hunk_blend(
                cx.theme().title_bar,
                cx.theme().muted,
                is_dark,
                0.16,
                0.24,
            ))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .child("Diff Sources"),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .flex_wrap()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(if self.review_compare_error.is_some() {
                                        cx.theme().danger
                                    } else if self.review_compare_loading {
                                        cx.theme().warning
                                    } else {
                                        cx.theme().muted_foreground
                                    })
                                    .child(status_message),
                            )
                            .child({
                                let view = view.clone();
                                Button::new("review-compare-reset")
                                    .compact()
                                    .outline()
                                    .rounded(px(7.0))
                                    .label("Reset")
                                    .disabled(!reset_available || self.review_compare_loading)
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.reset_review_compare_selection(cx);
                                        });
                                    })
                            }),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        v_flex()
                            .min_w(px(240.0))
                            .flex_1()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(picker_title)
                                    .child("Base"),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .p_1()
                                    .rounded(px(10.0))
                                    .border_1()
                                    .border_color(picker_border)
                                    .bg(picker_surface)
                                    .child(
                                        Select::new(&self.review_left_picker_state)
                                            .with_size(gpui_component::Size::Medium)
                                            .placeholder(left_label)
                                            .search_placeholder("Find a branch or worktree")
                                            .rounded(px(8.0))
                                            .w_full()
                                            .disabled(self.review_compare_sources.is_empty())
                                            .empty(
                                                h_flex()
                                                    .h(px(72.0))
                                                    .justify_center()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("No compare sources available."),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .mt(px(20.0))
                            .flex_none()
                            .text_color(arrow_color)
                            .child(Icon::new(IconName::ArrowRight).size(px(20.0))),
                    )
                    .child(
                        v_flex()
                            .min_w(px(240.0))
                            .flex_1()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(picker_title)
                                    .child("Compare"),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .p_1()
                                    .rounded(px(10.0))
                                    .border_1()
                                    .border_color(picker_border)
                                    .bg(picker_surface)
                                    .child(
                                        Select::new(&self.review_right_picker_state)
                                            .with_size(gpui_component::Size::Medium)
                                            .placeholder(right_label)
                                            .search_placeholder("Find a branch or worktree")
                                            .rounded(px(8.0))
                                            .w_full()
                                            .disabled(self.review_compare_sources.is_empty())
                                            .empty(
                                                h_flex()
                                                    .h(px(72.0))
                                                    .justify_center()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("No compare sources available."),
                                            ),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }
}
