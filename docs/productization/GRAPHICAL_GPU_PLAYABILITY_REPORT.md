# Graphical GPU Playability Report

Status: product-facing graphical GPU mode is now the default Bevy alpha
playground launch path. CPU fallback remains available but is presented as a
degraded safety mode rather than the target player experience.

## Command

```powershell
cargo run -p alife_game_app --features "bevy-app gpu-runtime" --bin alife_game_app -- graphical-playground crates/alife_world/tests/fixtures/gpu_alpha --gpu-mode static-plastic-cpu-shadow-guarded
```

Bounded manual smoke:

```powershell
cargo run -p alife_game_app --features "bevy-app gpu-runtime" --bin alife_game_app -- graphical-playground crates/alife_world/tests/fixtures/gpu_alpha --gpu-mode static-plastic-cpu-shadow-guarded --smoke-seconds 20
```

Windows launcher:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 20 -GpuMode static-plastic-cpu-shadow-guarded
```

GPU-required evidence smoke:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 20 -GpuMode static-plastic-cpu-shadow-guarded -RequireGpu
```

## What Is Visible

- Persistent `A-Life GPU Alpha Playground` window with the GPU alpha fixture.
- Creature, food, hazard, and obstacle markers from stable world IDs.
- Runtime overlay with CPU/GPU mode, tick status, selected action, sealed patch
  count, fallback status, and controls.
- Read-only inspector overlay with selected creature state and GPU runtime
  status.
- Presentation-only creature color cues:
  - gray: CPU fallback,
  - cyan: CPU-shadow-verified GPU scores are feeding proposals,
  - green: post-seal H_shadow learning signal has applied.

## GPU Runtime Evidence

The graphical path reuses the existing combined runtime claim:
`CpuShadowGuardedStaticPlusLiveHShadow`.

Earlier local 20-second graphical smoke result on this machine, before the
GPU alpha fixture became the default:

- Command: `cargo run -p alife_game_app --features "bevy-app gpu-runtime" --bin alife_game_app -- graphical-playground crates/alife_world/tests/fixtures/p34 --gpu-mode static-plastic-cpu-shadow-guarded --smoke-seconds 20`
- Selected GPU backend: `GpuPlastic`
- Fallback: `None`
- GPU scores used for proposals: `true`
- CPU shadow parity: `true`
- H_shadow applications visible in telemetry: `1`
- Sealed patches: `16`
- Packed logs: `16`
- Product claim: `CpuShadowGuardedStaticPlusLiveHShadow`

The current GPU-first launcher uses the `gpu_alpha` fixture by default so the
first screen includes real hazard and obstacle markers instead of P34
guide-only hazard/obstacle text.

The local Vulkan loader emitted warnings about a missing validation layer and a
deprecated GOG Galaxy overlay layer manifest. The graphical smoke still exited
successfully and selected the real GPU path.

The GPU path remains CPU-shadow guarded:

- GPU static scores feed proposal scores only after CPU shadow parity passes.
- Normal action arbitration remains in the live tick path.
- ExperiencePatch sealing remains required before H_shadow plasticity.
- H_shadow deltas apply through the `alife_core` post-seal lifetime delta
  contract.
- Active readback remains bounded to compact summaries; no bulk neural,
  per-synapse, per-lobe, or weight readback is exposed during active gameplay.
- `W_genetic_fixed`, lifetime-consolidated weights, and H_operational remain
  unchanged.

## Fallback Behavior

If GPU runtime is unavailable or forced off with:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
```

the graphical app continues through CPU reference ticks unless `-RequireGpu` is
set, and displays CPU fallback as degraded status. CPU fallback is not GPU
performance evidence.

Local forced-fallback graphical smoke result:

- Command: `ALIFE_GPU_RUNTIME_AVAILABLE=0` plus the same graphical smoke command
  with `--smoke-seconds 10`
- Selected backend: `CpuReference`
- Fallback reason: `HardwareUnavailable`
- GPU scores used for proposals: `false`
- H_shadow applications: `0`
- Sealed patches: `10`
- Product claim: `None`

## Controls

- Space: pause/run
- N: step once
- R: reset/restart alpha fixture
- 1/2/3: speed
- arrows/WASD: pan
- +/-: zoom
- Q/E: orbit
- F: follow selected stable ID
- Esc: quit

## Known Limitations

- This is not a full action-authoritative GPU runtime. CPU shadow remains the
  gate.
- Graphical smoke evidence depends on local windowing support.
- GPU timing/performance is local hardware evidence only and should not be used
  as a release-wide performance claim.
- No release tag was created.
