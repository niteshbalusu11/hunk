pub(super) fn commit_working_copy_changes(context: &mut RepoContext, message: &str) -> Result<()> {
    let workspace_name = context.workspace.workspace_name().to_owned();
    let wc_commit = current_wc_commit(context)?;

    let mut tx = context.repo.start_transaction();
    let committed = tx
        .repo_mut()
        .rewrite_commit(&wc_commit)
        .set_description(message)
        .write()
        .context("failed to create committed revision")?;
    let new_wc = tx
        .repo_mut()
        .new_commit(vec![committed.id().clone()], committed.tree())
        .write()
        .context("failed to create next working-copy revision")?;
    tx.repo_mut()
        .set_wc_commit(workspace_name.clone(), new_wc.id().clone())
        .context("failed to update working-copy commit")?;
    tx.repo_mut()
        .rebase_descendants()
        .context("failed to rebase descendants after commit")?;

    let repo = tx
        .commit(format!("commit: {message}"))
        .context("failed to finalize commit")?;
    persist_working_copy_state(context, repo, "after commit")
}

pub(super) fn commit_working_copy_selected_paths(
    context: &mut RepoContext,
    message: &str,
    selected_paths: &[String],
) -> Result<usize> {
    if selected_paths.is_empty() {
        return Err(anyhow!("no files selected for commit"));
    }

    let workspace_name = context.workspace.workspace_name().to_owned();
    let wc_commit = current_wc_commit(context)?;
    let base_tree = wc_commit.parent_tree(context.repo.as_ref())?;
    let wc_tree = wc_commit.tree();

    let mut normalized_paths = BTreeSet::new();
    for path in selected_paths {
        let normalized = normalize_path(path);
        if normalized.is_empty() {
            continue;
        }
        normalized_paths.insert(normalized);
    }
    if normalized_paths.is_empty() {
        return Err(anyhow!("no valid files selected for commit"));
    }

    let mut repo_paths = Vec::with_capacity(normalized_paths.len());
    for normalized in &normalized_paths {
        let repo_path = RepoPathBuf::from_relative_path(Path::new(normalized.as_str()))
            .with_context(|| format!("invalid repository path '{normalized}'"))?;
        repo_paths.push(repo_path);
    }

    let matcher = jj_lib::matchers::FilesMatcher::new(repo_paths.iter());
    let selected_tree = block_on(restore_tree(
        &wc_tree,
        &base_tree,
        "working copy".to_string(),
        "parent".to_string(),
        &matcher,
    ))
    .context("failed to select files for commit")?;

    if selected_tree.tree_ids_and_labels() == base_tree.tree_ids_and_labels() {
        return Err(anyhow!("selected files have no changes to commit"));
    }

    let mut tx = context.repo.start_transaction();
    let committed = tx
        .repo_mut()
        .rewrite_commit(&wc_commit)
        .set_description(message)
        .set_tree(selected_tree)
        .write()
        .context("failed to create commit for selected files")?;
    let new_wc = tx
        .repo_mut()
        .new_commit(vec![committed.id().clone()], wc_tree)
        .write()
        .context("failed to create next working-copy revision after partial commit")?;
    tx.repo_mut()
        .set_wc_commit(workspace_name.clone(), new_wc.id().clone())
        .context("failed to update working-copy commit after partial commit")?;
    tx.repo_mut()
        .rebase_descendants()
        .context("failed to rebase descendants after partial commit")?;

    let repo = tx
        .commit(format!("commit selected paths: {message}"))
        .context("failed to finalize partial commit")?;
    persist_working_copy_state(context, repo, "after partial commit")?;
    Ok(repo_paths.len())
}

pub(super) fn move_bookmark_to_parent_of_working_copy(
    context: &mut RepoContext,
    branch_name: &str,
) -> Result<bool> {
    let bookmark_target = context
        .repo
        .view()
        .get_local_bookmark(RefName::new(branch_name));
    if !bookmark_target.is_present() {
        return Ok(false);
    }

    let wc_commit = current_wc_commit(context)?;
    let Some(parent_id) = wc_commit.parent_ids().first().cloned() else {
        return Ok(false);
    };

    let mut tx = context.repo.start_transaction();
    tx.repo_mut()
        .set_local_bookmark_target(RefName::new(branch_name), RefTarget::normal(parent_id));
    let repo = tx
        .commit(format!("move bookmark {branch_name} to committed revision"))
        .with_context(|| format!("failed to advance bookmark '{branch_name}'"))?;
    persist_working_copy_state(context, repo, "after moving bookmark")?;
    Ok(true)
}

pub(super) fn checkout_existing_bookmark(
    context: &mut RepoContext,
    branch_name: &str,
) -> Result<()> {
    let workspace_name = context.workspace.workspace_name().to_owned();
    let bookmark_target = context
        .repo
        .view()
        .get_local_bookmark(RefName::new(branch_name));
    let commit_id = bookmark_target
        .as_normal()
        .cloned()
        .ok_or_else(|| anyhow!("bookmark '{branch_name}' is conflicted or has no target"))?;
    let target_commit = context
        .repo
        .store()
        .get_commit(&commit_id)
        .with_context(|| format!("failed to load bookmark target for '{branch_name}'"))?;

    let mut locked_workspace = context
        .workspace
        .start_working_copy_mutation()
        .context("failed to lock working copy for bookmark checkout")?;

    let mut tx = context.repo.start_transaction();
    let new_wc = tx
        .repo_mut()
        .new_commit(vec![target_commit.id().clone()], target_commit.tree())
        .write()
        .with_context(|| format!("failed to create working-copy commit for '{branch_name}'"))?;
    tx.repo_mut()
        .set_wc_commit(workspace_name.clone(), new_wc.id().clone())
        .with_context(|| format!("failed to set working-copy commit for '{branch_name}'"))?;
    tx.repo_mut()
        .rebase_descendants()
        .context("failed to rebase descendants after bookmark checkout")?;
    let repo = tx
        .commit(format!("checkout bookmark {branch_name}"))
        .context("failed to finalize bookmark checkout")?;

    let new_wc_commit = current_wc_commit_with_repo(repo.as_ref(), &workspace_name)?;
    block_on(locked_workspace.locked_wc().check_out(&new_wc_commit))
        .context("failed to update working-copy files for bookmark checkout")?;
    locked_workspace
        .finish(repo.op_id().clone())
        .context("failed to persist working-copy state after bookmark checkout")?;

    context.repo = repo;
    Ok(())
}

pub(super) fn create_bookmark_at_working_copy(
    context: &mut RepoContext,
    branch_name: &str,
) -> Result<()> {
    let wc_commit = current_wc_commit(context)?;

    let mut tx = context.repo.start_transaction();
    tx.repo_mut().set_local_bookmark_target(
        RefName::new(branch_name),
        RefTarget::normal(wc_commit.id().clone()),
    );
    let repo = tx
        .commit(format!("create bookmark {branch_name}"))
        .context("failed to create bookmark")?;
    persist_working_copy_state(context, repo, "after bookmark creation")
}

pub(super) fn push_bookmark(context: &mut RepoContext, branch_name: &str) -> Result<()> {
    ensure_bookmark_tip_identity(context, branch_name)?;
    let remote_name = resolve_push_remote_name(context, branch_name)?;
    let remote = RemoteName::new(remote_name.as_str());
    ensure_remote_bookmark_is_tracked(context, branch_name, remote, remote_name.as_str())?;

    let maybe_targets = context
        .repo
        .view()
        .local_remote_bookmarks(remote)
        .find(|(name, _)| name.as_str() == branch_name)
        .map(|(_, targets)| targets);

    let targets = maybe_targets
        .ok_or_else(|| anyhow!("bookmark '{branch_name}' does not exist in this repository"))?;

    let push_action = classify_bookmark_push_action(targets);
    let update = match push_action {
        BookmarkPushAction::Update(update) => update,
        BookmarkPushAction::AlreadyMatches => return Ok(()),
        BookmarkPushAction::LocalConflicted => {
            return Err(anyhow!(
                "bookmark '{branch_name}' is conflicted locally and cannot be pushed"
            ));
        }
        BookmarkPushAction::RemoteConflicted => {
            return Err(anyhow!(
                "remote tracking state for bookmark '{branch_name}' is conflicted"
            ));
        }
        BookmarkPushAction::RemoteUntracked => {
            return Err(anyhow!(
                "bookmark '{branch_name}' has an untracked remote ref after tracking attempt"
            ));
        }
    };

    let push_targets = git::GitBranchPushTargets {
        branch_updates: vec![(RefNameBuf::from(branch_name), update)],
    };
    let subprocess_options = GitSubprocessOptions::from_settings(&context.settings)
        .context("failed to resolve git subprocess settings")?;

    let mut tx = context.repo.start_transaction();
    let mut callback = NoopGitSubprocessCallback;
    git::push_branches(
        tx.repo_mut(),
        subprocess_options,
        remote,
        &push_targets,
        &mut callback,
    )
    .with_context(|| {
        format!("failed to push bookmark '{branch_name}' to remote '{remote_name}'")
    })?;

    let repo = tx
        .commit(format!("push bookmark {branch_name}"))
        .context("failed to finalize push operation")?;
    persist_working_copy_state(context, repo, "after push")
}

fn ensure_bookmark_tip_identity(context: &mut RepoContext, branch_name: &str) -> Result<()> {
    let target = context
        .repo
        .view()
        .get_local_bookmark(RefName::new(branch_name))
        .as_normal()
        .cloned();
    let Some(commit_id) = target else {
        return Ok(());
    };

    let commit = context
        .repo
        .store()
        .get_commit(&commit_id)
        .with_context(|| format!("failed to load bookmark target for '{branch_name}'"))?;
    if !commit_has_missing_identity(&commit) {
        return Ok(());
    }

    let signature = context.settings.signature();
    if signature.name.trim().is_empty() || signature.email.trim().is_empty() {
        return Err(anyhow!(
            "bookmark '{branch_name}' has a commit with missing author/committer metadata. \
Set user.name/user.email (JJ or Git config) and try again."
        ));
    }

    let mut tx = context.repo.start_transaction();
    let rewritten = tx
        .repo_mut()
        .rewrite_commit(&commit)
        .set_author(signature.clone())
        .set_committer(signature)
        .write()
        .with_context(|| format!("failed to rewrite bookmark commit metadata for '{branch_name}'"))?;
    tx.repo_mut().set_local_bookmark_target(
        RefName::new(branch_name),
        RefTarget::normal(rewritten.id().clone()),
    );
    tx.repo_mut()
        .rebase_descendants()
        .context("failed to rebase descendants after rewriting bookmark metadata")?;

    let repo = tx
        .commit(format!("update metadata for bookmark {branch_name}"))
        .context("failed to finalize bookmark metadata update")?;
    persist_working_copy_state(context, repo, "after rewriting bookmark metadata")
}

fn commit_has_missing_identity(commit: &Commit) -> bool {
    signature_has_missing_identity(commit.author()) || signature_has_missing_identity(commit.committer())
}

fn signature_has_missing_identity(signature: &jj_lib::backend::Signature) -> bool {
    signature.name.trim().is_empty() || signature.email.trim().is_empty()
}

pub(super) fn sync_bookmark_from_remote(
    context: &mut RepoContext,
    branch_name: &str,
) -> Result<()> {
    if !load_changed_files_from_context(context)?.is_empty() {
        return Err(anyhow!(
            "cannot sync while the working copy has uncommitted changes"
        ));
    }

    let remote_name = resolve_push_remote_name(context, branch_name)?;
    let remote = RemoteName::new(remote_name.as_str());
    let subprocess_options = GitSubprocessOptions::from_settings(&context.settings)
        .context("failed to resolve git subprocess settings")?;
    let import_options = git_import_options_from_settings(&context.settings)?;
    let fetch_refspecs = git::expand_fetch_refspecs(
        remote,
        git::GitFetchRefExpression {
            bookmark: StringExpression::exact(branch_name),
            tag: StringExpression::none(),
        },
    )
    .with_context(|| format!("failed to prepare fetch refspecs for bookmark '{branch_name}'"))?;

    let mut tx = context.repo.start_transaction();
    {
        let mut fetcher = git::GitFetch::new(tx.repo_mut(), subprocess_options, &import_options)
            .context("failed to initialize Git fetch operation")?;
        let mut callback = NoopGitSubprocessCallback;
        fetcher
            .fetch(remote, fetch_refspecs, &mut callback, None, None)
            .with_context(|| {
                format!("failed to fetch bookmark '{branch_name}' from remote '{remote_name}'")
            })?;
        fetcher
            .import_refs()
            .context("failed to import fetched refs into JJ view")?;
    }

    let repo = tx
        .commit(format!("sync bookmark {branch_name} from {remote_name}"))
        .context("failed to finalize sync operation")?;
    persist_working_copy_state(context, repo, "after sync")?;

    ensure_remote_bookmark_is_tracked(context, branch_name, remote, remote_name.as_str())?;
    checkout_existing_bookmark(context, branch_name)
        .with_context(|| format!("failed to refresh working copy for '{branch_name}'"))?;

    Ok(())
}

fn git_import_options_from_settings(settings: &UserSettings) -> Result<git::GitImportOptions> {
    let auto_local_bookmark = settings
        .get_bool("git.auto-local-bookmark")
        .context("failed to read git.auto-local-bookmark setting")?;
    let abandon_unreachable_commits = settings
        .get_bool("git.abandon-unreachable-commits")
        .context("failed to read git.abandon-unreachable-commits setting")?;

    Ok(git::GitImportOptions {
        auto_local_bookmark,
        abandon_unreachable_commits,
        remote_auto_track_bookmarks: HashMap::new(),
    })
}

fn ensure_remote_bookmark_is_tracked(
    context: &mut RepoContext,
    branch_name: &str,
    remote: &RemoteName,
    remote_name: &str,
) -> Result<()> {
    let Some((_, targets)) = context
        .repo
        .view()
        .local_remote_bookmarks(remote)
        .find(|(name, _)| name.as_str() == branch_name)
    else {
        return Ok(());
    };
    if targets.remote_ref.is_tracked() {
        return Ok(());
    }

    let symbol = RefName::new(branch_name).to_remote_symbol(remote);
    let mut tx = context.repo.start_transaction();
    tx.repo_mut()
        .track_remote_bookmark(symbol)
        .with_context(|| {
            format!(
                "failed to track remote bookmark '{}@{}' before operation",
                branch_name, remote_name
            )
        })?;

    let repo = tx
        .commit(format!("track remote bookmark {branch_name}@{remote_name}"))
        .context("failed to finalize remote bookmark tracking operation")?;
    persist_working_copy_state(context, repo, "after tracking remote bookmark")
}

fn resolve_push_remote_name(context: &RepoContext, branch_name: &str) -> Result<String> {
    let view = context.repo.view();
    let mut first_present_remote = None;

    for (remote, _) in view.remote_views() {
        if remote == REMOTE_NAME_FOR_LOCAL_GIT_REPO {
            continue;
        }

        let Some((_, targets)) = view
            .local_remote_bookmarks(remote)
            .find(|(name, _)| name.as_str() == branch_name)
        else {
            continue;
        };
        if !targets.remote_ref.is_present() {
            continue;
        }

        if targets.remote_ref.is_tracked() {
            return Ok(remote.as_str().to_string());
        }
        if first_present_remote.is_none() {
            first_present_remote = Some(remote.as_str().to_string());
        }
    }

    if let Some(remote_name) = first_present_remote {
        return Ok(remote_name);
    }

    if view
        .remote_views()
        .any(|(remote, _)| remote.as_str() == "origin")
    {
        return Ok("origin".to_string());
    }

    if let Some((remote, _)) = view
        .remote_views()
        .find(|(remote, _)| *remote != REMOTE_NAME_FOR_LOCAL_GIT_REPO)
    {
        return Ok(remote.as_str().to_string());
    }

    Err(anyhow!("no Git remote configured for push"))
}
