use hunk::git::{is_valid_branch_name, sanitize_branch_name};

#[test]
fn sanitizes_space_separated_branch_name() {
    let branch = sanitize_branch_name("some random branch");
    assert_eq!(branch, "some-random-branch");
    assert!(is_valid_branch_name(&branch));
}

#[test]
fn sanitizes_branch_with_special_characters() {
    let branch = sanitize_branch_name("Feature: Add [WIP]?");
    assert_eq!(branch, "feature-add-wip");
    assert!(is_valid_branch_name(&branch));
}

#[test]
fn preserves_path_like_branch_shape() {
    let branch = sanitize_branch_name("feature/my cool branch");
    assert_eq!(branch, "feature/my-cool-branch");
    assert!(is_valid_branch_name(&branch));
}

#[test]
fn falls_back_for_empty_name() {
    let branch = sanitize_branch_name("   ");
    assert_eq!(branch, "branch");
    assert!(is_valid_branch_name(&branch));
}

#[test]
fn avoids_reserved_head_name() {
    let branch = sanitize_branch_name("HEAD");
    assert_eq!(branch, "head-branch");
    assert!(is_valid_branch_name(&branch));
}
