# Hunk Editor Replacement Implementation Plan

Date: 2026-03-18
Owner: Codex
Status: Proposed
Scope: Replace the Helix-backed Files tab editor with a Hunk-owned editor stack that uses Tree-sitter for syntax highlighting, keeps Hunk's existing Files workflow, and removes Helix completely.

## Decision

We are going to replace the current Helix-backed Files editor with a Hunk-owned editor stack.

This is a breaking change by design.

The new stack should:

- keep Hunk's existing Files tab shell and controller authority
- move editor engine concerns out of `hunk-desktop`
- use Tree-sitter for syntax highlighting and language-aware structure
- be GPUI-native at the rendering layer
- remove Helix runtime bundling, Helix dependencies, and Helix key handling entirely

We should treat Zed as an architectural reference, not as code to copy verbatim, unless Hunk intentionally accepts GPL obligations for the copied code.

## Why We Are Doing This

The current Helix integration delivered a stronger editor surface quickly, but it now creates the wrong long-term shape for Hunk.

Problems with the current approach:

- build times and dependency weight increased materially
- Hunk now carries a terminal editor subsystem inside a GPUI app
- packaging has to bundle Helix runtime assets and discover `HELIX_RUNTIME`
- keybinding behavior is tied to Helix's modal model
- editor-specific logic is concentrated inside `hunk-desktop`, making ownership and testing harder

Reasons to replace it:

- Hunk wants a file viewer/editor experience that looks and feels native to the app
- Hunk should own the editor model, rendering, and UX decisions
- Tree-sitter gives us the syntax and structure we need without embedding a full external editor
- a dedicated editor stack lets us optimize for Files-tab workflows, diff-awareness, and future Hunk-specific features

## What We Are Building

We are building a Hunk-owned editor stack with 3 new crates and a thin GPUI integration layer.

Target crates:

- `crates/hunk-text`
  Buffer model, rope-backed snapshots, selections, anchors, transactions, search primitives, undo/redo.
- `crates/hunk-language`
  Language registry, grammar/query assets, Tree-sitter parsing, syntax layers, injections, highlight capture mapping, fold candidates, outline extraction.
- `crates/hunk-editor`
  Editor state, viewport, display map, wrapping, folding, selections, cursors, search highlights, diagnostics overlays, editor commands.
- `crates/hunk-desktop`
  GPUI rendering, hit testing, keyboard/mouse translation, clipboard integration, and Files-tab integration with Hunk's existing controllers.

This split follows the same broad layering that makes Zed effective:

- text layer
- language layer
- display/editor layer
- UI layer

## Non-Goals For The Initial Rewrite

These are explicitly not required for the first usable replacement:

- Vim mode
- multi-buffer editor architecture
- plugin system
- remote collaboration
- full Zed feature parity on day one
- support for every Tree-sitter grammar in the ecosystem

The first target is a strong single-file editor/viewer for the Files tab that is fast, polished, and maintainable.

## Architectural Principles

### Keep Hunk In Charge Of File Lifecycle

Hunk should continue owning:

- selected file path
- file tree state
- file switching logic
- unsaved-change guardrails
- save and reload actions
- markdown preview mode
- Files workspace layout

The editor stack should own:

- buffer state
- cursor and selection state
- viewport state
- incremental syntax state
- editor-local commands
- visual layout state

### Keep The Core Headless

`hunk-text`, `hunk-language`, and `hunk-editor` should not depend on GPUI widget code.

That gives us:

- cheaper tests
- cleaner crate boundaries
- fewer UI-driven abstractions leaking into the editor model
- the option to benchmark and fuzz core behavior independently

### Tree-Sitter First, Theme Driven

Syntax highlighting should be based on:

- Tree-sitter grammars
- highlight queries
- injection queries
- locals queries where useful
- theme-owned capture-name mapping

We should not build the new editor around `syntect`.

`syntect` has since been removed from Hunk. Any future syntax work should stay on the Tree-sitter path rather than reintroducing a second highlighting stack.

### Rendering Quality Matters As Much As Parsing

To get a viewer that feels as good as Zed or VS Code, we must own:

- line layout
- visible-range virtualization
- active line styling
- gutter and line number styling
- search match styling
- diagnostics and diff overlays
- selection and cursor painting
- whitespace and indent-guide rendering
- scroll behavior

We should not render code as a row made of many small token elements.
We should render shaped text layouts with highlight spans and paint overlays in layers.

## Syntax Highlighting Design

### Language Assets

`hunk-language` should vendor a curated first-party grammar set instead of bundling an external runtime directory.

Initial language set:

- Rust
- TypeScript / TSX
- JavaScript / JSX
- JSON
- YAML
- TOML
- Markdown
- HTML
- CSS
- Go
- Python
- Bash / shell

Each supported language needs:

- grammar source or pinned crate dependency
- highlight query
- injection query if needed
- locals query if needed
- file matcher metadata

### Parse Model

Each open buffer should own:

- current text snapshot
- current syntax snapshot
- root language assignment
- injected language layers
- parse status

After edits:

- update the text snapshot immediately
- interpolate the previous syntax tree synchronously
- allow a very small synchronous parse budget
- continue parsing in the background if needed
- publish a new syntax snapshot when parsing completes

This keeps the editor responsive while still converging toward a stable syntax tree.

### Highlight Model

Highlighting should work like this:

1. Determine the visible byte range plus overscan.
2. Query Tree-sitter captures only for that range.
3. Resolve capture names to theme styles using longest-match semantics.
4. Convert highlighted chunks into `TextRun` or equivalent layout spans.
5. Merge higher-priority overlays after syntax:
   search hits, selection, diagnostics, diff overlays, matching brackets, active line.

The theme mapping should be capture-name driven, not token-enum driven.

Examples of theme keys:

- `keyword`
- `string`
- `type`
- `function`
- `function.method`
- `variable`
- `variable.builtin`
- `comment`
- `constant`
- `property`
- `tag`
- `attribute`

## Display And Rendering Design

`hunk-editor` should own a display map layer that transforms buffer text into rendered rows.

Core display layers:

- fold map
- wrap map
- tab / invisibles map
- search highlight map
- diagnostic marker map
- diff overlay map

The display map should answer:

- which rows are visible
- which buffer ranges belong to each visible row
- where fold placeholders appear
- where wraps occur
- where overlays should be painted

The GPUI renderer in `hunk-desktop` should:

- request only visible rows plus overscan
- cache shaped text layouts per visible row
- paint backgrounds and overlays separately from text
- keep gutter rendering independent of text layout
- avoid large element trees for syntax fragments

## Target User-Facing Milestones

The replacement is considered successful when the Files tab supports:

- fast open and scroll for large files
- good syntax highlighting for the initial language set
- polished line numbers, gutter, active line, and selection rendering
- keyboard and mouse editing with standard bindings
- dirty state and save/reload parity
- markdown preview parity
- no Helix runtime dependency

Later milestones should add:

- folding
- find in file
- bracket matching
- indent guides
- diagnostics
- semantic tokens
- hover / go-to-definition / completions

## Phased Plan

## Phase 0: Alignment And Cleanup Boundary

What we are doing:

- establish the rewrite boundary
- document ownership
- stop adding new Helix-only behavior

Why we are doing it:

- the rewrite will fail if Helix and the new editor continue growing at the same time
- we need a stable seam before moving code into new crates

Todo:

- [ ] Freeze new feature work in the Helix editor path except for necessary fixes.
- [ ] Identify the smallest stable contract Hunk needs from the editor surface:
  `open_document`, `clear`, `current_text`, `is_dirty`, `focus`, `copy`, `cut`, `paste`, `scroll`, `status_snapshot`.
- [ ] Mark the current Helix document as superseded by this plan once the rewrite starts.
- [ ] Decide whether the old Helix implementation will live behind a temporary Cargo feature during migration or be removed immediately after phase 4.
- [ ] Review: inspect the existing Files-tab controller boundary for unnecessary editor coupling and note anything that should move out of `hunk-desktop` before implementation.

## Phase 1: Create The New Core Crates

What we are doing:

- add `hunk-text`, `hunk-language`, and `hunk-editor`
- define the public APIs and core data types

Why we are doing it:

- the rewrite needs a real architecture, not another large `hunk-desktop` subsystem
- crate boundaries make the core testable and keep GPUI concerns isolated

Todo:

- [ ] Create `crates/hunk-text` with crate-level tests.
- [ ] Create `crates/hunk-language` with crate-level tests.
- [ ] Create `crates/hunk-editor` with crate-level tests.
- [ ] Define `hunk-text` primitives:
  `BufferId`, `TextBuffer`, `TextSnapshot`, `Anchor`, `Selection`, `Transaction`, `SearchQuery`.
- [ ] Define `hunk-language` primitives:
  `LanguageId`, `LanguageDefinition`, `LanguageRegistry`, `SyntaxSnapshot`, `HighlightCapture`, `FoldCandidate`.
- [ ] Define `hunk-editor` primitives:
  `EditorState`, `Viewport`, `DisplaySnapshot`, `EditorCommand`, `EditorStatusSnapshot`.
- [ ] Add basic architecture docs in crate roots so responsibilities stay explicit.
- [ ] Wire the new crates into the workspace without changing runtime behavior yet.
- [ ] Review: audit the initial crate APIs for GPUI leakage, circular ownership, and over-generalization before moving to implementation.

## Phase 2: Build `hunk-text`

What we are doing:

- implement the text buffer core
- support snapshots, edits, selections, and undo/redo

Why we are doing it:

- every later phase depends on stable text snapshots and selection math
- bugs here will contaminate parsing, rendering, and save behavior

Todo:

- [ ] Choose the rope implementation and commit to it for the first version.
- [ ] Implement text snapshots with cheap cloning.
- [ ] Implement UTF-8 and line/column conversions.
- [ ] Implement anchors that survive edits across snapshots.
- [ ] Implement primary selection and range selection support.
- [ ] Implement transaction-based edits.
- [ ] Implement undo and redo.
- [ ] Implement search primitives that can power incremental find later.
- [ ] Add property-style tests for edit application, anchor preservation, and selection movement.
- [ ] Add large-file smoke tests for snapshot cloning and edit performance.
- [ ] Review: inspect the buffer API for hidden allocation cliffs, unstable coordinate systems, and unclear edit semantics.

## Phase 3: Build `hunk-language`

What we are doing:

- implement language registry and Tree-sitter integration
- support incremental parsing, injections, and highlight capture iteration

Why we are doing it:

- this is the foundation for syntax highlighting, fold ranges, and future language-aware features
- we need a Hunk-owned replacement for Helix runtime data

Todo:

- [ ] Create a curated language asset layout under `hunk-language`.
- [ ] Define how grammars and query files are loaded at build time or packaged at runtime.
- [ ] Implement `LanguageRegistry` with file matcher support.
- [ ] Implement root-language assignment for file paths.
- [ ] Implement syntax snapshots and incremental reparse scheduling.
- [ ] Implement a small synchronous parse budget plus background parse continuation.
- [ ] Implement injection layers for at least Markdown and HTML-style embedded languages.
- [ ] Implement highlight capture iteration for a visible byte range.
- [ ] Implement capture-name to theme-style mapping with longest-match behavior.
- [ ] Implement fold candidate extraction from syntax trees.
- [ ] Implement lightweight outline extraction for symbols where query support exists.
- [ ] Add tests for:
  language detection, incremental edits, injections, highlight captures, and fold extraction.
- [ ] Review: audit grammar/query loading, parse invalidation, and injection behavior for correctness and dead complexity.

## Phase 4: Build `hunk-editor` Display And Command Layers

What we are doing:

- implement editor-local state and display transformations
- support scrolling, wrapping, folding, and status computation

Why we are doing it:

- syntax parsing alone is not enough; editor quality depends on display modeling
- the display map is the seam between the core and GPUI painting

Todo:

- [ ] Implement `EditorState` around a single active text buffer.
- [ ] Implement viewport state and visible-row calculation.
- [ ] Implement wrap logic for editor width.
- [ ] Implement tab expansion and whitespace rendering metadata.
- [ ] Implement fold placeholders and folded-row projection.
- [ ] Implement status snapshot calculation:
  language, line/column, selection info, dirty state.
- [ ] Implement editor commands for movement, selection, insert, delete, replace, copy, cut, paste, undo, redo.
- [ ] Implement search highlight state for the visible range.
- [ ] Implement overlay descriptors for diagnostics and diff-aware decorations.
- [ ] Add tests for:
  wrapping, folding, cursor movement across folds/wraps, and display snapshot stability.
- [ ] Review: inspect the display-map layering for invalidation bugs, duplicated transforms, and APIs that are too UI-shaped.

## Phase 5: GPUI Read-Only Viewer Integration

What we are doing:

- add a new GPUI file viewer element in `hunk-desktop`
- render the new stack without enabling editing yet

Why we are doing it:

- the fastest path to visible progress is a read-only viewer with excellent rendering
- this de-risks layout, virtualization, and syntax styling before edit behavior is mixed in

Todo:

- [ ] Add a new `FilesEditorElement` backed by `hunk-editor`, separate from the Helix element.
- [ ] Implement row virtualization with overscan.
- [ ] Implement shaped text layout caching per visible row.
- [ ] Paint editor background, active line, line numbers, gutter separators, and syntax text.
- [ ] Paint search match backgrounds and selection backgrounds.
- [ ] Implement mouse hit testing for row/column location.
- [ ] Implement wheel scrolling and viewport updates.
- [ ] Integrate with existing Files-tab theming using `theme.rs` colors only.
- [ ] Add large-file profiling for open and scroll behavior.
- [ ] Review: inspect rendering for element-tree bloat, missing cache invalidation, and visual inconsistencies with Hunk's theme system.

## Phase 6: Editing And Files-Tab Parity

What we are doing:

- enable real editing on the new stack
- match current Files-tab save/reload and dirty-state behavior

Why we are doing it:

- the new viewer is not done until it can replace Helix in the Files workflow
- parity needs to be reached before Helix can be deleted

Todo:

- [ ] Implement text insertion, deletion, backspace, newline, indentation, and selection replacement.
- [ ] Implement standard desktop keybindings first, without modal behavior.
- [ ] Implement clipboard copy, cut, and paste.
- [ ] Implement undo and redo wiring through the controller.
- [ ] Wire `current_text`, `is_dirty`, `clear`, and `status_snapshot` into the existing controller path.
- [ ] Keep markdown preview mode working through the existing preview pipeline.
- [ ] Implement file reload behavior while preserving selection and viewport where reasonable.
- [ ] Implement unsaved-change guardrails using the existing controller authority.
- [ ] Run a focused parity pass on save, reload, file switching, and focus handling.
- [ ] Review: inspect command handling, dirty-state transitions, and file-switching behavior for regressions and controller/editor responsibility leaks.

## Phase 7: Quality Features That Make It Feel Serious

What we are doing:

- add the quality features users notice immediately
- make the editor feel polished rather than merely functional

Why we are doing it:

- visual and interaction polish is a major reason for doing this rewrite
- this phase is where the editor starts feeling like a real replacement

Todo:

- [ ] Add fold controls and fold persistence for the open session.
- [ ] Add indent guides.
- [ ] Add matching bracket and current scope styling.
- [ ] Add visible whitespace toggles.
- [ ] Add inline search UI and match navigation.
- [ ] Add diagnostics rendering in the gutter and inline underline style.
- [ ] Add sticky section headers if the display model supports them cleanly.
- [ ] Add file-type-specific defaults for soft wrap and invisibles.
- [ ] Do a focused typography and spacing pass to improve perceived quality.
- [ ] Review: inspect the feature set for interaction rough edges, duplicated overlay systems, and anything that should be postponed instead of merged half-finished.

## Phase 8: LSP-Ready Language Features

What we are doing:

- make the editor ready for code intelligence
- add the minimum architecture needed for semantic features without overcommitting early

Why we are doing it:

- Tree-sitter gets us syntax, but semantic polish needs language-server data
- we want a clean path to completions, hover, and semantic tokens without a second rewrite

Todo:

- [ ] Define `hunk-language` interfaces for diagnostics, semantic tokens, hover, definitions, and completion providers.
- [ ] Implement semantic-token overlays as a separate highlight layer above syntax.
- [ ] Implement hover target extraction from syntax and cursor position.
- [ ] Implement go-to-definition command plumbing.
- [ ] Implement completion trigger plumbing, even if UI stays minimal at first.
- [ ] Keep all LSP features optional and language-server driven, not hard-coded to any one backend.
- [ ] Add tests for semantic-token merge ordering and hover/definition range mapping.
- [ ] Review: inspect the API boundary between editor core and future LSP clients for unnecessary coupling and version-locking risk.

## Phase 9: Remove Helix Completely

What we are doing:

- delete the old implementation
- remove dead dependencies and packaging logic

Why we are doing it:

- carrying both implementations longer than necessary increases maintenance cost
- the whole point of the rewrite is to remove Helix weight from the app

Todo:

- [ ] Remove `helix-core`, `helix-loader`, `helix-term`, and `helix-view` from `hunk-desktop`.
- [ ] Remove Helix runtime discovery from startup.
- [ ] Remove Helix runtime packaging from macOS, Linux, and Windows scripts.
- [ ] Remove Helix-specific tests and runtime environment helpers.
- [ ] Remove obsolete documentation for the Helix-backed implementation or archive it clearly as historical context.
- [ ] Re-run workspace build and clippy once at the end of the removal pass.
- [ ] Review: inspect the workspace for dead code, stale docs, unused assets, and any accidental Helix-era abstractions left behind.

## Suggested First PR Sequence

To keep review manageable, the first pull requests should be narrow:

1. Create `hunk-text`, `hunk-language`, and `hunk-editor` crate skeletons with docs and tests.
2. Land `hunk-text` core snapshots, edits, selections, and undo/redo.
3. Land `hunk-language` registry and initial Tree-sitter parsing for the first language set.
4. Land `hunk-editor` viewport and display snapshot types.
5. Add a hidden read-only GPUI viewer behind a temporary feature flag.
6. Switch the Files tab to the new read-only viewer for non-markdown files.
7. Add editing parity and remove Helix.

## Success Criteria

The rewrite is successful when all of the following are true:

- Files tab no longer depends on Helix
- app packaging no longer bundles Helix runtime data
- syntax highlighting is Tree-sitter based for the supported language set
- the viewer scrolls and opens large files smoothly
- save/reload/dirty-state behavior matches or exceeds current Files-tab behavior
- the codebase is cleaner, more testable, and less coupled to `hunk-desktop`
- the editor is visually cohesive with Hunk's theme and feels native to the app

## Final Note

This plan should optimize for two things at the same time:

- real architectural ownership by Hunk
- visible product quality early

That means we should avoid both extremes:

- embedding another full external editor
- spending too long on abstract editor infrastructure before a polished viewer exists

The right sequence is:

- build the core correctly
- get a polished read-only viewer on screen early
- add editing parity
- then delete Helix completely
