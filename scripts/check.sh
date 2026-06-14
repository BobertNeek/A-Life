#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

if ! command -v cargo >/dev/null 2>&1 && [ -n "${USERPROFILE:-}" ] && [ -x "${USERPROFILE}/.cargo/bin/cargo.exe" ]; then
  export PATH="${USERPROFILE}/.cargo/bin:${PATH}"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for the A-Life validation gate" >&2
  exit 1
fi

cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
"${BASH}" scripts/check_core_boundaries.sh
"${BASH}" scripts/docs_check.sh
