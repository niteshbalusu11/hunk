# hunk

A macOS-first Git diff viewer built with `gpui` + `gpui-component`.

## What it includes

- Fast repo snapshot loading from `git2`
- File tree for changed files
- Side-by-side diff viewer with per-line styling and line numbers
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

## Hot Reload (Bacon)

Install bacon once:

```bash
cargo install bacon
```

Start hot reload (default job is `run`):

```bash
bacon
```

Useful jobs:

```bash
bacon check
bacon test
bacon clippy
```

Keybindings in bacon UI:

- `r` -> run
- `c` -> check
- `t` -> test
- `l` -> clippy

## Notes

- Current scope is macOS-first.
- Windows/Linux support can be added next by validating platform setup and appearance behavior.
