#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="$("$ROOT_DIR/scripts/resolve_cargo_target_dir.sh" "$ROOT_DIR")"
VERSION_LABEL="${HUNK_RELEASE_VERSION:-$("$ROOT_DIR/scripts/resolve_hunk_version.sh")}"
DIST_DIR="$TARGET_DIR/dist"
APP_PATH="$TARGET_DIR/release/bundle/osx/Hunk.app"
DMG_PATH="$DIST_DIR/Hunk-$VERSION_LABEL-macos-arm64.dmg"
DMG_STAGE_DIR="$TARGET_DIR/release/dmg-stage"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: macOS release packaging must run on macOS" >&2
  exit 1
fi

"$ROOT_DIR/scripts/download_codex_runtime_unix.sh" macos >/dev/null
"$ROOT_DIR/scripts/validate_codex_runtime_bundle.sh" --strict --platform macos >/dev/null

(
  cd "$ROOT_DIR"
  cargo bundle -p hunk-desktop --release --format osx
)

"$ROOT_DIR/scripts/inject_codex_runtime_into_macos_bundle.sh" "$APP_PATH" >/dev/null

if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
  codesign --force --deep --options runtime --sign "$APPLE_SIGNING_IDENTITY" "$APP_PATH"
  codesign --verify --deep --strict --verbose=2 "$APP_PATH"
fi

rm -rf "$DMG_STAGE_DIR" "$DMG_PATH"
mkdir -p "$DMG_STAGE_DIR" "$DIST_DIR"
cp -R "$APP_PATH" "$DMG_STAGE_DIR/Hunk.app"
ln -s /Applications "$DMG_STAGE_DIR/Applications"

hdiutil create \
  -volname "Hunk" \
  -srcfolder "$DMG_STAGE_DIR" \
  -ov \
  -format UDZO \
  "$DMG_PATH" >/dev/null

if [[ -n "${APPLE_NOTARY_KEY_ID:-}" || -n "${APPLE_NOTARY_ISSUER:-}" || -n "${APPLE_NOTARY_API_KEY_PATH:-}" ]]; then
  : "${APPLE_SIGNING_IDENTITY:?APPLE_SIGNING_IDENTITY is required for notarization}"
  : "${APPLE_NOTARY_KEY_ID:?APPLE_NOTARY_KEY_ID is required for notarization}"
  : "${APPLE_NOTARY_ISSUER:?APPLE_NOTARY_ISSUER is required for notarization}"
  : "${APPLE_NOTARY_API_KEY_PATH:?APPLE_NOTARY_API_KEY_PATH is required for notarization}"

  xcrun notarytool submit \
    "$DMG_PATH" \
    --key "$APPLE_NOTARY_API_KEY_PATH" \
    --key-id "$APPLE_NOTARY_KEY_ID" \
    --issuer "$APPLE_NOTARY_ISSUER" \
    --wait
  xcrun stapler staple "$APP_PATH"
  xcrun stapler staple "$DMG_PATH"
  xcrun stapler validate "$DMG_PATH"
fi

printf '%s\n' "$DMG_PATH"
