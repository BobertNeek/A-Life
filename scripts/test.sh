#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=rust_env.sh
source "${script_dir}/rust_env.sh"
ensure_cargo

cargo test --workspace --all-targets
