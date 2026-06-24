# Full GPU neural runtime report

Status: optional product smoke path implemented for CPU-shadow-guarded static
GPU action scoring, live post-seal H_shadow application in static-plastic
shadow mode, and a combined CPU-shadow-guarded static/plastic mode that performs
both in the same live path.

## Scope

The new product-facing command is:

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- full-gpu-runtime-smoke crates/alife_world/tests/fixtures/p34 --mode static-action-authoritative --ticks 3
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- full-gpu-runtime-smoke crates/alife_world/tests/fixtures/p34 --mode static-plastic-cpu-shadow-guarded --ticks 3
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
action-authoritative. The `static-plastic-shadow` mode now converts validated
GPU plasticity output into a core-owned post-seal H_shadow delta batch and
applies that batch to the live `CreatureMind` only after a sealed
`ExperiencePatch`. Because that mode does not use GPU output for action
proposals, its product runtime claim remains `ShadowOnly`.

The `static-plastic-cpu-shadow-guarded` mode combines the safe parts of both
paths. It uses CPU-shadow-verified GPU static scores for action proposals, runs
normal arbitration and patch sealing, then dispatches GPU plasticity and applies
H_shadow deltas through the core post-seal contract. Its product runtime claim
is `CpuShadowGuardedStaticPlusLiveHShadow`.

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
- boundary-scoped post-seal H_shadow diagnostic readback byte count and timing
- H_shadow application summary
- `W_genetic_fixed`, lifetime-consolidated, and H_operational unchanged status
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
- Timing: upload `0.1942 ms`, submit/poll `1.2196 ms`, compact readback `0.8307 ms`, CPU shadow `0.0242 ms`, total GPU runtime `2.2445 ms`

The static-plastic-shadow command selected `GpuPlastic` on the same adapter,
dispatched post-seal plasticity, converted the result into a core-owned delta
batch, and applied `2` H_shadow records to the live `CreatureMind` after sealed
patch sequence `1`. It reported `W_genetic_fixed`,
lifetime-consolidated weights, and H_operational unchanged. The measured
command output recorded upload `0.2092 ms`, submit/poll `1.1921 ms`, compact
readback `0.7289 ms`, CPU shadow `0.0237 ms`, and total GPU runtime wall timing
of `2.1302 ms`.

The combined `static-plastic-cpu-shadow-guarded` command selected `GpuPlastic`
on the same adapter, used GPU static output for action proposals after CPU
shadow parity, sealed three patches, dispatched post-seal plasticity, and
applied two H_shadow delta records to live `CreatureMind`. Measured wall
timings were upload `0.1996 ms`, GPU submit/poll `1.3207 ms`, compact active
action-summary readback `0.8925 ms`, CPU shadow `0.0238 ms`, and total GPU
runtime `2.4128 ms`. It also reported a separate post-seal diagnostic H_shadow
readback of `48` bytes with readback timing `1.2890 ms`; that readback is
boundary-scoped after patch sealing and is not an active bulk neural readback. The run
reported `W_genetic_fixed`, lifetime-consolidated weights, and H_operational
unchanged.

If `ALIFE_GPU_PLASTICITY_DIAGNOSTIC_AVAILABLE=0` is set after static GPU
scoring succeeds, the combined command degrades to the narrower
`CpuShadowGuarded` claim: GPU static scores may still feed proposals after CPU
shadow parity, no H_shadow deltas are applied, and the output records
`post-seal GPU plasticity unavailable`.

Forced fallback with `ALIFE_GPU_RUNTIME_AVAILABLE=0` reported `CpuReference`,
fallback `HardwareUnavailable`, no GPU plasticity dispatch, and still sealed
three patches.
