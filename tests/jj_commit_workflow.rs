use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use hunk::jj::{commit_staged, load_snapshot, stage_all, unstage_all};

#[test]
fn stage_actions_are_rejected_with_jj_backend() {
    let fixture = TempRepo::new("stage-actions-unsupported");
    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");

    let stage_all_err = stage_all(fixture.path()).expect_err("stage_all should fail under JJ");
    assert!(
        stage_all_err.to_string().contains("staging index"),
        "error should explain why stage_all is unsupported"
    );

    let unstage_all_err =
        unstage_all(fixture.path()).expect_err("unstage_all should fail under JJ");
    assert!(
        unstage_all_err.to_string().contains("staging index"),
        "error should explain why unstage_all is unsupported"
    );
}

#[test]
fn commit_staged_commits_working_copy_changes_with_jj() {
    let fixture = TempRepo::new("commit-staged-jj");
    let tracked = fixture.path().join("tracked.txt");
    write_file(tracked.clone(), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");

    write_file(tracked, "line one\nline two\n");
    commit_staged(fixture.path(), "update tracked").expect("second commit should succeed");

    let snapshot = load_snapshot(fixture.path()).expect("snapshot should load after commit");
    assert!(snapshot.files.is_empty(), "working copy should be clean");
    assert!(
        snapshot.last_commit_subject.as_deref() == Some("update tracked"),
        "last commit subject should match the latest commit"
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
