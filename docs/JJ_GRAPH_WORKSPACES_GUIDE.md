# JJ Graph Workspaces Guide

## Status
- Active
- Last Updated: 2026-03-03
- Related:
  - `docs/JJ_GRAPH_WORKSPACES_TODO.md`
  - `docs/JJ_GRAPH_WORKSPACE_RFC.md`

## Workflow Guide
1. Read refs in graph:
   - `W <name>@` chips are workspace refs.
   - `L <name>` and `R <name>@<remote>` chips are bookmark refs.
2. Open the top-right bookmark/workspace menu:
   - in `Active Workflow` mode, click the active bookmark button (or chevron),
   - the expanded panel contains both bookmark actions and workspace actions.
3. Create a task workspace:
   - stay in `default@` workspace,
   - stay on trunk bookmark (`main` or `master`),
   - keep working copy clean (no pending files),
   - enter task name in the `Workspaces` section,
   - click `Create`.
4. What create does:
   - creates workspace at `.jj/workspaces/<task-name>`,
   - creates/activates same-name bookmark `<task-name>@`,
   - switches app context into that new workspace root.
5. Code in task workspace mode:
   - use normal working flow: edit, `Create Revision`, `Push Revisions`, `Open PR/MR`,
   - bookmark mutation actions are intentionally disabled in non-default workspaces.
6. Switch or forget existing workspaces:
   - click a `W <name>@` chip in graph to select target workspace,
   - use `Switch` or `Forget` in the same `Workspaces` panel,
   - confirm guarded prompts when shown.
7. Start next task:
   - switch back to `default@` + trunk,
   - repeat create flow for next workspace.

## Troubleshooting
1. `Create` is disabled:
   - not in `default@` workspace,
   - active bookmark is not `main` or `master`,
   - working copy is dirty,
   - task name is invalid as a workspace or bookmark name,
   - workspace name already exists,
   - another workspace action is pending.
2. `Switch` is disabled:
   - no workspace chip selected in graph,
   - selected workspace is already current,
   - another workspace action is pending.
3. `Forget` is disabled:
   - no workspace chip selected in graph,
   - selected workspace is current workspace,
   - another workspace action is pending.
4. Selected workspace vanished:
   - graph snapshot changed after operation,
   - reselect workspace chip from current graph state.
5. Unexpected large file changes after workspace create:
   - fixed in current implementation by materializing tracked files on workspace creation,
   - if seen on older builds, recreate workspace on latest build.

## Glossary
1. Workspace:
   - JJ working-copy context with its own mutable working-copy commit and root path.
2. Workspace Ref (`<workspace>@`):
   - graph-visible reference to a workspace working-copy commit.
3. Active Workspace:
   - current app editing context (commit/restore/edit operations execute here).
4. Bookmark:
   - movable named pointer used for collaboration and stack organization.
5. Active Bookmark:
   - currently activated local bookmark in active workspace context.
6. Detached:
   - no active bookmark is selected for current working-copy context.
7. Working-Copy Commit:
   - mutable commit representing uncommitted local state in a workspace.

## Migration Notes
1. Graph snapshot contract:
   - `GraphSnapshot` now carries `current_workspace_name` and `workspaces`,
   - each `GraphNode` now carries `workspaces: Vec<GraphWorkspaceRef>`.
2. Desktop state and controller:
   - app state tracks workspace list, current workspace, and selected workspace separately from bookmarks,
   - `WorkspaceExecutionContext` is synchronized from snapshot state and reused by comment scoping.
3. Compatibility behavior retained:
   - bookmark payload shape and bookmark workflows remain unchanged,
   - comment DB keys stay bookmark-scoped (`bookmark_name` or `detached`) to preserve existing data.
4. Phase 8 cleanup applied:
   - removed temporary compatibility shim `workspace_execution_context_or_legacy`,
   - comment scope now reads only the synchronized `workspace_execution_context`.
5. Future migration path:
   - if comments become workspace-explicit, add workspace identity to DB keys in a schema migration while preserving old rows for readback.
