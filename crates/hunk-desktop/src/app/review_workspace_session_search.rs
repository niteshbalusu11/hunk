use std::collections::BTreeMap;
use std::{ops::Range, path::Path};

use hunk_editor::WorkspaceExcerptId;
use hunk_text::TextPosition;

use super::workspace_display_buffers::find_workspace_search_matches;
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewWorkspaceSearchTarget {
    pub(crate) path: String,
    pub(crate) excerpt_id: WorkspaceExcerptId,
    pub(crate) surface_order: usize,
    pub(crate) row_index: usize,
    pub(crate) raw_column_range: Option<Range<usize>>,
}

impl ReviewWorkspaceSession {
    pub(crate) fn review_search_target_for_workspace_match(
        &self,
        path: &Path,
        excerpt_id: WorkspaceExcerptId,
        surface_order: usize,
        start: TextPosition,
        end: TextPosition,
    ) -> Option<ReviewWorkspaceSearchTarget> {
        let excerpt = self.layout.excerpt(excerpt_id)?;
        if !excerpt.spec.line_range.contains(&start.line) {
            return None;
        }
        let row_index = excerpt.content_row_range().start
            + start.line.saturating_sub(excerpt.spec.line_range.start);
        let raw_column_range = (start.line == end.line && start.column < end.column)
            .then_some(start.column..end.column);
        Some(ReviewWorkspaceSearchTarget {
            path: path.to_string_lossy().to_string(),
            excerpt_id,
            surface_order,
            row_index,
            raw_column_range,
        })
    }

    pub(crate) fn workspace_search_matches(&self, query: &str) -> Vec<ReviewWorkspaceSearchTarget> {
        if query.trim().is_empty() {
            return Vec::new();
        }

        let document_snapshots = self
            .right_document_buffers
            .iter()
            .map(|(document_id, buffer)| (*document_id, buffer.snapshot()))
            .collect::<BTreeMap<_, _>>();

        find_workspace_search_matches(&self.layout, query, &document_snapshots)
            .into_iter()
            .filter_map(|candidate| {
                let snapshot = document_snapshots.get(&candidate.document_id)?;
                let document = self.layout.document(candidate.document_id)?;
                let start = snapshot.byte_to_position(candidate.byte_range.start).ok()?;
                let end = snapshot.byte_to_position(candidate.byte_range.end).ok()?;
                self.review_search_target_for_workspace_match(
                    document.path.as_path(),
                    candidate.excerpt_id,
                    candidate.surface_order,
                    start,
                    end,
                )
            })
            .collect()
    }
}
