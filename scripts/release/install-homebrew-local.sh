#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/../.." && pwd)"

tap="eternalmac/eternalmac"
formula_path=""

usage() {
  cat <<'USAGE'
Usage: scripts/release/install-homebrew-local.sh [options]

Copy a generated eternalMac formula into a local Homebrew tap and install it.

Options:
  --tap <user/repo>            Local tap name. Defaults to eternalmac/eternalmac,
                               which maps to GitHub repo homebrew-eternalmac.
  --formula <path>             Formula to copy into the tap.
                               Defaults to target/homebrew/Formula/eternalmac.rb.
  -h, --help                   Show this help.
USAGE
}

while (($#)); do
  case "$1" in
    --tap)
      tap="${2:?missing value for --tap}"
      shift 2
      ;;
    --formula)
      formula_path="${2:?missing value for --formula}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

cd "$repo_root"

formula_path="${formula_path:-${repo_root}/target/homebrew/Formula/eternalmac.rb}"
if [[ ! -f "$formula_path" ]]; then
  echo "formula does not exist: $formula_path" >&2
  echo "run scripts/release/package-homebrew.sh first" >&2
  exit 1
fi

if ! brew tap | grep -qx "$tap"; then
  brew tap-new "$tap"
fi

tap_repo="$(brew --repository "$tap")"
mkdir -p "${tap_repo}/Formula"
cp "$formula_path" "${tap_repo}/Formula/eternalmac.rb"

formula_ref="${tap}/eternalmac"
if brew list --formula eternalmac >/dev/null 2>&1; then
  brew reinstall "$formula_ref"
else
  brew install "$formula_ref"
fi

brew test "$formula_ref"

cat <<OUTPUT
installed: $formula_ref
formula:   ${tap_repo}/Formula/eternalmac.rb
OUTPUT
