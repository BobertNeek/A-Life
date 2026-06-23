# Post-Seal H_shadow Application Contract Report

Status: implemented and validated for H_shadow-only live application in the
optional GPU static-plastic shadow smoke path.

## Contract

`alife_core` now owns a versioned `PostSealLifetimeDeltaBatch` contract for
bounded H_shadow deltas. The batch carries stable organism, brain class, tick,
and sealed patch sequence metadata plus H_shadow-only target records. It carries
no GPU handles, wgpu types, Bevy types, renderer handles, raw buffers, or
engine-local IDs.

`CreatureMind` accepts the batch only through a sealed `ExperiencePatch` or a
token derived from one. The application path rejects:

- missing or unsealed patch evidence
- wrong organism, tick, sequence, brain class, or topology shape
- replayed or stale sequence IDs
- NaN, Inf, out-of-range values, and oversized batches
- duplicate target indices
- failed CPU shadow parity evidence
- non-H_shadow layers
- batches that claim genetic, lifetime-consolidated, or H_operational changes

Accepted deltas mutate only `SynapseWeightSplit::h_shadow`. `W_genetic_fixed`,
lifetime-consolidated weights, and H_operational remain unchanged.

## Live Runtime Evidence

Command:

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- full-gpu-runtime-smoke crates/alife_world/tests/fixtures/p34 --mode static-plastic-shadow --ticks 3
```

Local result on this machine:

- Adapter: NVIDIA GeForce RTX 3050
- Backend/API: Vulkan
- Selected backend: `GpuPlastic`
- Fallback reason: `None`
- Sealed patches: `3`
- Live H_shadow application: `true`
- Applied delta records: `2`
- Changed H_shadow records: `2`
- Max absolute delta: `0.112549`
- Applied sealed patch sequence: `1`
- CPU shadow parity: `true`
- `W_genetic_fixed` unchanged: `true`
- lifetime-consolidated unchanged: `true`
- H_operational unchanged: `true`
- Active bulk neural readback: forbidden

The smoke remains `ShadowOnly` because static-plastic shadow mode does not use
GPU output for action proposals. The separate static-action-authoritative smoke
uses GPU static scores for proposal scoring and remains CPU-shadow-guarded.

## Closed Gap

The previous missing alife_core-owned post-seal H_shadow application hook is
closed for validated H_shadow-only deltas.

## Remaining Gap

A-Life still does not claim full action-authoritative static+routing+plastic GPU
runtime. That requires one validated mode that combines GPU static/routing action
scoring, post-seal GPU plasticity application, CPU shadow parity, normal action
arbitration, and sealed ExperiencePatch ordering in the same live path.

## Release Status

Release/tag status is unchanged. GPU remains optional, CPU fallback remains
available, and the default/headless path does not require GPU hardware.
