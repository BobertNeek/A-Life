#!/usr/bin/env bash

# Shared Cargo discovery for repository shell entrypoints.
ensure_cargo() {
  if ! command -v cargo >/dev/null 2>&1 \
    && [ -n "${USERPROFILE:-}" ] \
    && [ -x "${USERPROFILE}/.cargo/bin/cargo.exe" ]; then
    export PATH="${USERPROFILE}/.cargo/bin:${PATH}"
  fi

  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required for this A-Life command" >&2
    return 1
  fi
}
