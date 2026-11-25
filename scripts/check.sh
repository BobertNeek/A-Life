#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

mode="${1:---full}"
if [ "$#" -gt 0 ]; then
  shift
fi
if [ "$#" -ne 0 ]; then
  echo "usage: scripts/check.sh [--quick|--full|--frontend|--training]" >&2
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
    cargo test --locked --workspace --all-targets
    cargo clippy --locked --workspace --all-targets -- -D warnings
    "${BASH}" scripts/check_core_boundaries.sh
    ;;
  --frontend)
    # shellcheck source=rust_env.sh
    source scripts/rust_env.sh
    ensure_cargo
    cargo fmt --all -- --check
    cargo test --locked -p alife_game_app --features production-voxel-frontend --test bevy_texture_decode
    cargo test --locked -p alife_game_app --features production-voxel-frontend --lib production_voxel_renderer:: -- --test-threads=1
    cargo test --locked -p alife_game_app --features production-voxel-frontend --lib production_conversation_lineage_ui:: -- --test-threads=1
    cargo clippy --locked -p alife_game_app --features production-voxel-frontend --all-targets -- -D warnings
    ;;
  --training)
    # shellcheck source=rust_env.sh
    source scripts/rust_env.sh
    ensure_cargo
    cargo fmt --all -- --check
    cargo test --locked -p alife_gpu_backend --all-targets --features training-rollout
    cargo test --locked -p alife_training --features cpu-backend --lib
    cargo test --locked -p alife_game_app --features foundation-training --lib foundation_training::
    cargo test --locked -p alife_game_app --features cpu-training --lib foundation_cpu_care::
    cargo clippy --locked -p alife_training --features cpu-backend --all-targets -- -D warnings
    cargo clippy --locked -p alife_game_app --features foundation-training --all-targets -- -D warnings
    py_cmd="python"
    if ! command -v "${py_cmd}" >/dev/null 2>&1; then
      py_cmd="python3"
    fi
    "${py_cmd}" -B -m unittest discover -s scripts -p "test_*.py" -v
    ;;
  *)
    echo "usage: scripts/check.sh [--quick|--full|--frontend|--training]" >&2
    exit 2
    ;;
esac
