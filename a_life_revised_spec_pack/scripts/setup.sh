#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1 && [ -n "${USERPROFILE:-}" ] && [ -x "${USERPROFILE}/.cargo/bin/cargo.exe" ]; then
  export PATH="${USERPROFILE}/.cargo/bin:${PATH}"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust/cargo is required. Install Rust from https://rustup.rs/ or with winget install Rustlang.Rustup."
  exit 1
fi

echo "Rust toolchain: $(cargo --version)"

if command -v graphify >/dev/null 2>&1 || { [ -n "${USERPROFILE:-}" ] && [ -x "${USERPROFILE}/.local/bin/graphify.exe" ]; }; then
  echo "Graphify is available for optional developer graph updates."
else
  echo "Graphify is optional. Install with: uv tool install graphifyy"
  echo "Then run: graphify install --project --platform codex"
fi
