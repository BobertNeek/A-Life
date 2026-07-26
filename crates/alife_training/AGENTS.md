# alife_training Instructions

This crate owns offline foundation training, curricula, evaluation, and later
evolutionary hardening.

- Train the exact compiled production sparse graph with WGSL.
- Keep training shaders, optimizer moments, gradients, targets, and auxiliary
  heads out of normal game binaries and production saves.
- Use the shared `GpuAuthoritativeSession` with the `Training` consumer kind;
  do not create a CPU neural trainer, live shadow, or fallback.
- Export candidates only through the canonical `FoundationWeightAsset` codec.
- Stage masks must preserve frozen weights bit-for-bit.
- N2048 is the only trained foundation until another class receives a separate
  approved curriculum and evidence program.
