#!/usr/bin/env bash
set -euo pipefail

DEFAULT_REPO_DIR="/tmp/hunk-large-diff-repo"
repo_dir="$DEFAULT_REPO_DIR"
file_count=1
lines_per_file=25000
force=0
snapshot_max_new_file_size=33554432
bookmark_name="main"
language="txt"

usage() {
    cat <<'USAGE'
Create a synthetic JJ repository with a very large text diff for Hunk performance testing.

Usage:
  ./scripts/create_large_diff_repo.sh [options]

Options:
  --dir <path>      Destination repo path (default: /tmp/hunk-large-diff-repo)
  --files <count>   Number of files with large diffs (default: 1)
  --lines <count>   Changed lines per file (default: 25000)
  --lang <kind>     Diff content kind: txt | js | ts (default: txt)
  --force           Replace destination directory if it already exists
  -h, --help        Show this help message

Examples:
  ./scripts/create_large_diff_repo.sh
  ./scripts/create_large_diff_repo.sh --files 4 --lines 6000 --lang ts --force
  ./scripts/create_large_diff_repo.sh --dir /tmp/hunk-stress --lines 30000 --force
USAGE
}

is_positive_integer() {
    [[ "$1" =~ ^[1-9][0-9]*$ ]]
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dir)
            [[ $# -ge 2 ]] || {
                echo "Missing value for --dir" >&2
                exit 1
            }
            repo_dir="$2"
            shift 2
            ;;
        --files)
            [[ $# -ge 2 ]] || {
                echo "Missing value for --files" >&2
                exit 1
            }
            file_count="$2"
            shift 2
            ;;
        --lines)
            [[ $# -ge 2 ]] || {
                echo "Missing value for --lines" >&2
                exit 1
            }
            lines_per_file="$2"
            shift 2
            ;;
        --force)
            force=1
            shift
            ;;
        --lang)
            [[ $# -ge 2 ]] || {
                echo "Missing value for --lang" >&2
                exit 1
            }
            language="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

is_positive_integer "$file_count" || {
    echo "--files must be a positive integer" >&2
    exit 1
}

is_positive_integer "$lines_per_file" || {
    echo "--lines must be a positive integer" >&2
    exit 1
}

case "$language" in
    txt|js|ts) ;;
    *)
        echo "--lang must be one of: txt, js, ts" >&2
        exit 1
        ;;
esac

if [[ -e "$repo_dir" ]]; then
    if [[ "$force" -eq 1 ]]; then
        rm -rf "$repo_dir"
    else
        echo "Destination already exists: $repo_dir" >&2
        echo "Use --force to replace it." >&2
        exit 1
    fi
fi

mkdir -p "$repo_dir"
jj --quiet git init "$repo_dir" >/dev/null 2>&1
jj --quiet -R "$repo_dir" config set --repo snapshot.max-new-file-size "$snapshot_max_new_file_size" >/dev/null 2>&1

create_file_contents() {
    local phase="$1"
    local output_path="$2"
    awk -v lines="$lines_per_file" -v phase="$phase" -v lang="$language" '
        BEGIN {
            for (i = 1; i <= lines; i++) {
                if (lang == "ts") {
                    if (phase == "before") {
                        printf "export const metric_%06d: number = %d + 17;\n", i, i
                    } else {
                        printf "export const metric_%06d: number = (%d * 3) - 11;\n", i, i
                    }
                } else if (lang == "js") {
                    if (phase == "before") {
                        printf "export const metric_%06d = %d + 17;\n", i, i
                    } else {
                        printf "export const metric_%06d = (%d * 3) - 11;\n", i, i
                    }
                } else {
                    printf "%s line %06d: hunk diff stress payload for throughput and frame pacing\n", phase, i
                }
            }
        }
    ' >"$output_path"
}

extension_for_language() {
    case "$language" in
        ts) echo "ts" ;;
        js) echo "js" ;;
        *) echo "txt" ;;
    esac
}

file_extension="$(extension_for_language)"

for i in $(seq 1 "$file_count"); do
    file_path="$repo_dir/stress/file_$(printf '%03d' "$i").$file_extension"
    mkdir -p "$(dirname "$file_path")"
    create_file_contents "before" "$file_path"
done

jj --quiet -R "$repo_dir" commit -m "Baseline for Hunk large-diff stress test" >/dev/null 2>&1

for i in $(seq 1 "$file_count"); do
    file_path="$repo_dir/stress/file_$(printf '%03d' "$i").$file_extension"
    create_file_contents "after" "$file_path"
done

jj --quiet -R "$repo_dir" bookmark create "$bookmark_name" -r @ >/dev/null 2>&1

total_changed_rows=$((file_count * lines_per_file))
total_changed_lines=$((total_changed_rows * 2))

printf "Created JJ repo: %s\n" "$repo_dir"
printf "Files changed: %d\n" "$file_count"
printf "Per-file paired rows in Hunk: %d\n" "$lines_per_file"
printf "Total paired rows in Hunk: %d\n" "$total_changed_rows"
printf "Total changed lines in patch (+/-): %d\n" "$total_changed_lines"
printf "Language mode: %s (.%s)\n" "$language" "$file_extension"
printf "Active bookmark: %s\n" "$bookmark_name"
printf "\nOpen this folder in Hunk and watch the FPS badge while scrolling.\n"
