# Full GPU neural runtime report

Status: optional product smoke path implemented for CPU-shadow-guarded static GPU action scoring.

## Scope

The new product-facing command is:

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- full-gpu-runtime-smoke crates/alife_world/tests/fixtures/p34 --mode static-action-authoritative --ticks 3
```

The command remains optional. Without the `gpu-runtime` feature, or when
`ALIFE_GPU_RUNTIME_AVAILABLE=0`, it falls back to the CPU reference path and
still seals normal patches.

## Runtime boundary

- Default/headless path remains CPU reference.
- GPU acceleration is explicit and feature-gated.
- The GPU static forward pass dispatches real WGSL compute when hardware is available.
- Active tick readback is bounded to a 64-byte action-summary record.
- Bulk activation, per-synapse, per-lobe, and weight readback remain forbidden for active gameplay.
- GPU-derived action scores are accepted only when the compact CPU shadow agrees.
- Action selection still flows through normal `ActionProposal` construction, CPU action arbitration, world execution, and sealed `ExperiencePatch` handling.

## Current claim

Product runtime claim: `CpuShadowGuarded` for the static action scorer when the
real GPU path is selected and parity passes.

This is not a claim that the complete plastic recurrent brain is GPU
action-authoritative. H_shadow plasticity remains diagnostic/shadow evidence
because live application requires a future core-owned post-seal lifetime-state
hook.

Requests for the `full-shadow` or `full-action-authoritative` modes fall back
with an unsupported-backend status instead of pretending the full
static+routing+plasticity runtime is complete.

## Evidence command fields

The smoke output reports:

- selected backend and fallback reason
- adapter/backend/driver when a real GPU path is selected
- ticks run
- selected actions
- sealed patches and packed logs
- CPU shadow parity
- routing active/skipped tile counters
- compact readback byte count
- H_shadow diagnostic summary
- `W_genetic_fixed` unchanged status
- upload, submit/poll, compact readback, CPU shadow, and total GPU runtime wall timings

## Forced fallback

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- full-gpu-runtime-smoke crates/alife_world/tests/fixtures/p34 --mode static-action-authoritative
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

The forced fallback must report `CpuReference` and still seal patches.

## Local evidence from this branch

On this Windows machine, the feature-enabled command selected:

- Adapter: `NVIDIA GeForce RTX 3050`
- Backend/API: `Vulkan`
- Driver info: `581.80`
- Selected backend: `GpuStatic`
- Fallback reason: `None`
- Product runtime claim: `CpuShadowGuarded`
- Ticks run: `3`
- Sealed patches: `3`
- Packed logs: `3`
- GPU output used for proposals: `true`
- CPU shadow parity: `true`
- Routing counters: `1 active tile / 1 total tile`, `0 skipped`, `4 active synapses`
- Compact active readback: `64` bytes
- Timing: upload `0.2032 ms`, submit/poll `1.2318 ms`, compact readback `0.9273 ms`, CPU shadow `0.0262 ms`, total GPU runtime `2.3623 ms`

The static-plastic-shadow command selected `GpuPlastic` on the same adapter,
dispatched the post-seal plasticity diagnostic, changed H_shadow shadow values,
reported `W_genetic_fixed` unchanged, and recorded total GPU runtime wall timing
of `3.7268 ms`. That plasticity result is not applied back into the live
`CreatureMind`.
