#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

find_graphify() {
  if command -v graphify >/dev/null 2>&1; then
    command -v graphify
    return 0
  fi

  if [ -n "${USERPROFILE:-}" ] && [ -x "${USERPROFILE}/.local/bin/graphify.exe" ]; then
    printf '%s\n' "${USERPROFILE}/.local/bin/graphify.exe"
    return 0
  fi

  if [ -n "${HOME:-}" ] && [ -x "${HOME}/.local/bin/graphify" ]; then
    printf '%s\n' "${HOME}/.local/bin/graphify"
    return 0
  fi

  return 1
}

if ! graphify_bin="$(find_graphify)"; then
  echo "Graphify is optional and not installed."
  echo "Install with: uv tool install graphifyy"
  echo "Then run: graphify install --project --platform codex"
  exit 0
fi

command="${1:-update}"
shift || true

case "${command}" in
  update)
    "${graphify_bin}" update . "$@"
    ;;
  extract)
    "${graphify_bin}" extract . "$@"
    ;;
  query)
    if [ "$#" -eq 0 ]; then
      echo 'Usage: scripts/graphify.sh query "question"'
      exit 2
    fi
    "${graphify_bin}" query "$@"
    ;;
  *)
    "${graphify_bin}" "${command}" "$@"
    ;;
esac
