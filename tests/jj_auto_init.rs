use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use hunk::jj::load_snapshot;

#[test]
fn load_snapshot_auto_initializes_jj_for_git_repo() {
    let fixture = TempDir::new("jj-auto-init-git");
    run_git(fixture.path(), ["init"]);
    write_file(fixture.path().join("hello.txt"), "hello\n");

    let snapshot = load_snapshot(fixture.path()).expect("snapshot should load for git repo");

    assert!(
        fixture.path().join(".jj").exists(),
        "JJ metadata should be auto-initialized in git repo"
    );
    assert!(
        snapshot.files.iter().any(|file| file.path == "hello.txt"),
        "snapshot should include working copy change after JJ auto-init"
    );
}

#[test]
fn load_snapshot_errors_when_no_jj_or_git_repo_exists() {
    let fixture = TempDir::new("jj-auto-init-none");

    let err = load_snapshot(fixture.path()).expect_err("snapshot should fail outside repositories");
    let message = err.to_string().to_lowercase();
    assert!(
        message.contains("failed to discover jj repository"),
        "error should explain JJ repository discovery failure"
    );
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("hunk-{prefix}-{unique}"));
        fs::create_dir_all(&path).expect("temp directory should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
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

fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .expect("git command should run");
    assert!(status.success(), "git command failed");
}
