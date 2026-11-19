#!/usr/bin/env bash
# Public release with a verified digest; a signed distribution package is a
# separate fallback when the public CDN is unavailable. Never accept partial
# downloads or silently change the recorded renderer between paired images.
set -euo pipefail
cd "${RUNNER_TEMP:?hosted review workspace required}"
archive=blender-4.3.2-linux-x64.tar.xz
release=https://download.blender.org/release/Blender4.3
if test -s "$archive" || curl -fsS --retry 2 --max-time 120 "$release/$archive" -o "$archive"; then
    echo "4da1c956673c0485e63054e563ee69198cc8f80d8157dd7592dffc8a6a5592e6  $archive" | sha256sum -c
    tar -xJf "$archive"
    echo "$RUNNER_TEMP/blender-4.3.2-linux-x64" >> "$GITHUB_PATH"
else
    sudo apt-get -o Acquire::Retries=2 -o Acquire::http::Timeout=30 -q update
    sudo apt-get -o Acquire::Retries=2 -o Acquire::http::Timeout=30 -y --no-install-recommends install blender
fi
