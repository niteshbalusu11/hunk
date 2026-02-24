use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use git2::{Diff, DiffFormat, DiffOptions, Repository, Status, StatusOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
    TypeChange,
    Conflicted,
    Unknown,
}

impl FileStatus {
    pub fn tag(self) -> &'static str {
        match self {
            Self::Added => "A",
            Self::Modified => "M",
            Self::Deleted => "D",
            Self::Renamed => "R",
            Self::Untracked => "U",
            Self::TypeChange => "T",
            Self::Conflicted => "!",
            Self::Unknown => "-",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub path: String,
    pub status: FileStatus,
}

#[derive(Debug, Clone)]
pub struct RepoSnapshot {
    pub root: PathBuf,
    pub branch_name: String,
    pub files: Vec<ChangedFile>,
    pub line_stats: LineStats,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LineStats {
    pub added: u64,
    pub removed: u64,
}

impl LineStats {
    pub fn changed(self) -> u64 {
        self.added + self.removed
    }
}

pub fn load_snapshot(cwd: &Path) -> Result<RepoSnapshot> {
    let repo = Repository::discover(cwd).context("failed to discover git repository")?;
    let root = repo_root(&repo)?;
    let branch_name = current_branch_name(&repo);

    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .include_unmodified(false);

    let statuses = repo
        .statuses(Some(&mut options))
        .context("failed to load repository status")?;

    let mut files = statuses
        .iter()
        .filter_map(|entry| {
            entry.path().map(|path| ChangedFile {
                path: normalize_path(path),
                status: map_status(entry.status()),
            })
        })
        .filter(|file| !file.path.is_empty())
        .collect::<Vec<_>>();

    files.sort_by(|a, b| a.path.cmp(&b.path));
    files.dedup_by(|a, b| a.path == b.path);

    let line_stats = repo_line_stats(&repo)?;

    Ok(RepoSnapshot {
        root,
        branch_name,
        files,
        line_stats,
    })
}

pub fn load_patch(repo_root: &Path, file_path: &str, _: FileStatus) -> Result<String> {
    let repo = Repository::open(repo_root)
        .or_else(|_| Repository::discover(repo_root))
        .context("failed to open git repository")?;

    if let Some(head_tree) = repo.head().ok().and_then(|head| head.peel_to_tree().ok()) {
        let mut options = diff_options(file_path);
        let diff = repo
            .diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut options))
            .context("failed to build workdir diff against HEAD")?;
        return render_patch(&diff);
    }

    let mut options = diff_options(file_path);
    let unstaged = repo
        .diff_index_to_workdir(None, Some(&mut options))
        .context("failed to build unstaged diff")?;
    let mut patch = render_patch(&unstaged)?;

    if patch.trim().is_empty() {
        let index = repo.index().context("failed to read index")?;
        let mut options = diff_options(file_path);
        let staged = repo
            .diff_tree_to_index(None, Some(&index), Some(&mut options))
            .context("failed to build staged diff")?;
        patch = render_patch(&staged)?;
    }

    Ok(patch)
}

fn repo_root(repo: &Repository) -> Result<PathBuf> {
    if let Some(workdir) = repo.workdir() {
        return Ok(workdir.to_path_buf());
    }

    repo.path()
        .parent()
        .map(|path| path.to_path_buf())
        .context("failed to resolve repository root")
}

fn map_status(status: Status) -> FileStatus {
    if status.is_conflicted() {
        return FileStatus::Conflicted;
    }

    if status.is_wt_new() {
        return FileStatus::Untracked;
    }

    if status.is_index_new() {
        return FileStatus::Added;
    }

    if status.is_wt_deleted() || status.is_index_deleted() {
        return FileStatus::Deleted;
    }

    if status.is_wt_renamed() || status.is_index_renamed() {
        return FileStatus::Renamed;
    }

    if status.is_wt_typechange() || status.is_index_typechange() {
        return FileStatus::TypeChange;
    }

    if status.is_wt_modified() || status.is_index_modified() {
        return FileStatus::Modified;
    }

    FileStatus::Unknown
}

fn normalize_path(path: &str) -> String {
    path.trim().trim_end_matches('/').to_string()
}

fn diff_options(file_path: &str) -> DiffOptions {
    let mut options = DiffOptions::new();
    apply_common_diff_options(&mut options);
    options.pathspec(file_path);
    options
}

fn render_patch(diff: &Diff<'_>) -> Result<String> {
    let mut patch = String::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = line.origin();
        let prefix = match origin {
            ' ' | '+' | '-' => Some(origin),
            // EOF line variants from git2, normalize to regular sigils.
            '=' => Some(' '),
            '>' => Some('+'),
            '<' => Some('-'),
            _ => None,
        };

        if let Some(prefix) = prefix {
            patch.push(prefix);
        }
        patch.push_str(std::str::from_utf8(line.content()).unwrap_or_default());
        true
    })
    .context("failed to render diff patch")?;

    Ok(patch)
}

fn current_branch_name(repo: &Repository) -> String {
    match repo.head() {
        Ok(head) => {
            if head.is_branch() {
                return head.shorthand().unwrap_or("unknown").to_string();
            }

            if let Some(oid) = head.target() {
                let short = oid.to_string();
                return format!("detached-{}", &short[..7]);
            }

            "detached".to_string()
        }
        Err(_) => "unborn".to_string(),
    }
}

fn repo_line_stats(repo: &Repository) -> Result<LineStats> {
    if let Some(head_tree) = repo.head().ok().and_then(|head| head.peel_to_tree().ok()) {
        let mut options = DiffOptions::new();
        apply_common_diff_options(&mut options);
        let diff = repo
            .diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut options))
            .context("failed to build repository diff against HEAD")?;
        return diff_line_stats(&diff);
    }

    let mut options = DiffOptions::new();
    apply_common_diff_options(&mut options);
    let unstaged = repo
        .diff_index_to_workdir(None, Some(&mut options))
        .context("failed to build unstaged repository diff")?;
    let mut total = diff_line_stats(&unstaged)?;

    let index = repo.index().context("failed to read index")?;
    let mut options = DiffOptions::new();
    apply_common_diff_options(&mut options);
    let staged = repo
        .diff_tree_to_index(None, Some(&index), Some(&mut options))
        .context("failed to build staged repository diff")?;
    let staged_stats = diff_line_stats(&staged)?;

    total.added += staged_stats.added;
    total.removed += staged_stats.removed;
    Ok(total)
}

fn diff_line_stats(diff: &Diff<'_>) -> Result<LineStats> {
    let stats = diff.stats().context("failed to build diff stats")?;
    Ok(LineStats {
        added: stats.insertions() as u64,
        removed: stats.deletions() as u64,
    })
}

fn apply_common_diff_options(options: &mut DiffOptions) {
    options
        .context_lines(3)
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);
}
