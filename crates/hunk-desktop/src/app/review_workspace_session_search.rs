use std::collections::BTreeMap;
use std::ops::Range;

use hunk_editor::WorkspaceExcerptId;

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
                let excerpt = self.layout.excerpt(candidate.excerpt_id)?;
                let snapshot = document_snapshots.get(&candidate.document_id)?;
                let document = self.layout.document(candidate.document_id)?;
                let start = snapshot.byte_to_position(candidate.byte_range.start).ok()?;
                let end = snapshot.byte_to_position(candidate.byte_range.end).ok()?;
                if !excerpt.spec.line_range.contains(&start.line) {
                    return None;
                }
                let row_index = excerpt.content_row_range().start
                    + start.line.saturating_sub(excerpt.spec.line_range.start);
                let raw_column_range = (start.line == end.line && start.column < end.column)
                    .then_some(start.column..end.column);
                Some(ReviewWorkspaceSearchTarget {
                    path: document.path.to_string_lossy().to_string(),
                    excerpt_id: candidate.excerpt_id,
                    surface_order: candidate.surface_order,
                    row_index,
                    raw_column_range,
                })
            })
            .collect()
    }
}
