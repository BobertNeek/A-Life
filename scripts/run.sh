#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

cargo run \
  -p alife_game_app \
  --features production-voxel-frontend \
  --bin alife_game_app \
  -- production-voxel "$@"
