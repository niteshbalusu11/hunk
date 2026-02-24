# hunk

A macOS-first Git diff viewer built with `gpui` + `gpui-component`.

## What it includes

- Fast repo snapshot loading from `git status --porcelain`
- File tree for changed files
- Diff viewer with per-line styling and line numbers
- Resizable split panes (tree + diff)
- Light/Dark mode toggle
- Refresh action
- `anyhow`-based error handling
- `tracing` + `tracing-subscriber` logging

## Requirements

- macOS
- Xcode + command line tools
- Metal toolchain for GPUI shader compilation

If you see a build error about missing `metal`, run:

```bash
xcodebuild -downloadComponent MetalToolchain
```

## Run

```bash
cargo run
```

Launch from inside a Git repository to view changes.

## Notes

- Current scope is macOS-first.
- Windows/Linux support can be added next by validating platform setup and appearance behavior.
