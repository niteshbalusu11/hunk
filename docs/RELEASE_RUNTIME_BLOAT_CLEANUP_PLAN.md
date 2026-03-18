# Release Runtime Bloat Cleanup Plan

## Summary

The current Hunk app no longer needs the old external grammar runtime that used to make sense with Helix-era integration. The app's file editor, diff preview, and markdown code highlighting now use `hunk-language` and built-in Tree-sitter grammars compiled into the binary.

The large `runtime/grammars` payload observed locally is not declared by Hunk's packager manifest. It appears only in an older macOS-specific bundle artifact and should be treated as stale packaging output until proven otherwise.

## What We Found

### Current Hunk packaging inputs

- [crates/hunk-desktop/Cargo.toml](/Volumes/hulk/dev/projects/hunk/crates/hunk-desktop/Cargo.toml) explicitly bundles only `../../assets/codex-runtime`.
- [assets/codex-runtime](/Volumes/hulk/dev/projects/hunk/assets/codex-runtime) is about 102 MB and contains only 6 files.
- `hunk-language` uses curated built-in Tree-sitter crates and local queries, not an external packaged runtime directory.

### Current local packager outputs

- [target-shared/packager/Hunk.app](/Volumes/hulk/dev/projects/hunk/target-shared/packager/Hunk.app) is about 196 MB and its `Contents/Resources` tree is clean:
  - `Hunk.icns`
  - `codex-runtime/...`
- [target-shared/packager/macos/Hunk.app](/Volumes/hulk/dev/projects/hunk/target-shared/packager/macos/Hunk.app) is about 2.3 GB and contains `Contents/Resources/runtime/grammars/...`.

### Important timestamp signal

- `target-shared/packager/macos/Hunk.app` is older than `target-shared/packager/Hunk.app`.
- That strongly suggests the 2.3 GB bundle is an older macOS release artifact produced before the current bundle validation and Helix cleanup tightened up the packaging path.

### Why the old macOS artifact was so slow

The old macOS release script signs every `*.dylib` and every executable-bit file under the app bundle:

- [package_macos_release.sh](/Volumes/hulk/dev/projects/hunk/scripts/package_macos_release.sh)

For the stale 2.3 GB app, that meant signing:

- hundreds of grammar `.dylib` files
- matching `.dSYM` trees
- thousands of executable-bit sample or helper files under grammar source directories

That work happened before DMG creation and notarization, so it was enough to add a large amount of time even before notarization started.

## Cross-Platform Assessment

### macOS

Risk is real because the signing loop is broad and expensive if stale runtime content reappears.

Current status:

- the clean plain packager output is small
- the stale older macOS release artifact is large
- the current validator already forbids `queries` and `grammars`, but only if the release script is run on a fresh artifact path

### Linux

Current packaging logic is leaner:

- [package_linux_release.sh](/Volumes/hulk/dev/projects/hunk/scripts/package_linux_release.sh) explicitly copies the app binary, launcher, shared libraries, and `codex-runtime/linux/codex`
- [validate_release_bundle_layout.sh](/Volumes/hulk/dev/projects/hunk/scripts/validate_release_bundle_layout.sh) already forbids `helix`, `hx-runtime`, `queries`, and `grammars`

Risk:

- if `cargo packager` or a dependency starts materializing a runtime tree in Linux packaging outputs, the validator should fail
- we do not currently have a fresh Linux artifact in the local worktree to confirm this empirically

### Windows

Current packaging logic is also leaner:

- [package_windows_release.ps1](/Volumes/hulk/dev/projects/hunk/scripts/package_windows_release.ps1) packages the app through `cargo packager` and validates the result
- [validate_windows_release_bundle.ps1](/Volumes/hulk/dev/projects/hunk/scripts/validate_windows_release_bundle.ps1) rejects Helix-era bundle inputs and manifest references

Risk:

- an MSI could still include stale runtime content if `cargo packager` injects it indirectly
- we do not currently have a fresh MSI artifact in the local worktree to confirm this empirically

## Plan

### Phase 1: Treat old macOS artifacts as invalid and make the current output authoritative

Why:

- There are currently two macOS app locations in `target-shared/packager`.
- The older large app can confuse local diagnosis and future debugging.

Todo:

- [ ] Standardize on one macOS app output path for release packaging.
- [ ] Remove or stop reusing stale macOS release artifact directories before packaging starts.
- [ ] Make the release script delete old `packager/macos/Hunk.app` before invoking `cargo packager`.
- [ ] Log the final bundle size and top-level resource layout after packaging.
- [ ] Review the release script for any path ambiguity that can mix old and new artifacts.

### Phase 2: Prevent stale grammar runtimes from shipping again

Why:

- Hunk no longer needs an external `runtime/grammars` tree.
- If it reappears, macOS signing time and bundle size explode.

Todo:

- [ ] Keep `validate_release_bundle_layout.sh` as the cross-platform content guard for `queries` and `grammars`.
- [ ] Move macOS validation earlier and later in the script:
  - once immediately after `cargo packager`
  - once again after all bundle mutations, before signing
- [ ] Add a top-level bundle inventory log on macOS so CI clearly shows whether `Resources/runtime` exists.
- [ ] Add equivalent artifact content logging for Linux and Windows packaging jobs.
- [ ] Review for any dependency or packager upgrade path that could reintroduce grammar payloads silently.

### Phase 3: Narrow macOS signing to actual Mach-O binaries

Why:

- Even if the bundle is clean, the current macOS signing loop is broader than necessary.
- Signing every executable-bit file is fragile and can become expensive again if any unexpected payload appears.

Todo:

- [ ] Replace the generic `find ... -perm -111` signing loop with Mach-O detection.
- [ ] Sign only:
  - the main app binary
  - bundled `.dylib` files
  - any additional Mach-O executables that are actually shipped intentionally
- [ ] Exclude text scripts, sample files, and other executable-bit non-binaries from signing.
- [ ] Keep the app-level codesign and verification step.
- [ ] Review the signing list output to confirm only intended binaries are signed.

### Phase 4: Verify Linux and Windows packaged artifacts in CI, not just scripts

Why:

- Right now local evidence for Linux and Windows is script-level, not artifact-level.
- We want explicit proof that those bundles contain only the intended runtime payloads.

Todo:

- [ ] In Linux CI, print the packaged directory tree roots and fail if `runtime`, `queries`, or `grammars` appear outside the allowed `codex-runtime` path.
- [ ] In Windows CI, inspect the MSI staging output and fail if `runtime`, `queries`, or `grammars` appear outside the allowed `codex-runtime` path.
- [ ] Store a short artifact inventory in CI logs for each platform.
- [ ] Confirm final bundle sizes are within expected bounds on each platform.
- [ ] Review the artifact inspection output and tighten validators where needed.

### Phase 5: Revisit `gpui-component` feature usage

Why:

- [crates/hunk-desktop/Cargo.toml](/Volumes/hulk/dev/projects/hunk/crates/hunk-desktop/Cargo.toml) still enables `gpui-component`'s `tree-sitter-languages` feature.
- That feature is not the direct cause of the stale bundle by itself, but it is broader than Hunk now needs.

Todo:

- [ ] Audit current Hunk uses of `gpui-component` highlighter or editor features.
- [ ] If Hunk no longer needs `gpui-component` language registration, remove the `tree-sitter-languages` feature.
- [ ] If some part still needs it, replace it with a narrower local feature or Hunk-owned syntax surface.
- [ ] Rebuild package outputs after the feature change and compare artifact size.
- [ ] Review for regressions in markdown, UI components, or any remaining code-editor helper usage.

## Recommended Order

1. Clean and standardize macOS output paths.
2. Strengthen bundle validation and artifact logging on all three platforms.
3. Narrow the macOS signing list to actual binaries.
4. Audit and possibly remove `gpui-component`'s `tree-sitter-languages` feature.

## Expected Outcome

If the stale grammar runtime is fully excluded and macOS signing is narrowed to real binaries:

- macOS bundle size should stay close to the smaller current output, not the 2.3 GB stale artifact
- macOS signing time should drop substantially
- DMG creation time should also drop because the input app is much smaller
- Linux and Windows packaging should have explicit guards against the same regression
