#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMMAND=(
  cargo run -p alife_tools --bin p35_playground --
  run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json
)

printf 'A-Life headless playground command:\n'
printf '%q ' "${COMMAND[@]}"
printf '\n'

if [[ "${1:-}" == "--dry-run" ]]; then
  exit 0
fi

cd "${ROOT}"
"${COMMAND[@]}"
