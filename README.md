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

`cargo run` starts from Terminal, so macOS may still present it like a terminal-launched app.
For a proper Dock app identity (name/icon) and normal app launching behavior, build and open the macOS bundle:

```bash
cargo install cargo-bundle
cargo bundle --release
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
open "$TARGET_DIR/release/bundle/osx/Hunk.app"
```

## Icons

Generate git-diff icon variants and rebuild the bundle:

```bash
./scripts/generate_diff_icons.py
./scripts/build_macos_icon.sh
cargo bundle --release
```

Generated assets:
- `assets/icons/hunk-icon-default.png` (default/full color)
- `assets/icons/hunk-icon-dark.png` (dark appearance variant)
- `assets/icons/hunk-icon-mono.png` (monochrome/tint-friendly variant)

Current bundling uses `hunk-icon-default.png` -> `Hunk.icns`.

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
