# alife_training Instructions

Root [AGENTS.md](../../AGENTS.md) applies. These local rules are current
implementation guardrails; they do not amend the controlling v2.0 architecture.

This crate owns offline foundation training, curricula, evaluation, and later
evolutionary hardening.

- GPU training executes the exact compiled production sparse graph with WGSL.
  The separately authorized, opt-in `cpu-backend` may execute the same graph and
  native learning semantics for offline cloud training without a graphics device.
  This follows root authorization and AOA-ADM-007 / AOA-TIME-006; it is not a live
  gameplay shadow, automatic fallback, or simplified reference controller.
- Keep training shaders, optimizer moments, gradients, targets, and auxiliary
  heads out of normal game binaries and production saves.
- GPU training uses the shared `GpuAuthoritativeSession` with the `Training`
  consumer kind. Offline CPU training has an explicit CPU evidence domain and
  native checkpoint boundary; never relabel it as GPU execution.
- Preserve all retained weights, moments, frozen coordinates and exact integer
  optimizer ages. Bind source checkpoint bytes before continuation; use the native
  serial continuity verifier for serial/correction exports. Numerical comparisons
  use functional tolerances; integer ages remain exact. Hardware and accepted-source
  qualification remain Unknown or Blocked until actually exercised.
- Export candidates only through the canonical `FoundationWeightAsset` codec.
- Stage masks must preserve frozen weights bit-for-bit.
- N2048 is the only trained foundation until another class receives a separate
  approved curriculum and evidence program.
- The approved Nano512 founder experiment uses the distinct, unpromoted fixed
  readout candidate. Record ordinary grounded world commands and body outcomes;
  keep teacher labels separate from sensory input. Fit genetic readout weights
  from complete production GPU receipts, preserving upstream genes bit-for-bit.
  First-frame-only sampling is a bounded diagnostic, not inspection-sequence,
  lifetime-adaptation, or founder-promotion evidence.
- Cross-run screening is capped at 64 candidates per run and 16 active-battery
  candidates. Ranking may display ancestry and genome distance but never apply
  an implicit kinship penalty.
- `active_battery.rs` owns the executable 15-challenge N2048 battery. It must
  use grounded headless worlds, the shared `Challenge` GPU session, world
  legality/outcomes, and sealed patches; synthetic scored frames are not
  active-battery evidence.
