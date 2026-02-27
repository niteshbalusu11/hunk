# JJ Workspace RFC

## Status
- Draft
- Owner: Hunk

## Why This RFC Exists
Hunk currently uses JJ as the backend but still presents a Git-first UX in key places. This causes user confusion because terms and interaction flow imply branch/index semantics that JJ does not use.

This RFC defines a JJ-native UX and migration plan that:
- preserves Hunk's diff-first value,
- aligns controls with JJ concepts (working copy, bookmarks, revisions),
- keeps Git interoperability for remotes and PR/MR workflows.

## Problem Statement
Current UX issues:
- Sidebar and commit footer use `branch` language while operations are bookmark-based.
- Users are pushed through a Git-like flow (`switch branch`, `push branch`, `sync branch`) instead of JJ mental model.
- Commit UX is partially JJ-native (no staging index) but still framed with Git wording and placement.

Impact:
- High cognitive load when reconciling UI labels with JJ behavior.
- Lower trust in workflows (users are unsure what the app will do).
- Feature growth gets harder because each new capability needs terminology workarounds.

## Product Goals
1. Make JJ workflows obvious without requiring CLI knowledge.
2. Keep diff review as the primary interaction surface.
3. Support Git remotes as transport and collaboration layer, not as core mental model.
4. Keep migration incremental and low-risk.

## Non-Goals
1. Recreate full JJ CLI surface in v1.
2. Remove Git compatibility.
3. Redesign diff rendering fundamentals.

## JJ Mental Model (Canonical In-App Model)
1. `Working Copy`: mutable state where edits happen.
2. `Revision`: a recorded change derived from working-copy state.
3. `Bookmark`: movable pointer to a revision stack.
4. `Remote`: publish/sync target for bookmarks.

Rule: UI must not introduce a concept that contradicts this model.

## Information Architecture
Introduce a dedicated `JJ Workspace` screen while keeping the current diff experience.

### Screen Regions
1. `Changes` pane (left): changed file list, include/exclude controls, working-copy status.
2. `Diff` pane (center): existing side-by-side diff viewer.
3. `JJ Control` pane (right or footer panel, depending on window width):
   - active bookmark selector,
   - bookmark list and create/rename actions,
   - publish/sync controls,
   - revision stack list for active bookmark.

### Mobile/Small Width Behavior
1. Keep diff as main surface.
2. JJ Control pane collapses into tabbed drawer (`Bookmarks`, `Stack`, `Remotes`).

## Core User Flows
1. Review and commit local changes:
   - select changed files/hunks,
   - enter message,
   - create revision,
   - working copy updates.
2. Activate or create bookmark:
   - choose bookmark from list or create new bookmark from input,
   - optional move-changes confirmation for non-clean working copy.
3. Publish bookmark:
   - publish if not tracked remotely,
   - push if tracked and ahead.
4. Sync bookmark:
   - fetch/import remote bookmark state,
   - update local view/stack,
   - surface conflicts with actionable guidance.
5. Open PR/MR:
   - after publish/push, open remote compare URL when available.

## Terminology Policy
Replace user-facing `branch` terms with `bookmark` terms.

Examples:
- `Switch branch` -> `Activate bookmark`
- `Publish branch` -> `Publish bookmark`
- `Push branch` -> `Push bookmark`
- `Sync branch` -> `Sync bookmark`
- `Select or create branch` -> `Select or create bookmark`

Notes:
- Internal API symbols can stay branch-shaped temporarily for compatibility.
- All user-visible strings must follow bookmark language.

## Git Interoperability Model
Git is treated as interop transport:
1. `publish/push` maps JJ bookmark updates to remote Git refs.
2. `sync` maps fetch/import to JJ view update.
3. PR/MR creation remains remote-provider behavior derived from published bookmark.

UI wording should never imply local Git branch checkout semantics.

## Architecture Changes
### Domain Layer (new)
Create JJ-focused view models for UI binding:
- `WorkingCopyState`
- `BookmarkState`
- `RevisionStackState`
- `RemoteSyncState`

### Existing Module Mapping
1. [src/jj.rs](/Volumes/hulk/dev/projects/hunk/src/jj.rs)
   - keep backend behavior,
   - progressively rename public APIs toward bookmark terminology where safe.
2. [src/app/controller/git_ops.rs](/Volumes/hulk/dev/projects/hunk/src/app/controller/git_ops.rs)
   - rename actions/messages to bookmark language,
   - split bookmark actions from commit actions.
3. [src/app/render/commit.rs](/Volumes/hulk/dev/projects/hunk/src/app/render/commit.rs)
   - convert branch picker UI to bookmark picker,
   - adjust labels/tooltips/buttons.
4. [src/app/render/tree.rs](/Volumes/hulk/dev/projects/hunk/src/app/render/tree.rs)
   - keep diff tree behavior,
   - add entry point to JJ workspace controls.
5. [src/app/controller/core.rs](/Volumes/hulk/dev/projects/hunk/src/app/controller/core.rs)
   - update input placeholders and state names incrementally.

## Rollout Plan
### Phase 1: Language and UI Copy Migration (no behavior changes)
- Update user-facing strings to bookmark terminology.
- Preserve current action semantics.
- Add regression tests for updated UI status messages where feasible.

### Phase 2: JJ Workspace Entry and Layout
- Add dedicated JJ workspace panel/screen.
- Move bookmark operations out of "Git footer" framing.

### Phase 3: Bookmark-Centric Workflows
- Bookmark list with active indicator and ahead/behind state.
- Rename bookmark flow.
- Move changes to selected bookmark via explicit UI action.

### Phase 4: Revision Stack View
- Show revisions for active bookmark.
- Support describe/edit message, abandon, and reorder/squash (scoped subset first).

### Phase 5: Remote Collaboration Layer
- Publish/push/sync refinements.
- PR/MR quick actions with remote URL integration.

### Phase 6: Legacy Flow Removal
- Remove old Git-centric entry points and copy.
- Keep migration shims only in internal APIs.

## Current Progress
- [x] Phase 1: Language and UI copy migrated to bookmark-centric wording (user-facing strings).
- [x] Phase 2: Dedicated JJ workspace surface implemented.
- [x] Phase 3: Bookmark workflows implemented:
  - [x] create/activate bookmark
  - [x] publish/push/sync bookmark
  - [x] rename bookmark flow
  - [x] explicit move-changes action when activating bookmark
- [~] Phase 4: Revision stack view and revision actions.
  - [x] read-only revision stack list for active bookmark
  - [~] revision actions
  - [x] describe/edit message for tip revision
  - [x] abandon tip revision
  - [x] squash tip revision into parent
  - [ ] reorder
- [~] Phase 5: PR/MR quick actions and remote collaboration refinements.
  - [x] copy review URL quick action from JJ workspace (GitHub/GitLab URL support)
  - [ ] additional remote collaboration refinements
- [ ] Phase 6: Full legacy flow removal and internal naming cleanup.

## Testing Strategy
1. Keep existing JJ workflow tests passing.
2. Add tests for bookmark-language flows and messaging.
3. Add integration tests for:
   - create/activate bookmark with dirty working copy,
   - publish then sync on tracked/untracked bookmarks,
   - remote selection fallback behavior.

## Telemetry and Success Metrics
1. Time-to-first-commit from opening repo.
2. Number of failed sync/publish attempts per active user.
3. Frequency of bookmark switching with dirty working copy.
4. Ratio of successful publish -> PR/MR action chain.

## Risks and Mitigations
1. Risk: user confusion during mixed terminology period.
   - Mitigation: complete Phase 1 quickly and consistently.
2. Risk: regressions from broad renaming.
   - Mitigation: keep behavior unchanged in Phase 1, test heavily.
3. Risk: overloading one panel with too many controls.
   - Mitigation: tabbed JJ control sections and responsive collapse.

## Acceptance Criteria
1. No user-facing action labels mention `branch` for JJ workflows.
2. Users can complete create/activate/publish/sync flows using bookmark terminology only.
3. Existing diff and commit behavior remains functionally unchanged.
4. `cargo check`, relevant tests, and clippy pass.
