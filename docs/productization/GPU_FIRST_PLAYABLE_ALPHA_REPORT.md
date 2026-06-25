# GPU-First Playable Alpha Report

Status: GPU-first graphical alpha surface added. No release tag was created.

## Launch

Recommended Windows command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -GpuMode static-plastic-cpu-shadow-guarded
```

Bounded smoke:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Strict GPU evidence:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded -RequireGpu
```

## Product Stance

The player-facing alpha is now framed as `A-Life GPU Alpha Playground`.
`static-plastic-cpu-shadow-guarded` is the default graphical GPU mode.

The current product claim remains:

```text
CpuShadowGuardedStaticPlusLiveHShadow
```

This means GPU scores feed proposal scoring only after CPU shadow parity passes,
normal action arbitration remains in place, and post-seal H_shadow learning is
applied through the core lifetime-delta contract. This is not full
action-authoritative GPU runtime.

## Visible Scenario

The default launcher uses:

```text
crates/alife_world/tests/fixtures/gpu_alpha
```

The fixture contains stable-ID markers for:

- creature `stable:1`
- food `stable:2`
- hazard `stable:3`

The legacy P34 fixture remains available for compatibility and headless tests.

## Fallback

CPU fallback remains available for validation and safety. It is no longer
presented as the expected player-facing mode. If fallback occurs, the UI labels
it as degraded mode and the launcher states that CPU fallback is not GPU
performance evidence. `-RequireGpu` converts fallback into a clear launch
failure instead of silent fallback.

## Controls

- Space: pause/run
- N: step once
- R: reset/restart alpha fixture
- 1/2/3: speed
- F: follow selected stable ID
- Esc: quit

## Known Limitations

- CPU shadow remains the gate; this is not full action-authoritative GPU
  runtime.
- Computer-use keyboard evidence can still be environment-sensitive; use
  `graphical-controls-smoke` as deterministic local control evidence.
- Graphics/GPU evidence remains local hardware evidence and not a public
  performance guarantee.
- No release tag was created.
