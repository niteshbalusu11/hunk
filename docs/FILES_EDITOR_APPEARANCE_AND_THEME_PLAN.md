# Files Editor Appearance And Theme Plan

## Goal

Make the native files editor feel much closer to Zed / VS Code in two ways:

1. Use space better by removing duplicate chrome and heavy boxing around the editor.
2. Match VS Code-style code colors and editor contrast instead of the current GitHub-like fallback palette.

This plan covers both the layout work and the syntax/theme work, with phased implementation and review steps.

## Why We Are Doing This

- The current files editor spends too much vertical and horizontal space on layered chrome.
- The editor is visually boxed twice: once by the file-level header and again by the editor-local shell.
- Search, wrap, invisibles, file metadata, save/reload, and status are split across multiple strips.
- Syntax colors do not match the user expectation of VS Code Dark+, which is also what the user uses in Zed.
- Editor chrome and syntax coloring come from different systems today, which makes the surface feel inconsistent.

## Target State

- One clean top strip for file metadata and editor controls.
- Editor canvas uses the available panel area edge-to-edge inside the Files pane.
- No heavy accent border around the entire editor.
- Editor chrome is visually quieter than the code.
- Syntax colors are theme-owned and can intentionally match VS Code Dark+.
- Diff preview, markdown code blocks, and native editor syntax all use the same color vocabulary.

## Architecture

### Layout

- `crates/hunk-desktop/src/app/render/file_editor.rs`
  Own the single top strip for file-level metadata and editor controls.
- `crates/hunk-desktop/src/app/render/file_editor_surface.rs`
  Render only the editor surface and input/event plumbing, not duplicate editor chrome.

### Theme

- `crates/hunk-desktop/src/app/theme.rs`
  Own the editor syntax palette and editor chrome colors.
- `crates/hunk-desktop/src/app/render/syntax_colors.rs`
  Read syntax token colors from the theme instead of hard-coded GitHub values.
- `crates/hunk-desktop/src/app/render/file_editor.rs`
  Reuse the same token palette for markdown preview code blocks.

## Phases

### Phase 1: Flatten The Editor Shell

What:

- Remove the duplicate inner editor toolbar/footer from `file_editor_surface.rs`.
- Move search, wrap, invisibles, and editor metadata into the existing file header in `file_editor.rs`.
- Let the editor surface occupy the full remaining area in the Files pane.

Why:

- This is the highest-value real-estate improvement.
- It removes the obvious “blue box inside another box” problem.
- It simplifies the surface before any color retuning.

Checklist:

- [x] Remove the inner editor toolbar from `file_editor_surface.rs`.
- [x] Remove the inner editor footer from `file_editor_surface.rs`.
- [x] Move editor controls into the file-level header in `file_editor.rs`.
- [x] Put search controls into the same strip as file metadata and actions.
- [x] Make the editor surface fill the remaining pane area without extra card padding.
- [ ] Review spacing and overflow behavior at narrow widths.
- [ ] Review markdown preview mode to ensure the same header feels correct there.
- [ ] Review and refactor any duplicated toolbar button styling that became more obvious after the merge.

### Phase 2: Clean Up Toolbar Hierarchy

What:

- Reduce the amount of “chip” styling and make the top strip quieter.
- Decide which metadata belongs inline and which can be deemphasized or removed.

Why:

- The current header still carries more visual treatment than Zed/VS Code.
- Once the duplicate shell is removed, the remaining noise becomes more visible.

Checklist:

- [x] Replace loud pill-style badges with quieter text or subtler controls where possible.
- [x] Rebalance file path, status, language, selection, and position so the path remains primary.
- [x] Keep action buttons compact and visually secondary to the code.
- [ ] Remove any labels that no longer add value.
- [ ] Review for toolbar crowding on smaller widths.
- [ ] Review for bugs and bad code, then refactor the toolbar composition if it is too repetitive.

### Phase 3: Move Syntax Colors Into Theme

What:

- Replace the hard-coded GitHub-like syntax palette with a theme-owned editor syntax palette.

Why:

- We cannot match VS Code reliably while syntax colors live in `syntax_colors.rs`.
- Theme-owned syntax colors make the editor, diff rows, and markdown preview coherent.

Checklist:

- [x] Add a dedicated editor syntax palette to `theme.rs`.
- [x] Include at least: plain, keyword, string, number, comment, function, type, constant, variable, operator.
- [x] Refactor `render/syntax_colors.rs` to resolve colors from the theme.
- [x] Refactor markdown code token coloring in `render/file_editor.rs` to use the same palette.
- [ ] Review for mismatches across diff preview, markdown preview, and native editor.
- [ ] Review for bugs and bad code, then refactor any duplicated color lookup logic.

### Phase 4: Tune For VS Code Dark+

What:

- Tune editor chrome and syntax colors to match the official VS Code Dark+ palette as closely as Hunk's simpler token model allows.

Why:

- The user explicitly wants the editor to feel like VS Code/Zed with a VS Code theme.
- After phase 3, the palette is centralized enough to tune intentionally.
- Using the official Microsoft theme files is better than screenshot-based guessing.

Checklist:

- [ ] Use the official VS Code default theme files as the source palette for editor chrome and syntax.
- [ ] Match editor background and foreground to VS Code Dark+.
- [ ] Match subdued line numbers and brighter active line number.
- [ ] Tune selection, selection highlight, active line, invisibles, and indent guides to VS Code-like values.
- [ ] Tune syntax token colors to VS Code Dark+ defaults for Rust, TypeScript, JSON, Markdown, and CSS.
- [ ] Review screenshots side-by-side against VS Code or Zed with the same theme.
- [ ] Review for bugs and bad code, then refactor theme naming if the palette API is awkward.

### Phase 5: Optional Zed-Inspired Polish

What:

- Borrow additional visual ideas from Zed where useful, without reintroducing heavy chrome.

Why:

- Once the basics are correct, small polish changes matter more than large structural ones.

Checklist:

- [ ] Evaluate whether the active-line treatment should be more Zed-like.
- [ ] Evaluate tighter gutter contrast and cleaner selection rendering.
- [ ] Consider more deliberate toolbar spacing and typography.
- [ ] Consider whether the footer can be removed entirely or replaced with lighter inline metadata.
- [ ] Review for bugs and bad code, then refactor any remaining layout rough edges.

## Current Status

- Phase 1 has started.
- The duplicate inner toolbar/footer has been removed from the editor surface.
- The outer file editor header now owns search, wrap, invisibles, and status metadata.
- Phase 2 has started.
- The merged toolbar has been quieted so the path and code area are more visually dominant.
- Phase 3 has started.
- Syntax colors now resolve from `theme.rs` instead of hard-coded GitHub-style mappings.
- Phase 4 has started.
- Editor chrome now uses centralized VS Code-style background, foreground, line number, selection, and guide values instead of mixing editor and app-surface colors.

## Notes

- If it helps, Zed can be used as a reference for layout density and key bindings.
- Direct code copying is acceptable for this project if needed, but the first appearance pass should mostly be Hunk-owned composition work rather than a straight port.
- VS Code reference themes:
  - `https://raw.githubusercontent.com/microsoft/vscode/main/extensions/theme-defaults/themes/dark_vs.json`
  - `https://raw.githubusercontent.com/microsoft/vscode/main/extensions/theme-defaults/themes/dark_plus.json`
