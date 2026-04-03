use hunk_editor::{
    Viewport, WorkspaceDocument, WorkspaceDocumentId, WorkspaceExcerptId, WorkspaceExcerptKind,
    WorkspaceExcerptSpec, WorkspaceRowKind, build_workspace_display_snapshot,
};
use hunk_text::BufferId;

#[test]
fn workspace_display_snapshot_projects_chrome_content_and_gap_rows() {
    let layout = hunk_editor::WorkspaceLayout::new(
        vec![
            WorkspaceDocument::new(
                WorkspaceDocumentId::new(1),
                "src/main.rs",
                BufferId::new(1),
                4,
            ),
            WorkspaceDocument::new(
                WorkspaceDocumentId::new(2),
                "src/lib.rs",
                BufferId::new(2),
                2,
            ),
        ],
        vec![
            WorkspaceExcerptSpec::new(
                WorkspaceExcerptId::new(10),
                WorkspaceDocumentId::new(1),
                WorkspaceExcerptKind::DiffHunk,
                1..3,
            )
            .with_chrome_rows(1, 1),
            WorkspaceExcerptSpec::new(
                WorkspaceExcerptId::new(20),
                WorkspaceDocumentId::new(2),
                WorkspaceExcerptKind::DiffHunk,
                0..1,
            ),
        ],
        1,
    )
    .expect("layout should build");

    let snapshot = build_workspace_display_snapshot(
        &layout,
        Viewport {
            first_visible_row: 0,
            visible_row_count: 6,
            horizontal_offset: 0,
        },
        4,
        false,
        |document_id, line| match (document_id.get(), line) {
            (1, 1) => Some("beta".to_string()),
            (1, 2) => Some("gamma".to_string()),
            (2, 0) => Some("delta".to_string()),
            _ => None,
        },
    );

    assert_eq!(snapshot.total_rows, 6);
    assert_eq!(snapshot.visible_rows.len(), 6);
    assert_eq!(
        snapshot.visible_rows[0]
            .location
            .as_ref()
            .map(|location| location.row_kind),
        Some(WorkspaceRowKind::LeadingChrome)
    );
    assert_eq!(snapshot.visible_rows[1].text, "beta");
    assert_eq!(
        snapshot.visible_rows[1]
            .location
            .as_ref()
            .and_then(|location| location.document_line),
        Some(1)
    );
    assert_eq!(snapshot.visible_rows[2].text, "gamma");
    assert_eq!(
        snapshot.visible_rows[3]
            .location
            .as_ref()
            .map(|location| location.row_kind),
        Some(WorkspaceRowKind::TrailingChrome)
    );
    assert!(snapshot.visible_rows[4].location.is_none());
    assert_eq!(snapshot.visible_rows[5].text, "delta");
}

#[test]
fn workspace_display_snapshot_expands_tabs_and_respects_viewport() {
    let layout = hunk_editor::WorkspaceLayout::new(
        vec![WorkspaceDocument::new(
            WorkspaceDocumentId::new(1),
            "src/main.rs",
            BufferId::new(1),
            2,
        )],
        vec![WorkspaceExcerptSpec::new(
            WorkspaceExcerptId::new(10),
            WorkspaceDocumentId::new(1),
            WorkspaceExcerptKind::FullFile,
            0..2,
        )],
        0,
    )
    .expect("layout should build");

    let snapshot = build_workspace_display_snapshot(
        &layout,
        Viewport {
            first_visible_row: 1,
            visible_row_count: 1,
            horizontal_offset: 0,
        },
        4,
        true,
        |_, line| match line {
            0 => Some("ignored".to_string()),
            1 => Some("\tvalue".to_string()),
            _ => None,
        },
    );

    assert_eq!(snapshot.total_rows, 2);
    assert_eq!(snapshot.visible_rows.len(), 1);
    assert_eq!(snapshot.visible_rows[0].row_index, 1);
    assert_eq!(snapshot.visible_rows[0].text, "    value");
    assert_eq!(snapshot.visible_rows[0].whitespace_markers.len(), 1);
    assert_eq!(snapshot.visible_rows[0].raw_end_column, 6);
}
