#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

export ALIFE_GPU_PROFILE="${ALIFE_GPU_PROFILE:-minimum-2gb}"
export ALIFE_ORGANISMS="${ALIFE_ORGANISMS:-8}"

cargo run -p alife --bin alife_demo
