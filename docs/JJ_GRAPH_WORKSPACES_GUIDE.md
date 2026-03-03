# JJ Graph Workspaces Guide

## Status
- Active
- Last Updated: 2026-03-03
- Related:
  - `docs/JJ_GRAPH_WORKSPACES_TODO.md`
  - `docs/JJ_GRAPH_WORKSPACE_RFC.md`

## Workflow Guide
1. Explore mixed refs in graph:
   - `W <name>@` chips are workspace refs.
   - `L <name>` and `R <name>@<remote>` chips are bookmark refs.
2. Inspect workspace safely:
   - click a workspace chip to inspect workspace metadata,
   - bookmark mutation actions are blocked while workspace focus is active.
3. Switch workspace explicitly:
   - use `Switch Workspace` in inspector,
   - if the working copy is dirty, confirm the guarded switch card before continuing.
4. Create workspace from graph revision:
   - select a revision node,
   - enter workspace name in inspector input,
   - run `Create Workspace`.
5. Forget workspace safely:
   - select a non-current workspace chip,
   - run `Forget Workspace`,
   - confirm in the explicit confirmation card.
6. Continue bookmark workflows unchanged:
   - create/fork/rename/move/publish/sync/review URL still operate through bookmark selection.

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
