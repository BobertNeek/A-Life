#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"
# shellcheck source=rust_env.sh
source scripts/rust_env.sh

ensure_cargo
command -v git >/dev/null 2>&1 || {
  echo "git is required for the A-Life workspace" >&2
  exit 1
}
command -v python >/dev/null 2>&1 || command -v python3 >/dev/null 2>&1 || {
  echo "Python 3 is required for production asset tooling" >&2
  exit 1
}
test -f crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json || {
  echo "production voxel asset manifest is missing" >&2
  exit 1
}

echo "Rust toolchain: $(cargo --version)"
echo "Git: $(git --version)"

if command -v vulkaninfo >/dev/null 2>&1; then
  echo "Vulkan tools: available"
else
  echo "Vulkan tools: not found; the production GPU preflight must verify the driver"
fi

if command -v blender >/dev/null 2>&1 \
  || [ -x "/c/Program Files/Blender Foundation/Blender 5.1/blender.exe" ]; then
  echo "Blender 5.1: available for GeneForge asset work"
else
  echo "Blender 5.1: not found; required only for GeneForge asset regeneration"
fi

if command -v graphify >/dev/null 2>&1 || { [ -n "${USERPROFILE:-}" ] && [ -x "${USERPROFILE}/.local/bin/graphify.exe" ]; }; then
  echo "Graphify is available for optional developer graph updates."
else
  echo "Graphify is optional. Install with: uv tool install graphifyy"
  echo "Then run: graphify install --project --platform codex"
fi
