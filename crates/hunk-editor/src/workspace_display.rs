use std::cmp::min;

use crate::display::ExpandedLine;
use crate::{
    Viewport, WhitespaceMarker, WorkspaceDocumentId, WorkspaceLayout, WorkspaceRowLocation,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDisplayRow {
    pub row_index: usize,
    pub location: Option<WorkspaceRowLocation>,
    pub raw_start_column: usize,
    pub raw_end_column: usize,
    pub raw_column_offsets: Vec<usize>,
    pub text: String,
    pub whitespace_markers: Vec<WhitespaceMarker>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDisplaySnapshot {
    pub viewport: Viewport,
    pub total_rows: usize,
    pub visible_rows: Vec<WorkspaceDisplayRow>,
}

pub fn build_workspace_display_snapshot<F>(
    layout: &WorkspaceLayout,
    viewport: Viewport,
    tab_width: usize,
    show_whitespace: bool,
    mut line_text_for: F,
) -> WorkspaceDisplaySnapshot
where
    F: FnMut(WorkspaceDocumentId, usize) -> Option<String>,
{
    let total_rows = layout.total_rows();
    let start = viewport.first_visible_row.min(total_rows);
    let end = min(start.saturating_add(viewport.visible_row_count), total_rows);
    let visible_rows = (start..end)
        .map(|row_index| {
            build_workspace_display_row(
                layout,
                row_index,
                tab_width.max(1),
                show_whitespace,
                &mut line_text_for,
            )
        })
        .collect();

    WorkspaceDisplaySnapshot {
        viewport,
        total_rows,
        visible_rows,
    }
}

fn build_workspace_display_row<F>(
    layout: &WorkspaceLayout,
    row_index: usize,
    tab_width: usize,
    show_whitespace: bool,
    line_text_for: &mut F,
) -> WorkspaceDisplayRow
where
    F: FnMut(WorkspaceDocumentId, usize) -> Option<String>,
{
    let Some(location) = layout.locate_row(row_index) else {
        return WorkspaceDisplayRow {
            row_index,
            location: None,
            raw_start_column: 0,
            raw_end_column: 0,
            raw_column_offsets: vec![0],
            text: String::new(),
            whitespace_markers: Vec::new(),
        };
    };

    let Some(document_line) = location.document_line else {
        return WorkspaceDisplayRow {
            row_index,
            location: Some(location),
            raw_start_column: 0,
            raw_end_column: 0,
            raw_column_offsets: vec![0],
            text: String::new(),
            whitespace_markers: Vec::new(),
        };
    };

    let line_text = line_text_for(location.document_id, document_line).unwrap_or_default();
    let expanded_line = ExpandedLine::from_line(line_text, tab_width, show_whitespace);
    let display_len = expanded_line.display_len();

    WorkspaceDisplayRow {
        row_index,
        location: Some(location),
        raw_start_column: 0,
        raw_end_column: expanded_line.raw_len(),
        raw_column_offsets: expanded_line.raw_offsets_in_range(0, display_len),
        text: expanded_line.display_text.clone(),
        whitespace_markers: expanded_line.markers_in_range(0, display_len),
    }
}
