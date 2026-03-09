#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${1:-$(pwd)}"

if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  printf '%s\n' "$CARGO_TARGET_DIR"
  exit 0
fi

if GIT_COMMON_DIR="$(git -C "$ROOT_DIR" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)"; then
  SHARED_ROOT="$(cd "$GIT_COMMON_DIR/.." && pwd)"
  printf '%s\n' "$SHARED_ROOT/target-shared"
  exit 0
fi

printf '%s\n' "$ROOT_DIR/target"
