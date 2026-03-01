# JJ Tree Mode Implementation Plan

Date: 2026-03-01
Owner: Codex
Scope: JJ Workspace graph only. Do not modify Diff view or File view workflows.

## Goals

- Replace the current linear graph rendering with a true tree/lane graph model.
- Keep JJ-native semantics: revisions are immutable nodes, bookmarks are movable pointers.
- Improve graph readability and visual polish while keeping interactions explicit and stable.
- Remove ambiguity between "viewing" and "active workflow" actions.

## Non-goals

- No drag-and-drop redesign in this iteration.
- No changes to diff/file mode logic.
- No backend storage format changes.

## Phase Checklist

### Phase 1: Spec and UX Contract

- [x] Document tree-mode behavior, lane semantics, and visual language.
- [x] Define explicit acceptance criteria for branch/merge/active tip rendering.
- [x] Define empty/edge-window behavior (parents outside loaded graph window).
- [x] Deep code review (phase gate):
  - [x] Audit touched docs/requirements for contradictions.
  - [x] Validate JJ mental-model alignment.
  - [x] Refine ambiguous wording before coding.

### Phase 2: Lane Layout Engine (Data Layer)

- [x] Implement deterministic lane assignment for graph rows.
- [x] Produce per-row render hints (lane index, top/bottom verticals, merge connectors).
- [x] Handle merges and truncated parents safely.
- [x] Add integration tests in `tests/` for:
  - [x] Linear chain
  - [x] Branch and merge
  - [x] Missing parent in window
- [x] Deep code review (phase gate):
  - [x] Validate algorithm invariants and determinism.
  - [x] Remove dead/duplicated logic.
  - [x] Check naming/API clarity.

### Phase 3: Tree Renderer in Graph Canvas

- [x] Render lane gutter with true lane signals (vertical continuity + merge connectors).
- [x] Keep node selection and bookmark chips fully functional.
- [x] Ensure performance remains acceptable with list virtualization.
- [x] Deep code review (phase gate):
  - [x] Verify render logic for edge cases.
  - [x] Check no regressions in selection/scroll behavior.
  - [x] Refactor repeated UI code.

### Phase 4: Bookmark UX Integration

- [x] Keep explicit move flow (menu + confirm) and make messaging tree-aware.
- [x] Ensure bookmark status visibility (local/remote/tracked/conflict) remains clear.
- [x] Ensure active/selected bookmark context is unambiguous.
- [x] Deep code review (phase gate):
  - [x] Verify action enable/disable correctness.
  - [x] Verify no stale state when snapshot refreshes.
  - [x] Improve copy/help text consistency.

### Phase 5: Visual Polish Pass

- [x] Refine spacing, typography, and hierarchy in graph rows and headers.
- [x] Improve contrast and lane legibility in both light and dark themes.
- [x] Keep controls resilient in narrow panel sizes.
- [x] Deep code review (phase gate):
  - [x] Check visual consistency and accessibility contrast.
  - [x] Remove styling noise / over-complexity.
  - [x] Confirm reduced-motion behavior still respected.

### Phase 6: Regression Hardening

- [x] Run `cargo fmt --all`.
- [x] Run `cargo check`.
- [x] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] Run `cargo test`.
- [x] Deep code review (phase gate):
  - [x] Re-read all touched files for bugs and refactor opportunities.
  - [x] Ensure no file exceeds maintainability constraints due to changes.
  - [x] Final cleanup of wording and dead code.

## Acceptance Criteria

- Graph visually communicates branching/merging as lanes, not just a linear list.
- Active/selected bookmark workflows remain explicit and predictable.
- Bookmark retargeting remains available via explicit controls and confirmation.
- Diff and file modes remain unchanged.
- Build, lint, and tests pass cleanly.
