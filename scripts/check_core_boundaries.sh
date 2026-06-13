#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

if ! command -v cargo >/dev/null 2>&1 && [ -n "${USERPROFILE:-}" ] && [ -x "${USERPROFILE}/.cargo/bin/cargo.exe" ]; then
  export PATH="${USERPROFILE}/.cargo/bin:${PATH}"
fi

forbidden_dep_regex='(^|[^[:alnum:]_])(bevy|avian|wgpu|winit|render|python|pyo3)([^[:alnum:]_]|$)'
forbidden_symbol_regex='(^|[^[:alnum:]_])(bevy|avian|wgpu|RenderDevice|RenderQueue|Entity|Vec3|Quat)([^[:alnum:]_]|$)'

if [ "${1:-}" = "--self-test" ]; then
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "${tmp_dir}"' EXIT
  printf 'bevy = "0.0"\n' > "${tmp_dir}/Cargo.toml"
  printf 'use wgpu::Device;\n' > "${tmp_dir}/bad.rs"
  grep -Eiq "${forbidden_dep_regex}" "${tmp_dir}/Cargo.toml"
  grep -Eiq "${forbidden_symbol_regex}" "${tmp_dir}/bad.rs"
  echo "alife_core boundary regex self-test passed"
  exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for core boundary checks" >&2
  exit 1
fi

tree_output="$(cargo tree -p alife_core --format "{lib}")"
if printf '%s\n' "${tree_output}" | grep -Eiq "${forbidden_dep_regex}"; then
  echo "alife_core has a forbidden engine/runtime dependency:" >&2
  printf '%s\n' "${tree_output}" >&2
  exit 1
fi

if grep -InE '^[[:space:]]*(bevy|avian|wgpu|winit|render|python|pyo3)[[:space:]]*=' crates/alife_core/Cargo.toml; then
  echo "alife_core Cargo.toml declares a forbidden dependency" >&2
  exit 1
fi

if grep -RInE --include='*.rs' "${forbidden_symbol_regex}" crates/alife_core/src crates/alife_core/tests; then
  echo "alife_core source references forbidden engine/runtime symbols" >&2
  exit 1
fi

echo "alife_core boundary checks passed"
