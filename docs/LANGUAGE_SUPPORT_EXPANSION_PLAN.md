# Language Support Expansion Plan

Date: 2026-03-18

## Goal

Add first-class Tree-sitter syntax highlighting support for these additional languages and config formats in Hunk:

- Java
- Kotlin
- C
- C++
- C#
- SQL
- Markdown
- Dockerfile
- Nix
- Terraform / HCL
- Swift

The work should stay on the current `hunk-language` architecture:

- one language registry
- one Tree-sitter-based highlighting stack
- no `syntect`
- theme-driven capture mapping
- tests in crate-level `tests/` directories

## Why

We already replaced Helix and removed `syntect`, so Hunk now has a single editor/highlighting path. The next step is broadening the language set so common repos still look correct.

This list covers three categories:

- mainstream app languages we are still missing: Java, Kotlin, C, C++, C#, Swift
- common data / query / docs formats: SQL, Markdown
- common infra / config formats: Dockerfile, Nix, Terraform

## Constraints

- Keep production parsing and highlighting in `crates/hunk-language`.
- Add only grammars we can support cleanly with the current Tree-sitter crate version.
- Prefer upstream highlight queries when they are good enough.
- Vendor small query files into `crates/hunk-language/src/queries/` when upstream crates do not expose the query we need or expose a poor one.
- Avoid introducing a second highlighting engine or ad hoc regex fallback.

## Current Shape

Today `hunk-language` registers built-in languages directly in [assets.rs](/Volumes/hulk/dev/projects/hunk/crates/hunk-language/src/assets.rs), using:

- a `LanguageDefinition`
- file matchers
- a Tree-sitter grammar
- highlight / injection / locals queries
- fold node hints
- alias names

That means each new language should land as:

1. dependency in [Cargo.toml](/Volumes/hulk/dev/projects/hunk/crates/hunk-language/Cargo.toml)
2. `LanguageDefinition` function in [assets.rs](/Volumes/hulk/dev/projects/hunk/crates/hunk-language/src/assets.rs)
3. tests in [syntax.rs](/Volumes/hulk/dev/projects/hunk/crates/hunk-language/tests/syntax.rs) and [preview_highlighting.rs](/Volumes/hulk/dev/projects/hunk/crates/hunk-language/tests/preview_highlighting.rs) as needed

## Phase Breakdown

### Phase 1: Straightforward Mainstream Languages

Scope:

- Java
- C
- C++
- C#
- Terraform / HCL
- Swift

Why:

- These are high-value additions.
- The current Rust Tree-sitter ecosystem appears to have direct grammar crates for them.
- None of these should require the same special fenced/injection handling as Markdown.

Todo:

- [ ] Add grammar crates for Java, C, C++, C#, HCL, and Swift to `crates/hunk-language/Cargo.toml`.
- [ ] Add `LanguageDefinition`s in `assets.rs`.
- [ ] Pick file matchers:
  - Java: `.java`
  - C: `.c`, `.h`
  - C++: `.cc`, `.cpp`, `.cxx`, `.hpp`, `.hh`, `.hxx`
  - C#: `.cs`
  - Terraform / HCL: `.tf`, `.tfvars`, `.hcl`
  - Swift: `.swift`
- [ ] Add aliases for language hints used in previews and the Files header.
- [ ] Add fold node lists that match each grammar’s block/container nodes.
- [ ] Add syntax tests proving at least keyword / type / string coverage for each language.
- [ ] Add preview-highlighting tests proving extension-based language detection works.
- [ ] Review: inspect query quality, wrong token classes, and file matcher overlap before moving on.

### Phase 2: Medium-Risk Languages With Likely Query Cleanup

Scope:

- Kotlin
- Nix

Why:

- These matter, but they are more likely to need local query adjustments or alias cleanup.
- Kotlin in particular may need us to choose between the older crate and the newer `-ng` variant based on compatibility and query quality.

Todo:

- [ ] Evaluate current Kotlin grammar options against our `tree-sitter` version and choose one explicitly.
- [ ] Add Kotlin support with `.kt` and `.kts`.
- [ ] Add Nix support with `.nix`.
- [ ] Vendor highlight queries if upstream coverage is incomplete or stylistically poor.
- [ ] Add fold node hints and alias names.
- [ ] Add regression tests for representative files in `syntax.rs`.
- [ ] Add preview-highlighting tests for `.kt`, `.kts`, and `.nix`.
- [ ] Review: inspect vendored query size, maintenance burden, and whether any capture mapping additions are needed in `theme.rs`.

### Phase 3: Structured Text Formats

Scope:

- SQL
- Dockerfile

Why:

- Both are common and valuable.
- Both may need more care than ordinary code languages:
  - SQL because there are multiple grammar choices and dialect concerns.
  - Dockerfile because filename matching matters more than extension matching.

Todo:

- [ ] Choose one SQL grammar path deliberately and document the dialect assumptions.
- [ ] Add SQL support for `.sql`.
- [ ] Choose Dockerfile grammar support and add filename matching for:
  - `Dockerfile`
  - `Containerfile`
  - optionally `*.Dockerfile`
- [ ] Vendor highlight queries if upstream crates do not export useful ones.
- [ ] Add syntax tests for SQL statements and Dockerfile instructions.
- [ ] Add preview-highlighting tests for `Dockerfile` and `.sql`.
- [ ] Review: inspect whether SQL keyword classification is too broad/noisy and whether Dockerfile filenames are matched consistently in all surfaces.

### Phase 4: Markdown

Scope:

- Markdown

Why:

- Markdown is special.
- We already highlight fenced code blocks via the Tree-sitter stack, but full Markdown syntax introduces:
  - headings
  - lists
  - emphasis
  - links
  - quotes
  - fenced regions
- It is also the most likely language here to interact with injections and nested code blocks in a meaningful way.

Todo:

- [ ] Add Markdown grammar support for `.md` and `.markdown`.
- [ ] Decide whether to support one combined Markdown language or separate inline/block parse modes.
- [ ] Map Markdown captures cleanly onto the existing canonical highlight names.
- [ ] Ensure fenced-code handling stays consistent with the existing markdown preview model.
- [ ] Add syntax tests for headings, emphasis, links, lists, code fences, and block quotes.
- [ ] Add preview-highlighting tests for Markdown file detection.
- [ ] Review: inspect whether Markdown highlighting in the file editor and markdown preview are visually coherent or need separate presentation rules.

## Shared Work

These tasks cut across all phases.

Todo:

- [ ] Keep `CANONICAL_HIGHLIGHT_NAMES` in [assets.rs](/Volumes/hulk/dev/projects/hunk/crates/hunk-language/src/assets.rs) in sync if any new useful capture classes appear.
- [ ] Extend theme mappings only when a new capture materially improves visual quality.
- [ ] Re-check `preview_tokens.rs` if new capture names need better coarse-token mapping for diff/preview surfaces.
- [ ] Keep alias names normalized so language hints from file names, extensions, and UI labels all resolve consistently.
- [ ] Avoid making `assets.rs` unwieldy; split it once the file starts becoming difficult to maintain.

## Proposed Order

1. Java
2. C
3. C++
4. C#
5. Terraform / HCL
6. Swift
7. Kotlin
8. Nix
9. SQL
10. Dockerfile
11. Markdown

Why this order:

- It gets the biggest mainstream gaps first.
- It leaves the more special-case text/config formats until after the straightforward language registrations are proven.
- Markdown comes last because it is the most likely to need dedicated handling beyond just “register another grammar.”

## Acceptance Criteria

- Each requested language/config format is registered in `hunk-language`.
- Files and previews detect the language correctly from common names/extensions.
- Syntax tests cover at least one representative sample per language.
- Preview-highlighting tests cover language detection for the new file types.
- No new highlighting engine is introduced.
- Workspace build, clippy, and tests pass after each implementation batch.
