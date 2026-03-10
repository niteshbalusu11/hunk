fn load_snapshot_internal(path: &Path) -> Result<RepoSnapshot> {
    let repo = open_repo(path)?;
    let seed = load_snapshot_seed(&repo, true, SnapshotLoadMode::RefreshWorkingCopy)?;
    let files = snapshot_files(seed.entries.values());
    let line_stats = sum_line_stats(seed.entries.values().map(|entry| entry.line_stats));
    let working_copy_commit_id =
        synthetic_working_copy_id(seed.head_commit_id.as_deref(), seed.entries.values());
    Ok(RepoSnapshot {
        root: seed.root,
        working_copy_commit_id,
        branch_name: seed.branch_name,
        branch_has_upstream: seed.branch_has_upstream,
        branch_ahead_count: seed.branch_ahead_count,
        branch_behind_count: seed.branch_behind_count,
        branches: seed.branches,
        files,
        line_stats,
        last_commit_subject: seed.last_commit_subject,
    })
}

fn load_workflow_snapshot_internal(
    path: &Path,
    mode: SnapshotLoadMode,
) -> Result<(RepoSnapshotFingerprint, WorkflowSnapshot)> {
    let repo = open_repo(path)?;
    let seed = load_snapshot_seed(&repo, false, mode)?;
    let fingerprint = snapshot_fingerprint(
        seed.root.clone(),
        seed.head_ref_name.clone(),
        seed.head_commit_id.clone(),
        seed.branch_has_upstream,
        seed.branch_ahead_count,
        seed.branch_behind_count,
        &seed.entries,
    );
    let working_copy_commit_id =
        synthetic_working_copy_id(seed.head_commit_id.as_deref(), seed.entries.values());
    let workflow = WorkflowSnapshot {
        root: seed.root,
        working_copy_commit_id,
        branch_name: seed.branch_name,
        branch_has_upstream: seed.branch_has_upstream,
        branch_ahead_count: seed.branch_ahead_count,
        branch_behind_count: seed.branch_behind_count,
        branches: seed.branches,
        files: snapshot_files(seed.entries.values()),
        last_commit_subject: seed.last_commit_subject,
    };
    Ok((fingerprint, workflow))
}

fn load_workflow_snapshot_if_changed_internal(
    path: &Path,
    previous_fingerprint: Option<&RepoSnapshotFingerprint>,
    mode: SnapshotLoadMode,
) -> Result<(RepoSnapshotFingerprint, Option<WorkflowSnapshot>)> {
    let (fingerprint, workflow) = load_workflow_snapshot_internal(path, mode)?;
    if previous_fingerprint == Some(&fingerprint) {
        return Ok((fingerprint, None));
    }
    Ok((fingerprint, Some(workflow)))
}

fn load_snapshot_fingerprint_internal(
    path: &Path,
    mode: SnapshotLoadMode,
) -> Result<RepoSnapshotFingerprint> {
    let repo = open_repo(path)?;
    let seed = load_snapshot_seed(&repo, false, mode)?;
    Ok(snapshot_fingerprint(
        seed.root,
        seed.head_ref_name,
        seed.head_commit_id,
        seed.branch_has_upstream,
        seed.branch_ahead_count,
        seed.branch_behind_count,
        &seed.entries,
    ))
}

fn load_snapshot_seed(
    repo: &GitRepo,
    include_line_stats: bool,
    mode: SnapshotLoadMode,
) -> Result<SnapshotSeed> {
    let head_ref_name = repo
        .repository()
        .head_name()
        .context("failed to resolve Git HEAD name")?
        .map(|name| name.to_string());
    let head_commit_id = repo.repository().head_id().ok().map(|id| id.to_string());
    let branch_name = branch_name_from_head_ref(head_ref_name.as_deref());
    let (branch_has_upstream, branch_ahead_count, branch_behind_count) =
        current_branch_tracking(repo.repository(), head_ref_name.as_deref())?;
    let branch_workspace_occupancy =
        list_branch_workspace_occupancy(repo.root()).unwrap_or_default();
    let branches = list_local_branches(
        repo.repository(),
        head_ref_name.as_deref(),
        &branch_workspace_occupancy,
    )?;
    let entries = match mode {
        SnapshotLoadMode::ReadOnlyLight => {
            collect_workspace_diff_entries_light(repo.repository(), repo.root(), None)?
        }
        SnapshotLoadMode::RefreshWorkingCopy => collect_workspace_diff_entries_full(
            repo.repository(),
            repo.root(),
            None,
            include_line_stats,
        )?,
    };
    let last_commit_subject = last_commit_subject(repo.repository())?;

    Ok(SnapshotSeed {
        root: repo.root().to_path_buf(),
        head_ref_name,
        head_commit_id,
        branch_name,
        branch_has_upstream,
        branch_ahead_count,
        branch_behind_count,
        branches,
        entries,
        last_commit_subject,
    })
}

fn snapshot_files<'a>(
    entries: impl IntoIterator<Item = &'a WorkspaceDiffEntry>,
) -> Vec<ChangedFile> {
    entries
        .into_iter()
        .map(|entry| entry.file.clone())
        .collect()
}

fn load_patches_for_files_from_repo(
    repo: &GitRepo,
    files: &[ChangedFile],
) -> Result<BTreeMap<String, String>> {
    let requested_paths = files
        .iter()
        .map(|file| normalize_path(file.path.as_str()))
        .filter(|path| !path.is_empty())
        .collect::<BTreeSet<_>>();

    if requested_paths.is_empty() {
        return Ok(BTreeMap::new());
    }

    render_patches_for_paths(repo, &requested_paths)
}

fn render_patch_for_path(repo: &GitRepo, file_path: &str) -> Result<String> {
    let mut requested_paths = BTreeSet::new();
    requested_paths.insert(normalize_path(file_path));
    if requested_paths.first().is_some_and(|path| path.is_empty()) {
        return Ok(String::new());
    }

    let patch_map = render_patches_for_paths(repo, &requested_paths)?;
    Ok(patch_map.into_values().next().unwrap_or_default())
}

fn render_patches_for_paths(
    repo: &GitRepo,
    requested_paths: &BTreeSet<String>,
) -> Result<BTreeMap<String, String>> {
    let resolved = resolve_workspace_files(repo.repository(), repo.root(), Some(requested_paths))?;
    let mut patch_map = requested_paths
        .iter()
        .cloned()
        .map(|path| (path, String::new()))
        .collect::<BTreeMap<_, _>>();

    for file in resolved {
        patch_map.insert(file.path.clone(), render_patch_for_resolved_file(&file)?);
    }

    Ok(patch_map)
}

fn load_repo_file_line_stats(path: &Path) -> Result<BTreeMap<String, LineStats>> {
    let repo = open_repo(path)?;
    let entries = collect_workspace_diff_entries_full(repo.repository(), repo.root(), None, true)?;
    Ok(entries
        .into_iter()
        .map(|(path, entry)| (path, entry.line_stats))
        .collect())
}
