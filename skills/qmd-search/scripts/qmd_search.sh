#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  qmd_search.sh --root PATH --collection NAME --query TEXT [options]

Options:
  --index PATH       SQLite index path (default: .qmd/index.sqlite)
  --pattern GLOB     Markdown glob pattern (default: **/*.md)
  --limit N          Max search results (default: 10)
  --qmd-bin PATH     Explicit qmd binary to use
  --help             Show this help
USAGE
}

index=".qmd/index.sqlite"
pattern="**/*.md"
limit="10"
root=""
collection=""
query=""
qmd_bin=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --index)
      index="${2:?missing value for --index}"
      shift 2
      ;;
    --pattern)
      pattern="${2:?missing value for --pattern}"
      shift 2
      ;;
    --limit)
      limit="${2:?missing value for --limit}"
      shift 2
      ;;
    --root)
      root="${2:?missing value for --root}"
      shift 2
      ;;
    --collection)
      collection="${2:?missing value for --collection}"
      shift 2
      ;;
    --query)
      query="${2:?missing value for --query}"
      shift 2
      ;;
    --qmd-bin)
      qmd_bin="${2:?missing value for --qmd-bin}"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [ -z "$root" ] || [ -z "$collection" ] || [ -z "$query" ]; then
  echo "error: --root, --collection, and --query are required" >&2
  usage >&2
  exit 2
fi

if [ ! -d "$root" ]; then
  echo "error: root is not a directory: $root" >&2
  exit 2
fi

run_qmd() {
  if [ -n "$qmd_bin" ]; then
    "$qmd_bin" "$@"
  elif command -v qmd >/dev/null 2>&1; then
    qmd "$@"
  else
    script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
    repo_root="$(CDPATH= cd -- "$script_dir/../../.." && pwd)"
    if [ -f "$repo_root/Cargo.toml" ] && [ -d "$repo_root/qmd-cli" ]; then
      cargo run --quiet -p qmd-cli --manifest-path "$repo_root/Cargo.toml" -- "$@"
    else
      echo "error: qmd binary not found and repository fallback is unavailable" >&2
      exit 127
    fi
  fi
}

run_qmd --index "$index" collection add "$root" --name "$collection" --pattern "$pattern" >/dev/null
run_qmd --index "$index" update --collection "$collection" >/dev/null
run_qmd --index "$index" search "$query" --limit "$limit" --json
