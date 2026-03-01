# JJ Native Mode UI Spec (Inspired by GG)

## Status
- Implemented
- Owner: Hunk
- Last Updated: 2026-03-01

## Scope
Workspace-only migration to a JJ-native mental model.

Out of scope:
- Diff view behavior changes
- File view behavior changes

## Principles
1. Revisions are graph nodes; bookmarks are pointers.
2. Working copy (`@`) is mutable and distinct from committed revisions.
3. Viewing a bookmark is separate from activating a bookmark for work.
4. Remote readiness and reviewability must be explicit in the UI.
5. Switching bookmarks with local changes must never feel like data loss.

## Shipped Interaction Model
1. Two explicit right-panel modes:
   - `Active Workflow`
   - `Selected Bookmark`
2. Bookmark single-click selects/explores only.
3. Bookmark double-click activates (local bookmarks only) with dirty-switch guard.
4. Dirty-switch guard options:
   - `Move Changes to Target`
   - `Snapshot Here, Then Switch`
   - `Cancel`
5. Recovery card in active workflow with:
   - metadata (source/destination/files/time)
   - `Restore Captured Changes`
   - `Discard Recovery Record`
6. PR/MR actions are gated with inline reasons (published + non-empty reviewable state).
7. Graph includes contextual drag/drop help and live drop-result hints.
8. JJ glossary is available in workspace panel (`JJ Terms`).

## Execution TODO (Completed)

### Phase A: Mode Clarity Hardening
- [x] Ensure single-click bookmark enters `Selected Bookmark` mode only.
- [x] Add `Activate This Bookmark` explicit mode transition.
- [x] Add bookmark double-click activation shortcut with same guard logic.
- [x] Add mode headers with one-line intent text in each mode.
- [x] Deep phase review gate complete.
- Phase A review note (2026-03-01): mode transitions now separate selection vs activation; activation paths route through one guarded switch flow.

### Phase B: Dirty Switch + Recovery Visibility
- [x] Implement explicit dirty-switch guard dialog with 3 choices.
- [x] Show recovery card consistently in `Active Workflow` when candidate exists.
- [x] Add `Discard Recovery Record` action.
- [x] Improve recovery status copy with source/destination/timestamp metadata.
- [x] Deep phase review gate complete.
- Phase B review note (2026-03-01): bookmark switch no longer performs implicit behavior when working copy is dirty; recovery controls are now explicit and reversible.

### Phase C: DnD Semantics + Discoverability
- [x] Add hover affordances that describe drop result before release.
- [x] Add a "How drag-and-drop works" contextual help panel.
- [x] Enforce drop rejection messages with exact reason.
- [x] Instrument DnD errors and aborted drops for UX tuning.
- [x] Deep phase review gate complete.
- Phase C review note (2026-03-01): live drop hints now explain valid/invalid targets and logs capture canceled/rejected drag flows.

### Phase D: PR/MR Safety + Remote Clarity
- [x] Gate `Open PR/MR` on non-empty reviewable state and remote readiness.
- [x] Show inline reason when disabled (not published, no new revisions, etc.).
- [x] Keep title prefill fallback chain deterministic.
- [x] Preserve self-hosted provider mapping behavior for open/copy URL flows.
- [x] Deep phase review gate complete.
- Phase D review note (2026-03-01): active and selected bookmark review actions now use explicit blockers and surface reasons directly in UI.

### Phase E: Label/Tooltip Cleanup
- [x] Rename ambiguous action labels to JJ-native wording.
- [x] Add/standardize tooltips for graph/bookmark/revision actions.
- [x] Add compact in-app JJ terms glossary entry point.
- [x] Deep phase review gate complete.
- Phase E review note (2026-03-01): `Edit Tip Revision` renamed to `Edit Working Revision`; glossary and tooltip coverage reduce JJ terminology ambiguity.

### Phase F: Validation + Release
- [x] `cargo fmt --all`
- [x] `cargo clippy --all-targets --all-features -- -D warnings`
- [x] `cargo test`
- [x] Manual QA pass for mode switching, dirty-switch flow, recovery flow, review-action gating, and narrow layouts.
- [x] Deep phase review gate complete.
- Phase F review note (2026-03-01): full workspace migration validated; no diff/file view modifications introduced.
