#!/usr/bin/env bash
set -euo pipefail

test -f docs/master_spec.md
test -f docs/architecture_decisions.md
test -f AGENTS.md
test -f docs/AGENTS.md
test -f docs/release_checklist.md
test -f docs/final_status_report.md
test -f docs/gpu_soak_performance_plan.md

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

require_all_text() {
  local file="$1"
  local description="$2"
  local expected
  shift 2
  semantic_checks=$((semantic_checks + 1))

  for expected in "$@"; do
    if ! contains_text "${file}" "${expected}"; then
      printf 'docs_check: missing %s in %s\n' "${description}" "${file}" >&2
      semantic_failures=$((semantic_failures + 1))
      return
    fi
  done
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

# Task 1 architecture semantics. Normalize whitespace so prose may be reflowed
# without weakening the checks or coupling them to line numbers.
forbid_text docs/master_spec.md 'prefer empty modules' 'empty-module authority'
forbid_text docs/master_spec.md 'Required crate stubs' 'crate-stub authority'
forbid_text crates/alife_gpu_backend/AGENTS.md 'dispatch placeholders' 'dispatch-placeholder authority'
forbid_text docs/master_spec.md 'Shadow old core behavior before control handoff' 'live-shadow migration step'
forbid_text docs/master_spec.md 'freeze, map, shadow, and gated unfreeze' 'live-shadow migration summary'

require_text docs/master_spec.md 'Implement reviewed contracts and runtime algorithms as focused, strongly named, compilable modules in their owning crates.' 'runtime-module implementation authority'
require_text docs/master_spec.md 'Required crates:' 'required-crates authority'
require_text crates/alife_gpu_backend/AGENTS.md 'dispatch scheduling, and production neural pipelines.' 'production GPU dispatch ownership'
require_text docs/master_spec.md 'Offline deterministic replay and fixture validation exercise the migrated brain without producing world actions.' 'offline migration validation rule'
require_text docs/master_spec.md 'The production handoff is atomic, and old and migrated neural brains never run concurrently in production.' 'atomic single-brain migration handoff'
require_text docs/master_spec.md 'Every production neural capacity request above N2048 returns a typed `ProductionCapacityGateError` before allocation or dispatch.' 'typed above-N2048 rejection'
require_text docs/master_spec.md '`requested_class: BrainClassId`' 'requested capacity class error field'
require_text docs/master_spec.md '`gate_status: ProductionCapacityGateStatus`' 'capacity gate status error field'
require_text docs/master_spec.md '`gate_status` is `Unmet` and identifies the missing causal, hardware, save, soak, memory, and performance gates.' 'unmet production-capacity gate status'
require_text docs/master_spec.md 'Both `alife_core` and `alife_world` depend on none of Bevy, wgpu, renderer types, or OS handles.' 'core and world engine-neutral boundary'
require_text docs/master_spec.md 'Bevy ECS ownership belongs only to adapter/app layers.' 'Bevy ECS ownership boundary'
require_text crates/alife_core/AGENTS.md 'Do not depend on Bevy, wgpu, renderer types, OS handles, or LLM providers.' 'core OS-handle boundary'
require_text crates/alife_world/AGENTS.md 'Do not depend on Bevy, wgpu, renderer types, or OS handles.' 'world OS-handle boundary'
require_text crates/alife_world/AGENTS.md 'Bevy ECS ownership belongs only to adapter/app layers.' 'world Bevy ECS ownership boundary'
require_text docs/master_spec.md 'Candidates contain only observations and command-transport fields; they never contain caller-provided utilities or scores.' 'candidate observation-only constraint'

legacy_duplicate_kind='shadow'
legacy_gate_kind='parity'
require_all_text docs/master_spec.md "single GPU-authoritative production execution without CPU ${legacy_duplicate_kind}, ${legacy_gate_kind}, or fallback" \
  'The production neural tick is one GPU-authoritative multi-pass causal loop:' \
  "Production does not dispatch a CPU ${legacy_duplicate_kind}, require CPU ${legacy_gate_kind}, or fall back automatically to CPU neural math."
require_all_text docs/master_spec.md 'world authority over candidate enumeration, legality, execution, and outcome' \
  'The engine-neutral world layer owns ecology, reproduction, death, lesson-world concepts, unscored candidate enumeration, action legality, and measured outcomes through stable IDs and versioned contracts.' \
  'Let the world validate and execute the structured command, then seal the outcome patch.'
require_all_text docs/master_spec.md 'exact promoted production capacity-class set' \
  'N512, N1024, and N2048 are the only promoted production neural capacity classes.'
require_all_text docs/master_spec.md 'selected-action and bounded-counter active-loop readback limit' \
  'Read back only the selected candidate index, logit, confidence, and bounded counters.' \
  'Active play never synchronously reads bulk activation, eligibility, topology, or weight state.'
require_all_text docs/master_spec.md 'WGSL-only production shader authority' \
  'All production shaders are WGSL. Do not create HLSL source files unless explicitly labelled as non-authoritative pseudocode.'
require_all_text docs/architecture_decisions.md 'ADR-024 presence and explicit supersession clause' \
  '## ADR-024: Closed-Loop Neural Cognition Is GPU-Authoritative' \
  "This decision supersedes the CPU consolidation authority in ADR-014, the P14 CPU-schema ownership clause in ADR-015, GPU ${legacy_gate_kind} gating in ADR-016, CPU fallback in ADR-019 and ADR-021, and the CPU-${legacy_duplicate_kind}/${legacy_gate_kind} authority clauses in ADR-023. Their save-safety, sparse-layout, world-authority, and sealed-patch boundaries remain in force where they do not conflict with ADR-024."

# N2048 foundation/language/lineage program specification alignment.
require_all_text docs/architecture_decisions.md 'ADR-027 Baldwinian foundation inheritance' \
  '## ADR-027: Curated Foundations Use Baldwinian Inheritance' \
  'W_genetic = foundation + compiled genome deltas' \
  'Lifetime weights, episodic or semantic memories, learned language bindings, eligibility, and transient state are not inherited.'
require_all_text docs/architecture_decisions.md 'ADR-028 grounded language and authentic narration' \
  '## ADR-028: Grounded Language and Narration Remain Neural' \
  'LanguageCodebookV1' \
  'Other creatures hear the raw token sequence selected by the speaker, never polished SLM output.'
require_all_text docs/architecture_decisions.md 'ADR-029 persistent neural identity and growth' \
  '## ADR-029: Persistent Neural Identity Enables Function-Preserving Growth' \
  'BLAKE3-256' \
  'Packed GPU offsets remain runtime-local.'
require_all_text docs/architecture_decisions.md 'ADR-030 durable archives and founder portability' \
  '## ADR-030: Durable Creature Archives Precede Retirement' \
  'Every creature receives an immutable genetic archive before GPU insertion.' \
  'A dying creature is archived before its GPU handle is scrubbed or its world entity despawns.'
require_all_text docs/master_spec.md 'N2048 curated foundation boundary' \
  '`N2048FoundationLayoutV1` is the first trained foundation.' \
  '`W_genetic = foundation + compiled genome deltas`' \
  'Genetic birth clears lifetime weights, fast weights, eligibility, semantic and episodic content, and learned lexicon bindings.'
require_all_text docs/master_spec.md 'limited language is an ABI rather than a neuron layout' \
  '`LanguageCodebookV1` has 256 stable logical codes independent of neuron indices.' \
  'Token IDs never identify neurons or packed GPU offsets.'
require_all_text docs/master_spec.md 'spatial speech and neural narration authority' \
  'Player speech is spatial perception, never a direct command channel.' \
  '`Vocalize` is an unscored world opportunity whose speech act and token payload are selected by the GPU brain.'
require_all_text docs/master_spec.md 'bounded SLM translation boundary' \
  '`SemanticPriorRequest` and `SpeechTranslationRequest` are separate request schemas.' \
  'The SLM never authors creature thought, action, reward, target, desirability, or hidden comprehension.'
require_all_text docs/master_spec.md 'persistent address and checkpoint boundary' \
  '`PersistentNeuronAddress { lobe, ordinal }`' \
  '`GeneticRebuild`, `DurableLearnedFounder`, and `ExactResume`' \
  'Packed GPU offsets remain runtime-local.'
require_all_text docs/master_spec.md 'durable archive and founder boundary' \
  'Every creature receives an immutable genetic archive before GPU insertion.' \
  '`GeneticFounder`, `MindStateClone`, and `GeneticOffspring`' \
  'A dying creature is archived before its GPU handle is scrubbed or its world entity despawns.'
require_all_text docs/schooling_and_teacher_architecture.md 'perception-only language nursery' \
  'The language nursery teaches through normal hearing, object presentation, action demonstration, and sealed world feedback.' \
  'The canonical codebook defines pronounceable symbols and grammatical roles, not inherited world meanings.' \
  'Language evaluation runs with SLM translation disabled.'
require_all_text AGENTS.md 'root N2048 foundation and language guardrails' \
  'N2048 is the first trained foundation; N4096 remains research-only.' \
  'Language token IDs are stable logical codes, never neuron indices or packed GPU offsets.' \
  'Archive every creature before GPU insertion and archive death before GPU retirement.'
require_all_text docs/AGENTS.md 'documentation foundation and lineage authority' \
  'ADR-027 through ADR-030 control curated foundations, grounded language, persistent neural identity, and durable archives.' \
  'Limited language is a scalable ABI, not a neuron-address layout.'
require_all_text crates/alife_core/AGENTS.md 'core foundation and language contract ownership' \
  'Own foundation, language-codebook, persistent-address, checkpoint, and archive-provenance contracts.' \
  'Never equate a language token ID with a neuron or packed GPU offset.'
require_all_text crates/alife_gpu_backend/AGENTS.md 'GPU speech and training authority' \
  'Neural `Vocalize` payload selection remains GPU-authoritative.' \
  'Training-only WGSL and optimizer state stay out of production game binaries and saves.'
require_all_text crates/alife_school/AGENTS.md 'school language nursery boundary' \
  'Teach vocabulary through spatial hearing, visible objects, demonstrations, and sealed outcomes.' \
  'Run language mastery gates with SLM translation disabled.'
require_all_text crates/alife_semantic/AGENTS.md 'separate SLM schemas and no authored cognition' \
  '`SemanticPriorRequest` and `SpeechTranslationRequest` remain separate schemas.' \
  'Translation may map or render bounded raw tokens; it may not author creature thought or speech.'
require_all_text crates/alife_world/AGENTS.md 'world speech and retirement authority' \
  'Player, creature, and teacher speech are spatial world perception.' \
  '`Vocalize` is an unscored opportunity whose payload is selected by the GPU brain.' \
  'Death archiving completes before GPU retirement and despawn.'

if (( semantic_failures > 0 )); then
  printf 'TASK_1_SEMANTIC_ASSERTIONS=FAIL (%d/%d failed)\n' "${semantic_failures}" "${semantic_checks}" >&2
  exit 1
fi

printf 'TASK_1_SEMANTIC_ASSERTIONS=PASS (%d/%d)\n' "${semantic_checks}" "${semantic_checks}"
