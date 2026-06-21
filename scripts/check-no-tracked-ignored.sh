#!/usr/bin/env bash
set -euo pipefail

tracked_ignored="$(git ls-files -ci --exclude-standard)"

if [[ -n "$tracked_ignored" ]]; then
  {
    echo "error: tracked files match .gitignore:"
    echo "$tracked_ignored"
    echo
    echo "Remove these files from the Git index with:"
    echo "  git rm --cached <path>..."
  } >&2
  exit 1
fi
