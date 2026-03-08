# Git Worktrees Implementation Plan

## Summary

This document breaks the Git worktrees feature into implementation phases for Hunk.

V1 decisions:

- Keep core worktree logic inside `crates/hunk-git`.
- Managed worktrees live at `<repo>/.hunkdiff/worktrees/<worktree_name>`.
- V1 includes create, list, switch, inspect, publish/push, Review compare, and AI target binding.
- V1 does not include delete/remove worktree UI.
- Worktree creation is from the current active checkout only.
- Review defaults to `base = remote default branch, else main, else master` and `target = active workspace target`.
- AI drafts and threads bind to the exact target path and become immutable after thread start.

## Key Architectural Goals

- Introduce a first-class workspace target model instead of treating the app as single-root and single-branch.
- Keep all Git behavior path-driven so existing snapshot, tree, history, and publish flows can work against either the primary checkout or a linked worktree.
- Replace the Review tab's implicit `HEAD vs working copy` model with a real compare-source abstraction.
- Keep AI workspace semantics aligned with the current cwd-based runtime by binding drafts and threads to exact target paths.

## Phase 1: Workspace Target Foundation

- [ ] Create shared workspace target types across `hunk-git`, `hunk-domain`, and `hunk-desktop`.
- [ ] Define the target model for `primary checkout`, `linked worktree`, stable target id, canonical root path, branch name, display label, and `managed/external` status.
- [ ] Add repo-local managed-worktree path helpers for `.hunkdiff/worktrees`.
- [ ] Make `.hunkdiff/worktrees` an explicit ignored subtree for repo tree rendering, status scans, and filesystem watch filtering.
- [ ] Extend persisted app state for per-primary-repo selected target and Review compare defaults.
- [ ] Add targeted tests for state serialization, target id stability, canonical-path handling, and ignore-path behavior.
- [ ] Deep code review of all Phase 1 code to check for path identity bugs, stale-state risks, bad abstractions, and refactor opportunities.

## Phase 2: Git Backend Worktree Catalog and Creation

- [ ] Add `hunk-git` APIs to list the primary checkout and all linked worktrees for a repository.
- [ ] Introduce public worktree-facing types such as `WorkspaceTargetSummary`, `WorkspaceTargetKind`, and `CreateWorktreeRequest`.
- [ ] Add `hunk-git` mutation APIs to create a managed worktree in `.hunkdiff/worktrees/<worktree_name>` from the current active checkout.
- [ ] Validate worktree names, branch names, path collisions, and branch collisions during creation.
- [ ] Keep the read path `gix`-first and use narrow `git2` fallback only if required for worktree creation.
- [ ] Ensure primary checkout scans do not surface managed worktrees as ordinary nested repos.
- [ ] Add targeted tests for worktree listing, managed-worktree creation, canonical-root resolution, collision handling, and externally-created linked worktrees.
- [ ] Deep code review of all Phase 2 code to check for Git correctness bugs, backend coupling, unsafe filesystem behavior, and code that should be simplified.

## Phase 3: Git Tab Worktree UX and Active Target Switching

- [ ] Add a worktree picker to the Git tab so the user can switch between the primary checkout and linked worktrees.
- [ ] Add a worktree creation form to the Git tab with required `worktree name` and `branch name` inputs.
- [ ] Rebind active workflow state when the selected target changes, including snapshot loading, recent commits, repo tree, repo watcher, toolbar/footer labels, and file editor state.
- [ ] Persist and restore the last selected target for each primary repository.
- [ ] Keep existing branch controls operating on the active target root so branch activation, publish, push, sync, and PR/MR flows remain target-specific.
- [ ] Ensure switching targets updates the Files tab and any selected/open file state safely.
- [ ] Add targeted desktop tests for target switching, cache hydration, watcher rebinding, file tree refresh, and editor refresh after target changes.
- [ ] Deep code review of all Phase 3 code to check for refresh ordering bugs, cache invalidation mistakes, UI state drift, and duplicated logic that should be consolidated.

## Phase 4: Review Compare Model

- [ ] Introduce compare-source types in `hunk-git` for primary checkout, linked worktree, and branch/ref targets.
- [ ] Replace the Review tab's implicit `HEAD vs working copy` backend with compare-aware snapshot, patch, changed-file, and line-stat loading keyed by `(left source, right source)`.
- [ ] Add two Review pickers backed by a shared compare-source delegate.
- [ ] Default the Review tab to `left = resolved base branch` and `right = active workspace target`.
- [ ] Allow manual Review comparisons for branch vs active checkout, branch vs worktree, and worktree vs worktree.
- [ ] Make diff headers, labels, changed-file lists, and refresh fingerprints compare-aware instead of active-root-only.
- [ ] Keep comment authoring enabled only for the default `active target vs base branch` comparison and make custom compare pairs read-only for comments in v1.
- [ ] Add targeted tests for compare-source resolution, diff loading, base-branch fallback behavior, worktree-to-worktree diffs, and comment disabling on custom compare pairs.
- [ ] Deep code review of all Phase 4 code to check for compare-semantics bugs, stale diff state, poor source-identity modeling, and refactors needed in the Review pipeline.

## Phase 5: AI Target Binding

- [ ] Add AI draft target selection with choices for the primary checkout and linked worktrees.
- [ ] Keep AI target selection editable only while the thread is still a draft.
- [ ] Bind `Cmd/Ctrl+N` to a new primary-checkout draft.
- [ ] Bind `Cmd/Ctrl+Shift+N` to a new worktree-target draft flow.
- [ ] Start AI workers and threads against the exact selected target path, matching the current cwd-based `hunk-codex` design.
- [ ] Keep existing threads immutable after start so later workspace switching does not retarget an already-started thread.
- [ ] Persist AI draft and session preferences per exact target path.
- [ ] Add targeted tests for draft target selection, thread immutability, worker rebinding by path, and shortcut behavior.
- [ ] Deep code review of all Phase 5 code to check for workspace/thread isolation bugs, lifecycle races, incorrect rebinding, and code that should be cleaned up before rollout.

## Phase 6: Polish, Documentation, and Final Validation

- [ ] Review all Git, Files, Review, and AI states for any remaining single-root or single-branch assumptions.
- [ ] Clean up temporary or transitional code introduced during earlier phases.
- [ ] Polish labels, loading states, disabled states, and error messaging so worktree context is always visible and unambiguous.
- [ ] Update user-facing docs as needed to describe managed worktree behavior, Review compare behavior, and AI target semantics.
- [ ] Run final workspace verification once at the end:
- [ ] `cargo build --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] Deep code review of the full feature set to check for cross-phase regressions, hidden bugs, bad code, stale state, and refactor opportunities before considering the feature complete.

## Expected Public Interface Changes

- `crates/hunk-git` gains worktree and compare-source public types plus compare-aware snapshot and patch APIs.
- `crates/hunk-domain::AppState` gains per-repo target persistence and Review compare persistence.
- `crates/hunk-desktop` gains explicit state for:
  - active workspace target selection
  - workspace target catalog
  - Review compare source selection
  - AI draft target selection

## Non-Goals for V1

- No delete/remove worktree UI.
- No branch-agnostic comment persistence for arbitrary compare pairs.
- No separate worktree crate unless early implementation proves the current crate boundaries cannot support the feature cleanly.
