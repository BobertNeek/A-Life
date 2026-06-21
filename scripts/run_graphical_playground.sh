#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

DRY_RUN=false
SMOKE_SECONDS=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --smoke-seconds)
      SMOKE_SECONDS="${2:-}"
      if [[ -z "${SMOKE_SECONDS}" ]]; then
        echo "--smoke-seconds requires a value" >&2
        exit 2
      fi
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

MODE_ARGS=(graphical-playground)
MODE_LABEL="persistent graphical playground"
if [[ -n "${SMOKE_SECONDS}" ]]; then
  MODE_ARGS=(graphical-playground-smoke --seconds "${SMOKE_SECONDS}")
  MODE_LABEL="bounded graphical playground smoke"
fi

COMMAND=(
  cargo run -p alife_game_app --features bevy-app --bin alife_game_app --
  "${MODE_ARGS[@]}" crates/alife_world/tests/fixtures/p34
)

printf 'A-Life %s command:\n' "${MODE_LABEL}"
printf '%q ' "${COMMAND[@]}"
printf '\n'
printf 'Manual graphics path: requires local windowing/graphics support. CPU fallback is used for cognition/backend status.\n'

if [[ "${DRY_RUN}" == true ]]; then
  exit 0
fi

cd "${ROOT}"
"${COMMAND[@]}"
