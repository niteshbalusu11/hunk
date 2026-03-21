#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ID="io.github.BlixtWallet.Hunk"
MANIFEST_PATH="$ROOT_DIR/flatpak/$APP_ID.yaml"
TARGET_DIR="$("$ROOT_DIR/scripts/resolve_cargo_target_dir.sh" "$ROOT_DIR")"
BUILD_DIR="$TARGET_DIR/flatpak/build"
REPO_DIR="$TARGET_DIR/flatpak/repo"
STATE_DIR="$TARGET_DIR/flatpak/state"
FORCE_CLEAN="${HUNK_FLATPAK_FORCE_CLEAN:-1}"
PREPARE_VENDOR="${HUNK_FLATPAK_PREPARE_VENDOR:-0}"
CLEAN_STATE="${HUNK_FLATPAK_CLEAN_STATE:-0}"

if ! command -v flatpak-builder >/dev/null 2>&1; then
  echo "error: flatpak-builder is required" >&2
  exit 1
fi

if ! appstreamcli compose --help >/dev/null 2>&1; then
  echo "error: appstreamcli compose support is required" >&2
  echo "install the distro package, for example: appstream-compose" >&2
  exit 1
fi

"$ROOT_DIR/scripts/download_codex_runtime_unix.sh" linux >/dev/null
"$ROOT_DIR/scripts/validate_codex_runtime_bundle.sh" --strict --platform linux >/dev/null

if [[ "$PREPARE_VENDOR" == "1" ]]; then
  "$ROOT_DIR/scripts/prepare_flatpak_vendor.sh"
fi

if [[ ! -f "$ROOT_DIR/flatpak/cargo-config.toml" ]]; then
  echo "error: missing flatpak/cargo-config.toml; run \`just flatpak-vendor\` first" >&2
  exit 1
fi

if [[ "$CLEAN_STATE" == "1" ]]; then
  rm -rf "$STATE_DIR"
fi

mkdir -p "$BUILD_DIR" "$REPO_DIR" "$STATE_DIR"

builder_args=(
  --state-dir="$STATE_DIR"
  --keep-build-dirs
  --repo="$REPO_DIR"
  --install-deps-from=flathub
)

if [[ "$FORCE_CLEAN" == "1" ]]; then
  builder_args+=(--force-clean)
fi

flatpak-builder \
  "${builder_args[@]}" \
  "$BUILD_DIR" \
  "$MANIFEST_PATH"

echo "Built Flatpak repo at $REPO_DIR"
