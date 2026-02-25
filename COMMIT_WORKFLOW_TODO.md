# Commit Workflow UI TODO

This document tracks the staged rollout for the new left-panel commit workflow.

## 1) Data + Git Operations
- [x] Extend `RepoSnapshot` with staged state per file
- [x] Add local branches data for picker
- [x] Add upstream/publish status for current branch
- [x] Add last commit subject for footer
- [x] Add git operations:
  - [x] stage file
  - [x] unstage file
  - [x] stage all
  - [x] unstage all
  - [x] switch/create branch
  - [x] push/publish current branch
  - [x] commit staged changes
- [x] Add branch-name sanitization helper

## 2) Controller Wiring
- [x] Add async action handlers for git operations
- [x] Refresh snapshot after each successful operation
- [x] Surface operation status/errors in UI state

## 3) Sidebar UI
- [x] Replace current tree panel with commit workflow panel
- [x] Add Tracked / Untracked sections
- [x] Add per-file stage checkboxes
- [x] Add stage-all / unstage-all control
- [x] Add branch controls row (picker + publish/push)
- [x] Add branch picker panel with branch list
- [x] Add branch create/switch input with sanitization
- [x] Add commit message input + commit button
- [x] Add footer showing latest commit message

## 4) Validation
- [x] Add tests for branch sanitization in `tests/`
- [x] `cargo fmt`
- [x] `cargo test`
- [x] `cargo clippy --all-targets --all-features`
