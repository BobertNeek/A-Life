# alife_gpu_backend Instructions

This crate controls wgpu resource planning, WGSL shader packaging, compute
backend traits, buffer descriptors, and dispatch placeholders.

Rules:

- Source shaders must be WGSL; do not add HLSL production shaders.
- Do not implement real neural runtime kernels during scaffold phase.
- Use sparse class-bucketed storage concepts, not dense `[M, N, N]` buffers.
- Keep GPU backend replaceable behind narrow traits.
- No raw host pointers may cross into shader-visible contracts.
