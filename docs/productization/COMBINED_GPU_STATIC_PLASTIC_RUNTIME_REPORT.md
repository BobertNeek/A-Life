# Combined GPU Static/Plastic Runtime Report

Status: implemented as an optional, CPU-shadow-guarded product smoke mode.

## Command

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- full-gpu-runtime-smoke crates/alife_world/tests/fixtures/p34 --mode static-plastic-cpu-shadow-guarded --ticks 3
```

## What Is Combined

The new mode runs these steps in one live path:

1. GPU static action scoring dispatches on the selected adapter.
2. CPU shadow static scoring verifies the compact GPU action summary.
3. GPU scores are converted into normal action proposal scores only if parity passes.
4. Existing action arbitration chooses the action.
5. The live tick executes through the existing world path and seals an `ExperiencePatch`.
6. GPU plasticity dispatches after sealing.
7. GPU plasticity output is converted into a core-owned `PostSealLifetimeDeltaBatch`.
8. `CreatureMind::apply_post_seal_lifetime_deltas` applies H_shadow deltas.

No raw GPU buffers, wgpu resources, Bevy types, renderer handles, or engine-local
IDs enter `alife_core`.

## Local Evidence

Local command result on this machine:

- Adapter: NVIDIA GeForce RTX 3050
- Backend/API: Vulkan
- Driver: 581.80
- Selected backend: `GpuPlastic`
- Fallback reason: `None`
- Ticks run: `3`
- Sealed patches: `3`
- GPU static dispatched: `true`
- GPU scores used for proposals: `true`
- CPU shadow parity: `true`
- Routing counters: `1 active tile / 1 total tile`, `0 skipped`, `4 active synapses`
- Compact active readback: `64` bytes
- Post-seal diagnostic H_shadow readback: `48` bytes, boundary-scoped after
  patch sealing
- Post-seal diagnostic H_shadow readback timing: `1.2890 ms`
- Post-seal H_shadow application: `true`
- Applied/changed H_shadow records: `2/2`
- Max absolute H_shadow delta: `0.112549`
- Applied sealed patch sequence: `1`
- `W_genetic_fixed` unchanged: `true`
- lifetime-consolidated unchanged: `true`
- H_operational unchanged: `true`
- Active-tick timing: upload `0.1996 ms`, GPU submit/poll `1.3207 ms`,
  compact action-summary readback `0.8925 ms`, CPU shadow `0.0238 ms`, total
  GPU runtime `2.4128 ms`

Product runtime claim: `CpuShadowGuardedStaticPlusLiveHShadow`.

## Manual Long-Run Soak Evidence

Manual soak command:

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-longrun-soak crates/alife_world/tests/fixtures/p34 --ticks 5000 --report-every 500
```

Local 5000-tick soak result:

- Selected backend: `GpuPlastic`
- Adapter: NVIDIA GeForce RTX 3050
- Backend/API: Vulkan
- Completed ticks: `5000`
- GPU static dispatch ticks: `5000`
- GPU proposal ticks: `5000`
- CPU shadow parity checks: `5000`
- Parity failures: `0`
- H_shadow applications: `1`
- H_shadow rejections: `0`
- Compact active readback: `320000` bytes total
- Post-seal H_shadow diagnostic readback: `48` bytes total
- Wall time: `142803.4531 ms`
- Average: `28.5607 ms/tick`
- Product runtime claim: `CpuShadowGuardedStaticPlusLiveHShadow`
- Full action-authoritative claim: `false`

See `docs/productization/GPU_LONGRUN_SOAK_REPORT.md` for the full manual
1000/5000 tick evidence and forced CPU fallback result.

## Manual Sustained-Learning Soak Evidence

Manual sustained-learning command:

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-sustained-learning-soak crates/alife_world/tests/fixtures/p34 --ticks 1000 --report-every 100
```

Local 5000-tick sustained-learning result:

- Selected backend: `GpuPlastic`
- Adapter: NVIDIA GeForce RTX 3050
- Backend/API: Vulkan
- Completed ticks: `5000`
- Episodes: `157`
- Sealed patches total: `5000`
- Packed logs total: `5000`
- GPU static dispatch ticks: `5000`
- GPU proposal ticks: `5000`
- CPU shadow parity checks: `5000`
- Parity failures: `0`
- H_shadow application attempts/succeeded/rejected: `157/157/0`
- H_shadow records applied: `314`
- Product runtime claim: `CpuShadowGuardedStaticPlusLiveHShadow`
- Full action-authoritative claim: `false`

This sustained-learning command uses deterministic episode rotation to collect
repeated valid post-seal H_shadow applications without replaying stale deltas.
See `docs/productization/GPU_SUSTAINED_LEARNING_SOAK_REPORT.md`.

## Graphical Product Surface

The combined mode is now exposed in the Bevy graphical playground:

```powershell
cargo run -p alife_game_app --features "bevy-app gpu-runtime" --bin alife_game_app -- graphical-playground crates/alife_world/tests/fixtures/p34 --gpu-mode static-plastic-cpu-shadow-guarded --smoke-seconds 20
```

The window overlays show requested GPU mode, selected backend/fallback, CPU
shadow parity, compact readback, sealed patches, and post-seal H_shadow
application status. The graphical presentation remains non-authoritative over
the world/brain model.

## Fallback Behavior

When GPU runtime availability is forced off with
`ALIFE_GPU_RUNTIME_AVAILABLE=0`, the command falls back to `CpuReference` and
still seals CPU patches. GPU is not required for default/headless validation.

When post-seal GPU plasticity diagnostics are disabled with
`ALIFE_GPU_PLASTICITY_DIAGNOSTIC_AVAILABLE=0`, the command still uses
CPU-shadow-verified GPU static scores for proposals when available, but it does
not apply H_shadow deltas and reports the narrower `CpuShadowGuarded` claim.

## Remaining Gap

This does not close the full action-authoritative gap. CPU shadow parity remains
a runtime gate before GPU proposal scores are used, and the report keeps
`unsupported_full_gap_remaining=true`.

## Release Status

No release tag was created. This is local product-smoke GPU evidence, not a
public release readiness claim.
