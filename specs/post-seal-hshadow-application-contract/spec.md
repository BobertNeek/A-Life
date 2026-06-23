# Post-Seal H_shadow Application Contract Spec

## Mode

Mode 2 - Full Spec Loop. The change crosses `alife_core`,
`alife_gpu_backend`, and `alife_game_app`, introduces a narrow public core
contract, and requires R2 review.

## Problem

The full GPU neural runtime can dispatch static action scoring and GPU
plasticity diagnostics, but plasticity cannot safely update live
`CreatureMind` state. The missing boundary is a core-owned, versioned,
post-seal H_shadow delta application contract.

## In Scope

- Define validated core packet/receipt/token types for post-seal H_shadow
  deltas.
- Let `CreatureMind` apply validated H_shadow-only deltas after a sealed
  `ExperiencePatch`.
- Preserve `W_genetic_fixed`, lifetime-consolidated weights, and
  `H_operational`.
- Convert GPU plasticity diagnostics into the core packet without exposing GPU
  handles or raw buffers to `alife_core`.
- Wire `full-gpu-runtime-smoke --mode static-plastic-shadow` to apply live
  H_shadow deltas when GPU parity passes.

## Non-Goals

- No mandatory GPU path.
- No full action-authoritative static+routing+plastic claim unless the runtime
  evidence supports it.
- No active bulk neural readback.
- No Bevy/wgpu/tooling dependency in `alife_core`.
- No save/schema redesign, release tag, S12, G25, or P37.

## Acceptance Criteria

- Core rejects missing/unsealed patch evidence, mismatched organism/tick/sequence,
  replay, NaN/Inf, out-of-range values, duplicate targets, oversized batches,
  parity-failed batches, and non-H_shadow layer changes.
- Applying a valid batch mutates only H_shadow values in `CreatureMind`.
- GPU backend produces core delta records from plasticity output only after CPU
  shadow parity and unchanged genetic/lifetime/operational layers.
- App summary reports live application status, sealed-patch gating, CPU shadow
  parity, W_genetic_fixed unchanged, and remaining runtime claim honestly.
- Full validation and focused GPU evidence commands pass or are honestly
  classified.

