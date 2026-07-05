#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PRODUCTION_LAUNCHER="${ROOT}/scripts/run_production_voxel_frontend.ps1"

DRY_RUN=false
SMOKE_SECONDS=""
PROFILE="MinSpecComfort1080p"
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
    --profile)
      PROFILE="${2:-}"
      if [[ -z "${PROFILE}" ]]; then
        echo "--profile requires a value" >&2
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

POWERSHELL_BIN="${POWERSHELL_BIN:-powershell}"
if ! command -v "${POWERSHELL_BIN}" >/dev/null 2>&1; then
  if command -v pwsh >/dev/null 2>&1; then
    POWERSHELL_BIN="pwsh"
  else
    echo "PowerShell is required for the production voxel launcher." >&2
    exit 127
  fi
fi

COMMAND=(
  "${POWERSHELL_BIN}" -NoProfile -ExecutionPolicy Bypass -File
  "${PRODUCTION_LAUNCHER}" -Profile "${PROFILE}"
)
if [[ -n "${SMOKE_SECONDS}" ]]; then
  COMMAND+=(-SmokeSeconds "${SMOKE_SECONDS}")
fi
if [[ "${DRY_RUN}" == true ]]; then
  COMMAND+=(-DryRun)
fi

printf 'FVR08 compatibility alias: scripts/run_graphical_playground.sh now routes to scripts/run_production_voxel_frontend.ps1.\n'
printf 'A-Life production voxel frontend command:\n'
printf '%q ' "${COMMAND[@]}"
printf '\n'
printf 'Manual graphics path: requires local windowing/graphics support. CPU fallback is explicit in production diagnostics.\n'

cd "${ROOT}"
"${COMMAND[@]}"
