#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
if [[ $(uname -s) != Linux || $(uname -m) != x86_64 ]]; then
  echo 'Este script requiere Linux x64.' >&2
  exit 1
fi
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
cargo build --release --locked --bin luma-launcher
python3 scripts/package-linux.py "${CARGO_TARGET_DIR:-target}/release/luma-launcher"
