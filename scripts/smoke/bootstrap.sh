#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/../.." && pwd)"

cd "$repo_root"

cargo build
./target/debug/eternalMac --help | grep -q "status"
./target/debug/eternalMac status --help >/dev/null
./target/debug/eternalMac doctor --help >/dev/null
./target/debug/eternalMac doctor | grep -q "config missing"
