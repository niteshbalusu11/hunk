# JJ Graph Workspaces TODO

## Status
- Proposed
- Owner: Hunk
- Last Updated: 2026-03-03
- Related docs:
  - `docs/JJ_GRAPH_WORKSPACE_RFC.md`
  - `docs/JJ_GRAPH_WORKSPACES_GUIDE.md`
  - `docs/JJ_NATIVE_MODE_SPEC.md`
  - `docs/JJ_WORKSPACE_RFC.md`

## Objective
Add first-class JJ workspace support to the graph view while keeping bookmark workflows intact.

This plan treats JJ workspaces as parallel mutable working-copy contexts (similar to Git worktrees), but not as a replacement for bookmarks. Both must coexist in the same graph UX.

## Product Requirements
1. Graph view must show workspace refs (`<workspace>@`) and bookmark refs together.
2. Current workspace remains the active editing context for commit/restore/publish/sync actions.
3. Non-current workspace refs are visible and explorable without unsafe implicit switching.
4. Bookmark workflows (create/fork/rename/move/publish/sync/review URL) must keep working.
5. Architecture must support future AI coding tab running parallel agents across workspaces.

## Non-Goals (This Milestone)
1. Multi-select operations.
2. Full parity with every `jj workspace` CLI command in v1.
3. Cross-workspace file operations in one panel.
4. Automated workspace lifecycle cleanup policies.

## Confirmed JJ Semantics To Preserve
1. Workspaces and bookmarks are different ref types and both can appear in `jj log`.
2. Each workspace has its own working-copy commit (`workspace@`).
3. The app process runs from one workspace root at a time; edit actions apply to that context only.
4. Workspace refs can be stale/missing and must be handled safely.

## High-Level Architecture Changes
1. `hunk-jj` graph model:
   - Add workspace reference types and expose them in graph snapshot payload.
   - Keep existing bookmark payloads unchanged.
2. `hunk-desktop` graph state:
   - Track current workspace identity and per-node workspace attachments.
   - Add workspace selection/focus state separate from bookmark selection.
3. Rendering:
   - Render workspace chips distinct from bookmark chips.
   - Keep action enablement strict (workspace inspect vs bookmark mutate vs active editing context).
4. Testing:
   - Add integration tests for mixed bookmark/workspace graph scenarios.
   - Add controller/render logic tests for guard rails and state reconciliation.

## Stop-The-Line Rules
1. Do not proceed to the next phase until current phase review gate and quality gate are complete.
2. If any regression appears in bookmark workflows, fix it before moving on.
3. If model/API shape causes ambiguous ownership between bookmark and workspace actions, refactor before continuing.

## Global Quality Gate (Required After Every Phase)
Run all of the following and fix failures before phase completion:
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo test --workspace`

## Deep Code Review Gate (Required After Every Phase)
Review all code written in that phase before proceeding. Verify:
1. Correctness:
   - no invalid JJ assumptions,
   - no action routed to wrong workspace context,
   - no stale selection state leaks after snapshot refresh.
2. Safety:
   - no implicit destructive behavior,
   - clear error handling for missing/stale workspace refs.
3. Design quality:
   - no unnecessary duplication,
   - cohesive API boundaries,
   - naming clarity (`bookmark` vs `workspace` vs `active workspace`).
4. UI/UX quality:
   - action labels/tooltips map to JJ mental model,
   - disabled states include actionable reasons.
5. Scalability/performance:
   - graph snapshot remains bounded/windowed,
   - no unnecessary O(N^2) scans on hot render paths.
6. Refactor pass:
   - apply cleanup before closing the phase (not deferred).

## Phase Plan

### Phase 0: Specification + Baseline Audit
- [x] Finalize exact data contract for workspace refs in graph snapshot.
- [x] Enumerate all single-working-copy assumptions in backend, controller, and rendering.
- [x] Define migration compatibility strategy so existing snapshot consumers do not break unexpectedly.
- [x] Write explicit acceptance criteria for each phase in this doc.
- [x] Deep code review gate complete.
- [x] Global quality gate complete.

Acceptance criteria:
1. Documented API shape for workspace refs is stable enough to implement without backtracking.
2. All impacted files are listed in a concrete execution map.

Phase 0 review note (2026-03-03):
1. Confirmed JJ workspaces appear in history as `<workspace>@` refs and can coexist with bookmarks.
2. Audited current single-working-copy assumptions in:
   - `crates/hunk-jj/src/jj.rs` (`GraphSnapshot` and `GraphNode` shape),
   - `crates/hunk-jj/src/jj/backend/graph.rs` (single working-copy commit/parent derivation),
   - `crates/hunk-desktop/src/app.rs` + `controller/core.rs` (single working-copy state fields),
   - `crates/hunk-desktop/src/app/render/jj_graph.rs` (single `@` marker assumptions).
3. Chosen migration strategy:
   - additive API changes first (new workspace graph fields),
   - preserve existing bookmark data and current working-copy fields during transition,
   - defer behavior changes in desktop actions until later phases.
4. Validation:
   - `cargo fmt --all -- --check` passed
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
   - `cargo test --workspace` passed

### Phase 1: Backend Graph Model (Read-Only Workspace Visibility)
- [x] Add new domain types in `crates/hunk-jj/src/jj.rs`:
  - `GraphWorkspaceRef`
  - `GraphWorkspaceStatus` (if stale/missing is represented)
- [x] Extend `GraphNode` with workspace attachments.
- [x] Extend `GraphSnapshot` with current workspace metadata and optional workspace summary list.
- [x] Update `build_graph_snapshot_from_context()` to attach workspace refs to graph nodes.
- [x] Include workspace working-copy commits in seed selection logic for graph windowing.
- [x] Preserve bookmark ordering and existing remote/local bookmark metadata.
- [x] Add/adjust tests in `crates/hunk-jj/tests/jj_graph_snapshot.rs`:
  - mixed bookmark/workspace graph,
  - multiple workspace refs visible,
  - current workspace marker correctness,
  - no bookmark metadata regression.
- [x] Deep code review gate complete.
- [x] Global quality gate complete.

Acceptance criteria:
1. Snapshot payload contains enough info to render workspace chips without extra JJ queries.
2. Existing bookmark behavior remains unchanged in tests.

Phase 1 review note (2026-03-03):
1. Added graph workspace payload with additive model changes:
   - `GraphWorkspaceRef` on nodes,
   - `GraphWorkspaceState` summary in snapshot,
   - `current_workspace_name` in snapshot.
2. Backend snapshot builder now:
   - maps `view.wc_commit_ids()` into workspace summary and per-node attachments,
   - includes workspace working-copy commits as graph seeds,
   - preserves existing bookmark metadata/sorting behavior.
3. Compatibility and correctness:
   - retained existing `working_copy_commit_id` and `working_copy_parent_commit_id` fields for incremental desktop migration,
   - fixed `WorkspaceName` string conversion via `as_str()` to match current `jj-lib` API.
4. Tests:
   - extended graph snapshot tests for multi-workspace visibility and current-workspace marker,
   - updated GraphNode fixtures in graph/bookmark tests for new field shape.
5. Validation:
   - `cargo fmt --all -- --check` passed
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
   - `cargo test --workspace` passed

### Phase 2: Desktop State Integration
- [x] Update app state in `crates/hunk-desktop/src/app.rs` to track:
  - current workspace name,
  - selected workspace ref (if any),
  - workspace-focused revision IDs (if needed).
- [x] Update snapshot application logic in `crates/hunk-desktop/src/app/controller/core.rs`:
  - map new graph payload fields,
  - reconcile workspace selections after refresh/window pagination.
- [x] Ensure no cross-contamination between selected bookmark state and selected workspace state.
- [x] Add controller-level tests for state reconciliation and mode fallbacks.
- [x] Deep code review gate complete.
- [x] Global quality gate complete.

Acceptance criteria:
1. Snapshot refresh does not leave invalid workspace/bookmark selections.
2. Current workspace remains the only editing context in state.

Phase 2 review note (2026-03-03):
1. Desktop state now tracks workspace graph context:
   - `graph_current_workspace_name`,
   - `graph_workspaces`,
   - `graph_selected_workspace`.
2. Snapshot application now hydrates workspace graph state and clears it on snapshot errors.
3. Reconciliation safeguards:
   - workspace selection is filtered against current snapshot workspace list,
   - bookmark selection and workspace selection are kept mutually exclusive when bookmark mode is active.
4. Mode fallback hardening:
   - selected-bookmark panel mode now uses an explicit reconciliation helper to fall back to active workflow when no bookmark is selected.
5. Added controller-level tests for:
   - workspace selection matching,
   - selected-bookmark mode fallback behavior.
6. Validation:
   - `cargo fmt --all -- --check` passed
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
   - `cargo test --workspace` passed

### Phase 3: Graph Rendering for Workspaces (Read-Only)
- [x] Update `crates/hunk-desktop/src/app/render/jj_graph.rs` to render workspace chips per node.
- [x] Create clear visual distinction:
  - bookmark chip style remains unchanged,
  - workspace chip style is unique and semantically labeled (`<workspace>@`).
- [x] Add current workspace highlight token.
- [x] Update legend and glossary text to explain bookmark vs workspace differences.
- [x] Ensure narrow-width wrapping and scrolling remain stable.
- [x] Add render/controller tests for:
  - chip labeling,
  - selection behavior,
  - no accidental bookmark action activation from workspace selection.
- [x] Deep code review gate complete.
- [x] Global quality gate complete.

Acceptance criteria:
1. User can visually identify local bookmark, remote bookmark, and workspace refs at a glance.
2. Workspace selection is inspect-only in this phase.

Phase 3 review note (2026-03-03):
1. Graph rows now render workspace chips (`W name@`) alongside bookmark chips using a distinct color system.
2. Workspace chips are inspect-only:
   - click selects workspace context and node,
   - bookmark selection is cleared to avoid mixed action contexts,
   - right panel mode is forced back to active workflow (no bookmark-action leakage).
3. Current workspace is highlighted with `[current]` token in chip labels.
4. Legend and JJ glossary copy were updated to explicitly document workspace refs vs bookmarks.
5. Controller coverage remains in place for mode fallback and workspace-selection matching behavior.
6. Validation:
   - `cargo fmt --all -- --check` passed
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
   - `cargo test --workspace` passed

### Phase 4: Workspace Inspector + Safe Actions (No Switching Yet)
- [x] Extend inspector (`crates/hunk-desktop/src/app/render/jj_graph_inspector.rs`) to show selected workspace details:
  - workspace name,
  - target commit,
  - whether it is current workspace.
- [x] Add guarded actions that are safe in v1:
  - copy workspace name,
  - focus workspace commit in graph.
- [x] Add disabled placeholders for future operations with clear reason text.
- [x] Ensure existing bookmark action block remains scoped to bookmark selection only.
- [x] Add tests for guard-rail behavior and disabled-reason text.
- [x] Deep code review gate complete.
- [x] Global quality gate complete.

Acceptance criteria:
1. Workspace and bookmark action surfaces are clearly separated.
2. No mutation action runs when a workspace ref is selected.

Phase 4 review note (2026-03-03):
1. Inspector now includes a dedicated selected-workspace panel with:
   - workspace name (`<name>@`),
   - target commit (short hash),
   - current/non-current workspace marker,
   - stale-selection fallback messaging when a selected workspace disappears from snapshot state.
2. Added safe v1 workspace actions:
   - copy selected workspace name to clipboard,
   - focus selected workspace commit in graph (with guard when commit is outside current graph window).
3. Added explicit future-operation placeholders with consistent reason text:
   - switch workspace (Phase 5),
   - create/forget workspace lifecycle (Phase 6).
4. Guard-rail hardening for mutation safety:
   - bookmark mutation actions are disabled in inspector while workspace focus is active,
   - controller-level mutation methods now enforce the same guard to block non-UI invocation paths,
   - pending bookmark-move confirmation is cleared when workspace focus is present.
5. Added controller tests for:
   - workspace inspect-only reason text,
   - future workspace action reason text,
   - workspace-selection mutation blocker behavior.
6. Deep review outcomes:
   - validated action isolation (`workspace inspect` vs `bookmark mutate`) and state reconciliation behavior,
   - verified no implicit workspace switching or destructive operation was introduced.
7. Validation:
   - `cargo fmt --all -- --check` passed
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
   - `cargo test --workspace` passed

### Phase 5: Explicit Workspace Switching Flow
- [x] Introduce backend APIs in `hunk-jj` for explicit workspace switch/open semantics.
- [x] Handle compatibility for JJ versions where workspace root/path capabilities differ.
- [x] Add explicit confirmation flow in UI before leaving current workspace context.
- [x] Preserve dirty-state guard behavior; never lose local changes silently.
- [x] Reinitialize repository watch/snapshot tasks safely after switch.
- [x] Add integration tests:
  - switch with clean working copy,
  - switch with dirty working copy (guarded),
  - stale/missing workspace handling.
- [x] Deep code review gate complete.
- [x] Global quality gate complete.

Acceptance criteria:
1. Switching workspace is always explicit and auditable.
2. Dirty working-copy protection is at least as safe as current bookmark-switch protection.

Phase 5 review note (2026-03-03):
1. Added explicit switch-target API in `hunk-jj`:
   - `resolve_workspace_switch_target(repo_root, workspace_name) -> WorkspaceSwitchTarget`,
   - backend workspace-store metadata access (`workspace_root_from_store`),
   - compatibility fallback discovery for stale/missing workspace-store paths.
2. Hardened workspace root resolution during review:
   - stored path candidates are now validated against both repository identity and workspace name before being accepted,
   - avoids false-positive switches to unrelated directories when stale relative paths exist.
3. Desktop controller/UI flow now supports explicit guarded switching:
   - `request_switch_selected_graph_workspace` resolves target asynchronously,
   - dirty working copy triggers explicit pending confirmation card,
   - confirm/cancel controls are isolated in inspector,
   - clean working copy switches immediately with watch/snapshot re-init.
4. UX and action-surface separation:
   - bookmark mutation remains blocked when workspace focus is active,
   - workspace switch button is enabled only when selection is valid,
   - lifecycle actions remain deferred to Phase 6 with updated reason text.
5. Tests:
   - added `crates/hunk-jj/tests/jj_workspace_switch.rs` for clean/missing/stale target resolution,
   - added desktop controller unit tests for workspace confirmation guard/message.
6. Validation:
   - `cargo fmt --all -- --check` passed
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
   - `cargo test --workspace` passed

### Phase 6: Workspace Lifecycle (Create/Forget) from Graph UX
- [x] Add controlled UI flows for:
  - create workspace from selected revision/bookmark context,
  - forget workspace with clear warnings.
- [x] Validate workspace naming constraints and duplicate handling.
- [x] Update graph refresh pipeline to reconcile newly created/forgotten workspace refs.
- [x] Add integration tests for lifecycle operations and rollback-safe errors.
- [x] Deep code review gate complete.
- [x] Global quality gate complete.

Acceptance criteria:
1. Workspace create/forget works without breaking bookmark graph workflows.
2. Error messages are clear for duplicate/stale/invalid targets.

Phase 6 review note (2026-03-03):
1. Added backend lifecycle APIs in `hunk-jj`:
   - `create_workspace_at_revision(repo_root, workspace_name, revision_id, workspace_root)`,
   - `forget_workspace(repo_root, workspace_name)`,
   - plus `WorkspaceCreationResult` output for UI messaging.
2. Lifecycle safety and validation:
   - create validates workspace name, revision id, destination emptiness/type, and duplicate workspace names,
   - create now validates target revision existence before workspace initialization to avoid partial workspace creation on invalid revisions,
   - forget blocks current-workspace deletion and missing workspace names.
3. Desktop graph lifecycle UX:
   - inspector now has a dedicated workspace-name input and active `Create Workspace` / `Forget Workspace` actions,
   - forget uses an explicit confirmation panel with clear non-destructive warning text,
   - action blockers prevent collisions with pending switch/forget flows and preserve explicit state transitions.
4. State reconciliation and refresh behavior:
   - added `pending_workspace_forget` state and reconciliation cleanup,
   - successful create/forget operations trigger forced snapshot refresh,
   - selection handlers clear stale pending lifecycle state.
5. Tests:
   - new integration suite `crates/hunk-jj/tests/jj_workspace_lifecycle.rs` covers:
     - create success,
     - duplicate-name rejection,
     - non-empty destination rejection without registration side effects,
     - forget success with non-destructive filesystem behavior,
     - current-workspace forget rejection.
   - extended desktop unit tests in `workspace_mode_tests` for name validation and destination-path derivation.
6. Deep review bugs found and fixed during phase:
   - fixed transaction panic on forget path by rebasing descendants before commit,
   - fixed async move/borrow bug in desktop forget confirmation task,
   - hardened workspace-name validation to reject both `/` and `\\` path separators.
7. Validation:
   - `cargo fmt --all -- --check` passed
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
   - `cargo test --workspace` passed

### Phase 7: Data Model Hardening (Comments + Future Agent Readiness)
- [x] Evaluate comment scoping model currently keyed by `bookmark_name`; define workspace-aware extension path.
- [x] Add internal context object suitable for future AI tab:
  - `workspace_name`,
  - workspace root path,
  - active bookmark,
  - working-copy commit ID.
- [x] Do not expose AI tab yet; only establish reliable context boundaries.
- [x] Add tests asserting context identity remains stable across snapshot refreshes and workspace switches.
- [x] Deep code review gate complete.
- [x] Global quality gate complete.

Acceptance criteria:
1. The codebase has a clean workspace-context abstraction reusable by future AI tab features.
2. Existing comment behavior remains backward-compatible for bookmark-only repositories.

Phase 7 review note (2026-03-03):
1. Added internal workspace context object on desktop state:
   - `WorkspaceExecutionContext { workspace_name, workspace_root, active_bookmark, working_copy_commit_id }`,
   - stored in `DiffViewer.workspace_execution_context`,
   - synchronized from snapshot state via dedicated controller helper.
2. Comment-scope hardening and extension path:
   - comment scope now derives from workspace context (`workspace_root` + bookmark key),
   - preserved existing DB scope key behavior (`bookmark_name` / `detached`) for backward compatibility,
   - centralized bookmark-scope derivation in workspace-context helper so future DB migration can add explicit workspace scoping without rewriting comment callers.
3. Snapshot/switch lifecycle integration:
   - workspace context is reset on project/workspace switches before refresh,
   - rehydrated on successful snapshot apply,
   - cleared on snapshot error/reset paths to avoid stale identity leakage.
4. Tests:
   - new controller unit tests in `workspace_context_tests` assert:
     - context identity stability for unchanged inputs,
     - identity changes across workspace switches,
     - bookmark-scope key behavior for detached vs active bookmark contexts.
5. Deep review outcomes:
   - verified no AI tab/UI exposure was added in this phase,
   - verified comments remain backward-compatible with existing bookmark-scoped data.
6. Validation:
   - `cargo fmt --all -- --check` passed
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
   - `cargo test --workspace` passed

### Phase 8: End-to-End Validation + Cleanup
- [x] Run full QA pass across Files, Review, and Graph modes after workspace features.
- [x] Remove temporary compatibility shims and dead code introduced during phased rollout.
- [x] Finalize docs updates:
  - workflow guide,
  - glossary updates,
  - migration notes.
- [x] Deep code review gate complete.
- [x] Global quality gate complete.

Acceptance criteria:
1. Graph workspace support is stable and production-ready.
2. Bookmark workflows remain fully functional and tested.
3. No known high-severity issues remain open.

Phase 8 review note (2026-03-03):
1. End-to-end QA coverage completed for core workspace rollout surfaces:
   - Files mode: `cargo test -p hunk-jj --test repo_tree`,
   - Review flows: `cargo test -p hunk-jj --test jj_branch_checkout_workflow`,
   - Graph/workspace flows: `cargo test -p hunk-jj --test jj_graph_snapshot --test jj_workspace_switch --test jj_workspace_lifecycle`.
2. Cleanup completed:
   - removed temporary desktop compatibility shim `workspace_execution_context_or_legacy`,
   - comment scope now uses synchronized `workspace_execution_context` only, preventing dual-path context derivation.
3. Documentation finalized in `docs/JJ_GRAPH_WORKSPACES_GUIDE.md`:
   - step-by-step workflow guide for mixed workspace/bookmark graph usage,
   - JJ glossary updates for workspace-vs-bookmark terms,
   - migration notes covering snapshot contract, desktop state, compatibility, and future DB migration path.
4. Deep review outcomes:
   - verified workspace actions remain explicitly guarded and separate from bookmark mutation surfaces,
   - verified bookmark workflows remain intact while workspace support is active,
   - verified no stale compatibility callsites remain in desktop controller code.
5. Validation:
   - `cargo fmt --all -- --check` passed
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
   - `cargo test --workspace` passed

## Execution Map (Expected Touch Points)
1. Backend model + snapshot:
   - `crates/hunk-jj/src/jj.rs`
   - `crates/hunk-jj/src/jj/backend/graph.rs`
   - `crates/hunk-jj/src/jj/backend/snapshot_diff.rs` (if helper reuse required)
2. Desktop state + controller:
   - `crates/hunk-desktop/src/app.rs`
   - `crates/hunk-desktop/src/app/controller/core.rs`
   - `crates/hunk-desktop/src/app/controller/jj_graph.rs`
   - `crates/hunk-desktop/src/app/controller/workspace_mode.rs`
3. Rendering:
   - `crates/hunk-desktop/src/app/render/jj_graph.rs`
   - `crates/hunk-desktop/src/app/render/jj_graph_inspector.rs`
   - `crates/hunk-desktop/src/app/render/root.rs` (if footer/status text changes)
4. Tests:
   - `crates/hunk-jj/tests/jj_graph_snapshot.rs`
   - new tests under `crates/hunk-jj/tests/` for workspace flows
   - desktop/controller tests where available

## Phase Completion Protocol
For each phase:
1. Mark implementation checkboxes complete.
2. Perform deep code review and address issues.
3. Run global quality gate commands and record pass/fail.
4. Add a short phase review note with:
   - major decisions,
   - bugs found and fixed during review,
   - refactors completed,
   - validation results.
5. Only then begin the next phase.
