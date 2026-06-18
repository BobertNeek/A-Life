#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMMAND=(
  cargo run -p alife_game_app --features bevy-app --bin alife_game_app --
  visible-world-smoke crates/alife_world/tests/fixtures/p34
)

printf 'A-Life graphical playground smoke command:\n'
printf '%q ' "${COMMAND[@]}"
printf '\n'
printf 'Manual graphics smoke only: requires local graphics support.\n'

if [[ "${1:-}" == "--dry-run" ]]; then
  exit 0
fi

cd "${ROOT}"
"${COMMAND[@]}"
