use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use futures::executor::{block_on, block_on_stream};
use jj_lib::commit::Commit;
use jj_lib::config::{ConfigLayer, ConfigSource, StackedConfig};
use jj_lib::conflicts::{
    ConflictMarkerStyle, ConflictMaterializeOptions, MaterializedTreeDiffEntry,
    materialized_diff_stream,
};
use jj_lib::copies::CopyRecords;
use jj_lib::diff_presentation::LineCompareMode;
use jj_lib::diff_presentation::unified::{
    DiffLineType, UnifiedDiffHunk, git_diff_part, unified_diff_hunks,
};
use jj_lib::git::{
    self, GitProgress, GitSidebandLineTerminator, GitSubprocessCallback, GitSubprocessOptions,
    REMOTE_NAME_FOR_LOCAL_GIT_REPO,
};
use jj_lib::matchers::{EverythingMatcher, NothingMatcher};
use jj_lib::merge::{Diff, MergedTreeValue};
use jj_lib::object_id::ObjectId as _;
use jj_lib::op_store::RefTarget;
use jj_lib::ref_name::{RefName, RefNameBuf, RemoteName, WorkspaceName};
use jj_lib::refs::{BookmarkPushAction, classify_bookmark_push_action};
use jj_lib::repo::{ReadonlyRepo, Repo as _, StoreFactories};
use jj_lib::repo_path::RepoPathBuf;
use jj_lib::rewrite::restore_tree;
use jj_lib::settings::UserSettings;
use jj_lib::str_util::StringExpression;
use jj_lib::working_copy::SnapshotOptions;
use jj_lib::workspace::{Workspace, default_working_copy_factories};

use super::*;

pub(super) struct RepoContext {
    pub(super) root: PathBuf,
    pub(super) settings: UserSettings,
    pub(super) workspace: Workspace,
    pub(super) repo: Arc<ReadonlyRepo>,
}

pub(super) struct RenderedPatch {
    pub(super) patch: String,
    pub(super) line_stats: LineStats,
}

struct NoopGitSubprocessCallback;

impl GitSubprocessCallback for NoopGitSubprocessCallback {
    fn needs_progress(&self) -> bool {
        false
    }

    fn progress(&mut self, _: &GitProgress) -> io::Result<()> {
        Ok(())
    }

    fn local_sideband(&mut self, _: &[u8], _: Option<GitSidebandLineTerminator>) -> io::Result<()> {
        Ok(())
    }

    fn remote_sideband(
        &mut self,
        _: &[u8],
        _: Option<GitSidebandLineTerminator>,
    ) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn load_repo_context(cwd: &Path, refresh_snapshot: bool) -> Result<RepoContext> {
    let root = discover_repo_root(cwd)?;
    load_repo_context_at_root(&root, refresh_snapshot)
}

pub(super) fn load_repo_context_at_root(
    repo_root: &Path,
    refresh_snapshot: bool,
) -> Result<RepoContext> {
    let root = discover_repo_root(repo_root)?;
    let settings = load_user_settings(Some(&root))?;
    let store_factories = StoreFactories::default();
    let working_copy_factories = default_working_copy_factories();

    let workspace = Workspace::load(&settings, &root, &store_factories, &working_copy_factories)
        .with_context(|| format!("failed to load jj workspace at {}", root.display()))?;
    let repo = workspace
        .repo_loader()
        .load_at_head()
        .context("failed to load jj repository")?;

    let mut context = RepoContext {
        root,
        settings,
        workspace,
        repo,
    };
    if refresh_snapshot {
        refresh_working_copy_snapshot(&mut context)?;
    }
    Ok(context)
}

fn load_user_settings(workspace_root: Option<&Path>) -> Result<UserSettings> {
    let mut config = StackedConfig::with_defaults();

    if let Some(home_dir) = dirs::home_dir() {
        load_config_if_exists(
            &mut config,
            ConfigSource::User,
            home_dir.join(".jjconfig.toml"),
        )?;
    }

    if let Some(config_dir) = dirs::config_dir() {
        load_config_if_exists(
            &mut config,
            ConfigSource::User,
            config_dir.join("jj").join("config.toml"),
        )?;
    }

    if let Some(root) = workspace_root {
        load_config_if_exists(
            &mut config,
            ConfigSource::Repo,
            root.join(".jj").join("repo").join("config.toml"),
        )?;
        load_config_if_exists(
            &mut config,
            ConfigSource::Workspace,
            root.join(".jj").join("config.toml"),
        )?;
        add_git_signing_fallback_config(&mut config, root)?;
    }

    UserSettings::from_config(config).context("failed to load jj settings")
}

fn add_git_signing_fallback_config(
    config: &mut StackedConfig,
    workspace_root: &Path,
) -> Result<()> {
    if has_explicit_signing_backend(config) {
        return Ok(());
    }

    let Some(git_signing) = read_git_signing_config(workspace_root) else {
        return Ok(());
    };
    let commit_gpgsign = git_signing.commit_gpgsign.unwrap_or(false);
    let git_signing_key = git_signing.signing_key.clone();

    if !commit_gpgsign && git_signing_key.is_none() {
        return Ok(());
    }

    let signing_backend = match git_signing.gpg_format.as_deref() {
        Some("ssh") => "ssh",
        Some("x509") => "gpgsm",
        _ => "gpg",
    };

    let mut fallback_layer = ConfigLayer::empty(ConfigSource::EnvBase);
    fallback_layer
        .set_value("signing.backend", signing_backend)
        .context("failed to apply Git signing backend fallback")?;
    if commit_gpgsign {
        fallback_layer
            .set_value("signing.behavior", "own")
            .context("failed to apply Git commit signing behavior fallback")?;
    }
    if let Some(signing_key) = git_signing_key {
        fallback_layer
            .set_value("signing.key", signing_key)
            .context("failed to apply Git signing key fallback")?;
    }
    if let Some(program) = git_signing.program_for_backend(signing_backend) {
        let key = match signing_backend {
            "ssh" => "signing.backends.ssh.program",
            "gpgsm" => "signing.backends.gpgsm.program",
            _ => "signing.backends.gpg.program",
        };
        fallback_layer
            .set_value(key, program)
            .context("failed to apply Git signing program fallback")?;
    }

    config.add_layer(fallback_layer);
    Ok(())
}

fn has_explicit_signing_backend(config: &StackedConfig) -> bool {
    config.layers().iter().any(|layer| {
        layer.source != ConfigSource::Default
            && matches!(layer.look_up_item("signing.backend"), Ok(Some(_)))
    })
}

#[derive(Default, Clone)]
struct GitSigningConfig {
    commit_gpgsign: Option<bool>,
    signing_key: Option<String>,
    gpg_format: Option<String>,
    gpg_program: Option<String>,
    gpg_ssh_program: Option<String>,
    gpg_x509_program: Option<String>,
}

impl GitSigningConfig {
    fn program_for_backend(&self, backend: &str) -> Option<String> {
        match backend {
            "ssh" => self.gpg_ssh_program.clone(),
            "gpgsm" => self.gpg_x509_program.clone(),
            _ => self.gpg_program.clone(),
        }
    }
}

fn read_git_signing_config(workspace_root: &Path) -> Option<GitSigningConfig> {
    let mut merged = GitSigningConfig::default();
    let mut saw_any = false;

    for path in git_signing_config_paths(workspace_root) {
        if merge_git_signing_config_file(&mut merged, path.as_path()) {
            saw_any = true;
        }
    }

    if saw_any { Some(merged) } else { None }
}

fn git_signing_config_paths(workspace_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(home_dir) = dirs::home_dir() {
        paths.push(home_dir.join(".gitconfig"));
    }

    let xdg_config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|path| path.join(".config")));
    if let Some(config_home) = xdg_config_home {
        paths.push(config_home.join("git").join("config"));
    }

    if let Some(path) = workspace_git_config_path(workspace_root)
        && !paths.contains(&path)
    {
        paths.push(path);
    }
    if let Some(path) = git_target_config_path(workspace_root)
        && !paths.contains(&path)
    {
        paths.push(path);
    }

    paths
}

fn workspace_git_config_path(workspace_root: &Path) -> Option<PathBuf> {
    let dot_git = workspace_root.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git.join("config"));
    }
    if dot_git.is_file() {
        let git_dir = fs::read_to_string(&dot_git).ok().and_then(|contents| {
            contents
                .lines()
                .find_map(|line| line.trim().strip_prefix("gitdir:"))
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .map(PathBuf::from)
        })?;
        let git_dir = if git_dir.is_absolute() {
            git_dir
        } else {
            workspace_root.join(git_dir)
        };
        return Some(git_dir.join("config"));
    }

    None
}

fn git_target_config_path(workspace_root: &Path) -> Option<PathBuf> {
    let store_root = workspace_root.join(".jj").join("repo").join("store");
    let git_target_path = store_root.join("git_target");
    let raw_target = fs::read_to_string(&git_target_path).ok()?;
    let target = raw_target.trim();
    if target.is_empty() {
        return None;
    }

    let git_repo_path = {
        let target_path = PathBuf::from(target);
        if target_path.is_absolute() {
            target_path
        } else {
            store_root.join(target_path)
        }
    };
    Some(git_repo_path.join("config"))
}

fn merge_git_signing_config_file(config: &mut GitSigningConfig, path: &Path) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return false;
    };

    let mut saw_any = false;
    let mut section = String::new();
    let mut subsection = None::<String>;

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let header = &line[1..line.len() - 1];
            let (name, sub) = parse_git_config_section_header(header);
            section = name;
            subsection = sub;
            continue;
        }

        let (key, value) = if let Some((key, value)) = line.split_once('=') {
            (
                key.trim().to_ascii_lowercase(),
                normalize_git_config_value(value),
            )
        } else {
            (line.to_ascii_lowercase(), "true".to_string())
        };
        if key.is_empty() {
            continue;
        }

        match (section.as_str(), subsection.as_deref(), key.as_str()) {
            ("commit", None, "gpgsign") => {
                if let Some(value) = parse_git_config_bool(value.as_str()) {
                    config.commit_gpgsign = Some(value);
                    saw_any = true;
                }
            }
            ("user", None, "signingkey") => {
                if !value.is_empty() {
                    config.signing_key = Some(value);
                    saw_any = true;
                }
            }
            ("gpg", None, "format") => {
                if !value.is_empty() {
                    config.gpg_format = Some(value.to_ascii_lowercase());
                    saw_any = true;
                }
            }
            ("gpg", None, "program") => {
                if !value.is_empty() {
                    config.gpg_program = Some(value);
                    saw_any = true;
                }
            }
            ("gpg", Some("ssh"), "program") => {
                if !value.is_empty() {
                    config.gpg_ssh_program = Some(value);
                    saw_any = true;
                }
            }
            ("gpg", Some("x509"), "program") => {
                if !value.is_empty() {
                    config.gpg_x509_program = Some(value);
                    saw_any = true;
                }
            }
            _ => {}
        }
    }

    saw_any
}

fn parse_git_config_section_header(header: &str) -> (String, Option<String>) {
    let mut parts = header.splitn(2, char::is_whitespace);
    let section = parts.next().unwrap_or_default().trim().to_ascii_lowercase();
    let subsection = parts
        .next()
        .map(str::trim)
        .map(normalize_git_config_value)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    (section, subsection)
}

fn normalize_git_config_value(value: &str) -> String {
    let value = value.trim();
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        value[1..value.len() - 1].trim().to_string()
    } else {
        value.to_string()
    }
}

fn parse_git_config_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn load_config_if_exists(
    config: &mut StackedConfig,
    source: ConfigSource,
    path: PathBuf,
) -> Result<()> {
    if path.is_file() {
        config
            .load_file(source, path.clone())
            .with_context(|| format!("failed to load jj config {}", path.display()))?;
    }
    Ok(())
}

fn refresh_working_copy_snapshot(context: &mut RepoContext) -> Result<()> {
    import_git_head_for_snapshot(context)?;
    ensure_local_bookmark_for_git_head(context)?;

    let workspace_name = context.workspace.workspace_name().to_owned();
    let wc_commit =
        current_wc_commit_with_repo(context.repo.as_ref(), context.workspace.workspace_name())?;
    let old_tree = wc_commit.tree();

    let mut locked_workspace = context
        .workspace
        .start_working_copy_mutation()
        .context("failed to lock jj working copy")?;

    let snapshot_options = SnapshotOptions {
        base_ignores: jj_lib::gitignore::GitIgnoreFile::empty(),
        progress: None,
        start_tracking_matcher: &EverythingMatcher,
        force_tracking_matcher: &NothingMatcher,
        max_new_file_size: u64::MAX,
    };

    let (new_tree, _) = block_on(locked_workspace.locked_wc().snapshot(&snapshot_options))
        .context("failed to snapshot jj working copy")?;

    let mut repo = context.repo.clone();
    if new_tree.tree_ids_and_labels() != old_tree.tree_ids_and_labels() {
        let mut tx = repo.start_transaction();
        let rewritten_wc = tx
            .repo_mut()
            .rewrite_commit(&wc_commit)
            .set_tree(new_tree)
            .write()
            .context("failed to record working-copy snapshot")?;
        tx.repo_mut()
            .set_wc_commit(workspace_name.clone(), rewritten_wc.id().clone())
            .context("failed to update working-copy commit")?;
        tx.repo_mut()
            .rebase_descendants()
            .context("failed to rebase descendants after snapshot")?;
        repo = tx
            .commit("snapshot working copy")
            .context("failed to finalize working-copy snapshot")?;
    }

    locked_workspace
        .finish(repo.op_id().clone())
        .context("failed to persist jj working-copy state")?;
    context.repo = repo;

    import_git_refs_for_snapshot(context)?;
    Ok(())
}

fn import_git_head_for_snapshot(context: &mut RepoContext) -> Result<()> {
    let mut tx = context.repo.start_transaction();
    git::import_head(tx.repo_mut()).context("failed to import Git HEAD into JJ view")?;
    if !tx.repo().has_changes() {
        return Ok(());
    }

    if let Some(new_git_head_id) = tx.repo().view().git_head().as_normal().cloned() {
        let workspace_name = context.workspace.workspace_name().to_owned();
        let new_git_head_commit = tx
            .repo()
            .store()
            .get_commit(&new_git_head_id)
            .context("failed to load imported Git HEAD commit")?;
        let wc_commit = tx
            .repo_mut()
            .check_out(workspace_name, &new_git_head_commit)
            .context("failed to reset working-copy parent to Git HEAD")?;

        let mut locked_workspace = context
            .workspace
            .start_working_copy_mutation()
            .context("failed to lock working copy while importing Git HEAD")?;
        block_on(locked_workspace.locked_wc().reset(&wc_commit))
            .context("failed to reset working-copy state to imported Git HEAD")?;
        tx.repo_mut()
            .rebase_descendants()
            .context("failed to rebase descendants after Git HEAD import")?;

        let repo = tx
            .commit("import git head")
            .context("failed to finalize Git HEAD import operation")?;
        locked_workspace
            .finish(repo.op_id().clone())
            .context("failed to persist working-copy state after importing Git HEAD")?;
        context.repo = repo;
        return Ok(());
    }

    let repo = tx
        .commit("import git head")
        .context("failed to record imported Git HEAD state")?;
    let locked_workspace = context
        .workspace
        .start_working_copy_mutation()
        .context("failed to lock working copy after importing Git HEAD")?;
    locked_workspace
        .finish(repo.op_id().clone())
        .context("failed to persist working-copy state after importing Git HEAD")?;
    context.repo = repo;
    Ok(())
}

fn ensure_local_bookmark_for_git_head(context: &mut RepoContext) -> Result<()> {
    let Some(branch_name) = git_head_branch_name_from_context(context) else {
        return Ok(());
    };

    if context
        .repo
        .view()
        .get_local_bookmark(RefName::new(branch_name.as_str()))
        .is_present()
    {
        return Ok(());
    }

    let Some(git_head_id) = context.repo.view().git_head().as_normal().cloned() else {
        return Ok(());
    };

    let mut tx = context.repo.start_transaction();
    tx.repo_mut().set_local_bookmark_target(
        RefName::new(branch_name.as_str()),
        RefTarget::normal(git_head_id),
    );

    let repo = tx
        .commit(format!("create bookmark {branch_name} from git head"))
        .with_context(|| {
            format!("failed to create local bookmark '{branch_name}' from Git HEAD")
        })?;
    let locked_workspace = context
        .workspace
        .start_working_copy_mutation()
        .context("failed to lock working copy after creating Git HEAD bookmark")?;
    locked_workspace
        .finish(repo.op_id().clone())
        .context("failed to persist working-copy state after creating Git HEAD bookmark")?;
    context.repo = repo;
    Ok(())
}

fn import_git_refs_for_snapshot(context: &mut RepoContext) -> Result<()> {
    let import_options = git_import_options_from_settings(&context.settings)?;
    let mut tx = context.repo.start_transaction();
    git::import_refs(tx.repo_mut(), &import_options)
        .context("failed to import Git refs into JJ view")?;
    if !tx.repo().has_changes() {
        return Ok(());
    }

    tx.repo_mut()
        .rebase_descendants()
        .context("failed to rebase descendants after importing Git refs")?;
    let repo = tx
        .commit("import git refs")
        .context("failed to finalize Git ref import operation")?;
    let locked_workspace = context
        .workspace
        .start_working_copy_mutation()
        .context("failed to lock working copy after importing Git refs")?;
    locked_workspace
        .finish(repo.op_id().clone())
        .context("failed to persist working-copy state after importing Git refs")?;
    context.repo = repo;
    Ok(())
}

fn current_wc_commit_with_repo(
    repo: &ReadonlyRepo,
    workspace_name: &WorkspaceName,
) -> Result<Commit> {
    let wc_commit_id = repo
        .view()
        .get_wc_commit_id(workspace_name)
        .ok_or_else(|| {
            anyhow!(
                "workspace '{}' has no working-copy commit",
                workspace_name.as_symbol()
            )
        })?;
    repo.store()
        .get_commit(wc_commit_id)
        .context("failed to load working-copy commit")
}

fn current_wc_commit(context: &RepoContext) -> Result<Commit> {
    current_wc_commit_with_repo(context.repo.as_ref(), context.workspace.workspace_name())
}

pub(super) fn load_changed_files_from_context(context: &RepoContext) -> Result<Vec<ChangedFile>> {
    let wc_commit = current_wc_commit(context)?;
    let base_tree = wc_commit.parent_tree(context.repo.as_ref())?;
    let current_tree = wc_commit.tree();

    let mut file_map = BTreeMap::<String, ChangedFile>::new();
    for entry in block_on_stream(base_tree.diff_stream(&current_tree, &EverythingMatcher)) {
        let path = normalize_path(entry.path.as_internal_file_string());
        if path.is_empty() {
            continue;
        }

        let values = entry.values?;
        let status = map_tree_diff_status(&values);
        let untracked = status == FileStatus::Added;
        let incoming = ChangedFile {
            path: path.clone(),
            status,
            staged: false,
            untracked,
        };

        file_map
            .entry(path)
            .and_modify(|existing| {
                existing.status = merge_file_status(existing.status, incoming.status);
                existing.untracked &= incoming.untracked;
            })
            .or_insert(incoming);
    }

    Ok(file_map.into_values().collect())
}

fn map_tree_diff_status(values: &Diff<MergedTreeValue>) -> FileStatus {
    if !values.before.is_resolved() || !values.after.is_resolved() {
        return FileStatus::Conflicted;
    }

    if values.before.is_absent() && !values.after.is_absent() {
        return FileStatus::Added;
    }
    if !values.before.is_absent() && values.after.is_absent() {
        return FileStatus::Deleted;
    }

    if values.before.is_tree() || values.after.is_tree() {
        return FileStatus::TypeChange;
    }

    FileStatus::Modified
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

pub(super) fn current_bookmarks_from_context(context: &RepoContext) -> Result<BTreeSet<String>> {
    let wc_commit = current_wc_commit(context)?;
    Ok(context
        .repo
        .view()
        .local_bookmarks_for_commit(wc_commit.id())
        .map(|(name, _)| name.as_str().to_string())
        .collect())
}

pub(super) fn git_head_branch_name_from_context(context: &RepoContext) -> Option<String> {
    let git_repo = git::get_git_repo(context.repo.store()).ok()?;
    let head_ref = git_repo.find_reference("HEAD").ok()?;
    let target = head_ref.target();
    let target_name = target.try_name()?;
    let target_name = std::str::from_utf8(target_name.as_bstr()).ok()?;
    target_name
        .strip_prefix("refs/heads/")
        .map(|name| name.to_string())
}

pub(super) fn bookmark_remote_sync_state(
    context: &RepoContext,
    branch_name: &str,
) -> (bool, usize) {
    let mut has_upstream = false;
    let mut needs_push = false;

    for (remote, _) in context.repo.view().remote_views() {
        if remote == REMOTE_NAME_FOR_LOCAL_GIT_REPO {
            continue;
        }

        let Some((_, targets)) = context
            .repo
            .view()
            .local_remote_bookmarks(remote)
            .find(|(name, _)| name.as_str() == branch_name)
        else {
            continue;
        };

        // Treat any present remote bookmark as upstream, even if tracking metadata
        // is temporarily conflicted.
        if !targets.remote_ref.is_present() {
            continue;
        }

        has_upstream = true;
        if matches!(
            classify_bookmark_push_action(targets),
            BookmarkPushAction::Update(_)
        ) {
            needs_push = true;
        }
    }

    (has_upstream, usize::from(needs_push))
}

pub(super) fn list_local_branches_from_context(
    context: &RepoContext,
    current: &BTreeSet<String>,
) -> Result<Vec<LocalBranch>> {
    let mut branches = Vec::new();
    for (name, target) in context.repo.view().local_bookmarks() {
        if !target.is_present() {
            continue;
        }

        let tip_unix_time = target
            .as_normal()
            .map(|id| -> Result<i64> {
                let commit = context.repo.store().get_commit(id)?;
                Ok(commit.committer().timestamp.timestamp.0 / 1000)
            })
            .transpose()?;

        branches.push(LocalBranch {
            is_current: current.contains(name.as_str()),
            name: name.as_str().to_string(),
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

pub(super) fn current_commit_id_from_context(context: &RepoContext) -> Result<Option<String>> {
    Ok(Some(current_wc_commit(context)?.id().hex()))
}

pub(super) fn last_commit_subject_from_context(context: &RepoContext) -> Result<Option<String>> {
    let wc_commit = current_wc_commit(context)?;
    let Some(parent_id) = wc_commit.parent_ids().first() else {
        return Ok(None);
    };

    let parent = context
        .repo
        .store()
        .get_commit(parent_id)
        .context("failed to load parent commit")?;
    let subject = parent
        .description()
        .lines()
        .next()
        .map(str::trim)
        .unwrap_or_default()
        .to_string();

    if subject.is_empty() {
        Ok(None)
    } else {
        Ok(Some(subject))
    }
}

pub(super) fn repo_line_stats_from_context(context: &RepoContext) -> Result<LineStats> {
    let materialize_options = conflict_materialize_options(context);
    let mut stats = LineStats::default();

    for entry in collect_materialized_diff_entries(context)? {
        let rendered = render_patch_for_entry(entry, &materialize_options)?;
        stats.added += rendered.line_stats.added;
        stats.removed += rendered.line_stats.removed;
    }

    Ok(stats)
}

pub(super) fn conflict_materialize_options(context: &RepoContext) -> ConflictMaterializeOptions {
    ConflictMaterializeOptions {
        marker_style: ConflictMarkerStyle::Git,
        marker_len: None,
        merge: context.repo.store().merge_options().clone(),
    }
}

pub(super) fn collect_materialized_diff_entries(
    context: &RepoContext,
) -> Result<Vec<MaterializedTreeDiffEntry>> {
    let wc_commit = current_wc_commit(context)?;
    let base_tree = wc_commit.parent_tree(context.repo.as_ref())?;
    let current_tree = wc_commit.tree();
    let copy_records = CopyRecords::default();

    let stream = materialized_diff_stream(
        context.repo.store().as_ref(),
        base_tree.diff_stream_with_copies(&current_tree, &EverythingMatcher, &copy_records),
        Diff::new(base_tree.labels(), current_tree.labels()),
    );

    let mut entries = Vec::new();
    for entry in block_on_stream(stream) {
        entries.push(entry);
    }
    Ok(entries)
}

pub(super) fn materialized_entry_matches_path(
    entry: &MaterializedTreeDiffEntry,
    file_path: &str,
) -> bool {
    let target = normalize_path(entry.path.target().as_internal_file_string());
    let source = normalize_path(entry.path.source().as_internal_file_string());
    target == file_path || source == file_path
}

pub(super) fn render_patch_for_entry(
    entry: MaterializedTreeDiffEntry,
    materialize_options: &ConflictMaterializeOptions,
) -> Result<RenderedPatch> {
    let values = entry.values?;
    let source_path = normalize_path(entry.path.source().as_internal_file_string());
    let target_path = normalize_path(entry.path.target().as_internal_file_string());

    let before_part = git_diff_part(entry.path.source(), values.before, materialize_options)?;
    let after_part = git_diff_part(entry.path.target(), values.after, materialize_options)?;

    let mut patch = String::new();
    let display_source = if source_path.is_empty() {
        target_path.as_str()
    } else {
        source_path.as_str()
    };
    let display_target = if target_path.is_empty() {
        source_path.as_str()
    } else {
        target_path.as_str()
    };

    patch.push_str(&format!(
        "diff --git a/{display_source} b/{display_target}\n"
    ));

    match (before_part.mode, after_part.mode) {
        (None, Some(new_mode)) => patch.push_str(&format!("new file mode {new_mode}\n")),
        (Some(old_mode), None) => patch.push_str(&format!("deleted file mode {old_mode}\n")),
        (Some(old_mode), Some(new_mode)) if old_mode != new_mode => {
            patch.push_str(&format!("old mode {old_mode}\n"));
            patch.push_str(&format!("new mode {new_mode}\n"));
        }
        _ => {}
    }

    match (before_part.mode, after_part.mode) {
        (Some(mode), Some(new_mode)) if mode == new_mode => {
            patch.push_str(&format!(
                "index {}..{} {mode}\n",
                before_part.hash, after_part.hash
            ));
        }
        _ => {
            patch.push_str(&format!(
                "index {}..{}\n",
                before_part.hash, after_part.hash
            ));
        }
    }

    let before_label = if before_part.mode.is_some() {
        format!("a/{display_source}")
    } else {
        "/dev/null".to_string()
    };
    let after_label = if after_part.mode.is_some() {
        format!("b/{display_target}")
    } else {
        "/dev/null".to_string()
    };

    if before_part.content.is_binary || after_part.content.is_binary {
        if before_part.content.contents != after_part.content.contents {
            patch.push_str(&format!(
                "Binary files {before_label} and {after_label} differ\n"
            ));
        }
        return Ok(RenderedPatch {
            patch,
            line_stats: LineStats::default(),
        });
    }

    let hunks = unified_diff_hunks(
        Diff::new(
            before_part.content.contents.as_ref(),
            after_part.content.contents.as_ref(),
        ),
        3,
        LineCompareMode::Exact,
    );

    let line_stats = line_stats_from_hunks(&hunks);
    if hunks.is_empty() {
        return Ok(RenderedPatch { patch, line_stats });
    }

    patch.push_str(&format!("--- {before_label}\n"));
    patch.push_str(&format!("+++ {after_label}\n"));

    for hunk in hunks {
        patch.push_str(&format_unified_hunk_header(&hunk));
        for (line_type, tokens) in hunk.lines {
            let prefix = match line_type {
                DiffLineType::Context => ' ',
                DiffLineType::Removed => '-',
                DiffLineType::Added => '+',
            };
            append_hunk_line(&mut patch, prefix, &tokens);
        }
    }

    Ok(RenderedPatch { patch, line_stats })
}

fn format_unified_hunk_header(hunk: &UnifiedDiffHunk<'_>) -> String {
    let (left_start, left_count) = format_unified_range(&hunk.left_line_range);
    let (right_start, right_count) = format_unified_range(&hunk.right_line_range);
    format!("@@ -{left_start},{left_count} +{right_start},{right_count} @@\n")
}

fn format_unified_range(range: &std::ops::Range<usize>) -> (usize, usize) {
    let count = range.end.saturating_sub(range.start);
    let start = if count == 0 {
        range.start
    } else {
        range.start.saturating_add(1)
    };
    (start, count)
}

fn append_hunk_line(
    patch: &mut String,
    prefix: char,
    tokens: &[(jj_lib::diff_presentation::DiffTokenType, &[u8])],
) {
    let mut bytes = Vec::new();
    for (_, part) in tokens {
        bytes.extend_from_slice(part);
    }

    patch.push(prefix);
    patch.push_str(String::from_utf8_lossy(&bytes).as_ref());
    if !bytes.ends_with(b"\n") {
        patch.push('\n');
    }
}

fn line_stats_from_hunks(hunks: &[UnifiedDiffHunk<'_>]) -> LineStats {
    let mut stats = LineStats::default();
    for hunk in hunks {
        for (line_type, _) in &hunk.lines {
            match line_type {
                DiffLineType::Added => stats.added += 1,
                DiffLineType::Removed => stats.removed += 1,
                DiffLineType::Context => {}
            }
        }
    }
    stats
}

pub(super) fn load_tracked_paths_from_context(context: &RepoContext) -> Result<BTreeSet<String>> {
    let wc_commit = current_wc_commit(context)?;
    let tree = wc_commit.tree();

    let mut tracked = BTreeSet::new();
    for (path, value) in tree.entries() {
        let value = value?;
        if value.is_absent() || value.is_tree() {
            continue;
        }
        let path = normalize_path(path.as_internal_file_string());
        if !path.is_empty() {
            tracked.insert(path);
        }
    }
    Ok(tracked)
}

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

    let locked_workspace = context
        .workspace
        .start_working_copy_mutation()
        .context("failed to lock working copy after commit")?;
    locked_workspace
        .finish(repo.op_id().clone())
        .context("failed to persist working-copy operation after commit")?;
    context.repo = repo;
    Ok(())
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

    let mut repo_paths = Vec::with_capacity(selected_paths.len());
    for path in selected_paths {
        let normalized = normalize_path(path);
        if normalized.is_empty() {
            continue;
        }
        let repo_path = RepoPathBuf::from_relative_path(Path::new(normalized.as_str()))
            .with_context(|| format!("invalid repository path '{path}'"))?;
        repo_paths.push(repo_path);
    }
    if repo_paths.is_empty() {
        return Err(anyhow!("no valid files selected for commit"));
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

    let locked_workspace = context
        .workspace
        .start_working_copy_mutation()
        .context("failed to lock working copy after partial commit")?;
    locked_workspace
        .finish(repo.op_id().clone())
        .context("failed to persist working-copy operation after partial commit")?;
    context.repo = repo;

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

    let locked_workspace = context
        .workspace
        .start_working_copy_mutation()
        .context("failed to lock working copy after moving bookmark")?;
    locked_workspace
        .finish(repo.op_id().clone())
        .context("failed to persist working-copy state after moving bookmark")?;
    context.repo = repo;

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
    let workspace_name = context.workspace.workspace_name().to_owned();
    let wc_commit = current_wc_commit(context)?;

    let mut tx = context.repo.start_transaction();
    tx.repo_mut().set_local_bookmark_target(
        RefName::new(branch_name),
        RefTarget::normal(wc_commit.id().clone()),
    );
    let repo = tx
        .commit(format!("create bookmark {branch_name}"))
        .context("failed to create bookmark")?;

    let locked_workspace = context
        .workspace
        .start_working_copy_mutation()
        .context("failed to lock working copy after bookmark creation")?;
    locked_workspace
        .finish(repo.op_id().clone())
        .context("failed to persist working-copy state after bookmark creation")?;

    context.repo = repo;
    let _ = workspace_name;
    Ok(())
}

pub(super) fn push_bookmark(context: &mut RepoContext, branch_name: &str) -> Result<()> {
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

    let locked_workspace = context
        .workspace
        .start_working_copy_mutation()
        .context("failed to lock working copy after push")?;
    locked_workspace
        .finish(repo.op_id().clone())
        .context("failed to persist working-copy state after push")?;

    context.repo = repo;
    Ok(())
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

    let locked_workspace = context
        .workspace
        .start_working_copy_mutation()
        .context("failed to lock working copy after sync")?;
    locked_workspace
        .finish(repo.op_id().clone())
        .context("failed to persist working-copy state after sync")?;

    context.repo = repo;

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

    let locked_workspace = context
        .workspace
        .start_working_copy_mutation()
        .context("failed to lock working copy after tracking remote bookmark")?;
    locked_workspace
        .finish(repo.op_id().clone())
        .context("failed to persist working-copy state after tracking remote bookmark")?;

    context.repo = repo;
    Ok(())
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

pub(super) fn walk_repo_tree(
    root: &Path,
    current: &Path,
    tracked_paths: &BTreeSet<String>,
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
        let name = name.to_string_lossy();
        if name == ".git" || name == ".jj" {
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
            let ignored = !path_is_tracked_or_ancestor(relative_path.as_str(), tracked_paths);
            entries.push(RepoTreeEntry {
                path: relative_path,
                kind: RepoTreeEntryKind::Directory,
                ignored,
            });
            walk_repo_tree(root, &child_path, tracked_paths, entries)?;
            continue;
        }

        if file_type.is_file() {
            let ignored = !tracked_paths.contains(relative_path.as_str());
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

fn path_is_tracked_or_ancestor(path: &str, tracked_paths: &BTreeSet<String>) -> bool {
    if tracked_paths.contains(path) {
        return true;
    }

    let prefix = format!("{path}/");
    tracked_paths
        .iter()
        .any(|tracked| tracked.starts_with(&prefix))
}

pub(super) fn normalize_path(path: &str) -> String {
    path.trim().trim_end_matches('/').to_string()
}

pub(super) fn discover_repo_root(cwd: &Path) -> Result<PathBuf> {
    if let Some(root) = find_jj_repo_ancestor(cwd) {
        return Ok(root);
    }

    if let Some(git_root) = find_git_repo_ancestor(cwd) {
        initialize_jj_for_git_repo(&git_root)
            .context("failed to auto-initialize JJ repository in Git checkout")?;

        if let Some(root) = find_jj_repo_ancestor(cwd).or_else(|| find_jj_repo_ancestor(&git_root))
        {
            return Ok(root);
        }
    }

    Err(anyhow!("There is no jj repo in '{}'", cwd.display()))
        .context("failed to discover jj repository")
}

fn initialize_jj_for_git_repo(git_root: &Path) -> Result<()> {
    if git_root.join(".jj").is_dir() {
        return Ok(());
    }

    let settings = load_user_settings(Some(git_root))?;
    let git_repo_path = git_root.join(".git");
    Workspace::init_external_git(&settings, git_root, &git_repo_path).with_context(|| {
        format!(
            "failed to initialize colocated JJ repo at {}",
            git_root.display()
        )
    })?;
    Ok(())
}

fn find_jj_repo_ancestor(path: &Path) -> Option<PathBuf> {
    let mut current = if path.is_file() {
        path.parent()
    } else {
        Some(path)
    };

    while let Some(dir) = current {
        if dir.join(".jj").is_dir() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }

    None
}

fn find_git_repo_ancestor(path: &Path) -> Option<PathBuf> {
    let mut current = if path.is_file() {
        path.parent()
    } else {
        Some(path)
    };

    while let Some(dir) = current {
        let marker = dir.join(".git");
        if marker.is_dir() || marker.is_file() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }

    None
}
