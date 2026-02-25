use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use hunk::git::{RepoTreeEntryKind, load_repo_tree};

#[test]
fn load_repo_tree_marks_gitignored_entries() {
    let fixture = TempRepo::new("repo-tree-ignored");
    write_file(fixture.path().join(".gitignore"), "target/\n*.log\n");
    write_file(fixture.path().join("src/main.rs"), "fn main() {}\n");
    write_file(fixture.path().join("target/cache.bin"), "cache\n");
    write_file(fixture.path().join("logs/app.log"), "hello\n");

    let entries = load_repo_tree(fixture.path()).expect("repo tree should load");

    assert!(entries.iter().any(|entry| {
        entry.path == "src" && entry.kind == RepoTreeEntryKind::Directory && !entry.ignored
    }));
    assert!(entries.iter().any(|entry| {
        entry.path == "src/main.rs" && entry.kind == RepoTreeEntryKind::File && !entry.ignored
    }));
    assert!(entries.iter().any(|entry| {
        entry.path == "target" && entry.kind == RepoTreeEntryKind::Directory && entry.ignored
    }));
    assert!(entries.iter().any(|entry| {
        entry.path == "target/cache.bin" && entry.kind == RepoTreeEntryKind::File && entry.ignored
    }));
    assert!(entries.iter().any(|entry| {
        entry.path == "logs/app.log" && entry.kind == RepoTreeEntryKind::File && entry.ignored
    }));
}

#[test]
fn load_repo_tree_excludes_git_internal_directory() {
    let fixture = TempRepo::new("repo-tree-no-dot-git");
    write_file(fixture.path().join("README.md"), "# hunk\n");

    let entries = load_repo_tree(fixture.path()).expect("repo tree should load");
    assert!(entries.iter().all(|entry| !entry.path.starts_with(".git")));
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
