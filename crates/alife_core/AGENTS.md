# alife_core Instructions

This crate controls engine-agnostic cognitive contracts: IDs, brain class
registry, lobe layout, routing masks, genome, chemistry, ExperiencePatch,
ActionCommand, sensory/action ABIs, profiles, lineage, and backend traits.

Rules:

- Do not depend on Bevy, wgpu, renderer types, or LLM providers.
- Do not reintroduce a fixed global 2048-neuron brain invariant.
- `Standard2048` may appear only as one `BrainScaleTier` reference class.
- Prefer explicit versioned structs for cross-layer contracts.
- Runtime GPU kernels do not belong here.
