use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use git2::{
    BranchType, Cred, CredentialType, Diff, DiffFormat, DiffOptions, Error, IndexAddOption,
    PushOptions, Reference, Repository, ResetType, Signature, Status, StatusOptions,
    build::CheckoutBuilder,
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    pub path: String,
    pub status: FileStatus,
    pub staged: bool,
    pub untracked: bool,
}

impl ChangedFile {
    pub fn is_tracked(&self) -> bool {
        !self.untracked
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalBranch {
    pub name: String,
    pub is_current: bool,
    pub tip_unix_time: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoTreeEntryKind {
    Directory,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoTreeEntry {
    pub path: String,
    pub kind: RepoTreeEntryKind,
    pub ignored: bool,
}

#[derive(Debug, Clone)]
pub struct RepoSnapshot {
    pub root: PathBuf,
    pub branch_name: String,
    pub branch_has_upstream: bool,
    pub branch_ahead_count: usize,
    pub branches: Vec<LocalBranch>,
    pub files: Vec<ChangedFile>,
    pub line_stats: LineStats,
    pub last_commit_subject: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSnapshotFingerprint {
    root: PathBuf,
    branch_name: String,
    head_target: Option<String>,
    changed_file_count: usize,
    changed_file_signature: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LineStats {
    pub added: u64,
    pub removed: u64,
}

impl LineStats {
    pub fn changed(self) -> u64 {
        self.added + self.removed
    }
}

const MAX_REPO_TREE_ENTRIES: usize = 60_000;

pub fn load_snapshot(cwd: &Path) -> Result<RepoSnapshot> {
    let repo = Repository::discover(cwd).context("failed to discover git repository")?;
    let root = repo_root(&repo)?;
    let branch_name = current_branch_name(&repo);
    let files = load_changed_files(&repo)?;
    let line_stats = repo_line_stats(&repo)?;
    let branches = list_local_branches(&repo, &branch_name)?;
    let branch_has_upstream = current_branch_has_upstream(&repo, &branch_name);
    let branch_ahead_count = current_branch_ahead_count(&repo, &branch_name);
    let last_commit_subject = last_commit_subject(&repo);

    Ok(RepoSnapshot {
        root,
        branch_name,
        branch_has_upstream,
        branch_ahead_count,
        branches,
        files,
        line_stats,
        last_commit_subject,
    })
}

pub fn load_snapshot_fingerprint(cwd: &Path) -> Result<RepoSnapshotFingerprint> {
    let repo = Repository::discover(cwd).context("failed to discover git repository")?;
    let root = repo_root(&repo)?;
    let branch_name = current_branch_name(&repo);
    let files = load_changed_files(&repo)?;
    Ok(snapshot_fingerprint(&repo, root, branch_name, &files))
}

pub fn load_patch(repo_root: &Path, file_path: &str, status: FileStatus) -> Result<String> {
    let repo = open_repo_for_patch(repo_root)?;
    load_patch_from_open_repo(&repo, file_path, status)
}

pub fn open_repo_for_patch(repo_root: &Path) -> Result<Repository> {
    open_repo(repo_root)
}

pub fn load_patch_from_open_repo(
    repo: &Repository,
    file_path: &str,
    _: FileStatus,
) -> Result<String> {
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

pub fn load_repo_tree(repo_root: &Path) -> Result<Vec<RepoTreeEntry>> {
    let repo = open_repo(repo_root)?;
    let mut entries = Vec::new();
    walk_repo_tree(repo_root, repo_root, &repo, &mut entries)?;
    Ok(entries)
}

pub fn stage_file(repo_root: &Path, file_path: &str) -> Result<()> {
    let repo = open_repo(repo_root)?;
    let mut index = repo.index().context("failed to read index")?;
    let status = repo
        .status_file(Path::new(file_path))
        .unwrap_or(Status::WT_NEW);

    if status.is_wt_deleted() {
        index
            .remove_path(Path::new(file_path))
            .with_context(|| format!("failed to stage deletion for {file_path}"))?;
    } else {
        index
            .add_path(Path::new(file_path))
            .with_context(|| format!("failed to stage {file_path}"))?;
    }

    index.write().context("failed to write index")
}

pub fn unstage_file(repo_root: &Path, file_path: &str) -> Result<()> {
    let repo = open_repo(repo_root)?;
    let target = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .map(|commit| commit.into_object());

    if let Some(target) = target.as_ref() {
        repo.reset_default(Some(target), [file_path])
            .with_context(|| format!("failed to unstage {file_path}"))?;
        return Ok(());
    }

    let mut index = repo.index().context("failed to read index")?;
    if index.get_path(Path::new(file_path), 0).is_some() {
        index
            .remove_path(Path::new(file_path))
            .with_context(|| format!("failed to unstage {file_path}"))?;
        index.write().context("failed to write index")?;
    }
    Ok(())
}

pub fn stage_all(repo_root: &Path) -> Result<()> {
    let repo = open_repo(repo_root)?;
    let mut index = repo.index().context("failed to read index")?;

    index
        .add_all(["."], IndexAddOption::DEFAULT, None)
        .context("failed to add files to index")?;
    index
        .update_all(["."], None)
        .context("failed to refresh tracked files in index")?;
    index.write().context("failed to write index")
}

pub fn unstage_all(repo_root: &Path) -> Result<()> {
    let repo = open_repo(repo_root)?;

    if let Some(commit) = repo.head().ok().and_then(|head| head.peel_to_commit().ok()) {
        repo.reset(commit.as_object(), ResetType::Mixed, None)
            .context("failed to unstage all files")?;
        return Ok(());
    }

    let mut index = repo.index().context("failed to read index")?;
    index.clear().context("failed to clear index")?;
    index.write().context("failed to write index")
}

pub fn commit_staged(repo_root: &Path, message: &str) -> Result<()> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("commit message cannot be empty"));
    }

    let repo = open_repo(repo_root)?;
    let mut index = repo.index().context("failed to read index")?;
    if index.is_empty() {
        return Err(anyhow!("no staged changes to commit"));
    }

    let tree_oid = index
        .write_tree()
        .context("failed to write tree from index")?;
    let tree = repo
        .find_tree(tree_oid)
        .context("failed to find staged tree")?;

    let parent_commit = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    if let Some(parent) = parent_commit.as_ref()
        && parent.tree_id() == tree_oid
    {
        return Err(anyhow!("no staged changes to commit"));
    }

    let signature = default_signature(&repo)?;
    let parents = parent_commit.iter().collect::<Vec<_>>();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        trimmed,
        &tree,
        parents.as_slice(),
    )
    .context("failed to create commit")?;

    index.write().context("failed to write index")?;
    Ok(())
}

pub fn checkout_or_create_branch(repo_root: &Path, branch_name: &str) -> Result<()> {
    let branch_name = branch_name.trim();
    if branch_name.is_empty() {
        return Err(anyhow!("branch name cannot be empty"));
    }

    if !is_valid_branch_name(branch_name) {
        return Err(anyhow!("invalid branch name: {branch_name}"));
    }

    let repo = open_repo(repo_root)?;
    let refname = format!("refs/heads/{branch_name}");

    if repo.find_branch(branch_name, BranchType::Local).is_err()
        && let Some(commit) = repo.head().ok().and_then(|head| head.peel_to_commit().ok())
    {
        repo.branch(branch_name, &commit, false)
            .with_context(|| format!("failed to create branch {branch_name}"))?;
    }

    repo.set_head(&refname)
        .with_context(|| format!("failed to set HEAD to {branch_name}"))?;

    if repo.head().ok().and_then(|head| head.target()).is_some() {
        let mut checkout = CheckoutBuilder::new();
        checkout.safe();
        repo.checkout_head(Some(&mut checkout))
            .with_context(|| format!("failed to checkout {branch_name}"))?;
    }

    Ok(())
}

pub fn push_current_branch(repo_root: &Path, branch_name: &str, has_upstream: bool) -> Result<()> {
    let repo = open_repo(repo_root)?;
    let branch_ref = format!("refs/heads/{branch_name}");

    let remote_name = if has_upstream {
        repo.branch_upstream_remote(&branch_ref)
            .ok()
            .and_then(|buf| buf.as_str().map(|name| name.to_string()))
            .filter(|name| !name.is_empty())
            .ok_or_else(|| anyhow!("failed to resolve upstream remote for {branch_name}"))?
    } else {
        preferred_remote_name(&repo)?
    };

    let config = repo.config().ok();
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(move |url, username_from_url, allowed| {
        if allowed.contains(CredentialType::SSH_KEY) {
            let username = username_from_url.unwrap_or("git");
            if let Ok(cred) = Cred::ssh_key_from_agent(username) {
                return Ok(cred);
            }
        }

        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT)
            && let Some(config) = config.as_ref()
            && let Ok(cred) = Cred::credential_helper(config, url, username_from_url)
        {
            return Ok(cred);
        }

        if allowed.contains(CredentialType::USERNAME) {
            return Cred::username(username_from_url.unwrap_or("git"));
        }

        if allowed.contains(CredentialType::DEFAULT) {
            return Cred::default();
        }

        Err(Error::from_str("no authentication method available"))
    });
    callbacks.push_update_reference(|reference, status| {
        if let Some(status) = status {
            Err(Error::from_str(&format!(
                "remote rejected {reference}: {status}"
            )))
        } else {
            Ok(())
        }
    });

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    let refspec = format!("refs/heads/{branch_name}:refs/heads/{branch_name}");
    let mut remote = repo
        .find_remote(&remote_name)
        .with_context(|| format!("failed to find remote {remote_name}"))?;
    remote
        .push(&[refspec], Some(&mut push_options))
        .with_context(|| format!("failed to push branch {branch_name} to {remote_name}"))?;

    if !has_upstream {
        let mut branch = repo
            .find_branch(branch_name, BranchType::Local)
            .with_context(|| format!("failed to resolve local branch {branch_name}"))?;
        branch
            .set_upstream(Some(&format!("{remote_name}/{branch_name}")))
            .with_context(|| format!("failed to set upstream for {branch_name}"))?;
    }

    Ok(())
}

pub fn sanitize_branch_name(input: &str) -> String {
    let lowered = input.trim().to_lowercase();

    let mut normalized = String::with_capacity(lowered.len());
    let mut last_dash = false;
    for ch in lowered.chars() {
        let mapped = match ch {
            'a'..='z' | '0'..='9' | '/' | '.' | '_' | '-' => ch,
            c if c.is_whitespace() => '-',
            _ => '-',
        };

        if mapped == '-' {
            if last_dash {
                continue;
            }
            last_dash = true;
        } else {
            last_dash = false;
        }

        normalized.push(mapped);
    }

    let mut segments = Vec::new();
    for segment in normalized.split('/') {
        let mut clean = segment
            .trim_matches(|c: char| c == '-' || c == '.')
            .replace("@{", "-")
            .replace(['~', '^', ':', '?', '*', '[', '\\'], "-");

        while clean.contains("--") {
            clean = clean.replace("--", "-");
        }

        while clean.contains("..") {
            clean = clean.replace("..", ".");
        }

        if clean.ends_with(".lock") {
            clean = clean
                .trim_end_matches(".lock")
                .trim_end_matches('.')
                .to_string();
        }

        if !clean.is_empty() {
            segments.push(clean);
        }
    }

    let mut candidate = if segments.is_empty() {
        "branch".to_string()
    } else {
        segments.join("/")
    };

    if candidate.eq_ignore_ascii_case("head") {
        candidate = "head-branch".to_string();
    }

    candidate = candidate.trim_matches('/').to_string();

    if !is_valid_branch_name(&candidate) {
        candidate = candidate
            .chars()
            .map(|ch| match ch {
                'a'..='z' | '0'..='9' | '-' | '_' | '.' | '/' => ch,
                _ => '-',
            })
            .collect::<String>();

        candidate = candidate
            .split('/')
            .filter_map(|segment| {
                let segment = segment.trim_matches(|c: char| c == '-' || c == '.');
                if segment.is_empty() {
                    None
                } else {
                    Some(segment.to_string())
                }
            })
            .collect::<Vec<_>>()
            .join("/");
    }

    if candidate.is_empty() {
        candidate = "branch".to_string();
    }

    if !is_valid_branch_name(&candidate) {
        "branch-new".to_string()
    } else {
        candidate
    }
}

pub fn is_valid_branch_name(name: &str) -> bool {
    if name.trim().is_empty() {
        return false;
    }

    Reference::is_valid_name(format!("refs/heads/{name}").as_str())
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

fn load_changed_files(repo: &Repository) -> Result<Vec<ChangedFile>> {
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

    let mut file_map = BTreeMap::<String, ChangedFile>::new();
    for entry in statuses.iter() {
        let Some(path) = entry.path() else {
            continue;
        };

        let normalized = normalize_path(path);
        if normalized.is_empty() {
            continue;
        }

        let status_bits = entry.status();
        let incoming = ChangedFile {
            path: normalized.clone(),
            status: map_status(status_bits),
            staged: is_staged(status_bits),
            untracked: status_bits.is_wt_new() && !status_bits.is_index_new(),
        };

        file_map
            .entry(normalized)
            .and_modify(|existing| {
                existing.staged |= incoming.staged;
                existing.untracked &= incoming.untracked;
                existing.status = merge_file_status(existing.status, incoming.status);
            })
            .or_insert(incoming);
    }

    Ok(file_map.into_values().collect::<Vec<_>>())
}

fn snapshot_fingerprint(
    repo: &Repository,
    root: PathBuf,
    branch_name: String,
    files: &[ChangedFile],
) -> RepoSnapshotFingerprint {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for file in files {
        file.path.hash(&mut hasher);
        file.status.tag().hash(&mut hasher);
        file.staged.hash(&mut hasher);
        file.untracked.hash(&mut hasher);
    }

    RepoSnapshotFingerprint {
        root,
        branch_name,
        head_target: repo
            .head()
            .ok()
            .and_then(|head| head.target())
            .map(|oid| oid.to_string()),
        changed_file_count: files.len(),
        changed_file_signature: hasher.finish(),
    }
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

fn merge_file_status(existing: FileStatus, incoming: FileStatus) -> FileStatus {
    let priority = |status: FileStatus| -> u8 {
        match status {
            FileStatus::Conflicted => 7,
            FileStatus::Deleted => 6,
            FileStatus::Renamed => 5,
            FileStatus::TypeChange => 4,
            FileStatus::Added => 3,
            FileStatus::Untracked => 2,
            FileStatus::Modified => 1,
            FileStatus::Unknown => 0,
        }
    };

    if priority(incoming) >= priority(existing) {
        incoming
    } else {
        existing
    }
}

fn is_staged(status: Status) -> bool {
    status.is_index_new()
        || status.is_index_modified()
        || status.is_index_deleted()
        || status.is_index_renamed()
        || status.is_index_typechange()
}

fn normalize_path(path: &str) -> String {
    path.trim().trim_end_matches('/').to_string()
}

fn walk_repo_tree(
    root: &Path,
    current: &Path,
    repo: &Repository,
    entries: &mut Vec<RepoTreeEntry>,
) -> Result<()> {
    if entries.len() >= MAX_REPO_TREE_ENTRIES {
        return Ok(());
    }

    let mut children = read_dir_sorted(current)?;
    for child in children.drain(..) {
        if entries.len() >= MAX_REPO_TREE_ENTRIES {
            break;
        }

        let name = child.file_name();
        if name.to_string_lossy() == ".git" {
            continue;
        }

        let Ok(file_type) = child.file_type() else {
            continue;
        };

        let child_path = child.path();
        let Ok(relative) = child_path.strip_prefix(root) else {
            continue;
        };
        let relative_path = normalize_path(&relative.to_string_lossy());
        if relative_path.is_empty() {
            continue;
        }

        if file_type.is_dir() {
            let ignored = path_is_ignored(repo, relative_path.as_str(), true);
            entries.push(RepoTreeEntry {
                path: relative_path,
                kind: RepoTreeEntryKind::Directory,
                ignored,
            });
            walk_repo_tree(root, &child_path, repo, entries)?;
            continue;
        }

        if file_type.is_file() {
            let ignored = path_is_ignored(repo, relative_path.as_str(), false);
            entries.push(RepoTreeEntry {
                path: relative_path,
                kind: RepoTreeEntryKind::File,
                ignored,
            });
        }
    }

    Ok(())
}

fn read_dir_sorted(path: &Path) -> Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(path)
        .with_context(|| format!("failed to read directory {}", path.display()))?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        left.file_name()
            .to_string_lossy()
            .cmp(&right.file_name().to_string_lossy())
    });
    Ok(entries)
}

fn path_is_ignored(repo: &Repository, path: &str, is_dir: bool) -> bool {
    if repo.status_should_ignore(Path::new(path)).unwrap_or(false) {
        return true;
    }

    if is_dir {
        let dir_path = format!("{path}/");
        return repo
            .status_should_ignore(Path::new(dir_path.as_str()))
            .unwrap_or(false);
    }

    false
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

fn current_branch_has_upstream(repo: &Repository, branch_name: &str) -> bool {
    if branch_name.is_empty() || branch_name == "unknown" || branch_name.starts_with("detached") {
        return false;
    }

    repo.find_branch(branch_name, BranchType::Local)
        .ok()
        .and_then(|branch| branch.upstream().ok())
        .is_some()
}

fn current_branch_ahead_count(repo: &Repository, branch_name: &str) -> usize {
    if branch_name.is_empty() || branch_name == "unknown" || branch_name.starts_with("detached") {
        return 0;
    }

    let Ok(local_branch) = repo.find_branch(branch_name, BranchType::Local) else {
        return 0;
    };
    let Ok(upstream_branch) = local_branch.upstream() else {
        return 0;
    };

    let local_oid = local_branch.get().target();
    let upstream_oid = upstream_branch.get().target();
    let (Some(local_oid), Some(upstream_oid)) = (local_oid, upstream_oid) else {
        return 0;
    };

    repo.graph_ahead_behind(local_oid, upstream_oid)
        .map(|(ahead, _behind)| ahead)
        .unwrap_or(0)
}

fn list_local_branches(repo: &Repository, current_branch_name: &str) -> Result<Vec<LocalBranch>> {
    let mut branches = Vec::new();

    for branch_result in repo
        .branches(Some(BranchType::Local))
        .context("failed to read local branches")?
    {
        let (branch, _) = branch_result.context("failed to read branch entry")?;
        let Some(name) = branch
            .name()
            .context("failed to read branch name")?
            .map(|name| name.to_string())
        else {
            continue;
        };

        let tip_unix_time = branch
            .get()
            .target()
            .and_then(|oid| repo.find_commit(oid).ok())
            .map(|commit| commit.time().seconds());

        branches.push(LocalBranch {
            is_current: name == current_branch_name,
            name,
            tip_unix_time,
        });
    }

    branches.sort_by(|a, b| {
        b.is_current
            .cmp(&a.is_current)
            .then_with(|| b.tip_unix_time.cmp(&a.tip_unix_time))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(branches)
}

fn last_commit_subject(repo: &Repository) -> Option<String> {
    let commit = repo.head().ok()?.peel_to_commit().ok()?;
    let message = String::from_utf8_lossy(commit.message_bytes())
        .trim_end()
        .to_string();
    if message.is_empty() {
        None
    } else {
        Some(message)
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

fn preferred_remote_name(repo: &Repository) -> Result<String> {
    let remotes = repo.remotes().context("failed to list remotes")?;

    if remotes.iter().flatten().any(|remote| remote == "origin") {
        return Ok("origin".to_string());
    }

    remotes
        .iter()
        .flatten()
        .next()
        .map(|remote| remote.to_string())
        .ok_or_else(|| anyhow!("no git remotes configured"))
}

fn open_repo(repo_root: &Path) -> Result<Repository> {
    Repository::open(repo_root)
        .or_else(|_| Repository::discover(repo_root))
        .context("failed to open git repository")
}

fn default_signature(repo: &Repository) -> Result<Signature<'static>> {
    repo.signature().or_else(|_| {
        Signature::now("hunk", "hunk@local")
            .context("missing git user.name/user.email and failed to construct fallback signature")
    })
}
