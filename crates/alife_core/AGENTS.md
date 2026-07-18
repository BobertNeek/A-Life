# alife_core Instructions

This crate controls engine-agnostic cognitive contracts: IDs, brain class
registry, lobe layout, routing masks, genome, chemistry, ExperiencePatch,
ActionCommand, sensory/action ABIs, profiles, lineage, and backend traits.

Rules:

- Do not depend on Bevy, wgpu, renderer types, OS handles, or LLM providers.
- Do not reintroduce a fixed global 2048-neuron brain invariant.
- `Standard2048` may appear only as one `BrainScaleTier` reference class.
- Prefer explicit versioned structs for cross-layer contracts.
- Production neural execution is GPU-authoritative WGSL; do not add a live CPU
  shadow, parity gate, or automatic CPU neural fallback.
- Keep pure CPU neural helpers test-only or developer-only.
- World code enumerates unscored candidates and remains authoritative for
  legality and outcomes.
- Promote only N512, N1024, and N2048 until larger tiers pass the documented
  causal and performance gates.
- Own foundation, language-codebook, persistent-address, checkpoint, and
  archive-provenance contracts.
- Never equate a language token ID with a neuron or packed GPU offset.
- Genetic birth must not inherit lifetime weights, memories, learned lexicon
  bindings, eligibility, or transient state.
- Runtime GPU kernels do not belong here.
- `foundation.rs`, `language.rs`, and `phenotype/persistent_address.rs` own the
  frozen N2048 layout/route/plasticity ABI, `LanguageCodebookV1`, and the
  BLAKE3-256 logical address map. Packed indices may appear only as validated
  runtime lookup metadata and are excluded from the persistent map digest.
- N2048 compiles exactly 24,576 recurrent, 4,096 action (3,072 candidate plus
  1,024 reserved speech), and 4,096 memory-decoder synapses. N512/N1024 remain
  valid independent procedural phenotypes until their own foundations exist.
