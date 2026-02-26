use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use hunk::jj::{checkout_or_create_branch, commit_staged, load_snapshot};

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
