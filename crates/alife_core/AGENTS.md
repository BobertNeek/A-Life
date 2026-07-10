# alife_core Instructions

This crate controls engine-agnostic cognitive contracts: IDs, brain class
registry, lobe layout, routing masks, genome, chemistry, ExperiencePatch,
ActionCommand, sensory/action ABIs, profiles, lineage, and backend traits.

Rules:

- Do not depend on Bevy, wgpu, renderer types, or LLM providers.
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
- Runtime GPU kernels do not belong here.
