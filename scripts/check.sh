#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

mode="${1:---full}"
if [ "$#" -gt 0 ]; then
  shift
fi
if [ "$#" -ne 0 ]; then
  echo "usage: scripts/check.sh [--quick|--full]" >&2
  exit 2
fi

case "${mode}" in
  --quick)
    git diff --check
    "${BASH}" scripts/check_core_boundaries.sh --static
    ;;
  --full)
    # shellcheck source=rust_env.sh
    source scripts/rust_env.sh
    ensure_cargo
    cargo fmt --all -- --check
    cargo check --locked --workspace --all-targets
    cargo test --locked --workspace --all-targets
    cargo clippy --locked --workspace --all-targets -- -D warnings
    "${BASH}" scripts/check_core_boundaries.sh
    ;;
  *)
    echo "usage: scripts/check.sh [--quick|--full]" >&2
    exit 2
    ;;
esac
