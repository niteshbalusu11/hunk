#!/usr/bin/env bash
set -euo pipefail

if [[ $# -eq 0 ]]; then
  echo "usage: $0 <command> [args...]" >&2
  exit 64
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  exec "$@"
fi

sdkroot="$(xcrun --sdk macosx --show-sdk-path)"

export SDKROOT="$sdkroot"
export LIBRARY_PATH="$sdkroot/usr/lib${LIBRARY_PATH:+:$LIBRARY_PATH}"
export CPATH="$sdkroot/usr/include${CPATH:+:$CPATH}"
export CFLAGS="-isysroot $sdkroot${CFLAGS:+ $CFLAGS}"
export CXXFLAGS="-isysroot $sdkroot${CXXFLAGS:+ $CXXFLAGS}"
export LDFLAGS="-L$sdkroot/usr/lib${LDFLAGS:+ $LDFLAGS}"
export RUSTFLAGS="-L native=$sdkroot/usr/lib${RUSTFLAGS:+ $RUSTFLAGS}"

exec "$@"
