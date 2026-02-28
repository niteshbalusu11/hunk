use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
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
use jj_lib::matchers::{EverythingMatcher, FilesMatcher, NothingMatcher};
use jj_lib::merge::{Diff, MergedTreeValue};
use jj_lib::object_id::ObjectId as _;
use jj_lib::op_store::RefTarget;
use jj_lib::ref_name::{RefName, RefNameBuf, RemoteName, WorkspaceName};
use jj_lib::refs::{BookmarkPushAction, classify_bookmark_push_action};
use jj_lib::repo::{ReadonlyRepo, Repo as _, StoreFactories};
use jj_lib::repo_path::RepoPathBuf;
use jj_lib::rewrite::{CommitWithSelection, restore_tree, squash_commits};
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
    pub(super) nested_repo_roots_cache: OnceCell<BTreeSet<String>>,
}

pub(super) struct RenderedPatch {
    pub(super) patch: String,
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
        nested_repo_roots_cache: OnceCell::new(),
    };
    if refresh_snapshot {
        refresh_working_copy_snapshot(&mut context)?;
    }
    Ok(context)
}

include!("backend/settings.rs");
include!("backend/snapshot_diff.rs");
include!("backend/operations.rs");
include!("backend/workspace.rs");
