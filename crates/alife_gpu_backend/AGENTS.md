# alife_gpu_backend Instructions

This crate controls wgpu resource planning, WGSL shader packaging, compute
backend traits, buffer descriptors, and dispatch placeholders.

Rules:

- Source shaders must be WGSL; do not add HLSL production shaders.
- Production neural execution is GPU-authoritative WGSL; do not add a live CPU
  shadow, parity gate, or automatic CPU neural fallback.
- Keep pure CPU neural helpers test-only or developer-only.
- World code enumerates unscored candidates and remains authoritative for
  legality and outcomes.
- Promote only N512, N1024, and N2048 until larger tiers pass the documented
  causal and performance gates.
- Use sparse class-bucketed storage concepts, not dense `[M, N, N]` buffers.
- Keep GPU backend replaceable behind narrow traits.
- No raw host pointers may cross into shader-visible contracts.
