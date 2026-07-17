# alife_gpu_backend Instructions

This crate controls wgpu resource planning, WGSL shader packaging, compute
backend traits, buffer descriptors, dispatch scheduling, and production neural
pipelines.

Rules:

- Source shaders must be WGSL; do not add HLSL production shaders.
- Production neural execution is GPU-authoritative WGSL; do not add a live CPU
  shadow, parity gate, or automatic CPU neural fallback.
- Keep pure CPU neural helpers test-only or developer-only.
- World code enumerates unscored candidates and remains authoritative for
  legality and outcomes.
- Promote only N512, N1024, and N2048 until larger tiers pass the documented
  causal and performance gates.
- Neural `Vocalize` payload selection remains GPU-authoritative.
- Training-only WGSL and optimizer state stay out of production game binaries
  and saves.
- Persistent logical addresses must be resolved to runtime-local packed offsets
  before dispatch; packed offsets are never durable identity.
- Use sparse class-bucketed storage concepts, not dense `[M, N, N]` buffers.
- Keep GPU backend replaceable behind narrow traits.
- No raw host pointers may cross into shader-visible contracts.
- GPU candidate index records contain only the `ActionCandidate` head. Reserved
  N2048 speech and memory decoder weights remain in the single immutable
  authoritative payload but cannot enter candidate arbitration before their
  dedicated reviewed WGSL passes are implemented.
