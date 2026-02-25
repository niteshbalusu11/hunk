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
- [ ] Add tests for syntax segment generation for plain-file rendering.
- [x] Run `cargo fmt`.
- [x] Run `cargo test`.
- [x] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] Run `cargo build`.
