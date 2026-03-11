#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(
  sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT_DIR/crates/hunk-desktop/Cargo.toml" \
    | head -n 1
)"

if [[ -z "$VERSION" ]]; then
  echo "error: failed to resolve Hunk version from crates/hunk-desktop/Cargo.toml" >&2
  exit 1
fi

printf '%s\n' "$VERSION"
