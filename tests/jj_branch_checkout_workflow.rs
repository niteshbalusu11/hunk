use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use hunk::jj::{
    abandon_branch_head, checkout_or_create_branch, checkout_or_create_branch_with_change_transfer,
    commit_staged, describe_branch_head, load_snapshot, rename_branch,
    review_url_for_branch, squash_branch_head_into_parent,
};

#[test]
fn checkout_existing_bookmark_switches_without_crashing() {
    let fixture = TempRepo::new("checkout-existing-bookmark");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");

    checkout_or_create_branch(fixture.path(), "master")
        .expect("creating master bookmark should succeed");

    checkout_or_create_branch(fixture.path(), "feature")
        .expect("creating feature bookmark should succeed");

    write_file(fixture.path().join("tracked.txt"), "line one\nline two\n");
    commit_staged(fixture.path(), "feature commit").expect("feature commit should succeed");

    checkout_or_create_branch(fixture.path(), "master")
        .expect("switching to existing master bookmark should succeed");

    let snapshot = load_snapshot(fixture.path()).expect("snapshot should load after checkout");
    assert_eq!(snapshot.branch_name, "master");
    assert!(
        snapshot.files.is_empty(),
        "switching to an existing bookmark should not surface committed diff as working changes"
    );
}

#[test]
fn committing_on_checked_out_bookmark_advances_that_bookmark() {
    let fixture = TempRepo::new("checkout-bookmark-commit-advance");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");
    checkout_or_create_branch(fixture.path(), "master")
        .expect("creating master bookmark should succeed");

    write_file(fixture.path().join("tracked.txt"), "line one\nline two\n");
    commit_staged(fixture.path(), "master update should move bookmark")
        .expect("commit on checked-out bookmark should succeed");

    let master_log = run_jj_capture(
        fixture.path(),
        ["log", "-r", "master", "-n", "1", "--no-graph"],
    );
    assert!(
        master_log.contains("master update should move bookmark"),
        "master bookmark should point to latest commit after commit_staged"
    );
}

#[test]
fn creating_bookmark_can_move_uncommitted_changes_off_current_bookmark() {
    let fixture = TempRepo::new("create-bookmark-move-uncommitted");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");
    checkout_or_create_branch(fixture.path(), "main")
        .expect("creating main bookmark should succeed");

    write_file(fixture.path().join("tracked.txt"), "line one\nline two\n");
    checkout_or_create_branch_with_change_transfer(fixture.path(), "feature", true)
        .expect("creating feature bookmark should succeed");

    let snapshot = load_snapshot(fixture.path()).expect("snapshot should load after branch create");
    assert_eq!(snapshot.branch_name, "feature");
    assert!(
        snapshot.files.iter().any(|file| file.path == "tracked.txt"),
        "uncommitted changes should remain in working copy after moving to feature"
    );

    let bookmark_listing = run_jj_capture(fixture.path(), ["bookmark", "list", "main", "feature"]);
    assert!(
        bookmark_listing.contains("main:"),
        "main bookmark should still exist after creating feature"
    );
    assert!(
        bookmark_listing.contains("feature:"),
        "feature bookmark should exist after creation"
    );

    let main_target = bookmark_listing
        .lines()
        .find(|line| line.starts_with("main:"))
        .and_then(|line| line.split_whitespace().nth(2))
        .expect("main target commit should be listed")
        .to_string();
    let feature_target = bookmark_listing
        .lines()
        .find(|line| line.starts_with("feature:"))
        .and_then(|line| line.split_whitespace().nth(2))
        .expect("feature target commit should be listed")
        .to_string();
    assert_ne!(
        main_target, feature_target,
        "moving changes should leave main and feature on different commits"
    );
}

#[test]
fn switching_to_existing_bookmark_can_move_uncommitted_changes() {
    let fixture = TempRepo::new("switch-bookmark-move-uncommitted");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");
    checkout_or_create_branch(fixture.path(), "main")
        .expect("creating main bookmark should succeed");
    checkout_or_create_branch(fixture.path(), "feature")
        .expect("creating feature bookmark should succeed");
    checkout_or_create_branch(fixture.path(), "main")
        .expect("switching back to main should succeed");

    write_file(fixture.path().join("tracked.txt"), "line one\nline two\n");
    checkout_or_create_branch_with_change_transfer(fixture.path(), "feature", true)
        .expect("switching to feature with moved changes should succeed");

    let snapshot = load_snapshot(fixture.path()).expect("snapshot should load after branch switch");
    assert_eq!(snapshot.branch_name, "feature");
    assert!(
        snapshot.files.iter().any(|file| file.path == "tracked.txt"),
        "uncommitted changes should remain in working copy after switching with move enabled"
    );
}

#[test]
fn renaming_bookmark_updates_active_bookmark_and_listing() {
    let fixture = TempRepo::new("rename-bookmark-active");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");
    checkout_or_create_branch(fixture.path(), "feature-old")
        .expect("creating source bookmark should succeed");

    rename_branch(fixture.path(), "feature-old", "feature-new")
        .expect("renaming bookmark should succeed");

    let snapshot = load_snapshot(fixture.path()).expect("snapshot should load after rename");
    assert_eq!(
        snapshot.branch_name, "feature-new",
        "active bookmark should update to the renamed bookmark"
    );

    let bookmark_listing = run_jj_capture(
        fixture.path(),
        ["bookmark", "list", "feature-old", "feature-new"],
    );
    assert!(
        bookmark_listing.contains("feature-new:"),
        "renamed bookmark should be listed"
    );
    assert!(
        !bookmark_listing.contains("feature-old:"),
        "old bookmark name should no longer exist"
    );
}

#[test]
fn renaming_bookmark_rejects_existing_target() {
    let fixture = TempRepo::new("rename-bookmark-existing-target");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");
    checkout_or_create_branch(fixture.path(), "feature-old")
        .expect("creating source bookmark should succeed");
    checkout_or_create_branch(fixture.path(), "feature-existing")
        .expect("creating target bookmark should succeed");

    let err = rename_branch(fixture.path(), "feature-old", "feature-existing")
        .expect_err("renaming should fail when destination bookmark already exists");
    assert!(
        err.to_string().contains("already exists"),
        "error should explain destination bookmark conflict"
    );
}

#[test]
fn snapshot_includes_revision_stack_for_active_bookmark() {
    let fixture = TempRepo::new("bookmark-revision-stack");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");
    checkout_or_create_branch(fixture.path(), "stack")
        .expect("creating stack bookmark should succeed");

    write_file(fixture.path().join("tracked.txt"), "line one\nline two\n");
    commit_staged(fixture.path(), "stack second commit")
        .expect("second commit should succeed");

    write_file(
        fixture.path().join("tracked.txt"),
        "line one\nline two\nline three\n",
    );
    commit_staged(fixture.path(), "stack third commit")
        .expect("third commit should succeed");

    let snapshot = load_snapshot(fixture.path()).expect("snapshot should load");
    assert_eq!(snapshot.branch_name, "stack");
    assert!(
        snapshot.bookmark_revisions.len() >= 2,
        "revision stack should include at least latest commits"
    );
    assert_eq!(
        snapshot.bookmark_revisions[0].subject, "stack third commit",
        "latest revision should be first in stack"
    );
    assert_eq!(
        snapshot.bookmark_revisions[1].subject, "stack second commit",
        "stack should be ordered from newest to oldest"
    );
}

#[test]
fn describing_bookmark_head_updates_latest_revision_subject() {
    let fixture = TempRepo::new("describe-bookmark-head");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");
    checkout_or_create_branch(fixture.path(), "describe-me")
        .expect("creating bookmark should succeed");

    write_file(fixture.path().join("tracked.txt"), "line one\nline two\n");
    commit_staged(fixture.path(), "original subject").expect("commit should succeed");

    describe_branch_head(
        fixture.path(),
        "describe-me",
        "updated revision description",
    )
    .expect("describing bookmark head should succeed");

    let snapshot = load_snapshot(fixture.path()).expect("snapshot should load");
    assert_eq!(snapshot.branch_name, "describe-me");
    assert_eq!(
        snapshot.bookmark_revisions[0].subject,
        "updated revision description",
        "latest revision subject should reflect updated description"
    );
}

#[test]
fn abandoning_bookmark_head_moves_stack_to_previous_revision() {
    let fixture = TempRepo::new("abandon-bookmark-head");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");
    checkout_or_create_branch(fixture.path(), "stack")
        .expect("creating stack bookmark should succeed");

    write_file(fixture.path().join("tracked.txt"), "line one\nline two\n");
    commit_staged(fixture.path(), "stack second commit")
        .expect("second commit should succeed");

    write_file(
        fixture.path().join("tracked.txt"),
        "line one\nline two\nline three\n",
    );
    commit_staged(fixture.path(), "stack third commit")
        .expect("third commit should succeed");

    let before = load_snapshot(fixture.path()).expect("snapshot should load before abandon");
    assert_eq!(before.branch_name, "stack");
    assert_eq!(before.bookmark_revisions[0].subject, "stack third commit");

    abandon_branch_head(fixture.path(), "stack")
        .expect("abandoning bookmark head should succeed");

    let after = load_snapshot(fixture.path()).expect("snapshot should load after abandon");
    assert_eq!(after.branch_name, "stack");
    assert_eq!(
        after.bookmark_revisions[0].subject, "stack second commit",
        "bookmark should move to previous revision after abandoning tip"
    );
}

#[test]
fn squashing_bookmark_head_into_parent_keeps_combined_changes() {
    let fixture = TempRepo::new("squash-bookmark-head");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");
    checkout_or_create_branch(fixture.path(), "stack")
        .expect("creating stack bookmark should succeed");

    write_file(fixture.path().join("tracked.txt"), "line one\nline two\n");
    commit_staged(fixture.path(), "stack second commit")
        .expect("second commit should succeed");

    write_file(
        fixture.path().join("tracked.txt"),
        "line one\nline two\nline three\n",
    );
    commit_staged(fixture.path(), "stack third commit")
        .expect("third commit should succeed");

    squash_branch_head_into_parent(fixture.path(), "stack")
        .expect("squashing bookmark head should succeed");

    let snapshot = load_snapshot(fixture.path()).expect("snapshot should load after squash");
    assert_eq!(snapshot.branch_name, "stack");
    assert_eq!(
        snapshot.bookmark_revisions[0].subject, "stack second commit",
        "parent revision should become the bookmark head after squash"
    );

    let stack_show = run_jj_capture(fixture.path(), ["show", "-r", "stack", "--git"]);
    assert!(
        stack_show.contains("+line three"),
        "squashed bookmark head should preserve tip changes in parent revision"
    );
}

#[test]
fn review_url_for_github_remote_uses_compare_link() {
    let fixture = TempRepo::new("review-url-github");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");
    checkout_or_create_branch(fixture.path(), "feature/review-url")
        .expect("creating bookmark should succeed");

    run_jj(
        fixture.path(),
        [
            "git",
            "remote",
            "add",
            "origin",
            "https://github.com/example-org/hunk.git",
        ],
    );

    let review_url = review_url_for_branch(fixture.path(), "feature/review-url")
        .expect("review URL should be computed")
        .expect("github remote should produce review URL");

    assert_eq!(
        review_url,
        "https://github.com/example-org/hunk/compare/feature%2Freview-url?expand=1",
        "github remotes should use compare links for review URL quick action"
    );
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn new(prefix: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("hunk-{prefix}-{unique}"));
        fs::create_dir_all(&path).expect("temp repo directory should be created");

        run_jj(&path, ["git", "init", "--colocate"]);
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write_file(path: PathBuf, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directories should be created");
    }
    fs::write(path, contents).expect("file should be written");
}

fn run_jj<const N: usize>(cwd: &Path, args: [&str; N]) {
    let status = Command::new("jj")
        .args(args)
        .current_dir(cwd)
        .status()
        .expect("jj command should run");
    assert!(status.success(), "jj command failed");
}

fn run_jj_capture<const N: usize>(cwd: &Path, args: [&str; N]) -> String {
    let output = Command::new("jj")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("jj command should run");
    assert!(
        output.status.success(),
        "jj command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}
