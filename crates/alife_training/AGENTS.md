# alife_training Instructions

Architecture authority:

- `../../docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`
  is the single normative source.
- This file records current implementation guardrails only. Existing Rust
  structures, processor placement, GPU layouts, constants, brain-size
  assumptions, adapters, tests, and fixtures do not amend v2.0.
- Earlier architecture documents are historical. Report conflicts as
  `AOA-*` gaps and do not start an unrequested repair pass.

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
