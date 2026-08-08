#!/usr/bin/env bash
set -euo pipefail

authorities=(
  README.md
  docs/AGENTS.md
  docs/VISION.md
  docs/STATUS.md
  docs/ARCHITECTURE.md
  docs/ROADMAP.md
  docs/DEVELOPMENT.md
  docs/EVIDENCE.md
  docs/REFERENCE.md
)

for authority in "${authorities[@]}"; do
  test -f "${authority}"
done

test -f crates/alife_tools/tests/fixtures/P04_5_performance_contract.md
test -f examples/ca43/TESTER_FEEDBACK_TEMPLATE.md

for obsolete in \
  docs/master_spec.md \
  docs/architecture_decisions.md \
  docs/release_checklist.md \
  docs/final_status_report.md \
  docs/gpu_soak_performance_plan.md \
  docs/playground_examples.md; do
  if [[ -e "${obsolete}" ]]; then
    printf 'docs_check: obsolete authority remains: %s\n' "${obsolete}" >&2
    exit 1
  fi
done

for crate_agents in crates/*/AGENTS.md; do
  test -f "${crate_agents}"
done

semantic_checks=0
semantic_failures=0

normalized_file() {
  tr '\r\n\t' '   ' < "$1" | tr -s ' '
}

normalized_text() {
  printf '%s' "$1" | tr '\r\n\t' '   ' | tr -s ' '
}

contains_text() {
  local haystack
  local needle
  haystack="$(normalized_file "$1")"
  needle="$(normalized_text "$2")"
  [[ "${haystack,,}" == *"${needle,,}"* ]]
}

require_text() {
  local file="$1"
  local expected="$2"
  local description="$3"
  semantic_checks=$((semantic_checks + 1))

  if ! contains_text "${file}" "${expected}"; then
    printf 'docs_check: missing %s in %s\n' "${description}" "${file}" >&2
    semantic_failures=$((semantic_failures + 1))
  fi
}

forbid_text() {
  local file="$1"
  local stale="$2"
  local description="$3"
  semantic_checks=$((semantic_checks + 1))

  if contains_text "${file}" "${stale}"; then
    printf 'docs_check: stale %s remains in %s\n' "${description}" "${file}" >&2
    semantic_failures=$((semantic_failures + 1))
  fi
}

require_text README.md 'production cognition remains GPU-authoritative' 'GPU-authoritative production statement'
require_text README.md 'renderer does not yet project live runtime transforms, births, or deaths' 'open presentation bridge'
require_text docs/STATUS.md 'active voxel renderer remains a save-derived projection' 'honest current product boundary'
require_text docs/STATUS.md 'EI1 retained 2,640 source-bound descendant receipts' 'EI1 retained receipt count'
require_text docs/STATUS.md 'Its promotion verdict is `Blocked`' 'EI1 blocked verdict'
require_text docs/ARCHITECTURE.md 'world perception + unscored legal candidates' 'score-free world candidate boundary'
require_text docs/ARCHITECTURE.md 'GPU unavailability is typed unavailability' 'typed GPU failure semantics'
require_text docs/ROADMAP.md 'Live GPU-to-voxel projection' 'first roadmap phase'
require_text docs/ROADMAP.md 'Autonomous production lifecycle' 'lifecycle roadmap phase'
require_text docs/DEVELOPMENT.md 'scripts/docs_check.ps1' 'Windows docs gate'
require_text docs/DEVELOPMENT.md 'source-bound physical-adapter evidence' 'hardware evidence rule'
require_text docs/EVIDENCE.md 'A report bound to an older source remains valid historical evidence for that source.' 'historical evidence scope'
require_text docs/EVIDENCE.md 'promotion verdict `Blocked`' 'EI1 evidence verdict'
require_text docs/REFERENCE.md 'CPU neural helpers: reference, test, or developer use only.' 'CPU helper boundary'
require_text docs/REFERENCE.md 'N4096 | Research-only migration/equivalence class' 'N4096 research boundary'
require_text docs/REFERENCE.md 'author creature thought or raw speech' 'SLM authority boundary'
require_text docs/AGENTS.md 'The Git history is the archive for superseded plans and specifications.' 'single documentation authority set'
require_text crates/alife_core/AGENTS.md 'Never equate a language token ID with a neuron or packed GPU offset.' 'core language boundary'
require_text crates/alife_gpu_backend/AGENTS.md 'Neural `Vocalize` payload selection remains GPU-authoritative.' 'GPU speech authority'
require_text crates/alife_school/AGENTS.md 'Run language mastery gates with SLM translation disabled.' 'school evaluation boundary'
require_text crates/alife_semantic/AGENTS.md 'Translation may map or render bounded raw tokens; it may not author creature thought or speech.' 'semantic translation boundary'
require_text crates/alife_world/AGENTS.md 'Death archiving completes before GPU retirement and despawn.' 'world retirement order'

for authority in "${authorities[@]}"; do
  forbid_text "${authority}" 'EI1 promotion passed' 'false EI1 promotion claim'
  forbid_text "${authority}" 'N4096 is production' 'false N4096 production claim'
  forbid_text "${authority}" 'GPU work, if any, is gated by CPU parity' 'obsolete CPU parity gate'
done

if (( semantic_failures > 0 )); then
  printf 'DOCS_ASSERTIONS=FAIL (%d/%d failed)\n' "${semantic_failures}" "${semantic_checks}" >&2
  exit 1
fi

printf 'DOCS_ASSERTIONS=PASS (%d/%d)\n' "${semantic_checks}" "${semantic_checks}"
