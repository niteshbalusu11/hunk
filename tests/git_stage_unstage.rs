use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use hunk::git::{load_snapshot, stage_all, unstage_all};

#[test]
fn unstage_all_clears_staged_flags_for_modified_files() {
    let fixture = TempRepo::new("unstage-all-modified");
    run_git(fixture.path(), ["config", "user.email", "hunk@test.local"]);
    run_git(fixture.path(), ["config", "user.name", "Hunk Test"]);

    let tracked = fixture.path().join("tracked.txt");
    write_file(tracked.clone(), "line one\n");
    run_git(fixture.path(), ["add", "."]);
    run_git(fixture.path(), ["commit", "-m", "initial"]);

    write_file(tracked, "line one\nline two\n");
    stage_all(fixture.path()).expect("stage all should succeed");

    let staged = load_snapshot(fixture.path()).expect("snapshot after stage all should load");
    let staged_file = staged
        .files
        .iter()
        .find(|file| file.path == "tracked.txt")
        .expect("tracked file should be present after stage all");
    assert!(staged_file.staged, "tracked file should be staged");

    unstage_all(fixture.path()).expect("unstage all should succeed");

    let unstaged = load_snapshot(fixture.path()).expect("snapshot after unstage all should load");
    let unstaged_file = unstaged
        .files
        .iter()
        .find(|file| file.path == "tracked.txt")
        .expect("tracked file should be present after unstage all");
    assert!(
        !unstaged_file.staged,
        "tracked file should not be staged after unstage all"
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

        run_git(&path, ["init"]);
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

fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .expect("git command should run");
    assert!(status.success(), "git command failed");
}
