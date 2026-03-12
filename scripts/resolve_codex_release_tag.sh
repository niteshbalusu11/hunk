#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CODEX_TAG="$(
  sed -n 's/.*tag = "\(rust-v[^"]*\)".*/\1/p' "$ROOT_DIR/crates/hunk-desktop/Cargo.toml" \
    | head -n 1
)"

if [[ -z "$CODEX_TAG" ]]; then
  echo "error: failed to resolve Codex release tag from crates/hunk-desktop/Cargo.toml" >&2
  exit 1
fi

printf '%s\n' "$CODEX_TAG"
