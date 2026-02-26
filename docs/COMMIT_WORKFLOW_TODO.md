# Commit Workflow UI TODO (JJ)

## Problem Statement
The old workflow was designed around Git staging (`stage/unstage`) but JJ has no staging index.
We still need partial commits from the UI without shelling out to `jj` CLI, and we need this to
work against a colocated Git backend for GitHub push/publish.

## Solution Approach
Use `jj-lib` as the single backend and model commit selection as:
- include/exclude changed files in the next commit (UI state),
- commit all changes when all files are included,
- create a partial commit via `jj-lib` tree rewriting when only a subset is included,
- keep unselected changes in the working-copy commit.

## Migration Tasks
### 1) JJ-Lib Backend
- [x] Add `jj-lib` dependency.
- [x] Keep stage/unstage APIs explicitly unsupported under JJ.
- [x] Add `commit_selected_paths(repo_root, message, paths)` public API in `src/jj.rs`.
- [x] Implement partial commit in `src/jj/backend.rs` with `FilesMatcher` + `restore_tree`.
- [x] Ensure bookmark advances to committed parent after commit actions.

### 2) Controller + State
- [x] Add `commit_excluded_files` UI state to track excluded files.
- [x] Add controller actions:
  - [x] toggle file include/exclude for commit
  - [x] include all files
  - [x] compute selected file list/count
- [x] Route commit button:
  - [x] full selection -> `commit_staged` (JJ working-copy commit)
  - [x] partial selection -> `commit_selected_paths`
- [x] Clear selection state after successful commit and on snapshot resets.

### 3) UI
- [x] Replace stage checkbox semantics with include/exclude commit toggle per file.
- [x] Add footer indicator `Commit includes X/Y files`.
- [x] Add `Include All` quick action.
- [x] Keep branch picker + push/publish controls aligned with JJ bookmark model.

### 4) Validation
- [x] Add integration test:
  - [x] selected-path commit only commits requested files.
- [x] Keep existing JJ commit + checkout workflow tests passing.
- [x] `cargo check`
- [x] `cargo test --test jj_commit_workflow --test jj_branch_checkout_workflow -- --nocapture`
- [x] `cargo test`
- [x] `cargo clippy --all-targets --all-features -- -D warnings`

## Follow-Ups
- [ ] Optional: hunk-level partial commits (line-level selection), not only file-level.
- [ ] Optional: clearer include iconography than the current compact `x` marker.
