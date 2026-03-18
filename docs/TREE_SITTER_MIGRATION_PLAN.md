# Tree-sitter Migration Plan

## Goal

Finish the migration away from `syntect` so all production syntax highlighting in Hunk uses the same Tree-sitter-based language stack.

## Current State

Now on Tree-sitter:

- `crates/hunk-desktop/src/app/native_files_editor.rs`
- `crates/hunk-language/src/lib.rs`
- `crates/hunk-desktop/src/app/highlight.rs`
- `crates/hunk-domain/src/markdown_preview.rs`

`syntect` has been removed from the active code paths and manifests. The remaining work in this plan is performance hardening and cleanup.

## Why We Are Doing This

- One parsing/highlighting stack is simpler to reason about than a mixed Tree-sitter plus `syntect` setup.
- Tree-sitter gives us better consistency between the native editor, diff preview, and markdown code blocks.
- The current `syntect` paths duplicate token classification logic that now belongs in `hunk-language`.
- Removing `syntect` reduces dependency surface and keeps the Helix removal direction consistent.

## Target State

- Native editor, diff preview, and markdown fenced-code preview all classify syntax through `hunk-language`.
- Surface renderers keep their current coarse token model during the migration.
- `syntect` is removed from `hunk-desktop` and `hunk-domain` only after all production paths are migrated and verified.

## Phases

### Phase 1: Shared Coarse Token Bridge

What:

- Add a shared coarse-token classifier in `hunk-language`.
- Map canonical Tree-sitter capture names into Hunk's coarse token categories.

Why:

- Diff preview and markdown preview currently expect coarse token kinds.
- A shared bridge lets us migrate syntax engines without rewriting every renderer first.

Checklist:

- [x] Add a shared coarse syntax token enum in `hunk-language`.
- [x] Add capture-name-to-token classification helpers in `hunk-language`.
- [x] Add tests for representative canonical captures.
- [x] Add thin conversions from the shared token enum into desktop and markdown preview token enums.
- [x] Review the mapping against Rust, TypeScript, JSON, TOML, and Markdown fenced code.
- [x] Review for bugs and bad code, then refactor naming if the bridge API feels awkward.

### Phase 2: Diff Preview Migration

What:

- Replace `syntect` usage in `crates/hunk-desktop/src/app/highlight.rs` with Tree-sitter-driven highlighting via `hunk-language`.

Why:

- This is the largest remaining `syntect` path in the desktop app.
- Diff rows should use the same syntax semantics as the native file editor.

Checklist:

- [x] Reimplement `build_line_segments` using `hunk-language`.
- [x] Reimplement `build_syntax_only_line_segments` using `hunk-language`.
- [x] Preserve existing intra-line diff behavior and only swap the syntax source.
- [x] Replace TOML fallback tokenization with Tree-sitter-driven classification.
- [x] Port the existing syntax preview tests.
- [x] Review for bugs, bad code, and performance regressions, then refactor caching if needed.

### Phase 3: Markdown Fenced-code Migration

What:

- Replace `syntect` usage in `crates/hunk-domain/src/markdown_preview.rs` with Tree-sitter-driven highlighting via `hunk-language`.

Why:

- Markdown fenced-code preview is the other remaining production syntax path on `syntect`.
- This keeps markdown code blocks visually consistent with the editor and diff preview.

Checklist:

- [x] Resolve fence language names through `LanguageRegistry`.
- [x] Highlight fenced code through `hunk-language`.
- [x] Preserve plain-text fallback for unknown fence languages.
- [x] Keep returning `MarkdownCodeTokenKind` during the migration.
- [x] Port markdown preview syntax tests.
- [x] Review for bugs and bad code, then refactor duplicate language-resolution logic if needed.

### Phase 4: Caching And Performance

What:

- Add lightweight caching around preview and markdown syntax highlighting.

Why:

- Tree-sitter is the right long-term engine, but naive reparsing for every preview surface would be wasteful.

Checklist:

- [ ] Cache diff/file preview syntax results when the same file content is reused.
- [ ] Cache markdown fenced-code syntax results by language and content hash.
- [ ] Avoid rebuilding `LanguageRegistry` ad hoc.
- [ ] Add or extend performance harness coverage for large diffs and large markdown payloads.
- [ ] Review for bugs and bad code, then refactor any duplicated cache logic into shared helpers.

### Phase 5: Remove `syntect`

What:

- Remove `syntect` from the workspace once all production syntax paths are migrated.

Why:

- Only remove the dependency after the migration is complete and verified.

Checklist:

- [x] Remove `syntect` from `crates/hunk-desktop/Cargo.toml`.
- [x] Remove `syntect` from `crates/hunk-domain/Cargo.toml`.
- [x] Delete `syntect`-specific helpers from `crates/hunk-desktop/src/app/highlight.rs`.
- [x] Delete `syntect`-specific helpers from `crates/hunk-domain/src/markdown_preview.rs`.
- [x] Update docs that still describe `syntect` as part of the active stack.
- [x] Review for dead code and stale tests, then clean them up.

## Notes

- The native editor is already the reference Tree-sitter integration.
- The migration should preserve current renderer token enums until the parsing migration is complete.
- The largest risk is preview performance, not correctness.
