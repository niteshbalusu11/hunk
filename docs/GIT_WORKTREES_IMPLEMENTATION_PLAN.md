# Git Worktrees Implementation Plan

## Summary

This document breaks the Git worktrees feature into implementation phases for Hunk.

Performance is a hard requirement for this entire feature. Regressions in Git tab and Review tab responsiveness are not acceptable. The app should continue to feel extremely fast, and any design that adds noticeable latency, jank, or unnecessary refresh churn should be treated as incorrect until fixed.

V1 decisions:

- Keep core worktree logic inside `crates/hunk-git`.
- Managed worktrees live at `~/.hunkdiff/worktrees/<repo-key>/<worktree_name>`.
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
- Preserve Hunk's current performance bar in the Git and Review tabs; performance regressions are release blockers for this feature.

## Repo-Agnostic Invariants

- Worktree behavior must be identical for any imported Git repository. No logic should depend on Hunk's own repository layout, Cargo, or any project-specific build system.
- One Hunk-managed AI worktree should represent one isolated task by default.
- A branch can only be checked out in one worktree at a time, so the UI must treat branch occupancy as first-class state instead of treating checkout failures as normal UX.
- Base branches such as `main` and `master` are source branches for new task worktrees, not long-lived AI task branches themselves.
- Parallel AI work must be keyed by workspace cwd, not by whichever workspace is currently visible in the UI.

## Current Gap Versus T3Code

- Hunk already matches the core `t3code` model of `new task -> new branch + new worktree + thread bound to that cwd`.
- The main gaps are:
  - branch lists do not say which worktree already owns a branch
  - selecting an occupied branch still attempts checkout instead of activating the owning worktree
  - new worktree flows always base from the synced default branch instead of letting the user pick the base branch
  - AI runtime state is still singleton-based, so only one workspace runtime stays live at a time

## Execution Plan

### Track A: Branch Occupancy and Worktree-Aware Branch Activation

- Extend `hunk-git::git::LocalBranch` with attached worktree metadata:
  - `attached_workspace_target_id`
  - `attached_workspace_target_root`
  - `attached_workspace_target_label`
- Build a branch-to-worktree occupancy map from `list_workspace_targets(...)` during workflow snapshot loading.
- Persist that metadata in `hunk-domain::state::CachedLocalBranchState`.
- Update the branch picker so occupied branches show where they are already checked out.
- Change branch selection behavior:
  - if the branch is already checked out in another workspace target, activate that target
  - otherwise keep the current dirty-tree guarded checkout/create flow
- Files:
  - `crates/hunk-git/src/git.rs`
  - `crates/hunk-domain/src/state.rs`
  - `crates/hunk-desktop/src/app/controller/core.rs`
  - `crates/hunk-desktop/src/app/controller/workspace_mode.rs`
  - `crates/hunk-desktop/src/app/branch_picker.rs`
- Tests:
  - `crates/hunk-git/tests/worktree_ops.rs`
  - `crates/hunk-desktop/tests/branch_picker.rs`
  - `crates/hunk-domain/tests/app_state.rs`

### Track B: Base Branch Selection for New Worktree Threads

- Keep the current default of using the synced default branch when the user does not choose another base.
- Add explicit base-branch selection before creating a new worktree-backed AI thread.
- Interpret branch selection in `new worktree thread` mode as `choose base branch`, not `check this branch out in the current workspace`.
- Never create a managed AI worktree directly on `main` or `master`; always create a task branch from the chosen base.
- Files:
  - `crates/hunk-desktop/src/app/controller/ai/runtime.rs`
  - `crates/hunk-desktop/src/app/controller/ai/helpers.rs`
  - `crates/hunk-desktop/src/app/render/ai.rs`
  - `crates/hunk-desktop/src/app/render/ai_helpers/*`

### Track C: Truly Parallel AI Workspaces

- Replace the single global AI runtime in `DiffViewer` with a runtime manager keyed by workspace cwd.
- Move transport handles, connection state, approvals, pending inputs, per-thread selection, and timeline caches into per-workspace runtime state.
- Stop using workspace switches to tear down AI transport. Switching workspaces should change visibility, not destroy background work in another worktree.
- Tag worker events and snapshots with `workspace_key` so one UI process can route multiple live runtime streams safely.
- Keep thread validity rules in `hunk-codex` cwd-scoped, which already matches the desired model.
- Implemented:
  - persist per-workspace AI UI/runtime state in `DiffViewer`
  - park active worker transports on workspace switch instead of shutting them down
  - restore and promote parked workers when returning to the same workspace
  - keep hidden worker event listeners alive so background work continues while another worktree is visible
  - tag worker transport config and worker events with explicit `workspace_key`
  - route worker commands through a workspace-keyed manager helper instead of assuming one visible runtime
- Future refinements:
  - collapse `visible runtime + hidden runtimes` into a dedicated runtime-manager type to reduce controller bookkeeping
  - add higher-level tests for workspace switching and concurrent in-flight turns
  - decide whether hidden-workspace approvals and follow-up inputs should surface globally or only when that workspace becomes visible
- Files:
  - `crates/hunk-desktop/src/app.rs`
  - `crates/hunk-desktop/src/app/controller/ai/core.rs`
  - `crates/hunk-desktop/src/app/controller/ai/runtime.rs`
  - `crates/hunk-desktop/src/app/controller/ai/workspace_runtime.rs`
  - `crates/hunk-desktop/src/app/ai_runtime/core.rs`
  - `crates/hunk-desktop/src/app/ai_runtime/sync.rs`
  - `crates/hunk-codex/src/state.rs`
  - `crates/hunk-codex/src/threads/notifications.rs`

### Recommended Order

- Land Track A first because it fixes the branch/worktree conflict the user hits today and aligns Hunk with the best `t3code` behavior.
- Land Track B second so new worktree threads are based from the right branch intentionally.
- Land Track C third because it is the largest refactor and depends on clear workspace identity semantics from Tracks A and B.

## Performance Requirements

- Git tab and Review tab performance are critical and must not regress as part of worktree support.
- Snapshot loading, changed-file loading, compare-source switching, repo tree refresh, and diff loading must remain incremental and avoid unnecessary full reloads.
- Worktree support must not introduce extra filesystem churn, nested-repo scanning overhead, or redundant background work for inactive targets.
- UI interactions such as switching targets, switching compare sources, opening the Git tab, and opening the Review tab should remain immediate and feel lightweight.
- Any implementation choice that harms responsiveness in the hot path should be rejected in favor of a faster design, even if it is more convenient to implement.

## Phase 1: Workspace Target Foundation

- [ ] Create shared workspace target types across `hunk-git`, `hunk-domain`, and `hunk-desktop`.
- [ ] Define the target model for `primary checkout`, `linked worktree`, stable target id, canonical root path, branch name, display label, and `managed/external` status.
- [ ] Add managed-worktree path helpers under `~/.hunkdiff/worktrees/<repo-key>`.
- [ ] Managed worktrees live outside the repo, so repo tree rendering, status scans, and filesystem watch filtering do not need repo-local managed-worktree exclusions.
- [ ] Extend persisted app state for per-primary-repo selected target and Review compare defaults.
- [ ] Review the target model and path rules specifically for hot-path cost so inactive worktrees do not add scan, watch, or refresh overhead to the active workspace.
- [ ] Add targeted tests for state serialization, target id stability, canonical-path handling, and ignore-path behavior.
- [ ] Deep code review of all Phase 1 code to check for path identity bugs, stale-state risks, bad abstractions, performance hazards, and refactor opportunities.

## Phase 2: Git Backend Worktree Catalog and Creation

- [ ] Add `hunk-git` APIs to list the primary checkout and all linked worktrees for a repository.
- [ ] Introduce public worktree-facing types such as `WorkspaceTargetSummary`, `WorkspaceTargetKind`, and `CreateWorktreeRequest`.
- [ ] Add `hunk-git` mutation APIs to create a managed worktree in `~/.hunkdiff/worktrees/<repo-key>/<worktree_name>` from the current active checkout.
- [ ] Validate worktree names, branch names, path collisions, and branch collisions during creation.
- [ ] Keep the read path `gix`-first and use narrow `git2` fallback only if required for worktree creation.
- [ ] Ensure primary checkout scans do not surface managed worktrees as ordinary nested repos.
- [ ] Measure and review backend hot paths so worktree discovery and target catalog refresh do not slow down normal Git tab refreshes.
- [ ] Add targeted tests for worktree listing, managed-worktree creation, canonical-root resolution, collision handling, and externally-created linked worktrees.
- [ ] Deep code review of all Phase 2 code to check for Git correctness bugs, backend coupling, unsafe filesystem behavior, performance regressions, and code that should be simplified.

## Phase 3: Git Tab Worktree UX and Active Target Switching

- [ ] Add a worktree picker to the Git tab so the user can switch between the primary checkout and linked worktrees.
- [ ] Add a worktree creation form to the Git tab with required `worktree name` and `branch name` inputs.
- [ ] Rebind active workflow state when the selected target changes, including snapshot loading, recent commits, repo tree, repo watcher, toolbar/footer labels, and file editor state.
- [ ] Persist and restore the last selected target for each primary repository.
- [ ] Keep existing branch controls operating on the active target root so branch activation, publish, push, sync, and PR/MR flows remain target-specific.
- [ ] Ensure switching targets updates the Files tab and any selected/open file state safely.
- [ ] Profile target switching and Git tab refresh behavior to ensure the tab still feels instant and does not accumulate extra work across multiple linked worktrees.
- [ ] Add targeted desktop tests for target switching, cache hydration, watcher rebinding, file tree refresh, and editor refresh after target changes.
- [ ] Deep code review of all Phase 3 code to check for refresh ordering bugs, cache invalidation mistakes, UI state drift, duplicated logic, and any Git tab performance regressions.

## Phase 4: Review Compare Model

- [ ] Introduce compare-source types in `hunk-git` for primary checkout, linked worktree, and branch/ref targets.
- [ ] Replace the Review tab's implicit `HEAD vs working copy` backend with compare-aware snapshot, patch, changed-file, and line-stat loading keyed by `(left source, right source)`.
- [ ] Add two Review pickers backed by a shared compare-source delegate.
- [ ] Default the Review tab to `left = resolved base branch` and `right = active workspace target`.
- [ ] Allow manual Review comparisons for branch vs active checkout, branch vs worktree, and worktree vs worktree.
- [ ] Make diff headers, labels, changed-file lists, and refresh fingerprints compare-aware instead of active-root-only.
- [ ] Keep comment authoring enabled only for the default `active target vs base branch` comparison and make custom compare pairs read-only for comments in v1.
- [ ] Validate that compare-source switching, diff rendering, and changed-file tree updates remain fast enough that the Review tab still feels immediate under large diffs and large repositories.
- [ ] Add targeted tests for compare-source resolution, diff loading, base-branch fallback behavior, worktree-to-worktree diffs, and comment disabling on custom compare pairs.
- [ ] Deep code review of all Phase 4 code to check for compare-semantics bugs, stale diff state, poor source-identity modeling, Review tab performance regressions, and refactors needed in the Review pipeline.

## Phase 5: AI Target Binding

- [ ] Add AI draft target selection with choices for the primary checkout and linked worktrees.
- [ ] Keep AI target selection editable only while the thread is still a draft.
- [ ] Bind `Cmd/Ctrl+N` to a new primary-checkout draft.
- [ ] Bind `Cmd/Ctrl+Shift+N` to a new worktree-target draft flow.
- [ ] Start AI workers and threads against the exact selected target path, matching the current cwd-based `hunk-codex` design.
- [ ] Keep existing threads immutable after start so later workspace switching does not retarget an already-started thread.
- [ ] Persist AI draft and session preferences per exact target path.
- [ ] Add targeted tests for draft target selection, thread immutability, worker rebinding by path, and shortcut behavior.
- [ ] Deep code review of all Phase 5 code to check for workspace/thread isolation bugs, lifecycle races, incorrect rebinding, performance side effects from target rebinding, and code that should be cleaned up before rollout.

## Phase 6: Polish, Documentation, and Final Validation

- [ ] Review all Git, Files, Review, and AI states for any remaining single-root or single-branch assumptions.
- [ ] Clean up temporary or transitional code introduced during earlier phases.
- [ ] Polish labels, loading states, disabled states, and error messaging so worktree context is always visible and unambiguous.
- [ ] Update user-facing docs as needed to describe managed worktree behavior, Review compare behavior, and AI target semantics.
- [ ] Run performance validation focused on Git tab refresh, Review tab compare switching, large-diff rendering, and multi-worktree repos; treat regressions as release blockers.
- [ ] Run final workspace verification once at the end:
- [ ] `cargo build --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] Deep code review of the full feature set to check for cross-phase regressions, hidden bugs, bad code, stale state, performance regressions, and refactor opportunities before considering the feature complete.

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
