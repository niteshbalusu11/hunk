#!/usr/bin/env bash
set -euo pipefail

if [[ $# -eq 0 ]]; then
  echo "error: expected a command to run" >&2
  exit 1
fi

append_path() {
  local candidate="$1"

  if [[ ! -d "$candidate" ]]; then
    return
  fi

  case ":${LD_LIBRARY_PATH:-}:" in
    *":$candidate:"*) ;;
    *)
      if [[ -n "${LD_LIBRARY_PATH:-}" ]]; then
        export LD_LIBRARY_PATH="${LD_LIBRARY_PATH}:$candidate"
      else
        export LD_LIBRARY_PATH="$candidate"
      fi
      ;;
  esac
}

IFS=':' read -r -a host_lib_dirs <<<"${HUNK_LINUX_HOST_GRAPHICS_LIBRARY_PATHS:-}"
for host_lib_dir in "${host_lib_dirs[@]}"; do
  [[ -n "$host_lib_dir" ]] || continue
  append_path "$host_lib_dir"
done

exec "$@"
