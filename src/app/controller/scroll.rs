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

    pub(super) fn start_horizontal_pan_drag(&mut self, event: &MouseDownEvent) -> bool {
        if self.diff_fit_to_width {
            return false;
        }

        let started_with_middle = event.button == MouseButton::Middle;
        let started_with_alt_left = event.button == MouseButton::Left && event.modifiers.alt;
        if !started_with_middle && !started_with_alt_left {
            return false;
        }

        self.horizontal_pan_dragging = true;
        self.horizontal_pan_last_x = Some(event.position.x);
        self.drag_selecting_rows = false;
        true
    }

    pub(super) fn update_horizontal_pan_drag(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.horizontal_pan_dragging {
            return false;
        }

        let Some(pressed_button) = event.pressed_button else {
            self.stop_horizontal_pan_drag();
            return false;
        };
        if pressed_button != MouseButton::Left && pressed_button != MouseButton::Middle {
            self.stop_horizontal_pan_drag();
            return false;
        }

        let last_x = self.horizontal_pan_last_x.unwrap_or(event.position.x);
        self.horizontal_pan_last_x = Some(event.position.x);
        let delta_x = event.position.x - last_x;
        let changed = self.scroll_diff_horizontal_by(delta_x);
        self.last_scroll_activity_at = Instant::now();
        if changed {
            cx.notify();
        }
        cx.stop_propagation();
        true
    }

    pub(super) fn stop_horizontal_pan_drag(&mut self) {
        self.horizontal_pan_dragging = false;
        self.horizontal_pan_last_x = None;
    }

    pub(super) fn on_diff_horizontal_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.diff_fit_to_width {
            return;
        }

        let mut delta = event.delta.pixel_delta(window.line_height());
        if delta.x.is_zero() && event.modifiers.shift && !delta.y.is_zero() {
            delta.x = delta.y;
            delta.y = px(0.);
        }
        let horizontal_intent =
            !delta.x.is_zero() && (delta.y.is_zero() || delta.x.abs() >= delta.y.abs());
        if !horizontal_intent {
            // Prevent GPUI's default overflow-x wheel remapping (y -> x) on this container.
            cx.stop_propagation();
            return;
        }

        let changed = self.scroll_diff_horizontal_by(delta.x);
        self.last_scroll_activity_at = Instant::now();
        if changed {
            cx.notify();
        }
        cx.stop_propagation();
    }

    pub(super) fn on_diff_list_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.diff_fit_to_width {
            self.last_scroll_activity_at = Instant::now();
            return;
        }

        let mut delta = event.delta.pixel_delta(window.line_height());
        if delta.x.is_zero() && event.modifiers.shift && !delta.y.is_zero() {
            delta.x = delta.y;
            delta.y = px(0.);
        }

        let horizontal_intent =
            !delta.x.is_zero() && (delta.y.is_zero() || delta.x.abs() >= delta.y.abs());
        if horizontal_intent {
            let changed = self.scroll_diff_horizontal_by(delta.x);
            self.last_scroll_activity_at = Instant::now();
            if changed {
                cx.notify();
            }
            cx.stop_propagation();
            return;
        }

        if !delta.y.is_zero() {
            self.last_scroll_activity_at = Instant::now();
        }
    }

    pub(super) fn on_file_preview_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut delta = event.delta.pixel_delta(window.line_height());
        if delta.x.is_zero() && event.modifiers.shift && !delta.y.is_zero() {
            delta.x = delta.y;
            delta.y = px(0.);
        }
        let horizontal_intent =
            !delta.x.is_zero() && (delta.y.is_zero() || delta.x.abs() >= delta.y.abs());
        if horizontal_intent {
            cx.stop_propagation();
            return;
        }

        if delta.y.is_zero() {
            return;
        }

        self.file_preview_list_state
            // GPUI list's scroll_by axis sign is opposite of native wheel delta for this use case.
            // Invert once so preview behavior matches diff view and OS natural-scroll settings.
            .scroll_by(delta.y * -FILE_PREVIEW_SCROLL_MULTIPLIER);
        self.last_scroll_activity_at = Instant::now();
        cx.notify();
        cx.stop_propagation();
    }

    fn scroll_diff_horizontal_by(&mut self, delta_x: gpui::Pixels) -> bool {
        if delta_x.is_zero() {
            return false;
        }

        let mut offset = self.diff_horizontal_scroll_handle.offset();
        offset.x += delta_x;
        offset.y = px(0.);

        let max_x = self
            .diff_horizontal_scroll_handle
            .max_offset()
            .width
            .max(px(0.));
        offset.x = offset.x.clamp(-max_x, px(0.));

        if offset != self.diff_horizontal_scroll_handle.offset() {
            self.diff_horizontal_scroll_handle.set_offset(offset);
            return true;
        }

        false
    }

    pub(super) fn toggle_diff_fit_to_width(&mut self, cx: &mut Context<Self>) {
        self.diff_fit_to_width = !self.diff_fit_to_width;
        self.config.diff_view = if self.diff_fit_to_width {
            DiffViewMode::Fit
        } else {
            DiffViewMode::Pan
        };
        self.persist_config();
        self.diff_horizontal_scroll_handle
            .set_offset(point(px(0.), px(0.)));
        self.last_scroll_activity_at = Instant::now();
        cx.notify();
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

    fn recompute_diff_pan_layout(&mut self) {
        let mut max_left_chars = 0usize;
        let mut max_right_chars = 0usize;
        let mut max_left_line_digits = DIFF_LINE_NUMBER_MIN_DIGITS;
        let mut max_right_line_digits = DIFF_LINE_NUMBER_MIN_DIGITS;

        for row in &self.diff_rows {
            match row.kind {
                DiffRowKind::Code => {
                    max_left_chars = max_left_chars.max(display_width(&row.left.text));
                    max_right_chars = max_right_chars.max(display_width(&row.right.text));
                    if let Some(line) = row.left.line {
                        max_left_line_digits = max_left_line_digits.max(decimal_digits(line));
                    }
                    if let Some(line) = row.right.line {
                        max_right_line_digits = max_right_line_digits.max(decimal_digits(line));
                    }
                }
                DiffRowKind::HunkHeader | DiffRowKind::Meta | DiffRowKind::Empty => {}
            }
        }

        let left_line_number_width = line_number_column_width(max_left_line_digits);
        let right_line_number_width = line_number_column_width(max_right_line_digits);
        let left_gutter =
            left_line_number_width + DIFF_MARKER_GUTTER_WIDTH + DIFF_CELL_SIDE_PADDING_WIDTH;
        let right_gutter =
            right_line_number_width + DIFF_MARKER_GUTTER_WIDTH + DIFF_CELL_SIDE_PADDING_WIDTH;

        let left_width =
            (max_left_chars as f32 * DIFF_MONO_CHAR_WIDTH + left_gutter + DIFF_PAN_COLUMN_PADDING)
                .max(DIFF_MIN_COLUMN_WIDTH);
        let right_width = (max_right_chars as f32 * DIFF_MONO_CHAR_WIDTH
            + right_gutter
            + DIFF_PAN_COLUMN_PADDING)
            .max(DIFF_MIN_COLUMN_WIDTH);

        self.diff_left_column_width = left_width;
        self.diff_right_column_width = right_width;
        self.diff_pan_content_width = (left_width + right_width).max(DIFF_MIN_CONTENT_WIDTH);
        self.diff_left_line_number_width = left_line_number_width;
        self.diff_right_line_number_width = right_line_number_width;
    }

    pub(super) fn clamp_diff_horizontal_scroll_offset(&mut self) {
        if self.diff_fit_to_width {
            self.diff_horizontal_scroll_handle
                .set_offset(point(px(0.), px(0.)));
            return;
        }

        let offset = self.diff_horizontal_scroll_handle.offset();
        let max_x = self
            .diff_horizontal_scroll_handle
            .max_offset()
            .width
            .max(px(0.));
        let clamped_x = offset.x.clamp(-max_x, px(0.));

        if clamped_x != offset.x || !offset.y.is_zero() {
            self.diff_horizontal_scroll_handle
                .set_offset(point(clamped_x, px(0.)));
        }
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
