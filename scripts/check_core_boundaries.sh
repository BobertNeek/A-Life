#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

if ! command -v cargo >/dev/null 2>&1 && [ -n "${USERPROFILE:-}" ] && [ -x "${USERPROFILE}/.cargo/bin/cargo.exe" ]; then
  export PATH="${USERPROFILE}/.cargo/bin:${PATH}"
fi

forbidden_dep_regex='(^|[^[:alnum:]_])(bevy|avian|wgpu|winit|render|python|pyo3)([^[:alnum:]_]|$)'
if [ "${1:-}" != "--static" ]; then
  # shellcheck source=rust_env.sh
  source scripts/rust_env.sh
  ensure_cargo
  tree_output="$(cargo tree -p alife_core --format "{lib}")"
  if printf '%s\n' "${tree_output}" | grep -Eiq "${forbidden_dep_regex}"; then
    echo "alife_core has a forbidden engine/runtime dependency:" >&2
    printf '%s\n' "${tree_output}" >&2
    exit 1
  fi
fi

if grep -InE '^[[:space:]]*(bevy|avian|wgpu|winit|render|python|pyo3)[[:space:]]*=' crates/alife_core/Cargo.toml; then
  echo "alife_core Cargo.toml declares a forbidden dependency" >&2
  exit 1
fi

echo "alife_core boundary checks passed"
