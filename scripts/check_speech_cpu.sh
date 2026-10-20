#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 0 ]; then
  echo "usage: bash scripts/check_speech_cpu.sh" >&2
  exit 2
fi
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"
# shellcheck source=rust_env.sh
source scripts/rust_env.sh
ensure_cargo

# This lane compiles the feature-gated production helpers but executes only CPU
# speech fixtures. Never enable gpu-tests, foundation-training, or all-features.
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
export CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export RUST_MIN_STACK=33554432
command=(cargo test --locked -p alife_game_app --lib --features bevy-app,gpu-runtime)
for package in alife_core alife_world alife_gpu_backend alife_runtime alife_game_app; do
  command+=(--config "profile.test.package.${package}.opt-level=0")
done

log="$(mktemp)"
trap 'rm -f "$log"' EXIT
module=production_conversation_lineage_ui
regression="${module}::speech_completion_state_tests::worker_completion_preserves_a_reopened_newer_unicode_draft"
for group in "${module}::speech_completion_state_tests:7" "${module}::speech_worker_tests:6" "${module}::speech_input_tests:11" "${module}::composer::tests:6" production_voxel_renderer::care_dock::tests:15; do
  filter="${group%:*}::"
  "${command[@]}" "$filter" -- --test-threads=1 --color=never | tee "$log"
  # Cargo considers an empty filter a success. Refuse missing or ignored coverage.
  python3 - "$log" "${group##*:}" <<'PY'
import pathlib, re, sys
log = pathlib.Path(sys.argv[1]).read_text()
results = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', log)
if len(results) != 1 or int(results[0][0]) < int(sys.argv[2]) or results[0][1:] != ('0', '0'):
    raise SystemExit('Focused speech coverage missing, failed, or ignored')
PY
  if [ "${group%:*}" = "${module}::speech_completion_state_tests" ]; then
    grep -Fqx "test ${regression} ... ok" "$log" || {
      echo "Required reopened-draft regression did not pass" >&2
      exit 1
    }
  fi
done
