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
- This crate owns the only production neural execution backend and the real
  Vulkan allocation, activity, learning, sleep, and replay evidence it emits.
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
- Closed-loop layout v3 stores lifetime weights, fast weights, recurrent
  eligibility, and decoder eligibility in separate double banks. Every slot
  extension and learning-state offset must be host-validated against its exact
  immutable or mutable arena before upload or dispatch.
- A successful waking dispatch owns exactly one pending eligibility transaction.
  The matching sealed outcome must apply it, or the caller must explicitly
  discard it, before another tick or slot retirement.
- Fixed class chunks are independent arenas. Admission may append an arena but
  must never grow, copy, or rebind a live arena's neural buffers.
- The one-thread row prepass validates the complete activity schedule and its
  checksum-bound tile/synapse receipt. Parallel recurrent and eligibility
  kernels consume only the prevalidated sentinel and direct route-mask words;
  do not restore per-neuron diagnostic atomics or repeated digest scans.
