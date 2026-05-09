#!/usr/bin/env bash
set -euo pipefail

cargo build
./target/debug/eternalMac --help >/dev/null
./target/debug/eternalMac status | grep -q "healthy"
