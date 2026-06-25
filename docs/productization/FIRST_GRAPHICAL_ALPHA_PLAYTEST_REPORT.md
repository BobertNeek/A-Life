# First Graphical Alpha Playtest Report

Status: first-player graphical alpha polish added for local tester evidence.

## Recommended Launch

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

The command launches the Bevy graphical playground with the combined optional
GPU mode requested, runs a bounded 30-second smoke, exits cleanly if windowing
support is available, and reports CPU fallback honestly if GPU runtime is
unavailable.

## Local Evidence Target

Expected first screen:

- title/status panel labelled `A-Life Alpha Playground`
- readable runtime mode, tick, action, patch, and controls
- visible creature and food markers from stable IDs
- explicit hazard guide text for tester recognition; P34 may be guide-only
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
show CPU fallback and must not claim GPU work. CPU fallback remains a supported
alpha path and is not GPU performance evidence.

## Known Limitations

- The P34 tiny fixture contains a creature and food object; the hazard is
  represented by explicit guide-only text unless a richer fixture is loaded.
- The reset flow is relaunch-based for this alpha pass.
- The inspector is intentionally read-only.
- GPU and graphics evidence is local/manual unless captured on a tester machine.
- Computer Use key injection may fail on this Windows desktop; use
  `graphical-controls-smoke` as local deterministic control evidence and keep
  independent human control evidence separate.
- No release tag was created.

## Playtest Checklist

Use `docs/productization/FIRST_GRAPHICAL_ALPHA_PLAYTEST_CHECKLIST.md` for the
external tester record.
