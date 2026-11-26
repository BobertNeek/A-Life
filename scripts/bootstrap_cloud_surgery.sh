#!/usr/bin/env bash
set -euo pipefail

# Zero-drift cloud recovery bootstrap for A-Life weight programming & V11 surgery.
# Restores software Vulkan (Mesa lavapipe), builds the pinned brain_surgery_eval
# evaluator, proves the REAL active base bundle (weights + V11 branches) survives a
# separate-process GPU reopen, and emits current surgery_loop watchdog state.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

state_file="${SURGERY_STATE_FILE:-SURGERY_STATE.json}"
profile_flag="--release"
target_subdir="release"
reopen_seed="${SURGERY_REOPEN_SEED:-700001}"
allow_dirty_git=0
skip_reopen=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --state)
      state_file="$2"
      shift 2
      ;;
    --debug)
      profile_flag=""
      target_subdir="debug"
      shift
      ;;
    --seed)
      reopen_seed="$2"
      shift 2
      ;;
    --allow-dirty)
      allow_dirty_git=1
      shift
      ;;
    --skip-reopen)
      skip_reopen=1
      shift
      ;;
    *)
      echo "usage: scripts/bootstrap_cloud_surgery.sh [--state PATH] [--debug] [--seed N] [--allow-dirty] [--skip-reopen]" >&2
      exit 2
      ;;
  esac
done

# 1. Configure Mesa software Vulkan automatically on headless cloud Linux hosts.
if [ -z "${VK_ICD_FILENAMES:-}" ] && [ -z "${VK_DRIVER_FILES:-}" ]; then
  for icd in \
    /usr/share/vulkan/icd.d/lvp_icd.x86_64.json \
    /usr/share/vulkan/icd.d/lvp_icd.json \
    /etc/vulkan/icd.d/lvp_icd.x86_64.json; do
    if [ -f "${icd}" ]; then
      export VK_ICD_FILENAMES="${icd}"
      export VK_DRIVER_FILES="${icd}"
      break
    fi
  done
fi
export WGPU_BACKEND="${WGPU_BACKEND:-vulkan}"
export ALIFE_SLM_PRIOR_BACKEND="${ALIFE_SLM_PRIOR_BACKEND:-deterministic}"

echo "software_vulkan_icd=${VK_ICD_FILENAMES:-auto} wgpu_backend=${WGPU_BACKEND} prior=${ALIFE_SLM_PRIOR_BACKEND}"

# 2. Enforce clean engine code before building evaluator unless explicitly bypassed.
if [ "${allow_dirty_git}" -eq 0 ] && [ -n "$(git status --porcelain -- crates/)" ]; then
  echo "ERROR: dirty files under crates/ detected; isolate engine work from weight surgery or pass --allow-dirty." >&2
  exit 2
fi

# 3. Build the pinned surgical evaluator binary.
# shellcheck source=rust_env.sh
source scripts/rust_env.sh
ensure_cargo

cargo build --locked -p alife_game_app --features foundation-training --bin brain_surgery_eval ${profile_flag}
eval_bin="${repo_root}/target/${target_subdir}/brain_surgery_eval"
if [ ! -x "${eval_bin}" ] && [ -x "${eval_bin}.exe" ]; then
  eval_bin="${eval_bin}.exe"
fi
if [ ! -x "${eval_bin}" ]; then
  echo "ERROR: built evaluator binary not found at ${eval_bin}" >&2
  exit 1
fi
echo "evaluator_bin=${eval_bin}"

py_cmd="python3"
if ! command -v "${py_cmd}" >/dev/null 2>&1; then
  py_cmd="python"
fi

if [ ! -f "${state_file}" ]; then
  echo "WARNING: ${state_file} does not exist yet."
  echo "Initialize an authentic base bundle first, e.g.:"
  echo "  ${eval_bin} init-base --output surgery/bases/r41 --base-id r41 --from-save SAVE.json --asset-root ASSETS"
  echo "  ${eval_bin} verify-reopen --bundle surgery/bases/r41"
  echo "  ${py_cmd} -B scripts/surgery_loop.py init --state ${state_file} --base-dir surgery/bases/r41 --base-score 92.5"
  exit 1
fi

# 4. Resolve active_base_dir from state_file and verify its hashes + REAL separate-process reopen.
active_base_dir="$("${py_cmd}" -c '
import json, pathlib, sys
state = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
print(state["active_base_dir"])
' "${state_file}")"

if [ ! -d "${active_base_dir}" ]; then
  echo "ERROR: active_base_dir does not exist: ${active_base_dir}" >&2
  echo "Restore the self-contained base backup (weights + V11 dendritic_branches.json + scorecard) before proceeding." >&2
  exit 1
fi

if [ "${skip_reopen}" -eq 0 ]; then
  echo "Running real active-base separate-process reopen check on ${active_base_dir} (no synthetic stock creature)..."
  "${eval_bin}" verify-reopen --bundle "${active_base_dir}" --seed "${reopen_seed}"
fi

# 5. Print watchdog status and fail if manual rollback is required without recovery.
"${py_cmd}" -B scripts/surgery_loop.py watchdog --state "${state_file}"
