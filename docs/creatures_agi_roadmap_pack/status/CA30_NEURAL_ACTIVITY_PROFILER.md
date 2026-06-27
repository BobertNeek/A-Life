# CA30 - Neural Activity and Lobe Profiler View

Status: implemented on `codex/CA30-neural-activity-lobe-profiler-view`.

## Summary

CA30 adds a compact, read-only neural activity profiler to the graphical alpha
and headless app smoke surface. The profiler mirrors bounded lobe activity
rows, active tile/synapse bounds, and GPU/CPU route status without reading raw
neural tensors during active play.

## Player/Developer Surface

- Graphical app panel: `Neural Profiler (compact)`.
- Main status line: compact `Neural:` lobe/tile/synapse/route summary.
- Lobe rows show stable core lobe labels, bounded activity bars, and the
  source of the summary signal.
- Tile/synapse rows report active counts against brain-class bounds.
- Route status shows requested mode/backend, CPU shadow gate, parity, compact
  readback bytes, post-seal boundary bytes, and fallback state.
- The panel states that active play is compact-summary only and that fuller
  export belongs at an offline/export boundary.

## Focused Evidence

```powershell
cargo test -p alife_game_app --test app_shell ca30 -- --nocapture
```

Observed:

- 2 CA30 tests passed.
- The tests validate lobe rows, tile/synapse bounds, route status, CPU shadow
  gate visibility, no Bevy `Entity` leakage, no full action-authoritative claim,
  no active bulk readback, no action authority, and no weight mutation.

```powershell
cargo run -p alife_game_app --bin alife_game_app -- neural-activity-profiler-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Observed:

- `lobes=6`
- `tiles=10/64`
- `syn=640/8192`
- `backend=CpuReference` in the headless smoke path
- `bulk_readback_blocked=true`
- `action_authority_blocked=true`
- `weight_mutation_blocked=true`

Graphical smoke remains the product-facing visual evidence command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Forced fallback remains explicit:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

## Boundaries

- No `alife_core` dependency change.
- No Bevy/wgpu/model-runtime dependency enters `alife_core`.
- The profiler is read-only and cannot emit actions.
- The profiler cannot mutate weights or cognition.
- Active play does not perform bulk neural, per-lobe, per-synapse, or weight
  readback.
- CPU fallback and CPU shadow parity remain unchanged.
- Product GPU claim is unchanged; this plan does not prove full
  action-authoritative GPU runtime.

## Known Limitations

- Lobe activity bars are compact presentation summaries derived from stable
  lobe layout plus bounded runtime signals; they are not raw activation tensor
  traces.
- Headless smoke reports CPU-summary route status. Graphical GPU route status
  updates from the existing graphical GPU telemetry resource during graphical
  smoke.
- Offline/export tooling for deeper neural traces remains future scope.

## Next Plan

CA31 - Player lab tools for behavior comparison.
