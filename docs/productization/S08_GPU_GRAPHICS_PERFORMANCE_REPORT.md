# S08 GPU, Graphics, and Performance Evidence Report

Status: implemented on `codex/S08-gpu-graphics-performance-evidence`.

S08 gathers product evidence for GPU fallback, graphics smoke, benchmark smoke,
and player/tester-facing settings status. It does not add GPU kernels, require
GPU hardware, change gameplay behavior, or claim GPU performance from CPU
fallback data.

## Scope

- Owned paths: `alife_game_app` status/evidence summaries, the S08 smoke
  command, graphical status text, tests, benchmark evidence, and this report.
- No changes to `alife_core`.
- No release tag, P37, G25, S12, or new implementation chain.
- GPU and graphics evidence remains measured-or-manual, never inferred.

## Player/Tester Status Surface

S08 adds a CI-safe command:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- gpu-graphics-performance-smoke crates/alife_world/tests/fixtures/p34
```

Expected signature shape:

```text
S08 GPU graphics performance schema=alife.s08.gpu_graphics_performance.v1 version=1 selected=CpuReference gpu_evidence=fallback-only graphics_evidence=manual-unknown fps_target=manual-unknown ...
```

The graphical runtime overlay now includes an S08 line:

```text
S08 GPU/Graphics: backend=CpuReference fallback=default-safe-path gpu_evidence=fallback-only 60 FPS target=manual-unknown | CPU fallback is not GPU performance | no active neural readback
```

This is display/status text only. It does not change neural computation,
simulation behavior, save/load contracts, or runtime backend selection.

## Evidence Captured

Environment graphics adapter reported by Windows:

```text
NVIDIA GeForce RTX 3050
driver 32.0.15.8180
adapter RAM 4293918720
```

### CPU Benchmark Smoke

Command:

```powershell
cargo run -p alife_tools --bin benchmark_tiers
```

Result: passed and wrote `target/artifacts/benchmark_tiers.md`.

Recorded smoke tiers:

| Population | Brain tier | Tick time ms | Patches/sec | Success |
|---:|---|---:|---:|---:|
| 1 | Nano512 | 1.621 | 617.017 | 1.000 |
| 10 | Nano512 | 10.734 | 1117.912 | 1.000 |

This is CPU reference smoke evidence, not GPU performance evidence.

### GPU Runtime Fallback Report

Command:

```powershell
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

Result: passed and wrote `target/artifacts/gpu_runtime_performance.md`.
Without hardware/validation flags, this command may honestly select CPU
fallback.

Hardware-flag command used for this pass:

```powershell
$env:ALIFE_GPU_RUNTIME_BACKEND='static'; $env:ALIFE_GPU_RUNTIME_FEATURE='1'; $env:ALIFE_GPU_RUNTIME_AVAILABLE='1'; $env:ALIFE_GPU_RUNTIME_VALIDATED='1'; cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

Result: passed and selected `GpuStatic` in the report, but the report explicitly
states that P20 CPU smoke metrics were copied and GPU neural time remains
`unknown`.

S08 conclusion: GPU backend selection/report plumbing works. GPU performance is
still manual/unknown because no real GPU neural timing was measured.

### App GPU Product Smoke

Command:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- gpu-product-smoke
```

Result: passed. The default non-`gpu-runtime` build selected CPU fallback,
reported `FeatureDisabled`, blocked active readback, and kept
`performance_status=unknown-unless-measured`.

### S08 App Evidence Smoke

Command:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- gpu-graphics-performance-smoke crates/alife_world/tests/fixtures/p34
```

Result: passed. It reported:

- selected backend: `CpuReference`
- GPU evidence: `fallback-only`
- graphics evidence: `manual-unknown`
- 60 FPS target: `manual-unknown`
- CPU fallback works: `true`
- no active readback: `true`

### Graphical Smoke

Dry-run command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun
```

Result: passed and printed the feature-gated Bevy command without opening a
window.

Bounded graphical smoke command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 5
```

Result: passed. The bounded window smoke opened the graphical playground and
exited cleanly. A timed shell run took about `11.13` seconds total including
cargo startup and the 5-second smoke window.

Observed environment warnings:

- `VK_LAYER_KHRONOS_validation` was unavailable.
- A GOG Galaxy Vulkan overlay manifest used deprecated `layer` nodes.
- Vulkan registry lookup warnings appeared.

These warnings did not fail the smoke command, but they are graphics
environment notes, not product GPU performance evidence.

## 60 FPS Target Status

The product target remains 60 FPS, but S08 did not measure stable in-window FPS
or GPU neural timing. The correct status is:

```text
60 FPS target: manual/unknown
```

CPU smoke tick times are useful regression evidence. They are not a graphics FPS
claim and not a GPU performance claim.

## Invariant Status

- `alife_core` remains engine-independent.
- GPU runtime remains optional with CPU fallback.
- Active gameplay does not require synchronous neural readback.
- The graphical smoke uses the feature-gated Bevy app path.
- No save/load schema or stable-ID policy changed.
- No large screenshots, captures, logs, or benchmark artifacts are committed.
- No P37, G25, or S12 was created.

## Known Limitations

- GPU neural time is still unknown unless a real backend run records timing.
- The forced `GpuStatic` report records selected backend state and copied CPU
  smoke timings, not GPU neural performance.
- The graphical smoke confirms launch/close behavior, but S08 did not capture a
  new screenshot artifact.
- The 60 FPS target is not proven by this pass.

## Recommendation

Proceed to S09 only after S08 review, merge, and main validation. Future GPU
performance claims require real measured `gpu_neural_ms` and graphics FPS
evidence, not CPU fallback or dry-run output.
