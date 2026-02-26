impl DiffViewer {
    fn scroll_selected_file_to_top(&mut self) {
        let Some(path) = self.selected_path.clone() else {
            return;
        };
        self.scroll_to_file_start(&path);
    }

    fn scroll_to_file_start(&mut self, path: &str) {
        let Some(start_row) = self
            .file_row_ranges
            .iter()
            .find(|range| range.path == path)
            .map(|range| range.start_row)
        else {
            return;
        };

        self.diff_list_state.scroll_to(ListOffset {
            item_ix: start_row,
            offset_in_item: px(0.),
        });
        self.last_diff_scroll_offset = None;
        self.last_scroll_activity_at = Instant::now();
    }

    pub(super) fn sync_selected_file_from_visible_row(
        &mut self,
        row_ix: usize,
        cx: &mut Context<Self>,
    ) {
        if self.last_visible_row_start == Some(row_ix) {
            return;
        }
        self.last_visible_row_start = Some(row_ix);

        let Some((next_path, next_status)) =
            self.selected_file_from_row_metadata(row_ix).or_else(|| {
                self.file_row_ranges
                    .iter()
                    .find(|range| row_ix < range.end_row)
                    .or_else(|| self.file_row_ranges.last())
                    .map(|range| (range.path.clone(), range.status))
            })
        else {
            return;
        };

        if self.selected_path.as_deref() == Some(next_path.as_str()) {
            return;
        }

        self.selected_path = Some(next_path);
        self.selected_status = Some(next_status);
        cx.notify();
    }

    fn selected_file_from_row_metadata(&self, row_ix: usize) -> Option<(String, FileStatus)> {
        let row = self.diff_row_metadata.get(row_ix)?;
        if row.kind == DiffStreamRowKind::EmptyState {
            return None;
        }

        let _stable_row_id = row.stable_id;
        let path = row.file_path.clone()?;
        let status = row.file_status.or_else(|| {
            self.files
                .iter()
                .find(|file| file.path == path)
                .map(|file| file.status)
        })?;

        Some((path, status))
    }

    pub(super) fn on_diff_list_scroll_wheel(
        &mut self,
        _: &ScrollWheelEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.last_scroll_activity_at = Instant::now();
    }

    pub(super) fn toggle_diff_show_whitespace(&mut self, cx: &mut Context<Self>) {
        self.diff_show_whitespace = !self.diff_show_whitespace;
        self.config.show_whitespace = self.diff_show_whitespace;
        self.persist_config();
        cx.notify();
    }

    pub(super) fn toggle_diff_show_eol_markers(&mut self, cx: &mut Context<Self>) {
        self.diff_show_eol_markers = !self.diff_show_eol_markers;
        self.config.show_eol_markers = self.diff_show_eol_markers;
        self.persist_config();
        cx.notify();
    }

    fn recompute_diff_layout(&mut self) {
        let mut max_left_line_digits = DIFF_LINE_NUMBER_MIN_DIGITS;
        let mut max_right_line_digits = DIFF_LINE_NUMBER_MIN_DIGITS;

        for row in &self.diff_rows {
            if row.kind != DiffRowKind::Code {
                continue;
            }
            if let Some(line) = row.left.line {
                max_left_line_digits = max_left_line_digits.max(decimal_digits(line));
            }
            if let Some(line) = row.right.line {
                max_right_line_digits = max_right_line_digits.max(decimal_digits(line));
            }
        }

        self.diff_left_line_number_width = line_number_column_width(max_left_line_digits);
        self.diff_right_line_number_width = line_number_column_width(max_right_line_digits);
    }

    fn sync_diff_list_state(&self) {
        let previous_top = self.diff_list_state.logical_scroll_top();
        self.diff_list_state.reset(self.diff_rows.len());
        let clamped_item_ix = if self.diff_rows.is_empty() {
            0
        } else {
            previous_top
                .item_ix
                .min(self.diff_rows.len().saturating_sub(1))
        };
        self.diff_list_state.scroll_to(ListOffset {
            item_ix: clamped_item_ix,
            offset_in_item: px(0.),
        });
    }
}
