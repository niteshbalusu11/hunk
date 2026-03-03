# JJ Task Workspace Mode TODO

## Status
- Proposed
- Owner: Hunk
- Last Updated: 2026-03-03 (Phase 4 complete)
- Related docs:
  - `docs/JJ_GRAPH_WORKSPACES_TODO.md`
  - `docs/JJ_GRAPH_WORKSPACES_GUIDE.md`

## Objective
Implement a strict task-workspace workflow in Graph View where workspace and bookmark are paired for feature work.

## Product Direction
1. Workspace creation is allowed only from trunk workflow context.
2. Creating a task workspace creates a same-name bookmark.
3. App switches into the new workspace immediately after creation.
4. While working in task workspace context, bookmark mutation UI is disabled to avoid mixed workflows.
5. Commit, push, and PR flow continue through active bookmark actions.

## Workflow Model
1. Start in default workspace on trunk bookmark (`main`/`master`) with clean working copy.
2. Enter task name and create workspace.
3. App performs:
   - create workspace at trunk target revision,
   - create/activate same-name bookmark in new workspace,
   - switch app context to new workspace root.
4. User writes code, commits, pushes bookmark, opens PR/MR.
5. Merge externally, then switch back to default workspace to start next task.

## Constraints
1. Workspace destination should be inside repository metadata area (`.jj/workspaces/<name>`).
2. No implicit workspace switch for existing operations.
3. No destructive cleanup of directories without explicit user action.
4. Keep existing bookmark workflows in default workspace intact.

## Quality Gates (Required After Every Phase)
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo test --workspace`
4. Deep code review of changed files before moving to next phase.

## Phase Plan

### Phase 0: Spec and Guardrail Definition
- [x] Document task-workspace semantics and constraints.
- [x] Define trunk-entry gating requirements.
- [x] Define workspace destination policy (`.jj/workspaces`).

Acceptance criteria:
1. Team has a single implementation plan for new workflow mode.
2. Gating rules and error states are explicit.

Review note (2026-03-03):
1. Agreed model is workspace+bookmark pairing for task execution.
2. Entry point is default workspace + trunk bookmark + clean working copy.
3. Workspace directories must live in `.jj/workspaces`.

### Phase 1: Entry Gating + Internal Destination Path
- [x] Add explicit task-workspace creation blocker API in desktop controller.
- [x] Enforce creation only when:
  - current workspace is `default`,
  - active bookmark is trunk (`main` or `master`),
  - working copy is clean,
  - no pending workspace switch/forget action.
- [x] Base task workspace creation on active trunk bookmark target revision (not arbitrary selected node).
- [x] Restore workspace destination derivation to `.jj/workspaces/<name>`.
- [x] Update UI tooltip/reason text to explain trunk-only entry.
- [x] Add/adjust unit tests for new blocker and destination policy.

Acceptance criteria:
1. Create action is blocked with clear reason outside trunk entry context.
2. New workspace roots are always under `.jj/workspaces`.

Review note (2026-03-03):
1. Added trunk/default/clean guardrails to `selected_graph_workspace_create_blocker`.
2. Removed selected-node dependency for task workspace creation; now resolves from active bookmark target.
3. Restored destination policy to `.jj/workspaces/<name>`.
4. Added unit coverage for gating and destination behavior.
5. Validation:
   - `cargo fmt --all -- --check` passed
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
   - `cargo test --workspace` passed

### Phase 2: Paired Create Flow (Workspace + Bookmark)
- [x] Extend create flow to run paired operations:
  - create workspace,
  - create/activate same-name bookmark in new workspace.
- [x] Switch app to new workspace root automatically on success.
- [x] Add robust error reporting and partial-failure handling.
- [x] Add integration coverage for paired creation behavior.

Acceptance criteria:
1. Successful create leaves user inside new workspace with matching active bookmark.
2. Failure path never silently leaves unknown state.

Review note (2026-03-03):
1. Task workspace creation now pairs workspace with same-name bookmark in one flow.
2. On success, app auto-switches to new workspace root and refreshes context.
3. On bookmark-pairing failure, flow attempts rollback via workspace forget and reports rollback errors explicitly.
4. Added preflight validation that task workspace names must be valid bookmark names.
5. Validation:
   - `cargo fmt --all -- --check` passed
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
   - `cargo test --workspace` passed

### Phase 3: Task Workspace Interaction Lockdown
- [x] Disable bookmark mutation actions in non-default task workspaces:
  - create/fork/rename/move bookmark.
- [x] Keep commit/push/review actions enabled for active bookmark.
- [x] Show concise “Task Workspace Mode” explanation in right panel.
- [x] Add tests for mutation blocker behavior in non-default workspaces.

Acceptance criteria:
1. Users cannot branch-hop inside task workspace mode via graph mutations.
2. Standard coding flow (commit/push/PR) remains intact.

Review note (2026-03-03):
1. Bookmark mutation blockers now apply in any non-default workspace context.
2. Graph mutation paths and bookmark picker create/rename controls surface clear blocker reasons.
3. Active workflow commit/push/review actions were left unchanged and available.
4. Added controller tests for non-default/default workspace mutation blocker behavior.
5. Validation:
   - `cargo fmt --all -- --check` passed
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
   - `cargo test --workspace` passed

### Phase 4: UX Pass + Documentation
- [x] Remove duplicate/ambiguous workspace action placements.
- [x] Ensure task workspace controls are visible near primary active-workflow actions.
- [x] Update `docs/JJ_GRAPH_WORKSPACES_GUIDE.md` with task workflow walkthrough.
- [x] Add troubleshooting section (missing trunk, dirty tree, duplicate names, stale workspace refs).

Acceptance criteria:
1. Users can discover and execute the task workspace flow without scrolling confusion.
2. Docs match UI behavior exactly.

Review note (2026-03-03):
1. Workspace quick actions were moved under the top bookmark/workspace picker panel in Active Workflow mode.
2. Duplicate workspace panel placement in the lower active-workflow stack was removed.
3. Pending workspace switch/forget prompts now auto-open the bookmark/workspace panel so confirm actions stay visible.
4. Bookmark picker copy now explicitly advertises workspace actions.
5. Guide updated to reflect trunk-gated create flow, paired bookmark behavior, and troubleshooting states.
6. Validation:
   - `cargo fmt --all -- --check` passed
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
   - `cargo test --workspace` passed

## Open Decisions
1. Auto-publish behavior:
   - Option A: create local bookmark only, publish on first explicit push.
   - Option B: auto-publish immediately during workspace creation.
2. Trunk configurability:
   - Option A: fixed `main|master`.
   - Option B: configurable trunk bookmark in app config.

## Immediate Next Step
Resolve open decisions (auto-publish and trunk configurability) before broad rollout.
