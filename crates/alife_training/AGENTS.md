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
- Cross-run screening is capped at 64 candidates per run and 16 active-battery
  candidates. Ranking may display ancestry and genome distance but never apply
  an implicit kinship penalty.
- `active_battery.rs` owns the executable 15-challenge N2048 battery. It must
  use grounded headless worlds, the shared `Challenge` GPU session, world
  legality/outcomes, and sealed patches; synthetic scored frames are not
  active-battery evidence.
