use hunk::diff::{
    DiffCellKind, DiffLineKind, DiffRowKind, parse_patch_document, parse_patch_side_by_side,
};

#[test]
fn pairs_multiple_removed_and_added_lines_in_one_block() {
    let patch = "\
diff --git a/file.txt b/file.txt
index 123..456 100644
--- a/file.txt
+++ b/file.txt
@@ -1,3 +1,3 @@
-one
-two
+alpha
+beta
 three";

    let rows = parse_patch_side_by_side(patch);
    let code_rows = rows
        .iter()
        .filter(|row| matches!(row.kind, DiffRowKind::Code))
        .collect::<Vec<_>>();

    assert_eq!(code_rows.len(), 3);

    assert_eq!(code_rows[0].left.kind, DiffCellKind::Removed);
    assert_eq!(code_rows[0].left.text, "one");
    assert_eq!(code_rows[0].right.kind, DiffCellKind::Added);
    assert_eq!(code_rows[0].right.text, "alpha");

    assert_eq!(code_rows[1].left.kind, DiffCellKind::Removed);
    assert_eq!(code_rows[1].left.text, "two");
    assert_eq!(code_rows[1].right.kind, DiffCellKind::Added);
    assert_eq!(code_rows[1].right.text, "beta");

    assert_eq!(code_rows[2].left.kind, DiffCellKind::Context);
    assert_eq!(code_rows[2].right.kind, DiffCellKind::Context);
}

#[test]
fn keeps_unbalanced_change_block_aligned() {
    let patch = "\
@@ -10,3 +10,2 @@
-one
-two
-three
+uno
+dos";

    let rows = parse_patch_side_by_side(patch);
    let code_rows = rows
        .iter()
        .filter(|row| matches!(row.kind, DiffRowKind::Code))
        .collect::<Vec<_>>();

    assert_eq!(code_rows.len(), 3);

    assert_eq!(code_rows[0].left.kind, DiffCellKind::Removed);
    assert_eq!(code_rows[0].right.kind, DiffCellKind::Added);

    assert_eq!(code_rows[1].left.kind, DiffCellKind::Removed);
    assert_eq!(code_rows[1].right.kind, DiffCellKind::Added);

    assert_eq!(code_rows[2].left.kind, DiffCellKind::Removed);
    assert_eq!(code_rows[2].right.kind, DiffCellKind::None);
}

#[test]
fn parses_structured_document_hunks_with_line_numbers() {
    let patch = "\
diff --git a/file.txt b/file.txt
index 123..456 100644
--- a/file.txt
+++ b/file.txt
@@ -10,2 +10,3 @@
-old one
 old two
+new one
+new two
\\ No newline at end of file";

    let document = parse_patch_document(patch);

    assert_eq!(document.prelude.len(), 4);
    assert_eq!(document.hunks.len(), 1);

    let hunk = &document.hunks[0];
    assert_eq!(hunk.header, "@@ -10,2 +10,3 @@");
    assert_eq!(hunk.old_start, 10);
    assert_eq!(hunk.new_start, 10);
    assert_eq!(hunk.lines.len(), 4);

    assert_eq!(hunk.lines[0].kind, DiffLineKind::Removed);
    assert_eq!(hunk.lines[0].old_line, Some(10));
    assert_eq!(hunk.lines[0].new_line, None);
    assert_eq!(hunk.lines[0].text, "old one");

    assert_eq!(hunk.lines[1].kind, DiffLineKind::Context);
    assert_eq!(hunk.lines[1].old_line, Some(11));
    assert_eq!(hunk.lines[1].new_line, Some(10));

    assert_eq!(hunk.lines[2].kind, DiffLineKind::Added);
    assert_eq!(hunk.lines[2].old_line, None);
    assert_eq!(hunk.lines[2].new_line, Some(11));

    assert_eq!(hunk.trailing_meta, vec!["\\ No newline at end of file"]);
}

#[test]
fn keeps_multiple_hunks_as_separate_structures() {
    let patch = "\
@@ -1,2 +1,2 @@
-one
+uno
 two
@@ -10,1 +10,2 @@
 ten
+diez";

    let document = parse_patch_document(patch);
    assert_eq!(document.hunks.len(), 2);
    assert_eq!(document.hunks[0].old_start, 1);
    assert_eq!(document.hunks[0].new_start, 1);
    assert_eq!(document.hunks[1].old_start, 10);
    assert_eq!(document.hunks[1].new_start, 10);
}
