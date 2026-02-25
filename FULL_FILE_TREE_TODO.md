# Full File Tree + File Preview TODO

## Goal
Implement a full repository tree (alongside existing diff tree) with a bottom switcher, defaulting to diff tree, and support opening any file with syntax-highlighted preview.

## Milestones

### 1. Data model and scanning
- [x] Add file-tree domain model for directories/files with ignored metadata.
- [x] Implement background-safe repository tree scan rooted at `repo_root`.
- [x] Integrate gitignore awareness (`ignored` shading metadata).
- [x] Exclude `.git` internals and keep deterministic sort order.

### 2. Sidebar mode switch
- [x] Add `SidebarTreeMode` enum with default `Diff`.
- [x] Add persistent or runtime state for active tree mode.
- [x] Add bottom UI switcher (`Diff` / `Files`) in left panel.

### 3. Full tree rendering
- [x] Add folder expansion state and flattening for rendering rows.
- [x] Render hierarchical rows with indentation and chevrons.
- [x] Add subtle file/folder glyphs/icons.
- [x] Shade gitignored files/folders distinctly.

### 4. Open file preview pane
- [x] Add right pane mode: diff view vs file preview.
- [x] Add async file load path with robust error handling.
- [x] Reuse syntax highlighting for full-file lines.
- [x] Handle non-UTF8/binary/very-large files gracefully.

### 5. Sync and interactions
- [x] Keep existing diff interactions unchanged in diff mode.
- [x] Selecting files from full tree should open preview.
- [x] Switching back to diff mode should restore diff behavior.
- [x] Refresh full tree with repository snapshot updates.

### 6. Tests and quality gates
- [x] Add unit tests under `tests/` for tree-building behavior.
- [x] Add tests for syntax segment generation for plain-file rendering.
- [x] Run `cargo fmt`.
- [x] Run `cargo test`.
- [x] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] Run `cargo build`.

---

# File Editor TODO

## Goal
Upgrade file preview into a proper code editor for repository files, with safe save behavior, keyboard-first editing, and performance that matches current diff/file rendering.

## Architecture decisions

### 0. Dependency and design spike
- [ ] Decide editor text buffer strategy (`String` MVP vs `ropey` for scalable edits).
- [ ] Decide syntax strategy for editing path (reuse current `syntect` pipeline initially, then evaluate incremental highlighting).
- [ ] Decide language parser strategy for future features (optional `tree-sitter` for incremental parse, symbols, folding).
- [ ] Define file limits and fallback behavior (very large files, binary files, non-UTF8).
- [ ] Write a short ADR in repo docs with chosen approach and tradeoffs before implementation starts.

### 1. Editor domain model and state
- [ ] Add `RightPaneMode::FileEditor` and routing between diff, preview, and editor modes.
- [ ] Add `EditorBuffer` model (path, text, version/epoch, dirty, read-only, line ending style).
- [ ] Add cursor/selection model (single cursor first; multi-cursor explicitly out of scope for MVP).
- [ ] Add undo/redo stack model with bounded history.
- [ ] Add open-buffer cache/LRU behavior so switching files does not always reload from disk.
- [ ] Keep GPUI task lifecycle safe: no dropped tasks, epoch guards for stale async results.

### 2. Loading, saving, and safety
- [ ] Replace read-only preview load path with editable document load path while retaining async I/O.
- [ ] Add explicit save flow (`Cmd/Ctrl+S`) with atomic write (`temp file -> fsync -> rename` where applicable).
- [ ] Track external file changes and define conflict behavior when buffer is dirty.
- [ ] Add unsaved-change guardrails when switching files, switching modes, and quitting app.
- [ ] Add status messaging for save success/failure/conflict in existing toolbar/status surfaces.

### 3. Core editing interactions (MVP)
- [ ] Add text insertion/deletion actions (`Backspace`, `Delete`, `Enter`, `Tab`, paste).
- [ ] Add navigation actions (arrows, word movement, home/end, page up/down).
- [ ] Add selection actions (shift+arrows, select-all, copy/cut/paste from focused editor).
- [ ] Add line number gutter + current line highlight + horizontal/vertical scrolling.
- [ ] Add optional soft-wrap toggle shared with current diff/file preview behavior where sensible.
- [ ] Maintain responsive rendering with virtualization for long files.

### 4. Editor UX completeness
- [ ] Add in-editor find (`Cmd/Ctrl+F`) with next/previous and match count.
- [ ] Add go-to-line (`Cmd/Ctrl+L`) and jump to selected tree item position (line 1 for now).
- [ ] Add dirty indicator in file tree rows and right-pane header.
- [ ] Add read-only/binary/non-text fallback view with clear explanation.
- [ ] Add indentation controls (tab width, spaces vs tabs) based on config defaults.
- [ ] Preserve and restore per-file cursor/scroll position while switching files.

### 5. Git and snapshot integration
- [ ] On successful save, trigger snapshot refresh and preserve current selection/editor state.
- [ ] Handle deleted/renamed files gracefully when file tree refreshes.
- [ ] Define behavior for editing tracked vs untracked files.
- [ ] Ensure diff tree mode remains unchanged and isolated from editor mode behavior.
- [ ] Optionally add quick action: "Open current file in diff view" when file has git changes.

### 6. Tests and quality gates
- [ ] Add tests under `tests/` for text buffer operations (insert/delete/newline/tab/selection transforms).
- [ ] Add tests under `tests/` for undo/redo correctness and history bounds.
- [ ] Add tests under `tests/` for save behavior (dirty flags, external change conflict, read-only errors).
- [ ] Add tests under `tests/` for non-UTF8/binary/large-file fallback behavior.
- [ ] Add tests under `tests/` for editor state transitions (preview <-> editor <-> diff).
- [ ] Run `cargo fmt`.
- [ ] Run `cargo test`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo build`.

## Suggested dependency additions (evaluate in milestone 0)
- [ ] `ropey` for scalable text editing in large files.
- [ ] `tree-sitter` (+ language crates on demand) for future incremental syntax/structure features.
- [ ] `encoding_rs` for robust decode/encode fallback beyond UTF-8-only paths.
- [ ] `similar` (optional) if we want in-editor dirty region diff hints later.

## Out of scope for first editor release
- [ ] Multi-cursor editing.
- [ ] LSP integration (hover, completion, diagnostics).
- [ ] Inline git blame, code actions, and refactor tooling.
