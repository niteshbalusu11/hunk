# JJ Graph Workspace RFC + Execution TODO

## Status
- Proposed
- Owner: Hunk
- Last Updated: 2026-02-28

## Goal
Redesign the JJ Workspace into an interactive graph-first workflow inspired by `jj log`, where bookmarks and revisions are visual, clickable, and editable.

## Product Decisions (Confirmed)
1. Bookmark click should reveal that bookmark's revision chain in a focused side view.
2. UI should support creating/forking bookmarks from selected revisions.
3. Start with single selection only (no multi-select in v1).
4. Support both local and remote bookmarks with visible distinction.
5. PR/MR actions should open directly in browser, with copy URL as fallback.
6. Self-hosted GitLab must be supported (current host detection is too narrow).

## UX Model (V1)
1. Center: revision graph canvas (jj-log-like lanes and nodes).
2. Right: bookmark focus strip showing revisions for selected bookmark.
3. Right/bottom inspector: actions for selected node/bookmark.
4. Inline badges:
   - Local-only bookmark
   - Published/tracked remote bookmark
   - Needs push
   - Diverged/conflicted state

## Non-Goals (V1)
1. Multi-select actions.
2. Full parity with all `jj` CLI operations.
3. Complex hunk-level revision surgery.

## Architecture Plan
1. Backend: add graph snapshot API for DAG + bookmark metadata.
2. Controller: add graph selection/focus state and actions.
3. Render: split JJ graph UI into dedicated modules to keep files maintainable.
4. Tests: add graph snapshot + workflow interaction tests under `tests/`.

## Execution Checklist

### Phase 0: Spec + Scaffolding
- [x] Capture product decisions and interaction model.
- [x] Add this RFC to docs index/reference locations if needed.
- [ ] Define acceptance criteria for each phase in PR descriptions.
- [ ] Deep phase review gate: review all Phase 0 edits for clarity, consistency, and missing constraints before starting Phase 1.

### Phase 1: Graph Data Model (Backend)
- [x] Add graph-focused domain structs in `src/jj.rs`:
  - `GraphSnapshot`
  - `GraphNode`
  - `GraphEdge`
  - `GraphBookmarkRef`
- [x] Implement `load_graph_snapshot(...)` in `src/jj/backend/*`.
- [x] Include local + remote bookmark attachment metadata per node.
- [x] Include active bookmark + active working-copy context.
- [x] Add pagination/windowing support for large histories.
- [x] Add backend tests in `tests/jj_graph_snapshot.rs`.
- [x] Deep phase review gate: review all Phase 1 backend code for correctness, edge cases, performance, and refactor opportunities before Phase 2.

### Phase 2: Read-Only Graph UI
- [x] Create `src/app/render/jj_graph.rs` (graph canvas rendering).
- [x] Create `src/app/render/jj_graph_inspector.rs` (selection details/actions).
- [x] Wire new surface into JJ workspace screen.
- [x] Add single-select node/bookmark state in app/controller.
- [x] Show local/remote bookmark visual distinction.
- [x] Keep frame-time stable on medium/large repos (virtualized rows or windowed render).
- [x] Deep phase review gate: review all Phase 2 UI/controller code for bugs, rendering issues, and structural cleanup before Phase 3.

### Phase 3: Bookmark Focus Experience
- [x] Clicking bookmark chip focuses that bookmark.
- [x] Show focused bookmark revision chain in side strip.
- [x] Selecting revision in strip highlights/jumps in graph.
- [x] Add keyboard navigation for next/previous revision in focused bookmark.
- [x] Add clear "return to full graph" control.
- [x] Deep phase review gate: review all Phase 3 interaction logic for UX regressions, state bugs, and refactor needs before Phase 4.
- Phase 3 review note (2026-02-28): fixed a focus-strip navigation edge case where controls could disable when the selected graph node was outside the focused chain.

### Phase 4: Interactive Bookmark Actions
- [x] Create bookmark from selected revision.
- [x] Fork bookmark from selected bookmark/revision.
- [x] Rename bookmark inline from graph chip.
- [x] Move bookmark target via action menu (non-drag fallback path).
- [x] Add confirmation UX for destructive changes.
- [x] Add tests for create/fork/rename/move flows.
- [x] Deep phase review gate: review all Phase 4 command/action code for safety, error handling, and maintainability before Phase 5.
- Phase 4 review note (2026-02-28): verified move retargeting always requires explicit confirmation; reconciles stale pending confirmations after snapshot refresh; added API-level integration coverage for create/fork/rename/move flows.

### Phase 5: Drag + Drop Retargeting
- [x] Drag bookmark chip to another revision node to retarget.
- [x] Add drag preview + valid target highlighting.
- [x] Validate/reject illegal drops with clear error messages.
- [x] Add undo-oriented status messaging where feasible.
- [x] Add integration tests for drag/drop retarget behavior.
- [x] Deep phase review gate: review all Phase 5 DnD behavior for correctness, race conditions, UX edge cases, and refactors before Phase 6.
- Phase 5 review note (2026-02-28): validated DnD state reconciliation on snapshot/selection updates, enforced local-only and same-target drop rejection with clear status messages, and re-ran strict lint (`clippy -D warnings`) plus Phase 3-5 integration test slice (35/35 passing).

### Phase 6: Remote Workflow + PR/MR
- [x] Add direct `Open PR/MR` browser action from selected bookmark.
- [x] Keep `Copy Review URL` as secondary fallback.
- [x] Fix self-hosted provider detection:
  - avoid hardcoding only `gitlab`/`github` host substrings
  - support explicit provider mapping/config
- [x] Add tests for GitHub, GitLab SaaS, and self-hosted GitLab URL generation.
- [x] Surface remote tracking and push/sync state per bookmark chip.
- [x] Deep phase review gate: review all Phase 6 remote/URL handling for provider correctness, security, and refactor candidates before Phase 7.
- Phase 6 review note (2026-02-28): added Open PR/MR + Copy Review URL actions for active and selected local bookmarks, introduced `review_provider_mappings` config for self-hosted remotes, expanded URL-generation coverage (including self-hosted GitLab mapping), and verified with `cargo check`, strict clippy, and targeted integration tests (41/41 passing).

### Phase 7: Motion + Polish
- [x] Add subtle animation for bookmark focus transitions.
- [x] Add smooth expand/collapse animation for revision strip.
- [x] Add drag target pulse/feedback animation.
- [x] Ensure reduced-motion friendly behavior.
- [x] Run visual QA for light/dark themes and small window sizes.
- [x] Deep phase review gate: review all Phase 7 animation/polish code for performance, accessibility, and cleanup before Phase 8.
- Phase 7 review note (2026-02-28): added focus-label transition and focus-strip expand/collapse animations plus drag-target pulse feedback, gated all new motion behind `reduce_motion` settings (`~/.hunkdiff/config.toml` + Settings UI), verified style fallbacks for light/dark and narrow layouts via render-path audit, and re-ran `cargo check`, strict clippy, and graph/config test slice (17/17 passing).

### Phase 8: Validation + Release
- [x] `cargo check`
- [x] `cargo test`
- [x] `cargo clippy --all-targets --all-features -- -D warnings`
- [x] Update docs/screenshots for JJ graph workflow.
- [x] No feature flag rollout: hard-cut to graph-first JJ workspace (breaking changes allowed).
- [x] Deep phase review gate: perform final end-to-end code review across all phases, fix remaining bugs/refactors, then release.
- Phase 8 review note (2026-02-28): hard-cut legacy JJ workspace wrappers in favor of the graph-first shell, renamed remaining JJ panel surface to graph operations terminology, completed full validation (`cargo check`, full `cargo test` 138 passed/0 failed, strict clippy), and finalized rollout as a breaking change without feature flags.

## Acceptance Criteria (V1)
1. User can click any bookmark and view its revision chain.
2. User can distinguish local vs remote bookmarks visually.
3. User can create/fork/rename/move bookmarks from graph context.
4. User can open PR/MR directly in browser for supported remotes.
5. Self-hosted GitLab review URL path works with configured provider mapping.
6. Single-select workflow is stable, tested, and performant.

## Check-off Protocol
1. Mark a task complete only after code is merged and tests for that behavior exist.
2. For backend behavior changes, include at least one integration test in `tests/`.
3. Keep progress updates in this document and reference PR numbers next to completed items.
