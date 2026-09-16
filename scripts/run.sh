#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"
# shellcheck source=rust_env.sh
source scripts/rust_env.sh
ensure_cargo

cargo run \
  -p alife_game_app \
  --features production-voxel-frontend \
  --bin alife_game_app \
  -- production-voxel "$@"
