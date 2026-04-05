use std::collections::BTreeMap;
use std::path::PathBuf;

use hunk_editor::EditorCommand;
use hunk_editor::{WorkspaceDocumentId, WorkspaceExcerptId};
use hunk_text::{Selection, TextPosition};

#[allow(clippy::duplicate_mod)]
#[path = "workspace_display_buffers.rs"]
mod workspace_display_buffers;

use workspace_display_buffers::find_workspace_search_matches;

use super::FilesEditor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceSearchTarget {
    pub(crate) path: PathBuf,
    pub(crate) document_id: WorkspaceDocumentId,
    pub(crate) excerpt_id: WorkspaceExcerptId,
    pub(crate) surface_order: usize,
    pub(crate) byte_range: std::ops::Range<usize>,
    pub(crate) start: TextPosition,
    pub(crate) end: TextPosition,
}

impl FilesEditor {
    pub(crate) fn workspace_search_matches(
        &self,
        query: &str,
    ) -> Option<Vec<WorkspaceSearchTarget>> {
        let layout = self.workspace_session.layout()?;
        let document_snapshots = layout
            .documents()
            .iter()
            .filter_map(|document| {
                let snapshot = if self.active_path() == Some(document.path()) {
                    Some(self.editor.buffer().snapshot())
                } else {
                    self.workspace_buffers
                        .get(document.path())
                        .map(|buffer| buffer.snapshot())
                }?;
                Some((document.id, snapshot))
            })
            .collect::<BTreeMap<_, _>>();

        Some(
            find_workspace_search_matches(layout, query, &document_snapshots)
                .into_iter()
                .filter_map(|candidate| {
                    let document = layout.document(candidate.document_id)?;
                    let snapshot = document_snapshots.get(&candidate.document_id)?;
                    let start = snapshot.byte_to_position(candidate.byte_range.start).ok()?;
                    let end = snapshot.byte_to_position(candidate.byte_range.end).ok()?;
                    Some(WorkspaceSearchTarget {
                        path: document.path.clone(),
                        document_id: candidate.document_id,
                        excerpt_id: candidate.excerpt_id,
                        surface_order: candidate.surface_order,
                        byte_range: candidate.byte_range,
                        start,
                        end,
                    })
                })
                .collect(),
        )
    }

    pub(crate) fn select_next_workspace_search_match(
        &mut self,
        matches: &[WorkspaceSearchTarget],
        forward: bool,
    ) -> bool {
        self.select_next_workspace_search_target(matches, forward)
            .is_some()
    }

    pub(crate) fn select_next_workspace_search_target(
        &mut self,
        matches: &[WorkspaceSearchTarget],
        forward: bool,
    ) -> Option<WorkspaceSearchTarget> {
        let target = self.next_workspace_search_target(matches, forward)?;
        if self.active_path() != Some(target.path.as_path())
            && self.activate_workspace_path(target.path.as_path()).ok() != Some(true)
        {
            return None;
        }
        self.workspace_session.activate_excerpt(target.excerpt_id);
        if self
            .editor
            .apply(EditorCommand::SetSelection(Selection::new(
                target.start,
                target.end,
            )))
            .selection_changed
        {
            return Some(target);
        }
        Some(target)
    }

    fn next_workspace_search_target(
        &self,
        matches: &[WorkspaceSearchTarget],
        forward: bool,
    ) -> Option<WorkspaceSearchTarget> {
        if matches.is_empty() {
            return None;
        }

        let current_path = self.active_path()?;
        let layout = self.workspace_session.layout()?;
        let current_doc_id = layout
            .documents()
            .iter()
            .find(|document| document.path.as_path() == current_path)?
            .id;
        let snapshot = self.editor.buffer().snapshot();
        let selection = self.editor.selection().range();
        let caret_start = snapshot.position_to_byte(selection.start).ok()?;
        let caret_end = snapshot.position_to_byte(selection.end).ok()?;
        let current_excerpt_id = layout
            .excerpts()
            .iter()
            .find_map(|excerpt| {
                (excerpt.spec.document_id == current_doc_id
                    && excerpt.spec.line_range.contains(&selection.start.line))
                .then_some(excerpt.spec.id)
            })
            .or_else(|| self.workspace_session.active_excerpt_id());
        let current_surface_order = current_excerpt_id
            .and_then(|excerpt_id| {
                layout
                    .excerpts()
                    .iter()
                    .enumerate()
                    .find_map(|(surface_order, excerpt)| {
                        (excerpt.spec.id == excerpt_id).then_some(surface_order)
                    })
            })
            .unwrap_or(0);

        if forward {
            matches
                .iter()
                .find(|target| {
                    target.surface_order > current_surface_order
                        || (Some(target.excerpt_id) == current_excerpt_id
                            && target.document_id == current_doc_id
                            && target.byte_range.start > caret_end)
                })
                .or_else(|| matches.first())
                .cloned()
        } else {
            matches
                .iter()
                .rev()
                .find(|target| {
                    target.surface_order < current_surface_order
                        || (Some(target.excerpt_id) == current_excerpt_id
                            && target.document_id == current_doc_id
                            && target.byte_range.end < caret_start)
                })
                .or_else(|| matches.last())
                .cloned()
        }
    }
}
