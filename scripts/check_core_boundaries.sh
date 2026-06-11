#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

if ! command -v cargo >/dev/null 2>&1 && [ -n "${USERPROFILE:-}" ] && [ -x "${USERPROFILE}/.cargo/bin/cargo.exe" ]; then
  export PATH="${USERPROFILE}/.cargo/bin:${PATH}"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for core boundary checks" >&2
  exit 1
fi

tree_output="$(cargo tree -p alife_core)"
if printf '%s\n' "${tree_output}" | grep -Eiq '(^|[^[:alnum:]_])(bevy|avian|wgpu|winit|render|python|pyo3)([^[:alnum:]_]|$)'; then
  echo "alife_core has a forbidden engine/runtime dependency:" >&2
  printf '%s\n' "${tree_output}" >&2
  exit 1
fi

if grep -RInE --include='*.rs' '(^|[^[:alnum:]_])(bevy|avian|wgpu|RenderDevice|RenderQueue|Entity|Vec3|Quat)([^[:alnum:]_]|$)' crates/alife_core/src crates/alife_core/tests; then
  echo "alife_core source references forbidden engine/runtime symbols" >&2
  exit 1
fi

echo "alife_core boundary checks passed"
