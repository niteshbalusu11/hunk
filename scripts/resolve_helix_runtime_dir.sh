#!/usr/bin/env bash
set -euo pipefail

HELIX_GIT_REV_PREFIX="${HELIX_GIT_REV_PREFIX:-78b999f}"

resolve_cargo_home() {
  if [[ -n "${CARGO_HOME:-}" ]]; then
    printf '%s\n' "$CARGO_HOME"
  elif [[ -d "/Volumes/hulk/dev/cache/cargo" ]]; then
    printf '%s\n' "/Volumes/hulk/dev/cache/cargo"
  else
    printf '%s\n' "$HOME/.cargo"
  fi
}

cargo_home="$(resolve_cargo_home)"
checkouts_dir="$cargo_home/git/checkouts"
if [[ ! -d "$checkouts_dir" ]]; then
  echo "error: Helix git checkouts directory was not found: $checkouts_dir" >&2
  exit 1
fi

for repo_dir in "$checkouts_dir"/helix-*; do
  [[ -d "$repo_dir" ]] || continue
  preferred_runtime="$repo_dir/$HELIX_GIT_REV_PREFIX/runtime"
  if [[ -d "$preferred_runtime" ]]; then
    printf '%s\n' "$preferred_runtime"
    exit 0
  fi
done

for repo_dir in "$checkouts_dir"/helix-*; do
  [[ -d "$repo_dir" ]] || continue
  for runtime_dir in "$repo_dir"/*/runtime; do
    if [[ -d "$runtime_dir" ]]; then
      printf '%s\n' "$runtime_dir"
      exit 0
    fi
  done
done

echo "error: failed to locate a Helix runtime under $checkouts_dir" >&2
exit 1
