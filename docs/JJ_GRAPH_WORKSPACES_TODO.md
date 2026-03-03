# JJ Graph Workspaces TODO

## Status
- Proposed
- Owner: Hunk
- Last Updated: 2026-03-03
- Related docs:
  - `docs/JJ_GRAPH_WORKSPACE_RFC.md`
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
- [ ] Add new domain types in `crates/hunk-jj/src/jj.rs`:
  - `GraphWorkspaceRef`
  - `GraphWorkspaceStatus` (if stale/missing is represented)
- [ ] Extend `GraphNode` with workspace attachments.
- [ ] Extend `GraphSnapshot` with current workspace metadata and optional workspace summary list.
- [ ] Update `build_graph_snapshot_from_context()` to attach workspace refs to graph nodes.
- [ ] Include workspace working-copy commits in seed selection logic for graph windowing.
- [ ] Preserve bookmark ordering and existing remote/local bookmark metadata.
- [ ] Add/adjust tests in `crates/hunk-jj/tests/jj_graph_snapshot.rs`:
  - mixed bookmark/workspace graph,
  - multiple workspace refs visible,
  - current workspace marker correctness,
  - no bookmark metadata regression.
- [ ] Deep code review gate complete.
- [ ] Global quality gate complete.

Acceptance criteria:
1. Snapshot payload contains enough info to render workspace chips without extra JJ queries.
2. Existing bookmark behavior remains unchanged in tests.

### Phase 2: Desktop State Integration
- [ ] Update app state in `crates/hunk-desktop/src/app.rs` to track:
  - current workspace name,
  - selected workspace ref (if any),
  - workspace-focused revision IDs (if needed).
- [ ] Update snapshot application logic in `crates/hunk-desktop/src/app/controller/core.rs`:
  - map new graph payload fields,
  - reconcile workspace selections after refresh/window pagination.
- [ ] Ensure no cross-contamination between selected bookmark state and selected workspace state.
- [ ] Add controller-level tests for state reconciliation and mode fallbacks.
- [ ] Deep code review gate complete.
- [ ] Global quality gate complete.

Acceptance criteria:
1. Snapshot refresh does not leave invalid workspace/bookmark selections.
2. Current workspace remains the only editing context in state.

### Phase 3: Graph Rendering for Workspaces (Read-Only)
- [ ] Update `crates/hunk-desktop/src/app/render/jj_graph.rs` to render workspace chips per node.
- [ ] Create clear visual distinction:
  - bookmark chip style remains unchanged,
  - workspace chip style is unique and semantically labeled (`<workspace>@`).
- [ ] Add current workspace highlight token.
- [ ] Update legend and glossary text to explain bookmark vs workspace differences.
- [ ] Ensure narrow-width wrapping and scrolling remain stable.
- [ ] Add render/controller tests for:
  - chip labeling,
  - selection behavior,
  - no accidental bookmark action activation from workspace selection.
- [ ] Deep code review gate complete.
- [ ] Global quality gate complete.

Acceptance criteria:
1. User can visually identify local bookmark, remote bookmark, and workspace refs at a glance.
2. Workspace selection is inspect-only in this phase.

### Phase 4: Workspace Inspector + Safe Actions (No Switching Yet)
- [ ] Extend inspector (`crates/hunk-desktop/src/app/render/jj_graph_inspector.rs`) to show selected workspace details:
  - workspace name,
  - target commit,
  - whether it is current workspace.
- [ ] Add guarded actions that are safe in v1:
  - copy workspace name,
  - focus workspace commit in graph.
- [ ] Add disabled placeholders for future operations with clear reason text.
- [ ] Ensure existing bookmark action block remains scoped to bookmark selection only.
- [ ] Add tests for guard-rail behavior and disabled-reason text.
- [ ] Deep code review gate complete.
- [ ] Global quality gate complete.

Acceptance criteria:
1. Workspace and bookmark action surfaces are clearly separated.
2. No mutation action runs when a workspace ref is selected.

### Phase 5: Explicit Workspace Switching Flow
- [ ] Introduce backend APIs in `hunk-jj` for explicit workspace switch/open semantics.
- [ ] Handle compatibility for JJ versions where workspace root/path capabilities differ.
- [ ] Add explicit confirmation flow in UI before leaving current workspace context.
- [ ] Preserve dirty-state guard behavior; never lose local changes silently.
- [ ] Reinitialize repository watch/snapshot tasks safely after switch.
- [ ] Add integration tests:
  - switch with clean working copy,
  - switch with dirty working copy (guarded),
  - stale/missing workspace handling.
- [ ] Deep code review gate complete.
- [ ] Global quality gate complete.

Acceptance criteria:
1. Switching workspace is always explicit and auditable.
2. Dirty working-copy protection is at least as safe as current bookmark-switch protection.

### Phase 6: Workspace Lifecycle (Create/Forget) from Graph UX
- [ ] Add controlled UI flows for:
  - create workspace from selected revision/bookmark context,
  - forget workspace with clear warnings.
- [ ] Validate workspace naming constraints and duplicate handling.
- [ ] Update graph refresh pipeline to reconcile newly created/forgotten workspace refs.
- [ ] Add integration tests for lifecycle operations and rollback-safe errors.
- [ ] Deep code review gate complete.
- [ ] Global quality gate complete.

Acceptance criteria:
1. Workspace create/forget works without breaking bookmark graph workflows.
2. Error messages are clear for duplicate/stale/invalid targets.

### Phase 7: Data Model Hardening (Comments + Future Agent Readiness)
- [ ] Evaluate comment scoping model currently keyed by `bookmark_name`; define workspace-aware extension path.
- [ ] Add internal context object suitable for future AI tab:
  - `workspace_name`,
  - workspace root path,
  - active bookmark,
  - working-copy commit ID.
- [ ] Do not expose AI tab yet; only establish reliable context boundaries.
- [ ] Add tests asserting context identity remains stable across snapshot refreshes and workspace switches.
- [ ] Deep code review gate complete.
- [ ] Global quality gate complete.

Acceptance criteria:
1. The codebase has a clean workspace-context abstraction reusable by future AI tab features.
2. Existing comment behavior remains backward-compatible for bookmark-only repositories.

### Phase 8: End-to-End Validation + Cleanup
- [ ] Run full QA pass across Files, Review, and Graph modes after workspace features.
- [ ] Remove temporary compatibility shims and dead code introduced during phased rollout.
- [ ] Finalize docs updates:
  - workflow guide,
  - glossary updates,
  - migration notes.
- [ ] Deep code review gate complete.
- [ ] Global quality gate complete.

Acceptance criteria:
1. Graph workspace support is stable and production-ready.
2. Bookmark workflows remain fully functional and tested.
3. No known high-severity issues remain open.

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
