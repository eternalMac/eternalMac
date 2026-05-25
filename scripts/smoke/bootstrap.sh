#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/../.." && pwd)"

cd "$repo_root"

original_home="${HOME}"
smoke_home="$(mktemp -d)"
trap 'rm -rf "$smoke_home"' EXIT

export CARGO_HOME="${CARGO_HOME:-${original_home}/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-${original_home}/.rustup}"
export HOME="$smoke_home"

cargo build
./target/debug/eternalMac --help | grep -q "status"
./target/debug/eternalMac status --help >/dev/null
./target/debug/eternalMac doctor --help >/dev/null
doctor_output="$(./target/debug/eternalMac doctor 2>&1)" && {
  echo "doctor unexpectedly succeeded on an unconfigured HOME"
  exit 1
}
grep -q "config missing" <<<"$doctor_output"
