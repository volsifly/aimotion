#!/usr/bin/env bash
set -euo pipefail
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
cargo build --locked
exec "$root/target/debug/aimotion-desktop" --data-file "$root/current.json"
