# First Graphical Alpha Playtest Report

Status: GPU-first graphical alpha polish added for local tester evidence.

## Recommended Launch

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

The command launches the Bevy graphical playground with the combined optional
GPU mode requested, runs a bounded 30-second smoke, exits cleanly if windowing
support is available, and reports CPU fallback as degraded safety mode if GPU
runtime is unavailable.

To require real GPU evidence instead of fallback, add `-RequireGpu`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded -RequireGpu
```

## Local Evidence Target

Expected first screen:

- title/status panel labelled `A-Life GPU Alpha Playground`
- readable GPU-first runtime mode, tick, action, patch, and controls
- visible creature, food, hazard, and obstacle markers from stable IDs in the GPU alpha fixture
- compact read-only inspector with selected stable ID and creature state
- GPU section with requested mode, selected backend, fallback, product claim,
  CPU shadow parity, and H_shadow application count

## GPU Mode Result

The current product claim remains:

```text
CpuShadowGuardedStaticPlusLiveHShadow
```

This means GPU scores can feed proposal scores only after CPU shadow parity
passes, and post-seal H_shadow learning is applied through the core lifetime
delta contract. This is not full action-authoritative GPU runtime.

## Fallback Result

When `ALIFE_GPU_RUNTIME_AVAILABLE=0` is set, the same graphical command should
show CPU fallback as a degraded state and must not claim GPU work. CPU fallback
remains supported for validation and safety, but it is not the expected
player-facing mode and is not GPU performance evidence.

## Known Limitations

- The legacy P34 tiny fixture contains creature and food objects; the default
  GPU alpha launcher now uses `crates/alife_world/tests/fixtures/gpu_alpha`,
  which adds real stable-ID hazard and obstacle markers.
- The reset flow is available through `R` in the graphical shell; relaunch also
  remains safe.
- The inspector is intentionally read-only.
- GPU and graphics evidence is local/manual unless captured on a tester machine.
- Computer Use key injection may fail on this Windows desktop; use
  `graphical-controls-smoke` as local deterministic control evidence and keep
  independent human control evidence separate.
- No release tag was created.

## Playtest Checklist

Use `docs/productization/FIRST_GRAPHICAL_ALPHA_PLAYTEST_CHECKLIST.md` for the
external tester record.
