#!/usr/bin/env bash
set -euo pipefail

if ! command -v rustup >/dev/null 2>&1; then
  if command -v winget.exe >/dev/null 2>&1; then
    winget.exe install --id Rustlang.Rustup -e --accept-package-agreements --accept-source-agreements
  else
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  fi
fi

if ! command -v gh >/dev/null 2>&1 && command -v winget.exe >/dev/null 2>&1; then
  winget.exe install --id GitHub.cli -e --accept-package-agreements --accept-source-agreements
fi

rustup target add wasm32-unknown-unknown

export ALIFE_GPU_PROFILE="${ALIFE_GPU_PROFILE:-minimum-2gb}"
export ALIFE_ORGANISMS="${ALIFE_ORGANISMS:-8}"

echo "A-Life environment ready"
echo "ALIFE_GPU_PROFILE=${ALIFE_GPU_PROFILE}"
echo "ALIFE_ORGANISMS=${ALIFE_ORGANISMS}"
