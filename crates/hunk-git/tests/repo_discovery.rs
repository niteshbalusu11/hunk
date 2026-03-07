use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use hunk_git::git::{discover_repo_root, load_snapshot_fingerprint, open_repo, open_repo_at_root};
use tempfile::TempDir;

#[test]
fn discovers_repo_root_from_nested_directory() -> Result<()> {
    let fixture = TempGitRepo::new()?;
    let nested = fixture.root().join("src/nested/path");
    fs::create_dir_all(&nested)?;

    let discovered = discover_repo_root(nested.as_path())?;

    assert_eq!(discovered, fixture.root());
    Ok(())
}

#[test]
fn opens_repository_from_worktree_or_git_dir() -> Result<()> {
    let fixture = TempGitRepo::new()?;

    let from_worktree = open_repo(fixture.root())?;
    let from_git_dir = open_repo(fixture.root().join(".git").as_path())?;
    let direct = open_repo_at_root(fixture.root())?;

    assert_eq!(from_worktree.root(), fixture.root());
    assert_eq!(from_git_dir.root(), fixture.root());
    assert_eq!(direct.root(), fixture.root());
    assert_eq!(
        from_worktree.git_dir(),
        fixture.root().join(".git").as_path()
    );
    assert_eq!(
        from_git_dir.git_dir(),
        fixture.root().join(".git").as_path()
    );
    Ok(())
}

#[test]
fn fingerprint_reports_unborn_head_without_commit_id() -> Result<()> {
    let fixture = TempGitRepo::new()?;

    let fingerprint = load_snapshot_fingerprint(fixture.root())?;

    assert_eq!(fingerprint.root(), fixture.root());
    assert!(fingerprint.head_ref_name().is_some());
    assert_eq!(fingerprint.head_commit_id(), None);
    Ok(())
}

struct TempGitRepo {
    _tempdir: TempDir,
    root: PathBuf,
}

impl TempGitRepo {
    fn new() -> Result<Self> {
        let tempdir = tempfile::tempdir()?;
        let root = tempdir.path().join("repo");
        gix::init(root.as_path())?;
        let root = fs::canonicalize(root)?;
        Ok(Self {
            _tempdir: tempdir,
            root,
        })
    }

    fn root(&self) -> &Path {
        &self.root
    }
}
