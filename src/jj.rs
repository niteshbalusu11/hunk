use std::collections::BTreeSet;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use jj_lib::ref_name::RefName;
use tracing::warn;

mod backend;

use backend::{
    bookmark_remote_sync_state, checkout_existing_bookmark, collect_materialized_diff_entries,
    commit_working_copy_changes, commit_working_copy_selected_paths, conflict_materialize_options,
    create_bookmark_at_working_copy, current_bookmarks_from_context,
    current_commit_id_from_context, discover_repo_root, git_head_branch_name_from_context,
    last_commit_subject_from_context, list_local_branches_from_context,
    load_changed_files_from_context, load_repo_context, load_repo_context_at_root,
    load_tracked_paths_from_context, materialized_entry_matches_path,
    move_bookmark_to_parent_of_working_copy, normalize_path, push_bookmark, render_patch_for_entry,
    repo_line_stats_from_context, sync_bookmark_from_remote, walk_repo_tree,
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

#[derive(Debug, Clone)]
pub struct JjRepo {
    root: PathBuf,
}

pub(super) const MAX_REPO_TREE_ENTRIES: usize = 60_000;
const JJ_STAGE_UNSUPPORTED: &str =
    "JJ does not use a staging index. Stage/unstage actions are unavailable.";
const ACTIVE_BOOKMARK_FILE: &str = "hunk-active-bookmark";

pub fn load_snapshot(cwd: &Path) -> Result<RepoSnapshot> {
    let context = load_repo_context(cwd, true)?;
    let files = load_changed_files_from_context(&context)?;
    let line_stats = repo_line_stats_from_context(&context)?;
    let current_bookmarks = current_bookmarks_from_context(&context)?;
    let active_bookmark = load_active_bookmark_preference(&context.root);
    let git_head_branch = git_head_branch_name_from_context(&context);
    let branch_name = select_snapshot_branch_name(
        &context,
        &current_bookmarks,
        active_bookmark,
        git_head_branch,
    );
    let mut branch_selection = current_bookmarks.clone();
    if branch_selection.is_empty() && branch_name != "detached" {
        branch_selection.insert(branch_name.clone());
    }
    let branches = list_local_branches_from_context(&context, &branch_selection)?;
    let (branch_has_upstream, branch_ahead_count) = if branch_name == "detached" {
        (false, 0)
    } else {
        bookmark_remote_sync_state(&context, branch_name.as_str())
    };
    let last_commit_subject = last_commit_subject_from_context(&context)?;

    Ok(RepoSnapshot {
        root: context.root,
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
    let context = load_repo_context(cwd, true)?;
    let files = load_changed_files_from_context(&context)?;
    let current_bookmarks = current_bookmarks_from_context(&context)?;
    let active_bookmark = load_active_bookmark_preference(&context.root);
    let git_head_branch = git_head_branch_name_from_context(&context);
    let branch_name = select_snapshot_branch_name(
        &context,
        &current_bookmarks,
        active_bookmark,
        git_head_branch,
    );
    let head_target = current_commit_id_from_context(&context)?;
    Ok(snapshot_fingerprint(
        context.root,
        branch_name,
        head_target,
        &files,
    ))
}

pub fn load_patch(repo_root: &Path, file_path: &str, status: FileStatus) -> Result<String> {
    let repo = open_repo_for_patch(repo_root)?;
    load_patch_from_open_repo(&repo, file_path, status)
}

pub fn open_repo_for_patch(repo_root: &Path) -> Result<JjRepo> {
    let root = discover_repo_root(repo_root)?;
    Ok(JjRepo { root })
}

pub fn load_patch_from_open_repo(repo: &JjRepo, file_path: &str, _: FileStatus) -> Result<String> {
    let context = load_repo_context_at_root(&repo.root, true)?;
    let normalized_file = normalize_path(file_path);
    let materialize_options = conflict_materialize_options(&context);

    for entry in collect_materialized_diff_entries(&context)? {
        if !materialized_entry_matches_path(&entry, normalized_file.as_str()) {
            continue;
        }
        let rendered = render_patch_for_entry(entry, &materialize_options)?;
        return Ok(rendered.patch);
    }

    Ok(String::new())
}

pub fn load_patches_for_files(
    repo_root: &Path,
    files: &[ChangedFile],
) -> Result<std::collections::BTreeMap<String, String>> {
    let context = load_repo_context_at_root(repo_root, true)?;
    let materialize_options = conflict_materialize_options(&context);
    let requested_paths = files
        .iter()
        .map(|file| normalize_path(file.path.as_str()))
        .filter(|path| !path.is_empty())
        .collect::<BTreeSet<_>>();

    if requested_paths.is_empty() {
        return Ok(std::collections::BTreeMap::new());
    }

    let mut patch_map = std::collections::BTreeMap::new();
    for entry in collect_materialized_diff_entries(&context)? {
        let source_path = normalize_path(entry.path.source().as_internal_file_string());
        let target_path = normalize_path(entry.path.target().as_internal_file_string());
        let source_matches =
            !source_path.is_empty() && requested_paths.contains(source_path.as_str());
        let target_matches =
            !target_path.is_empty() && requested_paths.contains(target_path.as_str());
        if !source_matches && !target_matches {
            continue;
        }

        let rendered = render_patch_for_entry(entry, &materialize_options)?;
        if target_matches {
            patch_map.insert(target_path.clone(), rendered.patch.clone());
        }
        if source_matches && source_path != target_path {
            patch_map.insert(source_path, rendered.patch);
        }
    }

    for path in requested_paths {
        patch_map.entry(path).or_default();
    }

    Ok(patch_map)
}

pub fn load_repo_tree(repo_root: &Path) -> Result<Vec<RepoTreeEntry>> {
    let context = load_repo_context_at_root(repo_root, true)?;
    let tracked_paths = load_tracked_paths_from_context(&context)?;
    let mut entries = Vec::new();
    walk_repo_tree(
        context.root.as_path(),
        context.root.as_path(),
        &tracked_paths,
        &mut entries,
    )?;
    Ok(entries)
}

pub fn stage_file(_: &Path, _: &str) -> Result<()> {
    Err(anyhow!(JJ_STAGE_UNSUPPORTED))
}

pub fn unstage_file(_: &Path, _: &str) -> Result<()> {
    Err(anyhow!(JJ_STAGE_UNSUPPORTED))
}

pub fn stage_all(_: &Path) -> Result<()> {
    Err(anyhow!(JJ_STAGE_UNSUPPORTED))
}

pub fn unstage_all(_: &Path) -> Result<()> {
    Err(anyhow!(JJ_STAGE_UNSUPPORTED))
}

pub fn commit_staged(repo_root: &Path, message: &str) -> Result<()> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("commit message cannot be empty"));
    }

    let mut context = load_repo_context_at_root(repo_root, true)?;
    if load_changed_files_from_context(&context)?.is_empty() {
        return Err(anyhow!("no changes to commit"));
    }

    commit_working_copy_changes(&mut context, trimmed)?;

    if let Some(active_bookmark) = load_active_bookmark_preference(&context.root) {
        move_bookmark_to_parent_of_working_copy(&mut context, active_bookmark.as_str())?;
    }

    Ok(())
}

pub fn commit_selected_paths(
    repo_root: &Path,
    message: &str,
    selected_paths: &[String],
) -> Result<usize> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("commit message cannot be empty"));
    }
    if selected_paths.is_empty() {
        return Err(anyhow!("no files selected for commit"));
    }

    let mut context = load_repo_context_at_root(repo_root, true)?;
    if load_changed_files_from_context(&context)?.is_empty() {
        return Err(anyhow!("no changes to commit"));
    }

    let committed_count =
        commit_working_copy_selected_paths(&mut context, trimmed, selected_paths)?;

    if let Some(active_bookmark) = load_active_bookmark_preference(&context.root) {
        move_bookmark_to_parent_of_working_copy(&mut context, active_bookmark.as_str())?;
    }

    Ok(committed_count)
}

pub fn checkout_or_create_branch(repo_root: &Path, branch_name: &str) -> Result<()> {
    let branch_name = branch_name.trim();
    if branch_name.is_empty() {
        return Err(anyhow!("branch name cannot be empty"));
    }
    if !is_valid_branch_name(branch_name) {
        return Err(anyhow!("invalid branch name: {branch_name}"));
    }

    let mut context = load_repo_context_at_root(repo_root, true)?;
    let ref_name = RefName::new(branch_name);
    let bookmark_target = context.repo.view().get_local_bookmark(ref_name);
    if bookmark_target.is_present() {
        checkout_existing_bookmark(&mut context, branch_name)?;
    } else {
        create_bookmark_at_working_copy(&mut context, branch_name)?;
    }

    if let Err(err) = persist_active_bookmark_preference(&context.root, branch_name) {
        warn!(
            "failed to persist active bookmark preference for '{}': {err:#}",
            branch_name
        );
    }

    Ok(())
}

pub fn push_current_branch(repo_root: &Path, branch_name: &str, _: bool) -> Result<()> {
    let branch_name = branch_name.trim();
    if branch_name.is_empty() || branch_name == "detached" {
        return Err(anyhow!("cannot push without a bookmark name"));
    }

    let mut context = load_repo_context_at_root(repo_root, false)?;
    push_bookmark(&mut context, branch_name)
}

pub fn sync_current_branch(repo_root: &Path, branch_name: &str) -> Result<()> {
    let branch_name = branch_name.trim();
    if branch_name.is_empty() || branch_name == "detached" {
        return Err(anyhow!("cannot sync without a bookmark name"));
    }

    let mut context = load_repo_context_at_root(repo_root, true)?;
    sync_bookmark_from_remote(&mut context, branch_name)
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

    if name.starts_with('/') || name.ends_with('/') {
        return false;
    }

    if name.starts_with('.') || name.ends_with('.') {
        return false;
    }

    if name.contains("//") || name.contains("..") || name.contains("@{") || name.ends_with(".lock")
    {
        return false;
    }

    if name.chars().any(|ch| {
        ch.is_ascii_control()
            || ch.is_whitespace()
            || matches!(ch, '~' | '^' | ':' | '?' | '*' | '[' | '\\')
    }) {
        return false;
    }

    name.split('/').all(|segment| {
        !segment.is_empty()
            && !segment.starts_with('.')
            && !segment.ends_with('.')
            && segment != "@"
    })
}

fn active_bookmark_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".jj").join(ACTIVE_BOOKMARK_FILE)
}

fn load_active_bookmark_preference(repo_root: &Path) -> Option<String> {
    let path = active_bookmark_path(repo_root);
    let raw = fs::read_to_string(path).ok()?;
    let bookmark = raw.trim();
    if bookmark.is_empty() {
        None
    } else {
        Some(bookmark.to_string())
    }
}

fn persist_active_bookmark_preference(repo_root: &Path, branch_name: &str) -> Result<()> {
    let path = active_bookmark_path(repo_root);
    fs::write(&path, format!("{branch_name}\n")).map_err(|err| {
        anyhow!(
            "failed to write active bookmark preference {}: {err}",
            path.display()
        )
    })
}

fn select_snapshot_branch_name(
    context: &backend::RepoContext,
    current_bookmarks: &BTreeSet<String>,
    preferred: Option<String>,
    git_head_branch: Option<String>,
) -> String {
    if let Some(preferred) = preferred
        && (current_bookmarks.contains(preferred.as_str())
            || context
                .repo
                .view()
                .get_local_bookmark(RefName::new(preferred.as_str()))
                .is_present())
    {
        return preferred;
    }

    if let Some(git_head_branch) = git_head_branch
        && (current_bookmarks.contains(git_head_branch.as_str())
            || context
                .repo
                .view()
                .get_local_bookmark(RefName::new(git_head_branch.as_str()))
                .is_present())
    {
        return git_head_branch;
    }

    current_bookmarks
        .iter()
        .next()
        .cloned()
        .unwrap_or_else(|| "detached".to_string())
}

fn snapshot_fingerprint(
    root: PathBuf,
    branch_name: String,
    head_target: Option<String>,
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
        head_target,
        changed_file_count: files.len(),
        changed_file_signature: hasher.finish(),
    }
}
