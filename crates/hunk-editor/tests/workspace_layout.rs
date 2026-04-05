use hunk_editor::{
    WorkspaceDocument, WorkspaceDocumentId, WorkspaceExcerptId, WorkspaceExcerptKind,
    WorkspaceExcerptSpec, WorkspaceLayout, WorkspaceLayoutError, WorkspaceRowKind,
};
use hunk_text::BufferId;

#[test]
fn workspace_layout_maps_rows_for_single_full_file_excerpt() {
    let document_id = WorkspaceDocumentId::new(1);
    let layout = WorkspaceLayout::new(
        vec![WorkspaceDocument::new(
            document_id,
            "src/app.rs",
            BufferId::new(11),
            12,
        )],
        vec![
            WorkspaceExcerptSpec::new(
                WorkspaceExcerptId::new(1),
                document_id,
                WorkspaceExcerptKind::FullFile,
                0..12,
            )
            .with_chrome_rows(1, 1),
        ],
        0,
    )
    .expect("layout should build");

    assert_eq!(layout.total_rows(), 14);

    let header = layout.locate_row(0).expect("header row");
    assert_eq!(header.row_kind, WorkspaceRowKind::LeadingChrome);
    assert_eq!(header.document_line, None);

    let first_content = layout.locate_row(1).expect("first content row");
    assert_eq!(first_content.row_kind, WorkspaceRowKind::Content);
    assert_eq!(first_content.document_line, Some(0));

    let trailing = layout.locate_row(13).expect("trailing row");
    assert_eq!(trailing.row_kind, WorkspaceRowKind::TrailingChrome);
    assert_eq!(trailing.document_line, None);
}

#[test]
fn workspace_layout_maps_multiple_excerpts_with_gap_rows() {
    let left = WorkspaceDocument::new(
        WorkspaceDocumentId::new(1),
        "src/left.rs",
        BufferId::new(21),
        20,
    );
    let right = WorkspaceDocument::new(
        WorkspaceDocumentId::new(2),
        "src/right.rs",
        BufferId::new(22),
        30,
    );
    let layout = WorkspaceLayout::new(
        vec![left.clone(), right.clone()],
        vec![
            WorkspaceExcerptSpec::new(
                WorkspaceExcerptId::new(1),
                left.id,
                WorkspaceExcerptKind::DiffHunk,
                4..8,
            )
            .with_chrome_rows(1, 0),
            WorkspaceExcerptSpec::new(
                WorkspaceExcerptId::new(2),
                right.id,
                WorkspaceExcerptKind::DiffHunk,
                10..14,
            )
            .with_chrome_rows(1, 1),
        ],
        2,
    )
    .expect("layout should build");

    assert_eq!(layout.total_rows(), 13);
    assert!(
        layout.locate_row(5).is_none(),
        "gap rows should not resolve to an excerpt"
    );

    let first = layout.locate_row(1).expect("first excerpt row");
    assert_eq!(first.document_id, left.id);
    assert_eq!(first.document_line, Some(4));

    let second_header = layout.locate_row(7).expect("second excerpt header");
    assert_eq!(second_header.document_id, right.id);
    assert_eq!(second_header.row_kind, WorkspaceRowKind::LeadingChrome);

    let second_content = layout.locate_row(8).expect("second excerpt content");
    assert_eq!(second_content.document_line, Some(10));
}

#[test]
fn workspace_layout_rejects_out_of_bounds_excerpt_ranges() {
    let err = WorkspaceLayout::new(
        vec![WorkspaceDocument::new(
            WorkspaceDocumentId::new(1),
            "src/app.rs",
            BufferId::new(11),
            3,
        )],
        vec![WorkspaceExcerptSpec::new(
            WorkspaceExcerptId::new(1),
            WorkspaceDocumentId::new(1),
            WorkspaceExcerptKind::DiffHunk,
            2..5,
        )],
        0,
    )
    .expect_err("layout should reject out-of-bounds excerpts");

    assert!(matches!(
        err,
        WorkspaceLayoutError::LineRangeOutOfBounds { .. }
    ));
}
