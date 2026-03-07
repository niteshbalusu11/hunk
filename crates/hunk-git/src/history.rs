use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use gix::traverse::commit::simple::CommitTimeOrder;

use crate::git::open_repo;

pub const DEFAULT_RECENT_AUTHORED_COMMIT_LIMIT: usize = 15;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentCommitSummary {
    pub commit_id: String,
    pub subject: String,
    pub committed_unix_time: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentCommitsSnapshot {
    pub root: PathBuf,
    pub author_label: Option<String>,
    pub commits: Vec<RecentCommitSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentCommitsFingerprint {
    root: PathBuf,
    author_key: Option<String>,
    local_branch_tip_ids: Vec<String>,
    limit: usize,
}

impl RecentCommitsFingerprint {
    pub fn root(&self) -> &Path {
        self.root.as_path()
    }
}

#[derive(Debug, Clone)]
struct RecentCommitAuthorMatcher {
    key: Option<String>,
    label: Option<String>,
    email: Option<String>,
    name: Option<String>,
}

impl RecentCommitAuthorMatcher {
    fn from_repo(repo: &gix::Repository) -> Result<Self> {
        let signature = match repo.author() {
            Some(result) => Some(result.context("failed to parse Git author identity")?),
            None => None,
        };
        let Some(signature) = signature else {
            return Ok(Self {
                key: None,
                label: None,
                email: None,
                name: None,
            });
        };

        let email = normalize_identity_value(signature.email);
        let name = normalize_identity_value(signature.name);
        let label = match (name.as_deref(), email.as_deref()) {
            (Some(name), Some(email)) => Some(format!("{name} <{email}>")),
            (Some(name), None) => Some(name.to_string()),
            (None, Some(email)) => Some(email.to_string()),
            (None, None) => None,
        };
        let key = email
            .as_ref()
            .map(|email| format!("email:{}", email.to_ascii_lowercase()))
            .or_else(|| name.as_ref().map(|name| format!("name:{name}")));

        Ok(Self {
            key,
            label,
            email,
            name,
        })
    }

    fn matches(&self, signature: gix::actor::SignatureRef<'_>) -> bool {
        if let Some(expected_email) = self.email.as_deref() {
            return normalize_identity_value(signature.email)
                .is_some_and(|email| email.eq_ignore_ascii_case(expected_email));
        }

        if let Some(expected_name) = self.name.as_deref() {
            return normalize_identity_value(signature.name)
                .is_some_and(|name| name == expected_name);
        }

        false
    }
}

pub fn load_recent_authored_commits(path: &Path, limit: usize) -> Result<RecentCommitsSnapshot> {
    let (_, snapshot) = load_recent_authored_commits_with_fingerprint(path, limit)?;
    Ok(snapshot)
}

pub fn load_recent_authored_commits_with_fingerprint(
    path: &Path,
    limit: usize,
) -> Result<(RecentCommitsFingerprint, RecentCommitsSnapshot)> {
    let (repo, author, tip_ids, fingerprint) = recent_commits_context(path, limit)?;
    let commits =
        load_recent_authored_commits_from_context(repo.repository(), &author, tip_ids, limit)?;

    Ok((
        fingerprint,
        RecentCommitsSnapshot {
            root: repo.root().to_path_buf(),
            author_label: author.label,
            commits,
        },
    ))
}

pub fn load_recent_authored_commits_if_changed(
    path: &Path,
    limit: usize,
    previous_fingerprint: Option<&RecentCommitsFingerprint>,
) -> Result<(RecentCommitsFingerprint, Option<RecentCommitsSnapshot>)> {
    let (repo, author, tip_ids, fingerprint) = recent_commits_context(path, limit)?;
    if previous_fingerprint.is_some_and(|previous| previous == &fingerprint) {
        return Ok((fingerprint, None));
    }
    let commits =
        load_recent_authored_commits_from_context(repo.repository(), &author, tip_ids, limit)?;
    Ok((
        fingerprint,
        Some(RecentCommitsSnapshot {
            root: repo.root().to_path_buf(),
            author_label: author.label,
            commits,
        }),
    ))
}

fn recent_commits_context(
    path: &Path,
    limit: usize,
) -> Result<(
    crate::git::GitRepo,
    RecentCommitAuthorMatcher,
    Vec<gix::ObjectId>,
    RecentCommitsFingerprint,
)> {
    let repo = open_repo(path)?;
    let author = RecentCommitAuthorMatcher::from_repo(repo.repository())?;
    let (tip_ids, tip_id_strings) = local_branch_tip_ids(repo.repository())?;
    let fingerprint = RecentCommitsFingerprint {
        root: repo.root().to_path_buf(),
        author_key: author.key.clone(),
        local_branch_tip_ids: tip_id_strings,
        limit,
    };
    Ok((repo, author, tip_ids, fingerprint))
}

fn load_recent_authored_commits_from_context(
    repo: &gix::Repository,
    author: &RecentCommitAuthorMatcher,
    tip_ids: Vec<gix::ObjectId>,
    limit: usize,
) -> Result<Vec<RecentCommitSummary>> {
    if author.key.is_none() || tip_ids.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    collect_recent_authored_commits(repo, author, tip_ids, limit)
}

fn collect_recent_authored_commits(
    repo: &gix::Repository,
    author: &RecentCommitAuthorMatcher,
    tip_ids: Vec<gix::ObjectId>,
    limit: usize,
) -> Result<Vec<RecentCommitSummary>> {
    let walk = repo
        .rev_walk(tip_ids)
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            CommitTimeOrder::NewestFirst,
        ))
        .all()
        .context("failed to start Git recent-commit traversal")?;
    let mut commits = Vec::with_capacity(limit);

    for info in walk {
        let info = info.context("failed to walk recent Git history")?;
        let commit = info
            .object()
            .with_context(|| format!("failed to load commit {}", info.id))?;
        let commit_author = commit
            .author()
            .with_context(|| format!("failed to read commit author for {}", info.id))?;
        if !author.matches(commit_author) {
            continue;
        }

        commits.push(RecentCommitSummary {
            commit_id: info.id.to_string(),
            subject: commit_subject(&commit),
            committed_unix_time: Some(info.commit_time()),
        });
        if commits.len() >= limit {
            break;
        }
    }

    Ok(commits)
}

fn local_branch_tip_ids(repo: &gix::Repository) -> Result<(Vec<gix::ObjectId>, Vec<String>)> {
    let refs_platform = repo
        .references()
        .context("failed to access Git references for recent commits")?;
    let refs = refs_platform
        .local_branches()
        .context("failed to iterate local Git branches for recent commits")?
        .peeled()
        .context("failed to enable peeled Git branch iteration for recent commits")?;
    let mut tip_ids_by_hex = BTreeMap::<String, gix::ObjectId>::new();

    for reference in refs {
        let mut reference = reference
            .map_err(|err| anyhow::anyhow!("failed to read Git branch reference: {err}"))?;
        let Ok(tip_id) = reference.peel_to_id() else {
            continue;
        };
        tip_ids_by_hex
            .entry(tip_id.to_string())
            .or_insert_with(|| tip_id.detach());
    }

    let tip_ids = tip_ids_by_hex.values().cloned().collect::<Vec<_>>();
    let tip_id_strings = tip_ids_by_hex.into_keys().collect::<Vec<_>>();
    Ok((tip_ids, tip_id_strings))
}

fn normalize_identity_value(value: &gix::bstr::BStr) -> Option<String> {
    let normalized = String::from_utf8_lossy(value.as_ref()).trim().to_string();
    (!normalized.is_empty()).then_some(normalized)
}

fn commit_subject(commit: &gix::Commit<'_>) -> String {
    String::from_utf8_lossy(commit.message_raw_sloppy().as_ref())
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| "(no subject)".to_string())
}
