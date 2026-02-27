#[derive(Debug, Clone)]
pub(super) struct RowCommentAnchor {
    pub(super) file_path: String,
    pub(super) line_side: CommentLineSide,
    pub(super) old_line: Option<u32>,
    pub(super) new_line: Option<u32>,
    pub(super) hunk_header: Option<String>,
    pub(super) line_text: String,
    pub(super) context_before: String,
    pub(super) context_after: String,
    pub(super) anchor_hash: String,
}

impl DiffViewer {
    fn load_database_store() -> Option<DatabaseStore> {
        match DatabaseStore::new() {
            Ok(store) => Some(store),
            Err(err) => {
                error!("failed to initialize sqlite database path: {err:#}");
                None
            }
        }
    }

    fn clear_comment_ui_state(&mut self) {
        self.hovered_comment_row = None;
        self.active_comment_editor_row = None;
        self.comments_preview_open = false;
    }

    fn auto_show_non_open_if_open_empty(&mut self) {
        if self.comments_show_non_open {
            return;
        }
        if !self.comments_cache.is_empty() && self.comments_open_count() == 0 {
            self.comments_show_non_open = true;
        }
    }

    fn clamp_comment_rows_to_diff(&mut self) {
        if self.diff_rows.is_empty() {
            self.hovered_comment_row = None;
            self.active_comment_editor_row = None;
            return;
        }

        let max_ix = self.diff_rows.len().saturating_sub(1);
        self.hovered_comment_row = self.hovered_comment_row.map(|ix| ix.min(max_ix));
        self.active_comment_editor_row = self.active_comment_editor_row.map(|ix| ix.min(max_ix));
    }

    fn comment_scope_repo_root(&self) -> Option<String> {
        self.repo_root
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
    }

    fn comment_scope_bookmark_name(&self) -> String {
        let name = self.branch_name.trim();
        if name.is_empty() || name == "unknown" {
            "detached".to_string()
        } else {
            name.to_string()
        }
    }

    fn refresh_comments_cache_from_store(&mut self) {
        let Some(store) = self.database_store.clone() else {
            self.comments_cache.clear();
            return;
        };
        let Some(repo_root) = self.comment_scope_repo_root() else {
            self.comments_cache.clear();
            return;
        };
        let bookmark_name = self.comment_scope_bookmark_name();

        match store.list_comments(repo_root.as_str(), bookmark_name.as_str(), true) {
            Ok(records) => {
                self.comments_cache = records;
                let open_ids = self
                    .comments_cache
                    .iter()
                    .filter(|comment| comment.status == CommentStatus::Open)
                    .map(|comment| comment.id.clone())
                    .collect::<BTreeSet<_>>();
                self.comment_miss_streaks
                    .retain(|comment_id, _| open_ids.contains(comment_id));
                self.auto_show_non_open_if_open_empty();
                self.comment_status_message = None;
            }
            Err(err) => {
                error!(
                    "failed to load comments for repo '{}' bookmark '{}': {err:#}",
                    repo_root, bookmark_name
                );
                self.comments_cache.clear();
                self.comment_status_message =
                    Some("Failed to load comments from local database.".to_string());
            }
        }
    }

    fn prune_expired_comments(&mut self) {
        let Some(store) = self.database_store.clone() else {
            return;
        };
        let retention_ms = COMMENT_RETENTION_DAYS.saturating_mul(24 * 60 * 60 * 1000);
        let cutoff = now_unix_ms().saturating_sub(retention_ms);
        if let Err(err) = store.prune_non_open_comments(cutoff) {
            error!("failed to prune old comments: {err:#}");
        }
    }

    pub(super) fn comments_open_count(&self) -> usize {
        self.comments_cache
            .iter()
            .filter(|comment| comment.status == CommentStatus::Open)
            .count()
    }

    pub(super) fn comments_preview_records(&self) -> Vec<CommentRecord> {
        self.comments_cache
            .iter()
            .filter(|comment| {
                self.comments_show_non_open || comment.status == CommentStatus::Open
            })
            .take(COMMENT_PREVIEW_MAX_ITEMS)
            .cloned()
            .collect::<Vec<_>>()
    }

    pub(super) fn set_comments_show_non_open(
        &mut self,
        show_non_open: bool,
        cx: &mut Context<Self>,
    ) {
        if self.comments_show_non_open == show_non_open {
            return;
        }
        self.comments_show_non_open = show_non_open;
        cx.notify();
    }

    pub(super) fn toggle_comments_preview(&mut self, cx: &mut Context<Self>) {
        if !self.comments_preview_open {
            self.auto_show_non_open_if_open_empty();
        }
        self.comments_preview_open = !self.comments_preview_open;
        cx.notify();
    }

    pub(super) fn close_comments_preview(&mut self, cx: &mut Context<Self>) {
        if !self.comments_preview_open {
            return;
        }
        self.comments_preview_open = false;
        cx.notify();
    }

    pub(super) fn row_supports_comments(&self, row_ix: usize) -> bool {
        self.diff_rows.get(row_ix).is_some_and(|row| {
            matches!(
                row.kind,
                DiffRowKind::Code | DiffRowKind::Meta | DiffRowKind::Empty
            )
        })
    }

    pub(super) fn on_diff_row_hover(&mut self, row_ix: usize, cx: &mut Context<Self>) {
        if !self.row_supports_comments(row_ix) {
            return;
        }
        if self.hovered_comment_row == Some(row_ix) {
            return;
        }
        self.hovered_comment_row = Some(row_ix);
        cx.notify();
    }

    pub(super) fn row_open_comment_count(&self, row_ix: usize) -> usize {
        self.comments_cache
            .iter()
            .filter(|comment| comment.status == CommentStatus::Open)
            .filter(|comment| self.row_exact_anchor_match(row_ix, comment))
            .count()
    }

    pub(super) fn open_comment_editor_for_row(
        &mut self,
        row_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.row_supports_comments(row_ix) {
            return;
        }
        self.active_comment_editor_row = Some(row_ix);
        self.comment_status_message = None;
        let state = self.comment_input_state.clone();
        state.update(cx, |input, cx| {
            input.set_value("", window, cx);
        });
        cx.notify();
    }

    pub(super) fn cancel_comment_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_comment_editor_row = None;
        let state = self.comment_input_state.clone();
        state.update(cx, |input, cx| {
            input.set_value("", window, cx);
        });
        cx.notify();
    }

    pub(super) fn save_active_comment(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(store) = self.database_store.clone() else {
            self.comment_status_message =
                Some("Comments database is unavailable on this machine.".to_string());
            cx.notify();
            return;
        };
        let Some(row_ix) = self.active_comment_editor_row else {
            return;
        };

        let comment_text = self.comment_input_state.read(cx).value().trim().to_string();
        if comment_text.is_empty() {
            self.comment_status_message = Some("Comment text cannot be empty.".to_string());
            cx.notify();
            return;
        }

        let Some(anchor) = self.build_row_comment_anchor(row_ix) else {
            self.comment_status_message =
                Some("Could not resolve a stable anchor for this diff row.".to_string());
            cx.notify();
            return;
        };
        let Some(repo_root) = self.comment_scope_repo_root() else {
            self.comment_status_message = Some("No repository is open.".to_string());
            cx.notify();
            return;
        };

        let input = NewComment {
            repo_root,
            bookmark_name: self.comment_scope_bookmark_name(),
            created_head_commit: None,
            file_path: anchor.file_path,
            line_side: anchor.line_side,
            old_line: anchor.old_line,
            new_line: anchor.new_line,
            row_stable_id: self
                .diff_row_metadata
                .get(row_ix)
                .map(|row| row.stable_id),
            hunk_header: anchor.hunk_header,
            line_text: anchor.line_text,
            context_before: anchor.context_before,
            context_after: anchor.context_after,
            anchor_hash: anchor.anchor_hash,
            comment_text,
        };

        match store.create_comment(&input) {
            Ok(_) => {
                self.active_comment_editor_row = None;
                self.comments_preview_open = true;
                self.comment_status_message = Some("Comment added.".to_string());
                let state = self.comment_input_state.clone();
                state.update(cx, |input, cx| {
                    input.set_value("", window, cx);
                });
                self.refresh_comments_cache_from_store();
            }
            Err(err) => {
                error!("failed to create diff comment: {err:#}");
                self.comment_status_message = Some("Failed to save comment.".to_string());
            }
        }
        cx.notify();
    }

    pub(super) fn copy_comment_bundle_by_id(&mut self, id: String, cx: &mut Context<Self>) {
        let Some(comment) = self.comments_cache.iter().find(|comment| comment.id == id) else {
            return;
        };
        let blob = format_comment_clipboard_blob(comment);
        cx.write_to_clipboard(ClipboardItem::new_string(blob));
        self.comment_status_message = Some("Copied comment bundle.".to_string());
        cx.notify();
    }

    pub(super) fn copy_all_open_comment_bundles(&mut self, cx: &mut Context<Self>) {
        let blobs = self
            .comments_cache
            .iter()
            .filter(|comment| comment.status == CommentStatus::Open)
            .map(format_comment_clipboard_blob)
            .collect::<Vec<_>>();
        if blobs.is_empty() {
            self.comment_status_message = Some("No open comments to copy.".to_string());
            cx.notify();
            return;
        }

        let combined = blobs.join("\n\n---\n\n");
        cx.write_to_clipboard(ClipboardItem::new_string(combined));
        self.comment_status_message = Some(format!("Copied {} comment bundles.", blobs.len()));
        cx.notify();
    }

    pub(super) fn delete_comment_by_id(&mut self, id: String, cx: &mut Context<Self>) {
        let Some(store) = self.database_store.clone() else {
            return;
        };

        match store.delete_comment(id.as_str()) {
            Ok(_) => {
                self.comment_miss_streaks.remove(id.as_str());
                self.refresh_comments_cache_from_store();
                self.comment_status_message = Some("Comment deleted.".to_string());
            }
            Err(err) => {
                error!("failed to delete comment {id}: {err:#}");
                self.comment_status_message = Some("Failed to delete comment.".to_string());
            }
        }
        cx.notify();
    }

    pub(super) fn reopen_comment_by_id(&mut self, id: String, cx: &mut Context<Self>) {
        let Some(store) = self.database_store.clone() else {
            return;
        };

        match store.mark_comment_status(id.as_str(), CommentStatus::Open, None, now_unix_ms()) {
            Ok(updated) => {
                if updated {
                    self.comment_miss_streaks.remove(id.as_str());
                    self.refresh_comments_cache_from_store();
                    self.comment_status_message = Some("Comment reopened.".to_string());
                }
            }
            Err(err) => {
                error!("failed to reopen comment {id}: {err:#}");
                self.comment_status_message = Some("Failed to reopen comment.".to_string());
            }
        }
        cx.notify();
    }

    pub(super) fn jump_to_comment_by_id(&mut self, id: String, cx: &mut Context<Self>) {
        let Some(comment) = self
            .comments_cache
            .iter()
            .find(|comment| comment.id == id)
            .cloned()
        else {
            return;
        };

        if let Some(row_ix) = self.find_matching_row_for_comment(&comment) {
            self.comments_preview_open = false;
            self.select_row_and_scroll(row_ix, false, cx);
            self.hovered_comment_row = Some(row_ix);
            self.comment_status_message = Some("Jumped to comment location.".to_string());
            cx.notify();
            return;
        }

        if let Some((status, start_row)) = self
            .file_row_ranges
            .iter()
            .find(|range| range.path == comment.file_path)
            .map(|range| (range.status, range.start_row))
        {
            self.comments_preview_open = false;
            self.selected_path = Some(comment.file_path);
            self.selected_status = Some(status);
            self.right_pane_mode = RightPaneMode::Diff;
            self.select_row_and_scroll(start_row, false, cx);
            self.comment_status_message =
                Some("Comment anchor not found; jumped to file.".to_string());
            cx.notify();
            return;
        }

        self.comment_status_message = Some("Comment location is not visible in this diff.".to_string());
        cx.notify();
    }

    pub(super) fn reconcile_comments_with_loaded_diff(&mut self) {
        self.refresh_comments_cache_from_store();
        let Some(store) = self.database_store.clone() else {
            return;
        };
        if self.comments_cache.is_empty() {
            return;
        }

        let now = now_unix_ms();
        let changed_paths = self
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<BTreeSet<_>>();
        let mut should_reload = false;

        for comment in self
            .comments_cache
            .clone()
            .into_iter()
            .filter(|comment| comment.status == CommentStatus::Open)
        {
            if self.find_matching_row_for_comment(&comment).is_some() {
                self.comment_miss_streaks.remove(comment.id.as_str());
                if let Err(err) = store.touch_comment_seen(comment.id.as_str(), now) {
                    error!("failed to update comment last_seen for {}: {err:#}", comment.id);
                }
                continue;
            }

            let next_miss_streak = self
                .comment_miss_streaks
                .get(comment.id.as_str())
                .copied()
                .unwrap_or(0)
                .saturating_add(1);
            if next_miss_streak < COMMENT_RECONCILE_MISS_THRESHOLD {
                self.comment_miss_streaks
                    .insert(comment.id.clone(), next_miss_streak);
                continue;
            }
            self.comment_miss_streaks.remove(comment.id.as_str());

            let (next_status, stale_reason) =
                next_status_for_unmatched_anchor(changed_paths.contains(comment.file_path.as_str()));
            match store.mark_comment_status(comment.id.as_str(), next_status, stale_reason, now) {
                Ok(updated) => {
                    should_reload |= updated;
                }
                Err(err) => {
                    error!("failed to update comment {} status: {err:#}", comment.id);
                }
            }
        }

        if should_reload {
            self.refresh_comments_cache_from_store();
        }
    }

    fn find_matching_row_for_comment(&self, comment: &CommentRecord) -> Option<usize> {
        let mut fallback = None;

        for row_ix in 0..self.diff_rows.len() {
            if self.row_file_path(row_ix).as_deref() != Some(comment.file_path.as_str()) {
                continue;
            }
            if self.row_exact_anchor_match(row_ix, comment) {
                return Some(row_ix);
            }

            if fallback.is_none()
                && let Some(anchor) = self.build_row_comment_anchor(row_ix)
                && anchor.anchor_hash == comment.anchor_hash
            {
                fallback = Some(row_ix);
            }
        }

        fallback
    }

    fn row_exact_anchor_match(&self, row_ix: usize, comment: &CommentRecord) -> bool {
        if self.row_file_path(row_ix).as_deref() != Some(comment.file_path.as_str()) {
            return false;
        }
        let Some(row) = self.diff_rows.get(row_ix) else {
            return false;
        };

        if row.kind != DiffRowKind::Code {
            if comment.line_side != CommentLineSide::Meta {
                return false;
            }
            let line_text = Self::row_diff_lines(row).join("\n");
            return line_text == comment.line_text;
        }

        match comment.line_side {
            CommentLineSide::Left => {
                row.left.line == comment.old_line
                    && (comment.new_line.is_none() || row.right.line == comment.new_line)
            }
            CommentLineSide::Right => {
                row.right.line == comment.new_line
                    && (comment.old_line.is_none() || row.left.line == comment.old_line)
            }
            CommentLineSide::Meta => false,
        }
    }

    fn row_file_path(&self, row_ix: usize) -> Option<String> {
        if self.diff_row_metadata.len() == self.diff_rows.len() {
            return self
                .diff_row_metadata
                .get(row_ix)
                .and_then(|row| row.file_path.clone());
        }
        self.selected_path.clone()
    }

    fn row_hunk_header(&self, row_ix: usize) -> Option<String> {
        let hunk_ix = self
            .diff_visible_hunk_header_lookup
            .get(row_ix)
            .copied()
            .flatten()?;
        self.diff_rows.get(hunk_ix).map(|row| row.text.clone())
    }

    pub(super) fn build_row_comment_anchor(&self, row_ix: usize) -> Option<RowCommentAnchor> {
        let row = self.diff_rows.get(row_ix)?;
        let file_path = self.row_file_path(row_ix)?;
        let hunk_header = self.row_hunk_header(row_ix);
        let line_text = Self::row_diff_lines(row).join("\n");

        let (line_side, old_line, new_line) = if row.kind == DiffRowKind::Code {
            if row.right.kind != DiffCellKind::None {
                (CommentLineSide::Right, row.left.line, row.right.line)
            } else if row.left.kind != DiffCellKind::None {
                (CommentLineSide::Left, row.left.line, row.right.line)
            } else {
                (CommentLineSide::Meta, None, None)
            }
        } else {
            (CommentLineSide::Meta, None, None)
        };

        let context_before = self.collect_row_context(row_ix, true);
        let context_after = self.collect_row_context(row_ix, false);
        let anchor_hash = compute_comment_anchor_hash(
            file_path.as_str(),
            hunk_header.as_deref(),
            line_text.as_str(),
            context_before.as_str(),
            context_after.as_str(),
        );

        Some(RowCommentAnchor {
            file_path,
            line_side,
            old_line,
            new_line,
            hunk_header,
            line_text,
            context_before,
            context_after,
            anchor_hash,
        })
    }

    fn collect_row_context(&self, row_ix: usize, before: bool) -> String {
        if self.diff_rows.is_empty() {
            return String::new();
        }

        let range = if before {
            let start = row_ix.saturating_sub(COMMENT_CONTEXT_RADIUS_ROWS);
            start..row_ix
        } else {
            let start = row_ix.saturating_add(1);
            let end = start
                .saturating_add(COMMENT_CONTEXT_RADIUS_ROWS)
                .min(self.diff_rows.len());
            start..end
        };

        let mut lines = Vec::new();
        for ix in range {
            if let Some(row) = self.diff_rows.get(ix) {
                lines.extend(Self::row_diff_lines(row));
            }
        }
        lines.join("\n")
    }

    pub(super) fn row_diff_lines(row: &SideBySideRow) -> Vec<String> {
        let mut lines = Vec::new();
        match row.kind {
            DiffRowKind::Code => {
                if row.left.kind == DiffCellKind::Removed {
                    lines.push(format!("-{}", row.left.text));
                }
                if row.right.kind == DiffCellKind::Added {
                    lines.push(format!("+{}", row.right.text));
                }
                if row.left.kind == DiffCellKind::Context {
                    lines.push(format!(" {}", row.left.text));
                }
                if row.left.kind == DiffCellKind::None
                    && row.right.kind == DiffCellKind::None
                    && !row.text.is_empty()
                {
                    lines.push(row.text.clone());
                }
            }
            DiffRowKind::HunkHeader => {}
            DiffRowKind::Meta | DiffRowKind::Empty => {
                lines.push(row.text.clone());
            }
        }
        lines
    }

    pub(super) fn comment_status_label(status: CommentStatus) -> &'static str {
        match status {
            CommentStatus::Open => "open",
            CommentStatus::Stale => "stale",
            CommentStatus::Resolved => "resolved",
        }
    }
}
