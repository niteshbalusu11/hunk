use hunk::diff::{DiffCellKind, DiffRowKind, parse_patch_side_by_side};

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
