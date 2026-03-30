extern crate self as hunk_domain;
extern crate self as hunk_editor;

pub mod diff {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DiffCellKind {
        None,
        Added,
        Removed,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DiffRowKind {
        Code,
        Meta,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DiffCell {
        pub line: Option<u32>,
        pub text: String,
        pub kind: DiffCellKind,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SideBySideRow {
        pub kind: DiffRowKind,
        pub left: DiffCell,
        pub right: DiffCell,
        pub text: String,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OverlayKind {
    DiffAddition,
    DiffDeletion,
    DiffModification,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayDescriptor {
    pub line: usize,
    pub kind: OverlayKind,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldRegion {
    pub start_line: usize,
    pub end_line: usize,
}

impl FoldRegion {
    pub fn new(start_line: usize, end_line: usize) -> Option<Self> {
        (end_line > start_line).then_some(Self {
            start_line,
            end_line,
        })
    }
}

#[path = "../src/app/review_editor_model.rs"]
mod review_editor_model;

use diff::{DiffCell, DiffCellKind, DiffRowKind, SideBySideRow};
use review_editor_model::{
    build_review_editor_overlays, build_review_editor_overlays_from_texts,
    build_review_editor_presentation_from_texts, build_review_editor_right_line_anchor_from_texts,
    should_preserve_dirty_review_editor_right,
};

#[test]
fn review_editor_overlays_mark_modified_and_added_lines() {
    let rows = vec![
        SideBySideRow {
            kind: DiffRowKind::Code,
            left: DiffCell {
                line: Some(4),
                text: "before".to_string(),
                kind: DiffCellKind::Removed,
            },
            right: DiffCell {
                line: Some(4),
                text: "after".to_string(),
                kind: DiffCellKind::Added,
            },
            text: String::new(),
        },
        SideBySideRow {
            kind: DiffRowKind::Code,
            left: DiffCell {
                line: None,
                text: String::new(),
                kind: DiffCellKind::None,
            },
            right: DiffCell {
                line: Some(9),
                text: "new".to_string(),
                kind: DiffCellKind::Added,
            },
            text: String::new(),
        },
    ];

    let (left, right) = build_review_editor_overlays(&rows);

    assert_eq!(left.len(), 1);
    assert_eq!(left[0].line, 3);
    assert_eq!(left[0].kind, OverlayKind::DiffModification);
    assert_eq!(right.len(), 2);
    assert_eq!(right[0].line, 3);
    assert_eq!(right[0].kind, OverlayKind::DiffModification);
    assert_eq!(right[1].line, 8);
    assert_eq!(right[1].kind, OverlayKind::DiffAddition);
}

#[test]
fn review_editor_overlays_mark_removed_only_lines_on_left() {
    let rows = vec![
        SideBySideRow {
            kind: DiffRowKind::Meta,
            left: DiffCell {
                line: Some(1),
                text: "@@".to_string(),
                kind: DiffCellKind::None,
            },
            right: DiffCell {
                line: Some(1),
                text: "@@".to_string(),
                kind: DiffCellKind::None,
            },
            text: String::new(),
        },
        SideBySideRow {
            kind: DiffRowKind::Code,
            left: DiffCell {
                line: Some(12),
                text: "deleted".to_string(),
                kind: DiffCellKind::Removed,
            },
            right: DiffCell {
                line: None,
                text: String::new(),
                kind: DiffCellKind::None,
            },
            text: String::new(),
        },
    ];

    let (left, right) = build_review_editor_overlays(&rows);

    assert_eq!(left.len(), 1);
    assert_eq!(left[0].line, 11);
    assert_eq!(left[0].kind, OverlayKind::DiffDeletion);
    assert!(right.is_empty());
}

#[test]
fn text_overlays_pair_changed_blocks_as_modifications() {
    let left = "alpha\nbeta\ngamma\n";
    let right = "alpha\nbeta changed\ngamma\n";

    let (left_overlays, right_overlays) = build_review_editor_overlays_from_texts(left, right);

    assert_eq!(left_overlays.len(), 1);
    assert_eq!(left_overlays[0].line, 1);
    assert_eq!(left_overlays[0].kind, OverlayKind::DiffModification);
    assert_eq!(right_overlays.len(), 1);
    assert_eq!(right_overlays[0].line, 1);
    assert_eq!(right_overlays[0].kind, OverlayKind::DiffModification);
}

#[test]
fn text_overlays_mark_insertions_and_deletions() {
    let left = "alpha\nbeta\ngamma\n";
    let right = "alpha\ninserted\nbeta\n";

    let (left_overlays, right_overlays) = build_review_editor_overlays_from_texts(left, right);

    assert_eq!(left_overlays.len(), 1);
    assert_eq!(left_overlays[0].line, 2);
    assert_eq!(left_overlays[0].kind, OverlayKind::DiffDeletion);
    assert_eq!(right_overlays.len(), 1);
    assert_eq!(right_overlays[0].line, 1);
    assert_eq!(right_overlays[0].kind, OverlayKind::DiffAddition);
}

#[test]
fn right_line_anchor_tracks_modified_line_numbers_and_context() {
    let left = "alpha\nbeta\ngamma\n";
    let right = "alpha\nbeta changed\ngamma\n";

    let anchor = build_review_editor_right_line_anchor_from_texts(left, right, 1, 1)
        .expect("anchor should exist");

    assert_eq!(anchor.old_line, Some(2));
    assert_eq!(anchor.new_line, Some(2));
    assert_eq!(anchor.line_text, "+beta changed");
    assert_eq!(anchor.context_before, " alpha");
    assert_eq!(anchor.context_after, " gamma");
}

#[test]
fn presentation_folds_unchanged_regions_around_changed_hunks() {
    let left = "one\ntwo\nthree\nfour\nfive\nsix";
    let right = "one\ntwo changed\nthree\nfour\nfive\nsix";

    let presentation = build_review_editor_presentation_from_texts(left, right, 1, None);

    assert_eq!(
        presentation.left_folds,
        vec![FoldRegion {
            start_line: 3,
            end_line: 5,
        }]
    );
    assert_eq!(
        presentation.right_folds,
        vec![FoldRegion {
            start_line: 3,
            end_line: 5,
        }]
    );
}

#[test]
fn presentation_keeps_selected_right_line_visible_even_when_unchanged() {
    let left = "zero\none\ntwo\nthree\nfour\nfive\nsix";
    let right = "zero\none changed\ntwo\nthree\nfour\nfive\nsix";

    let presentation = build_review_editor_presentation_from_texts(left, right, 1, Some(5));

    assert!(
        !presentation
            .right_folds
            .iter()
            .any(|region| region.start_line <= 5 && region.end_line >= 5)
    );
}

#[test]
fn dirty_right_text_is_preserved_only_for_same_path_and_compare_pair() {
    assert!(should_preserve_dirty_review_editor_right(
        Some("src/lib.rs"),
        Some("workspace-head"),
        Some("workspace-target"),
        "src/lib.rs",
        Some("workspace-head"),
        Some("workspace-target"),
        true,
    ));

    assert!(!should_preserve_dirty_review_editor_right(
        Some("src/lib.rs"),
        Some("workspace-head"),
        Some("workspace-target"),
        "src/lib.rs",
        Some("main"),
        Some("workspace-target"),
        true,
    ));

    assert!(!should_preserve_dirty_review_editor_right(
        Some("src/lib.rs"),
        Some("workspace-head"),
        Some("workspace-target"),
        "src/other.rs",
        Some("workspace-head"),
        Some("workspace-target"),
        true,
    ));

    assert!(!should_preserve_dirty_review_editor_right(
        Some("src/lib.rs"),
        Some("workspace-head"),
        Some("workspace-target"),
        "src/lib.rs",
        Some("workspace-head"),
        Some("workspace-target"),
        false,
    ));
}
