# CA29 - Creature Memory/History Journal

Status: implemented on `codex/CA29-creature-memory-history-journal`.

## Summary

CA29 adds a read-only creature memory/history journal to the graphical alpha
app. The journal mirrors bounded recent sealed patches, stored memory records,
and memory expectancy bias summaries from `alife_core` into player-facing text.
It is inspection-only: memory remains expectancy/context bias and cannot replay
actions, emit actions, bypass arbitration, or mutate cognition.

## Player/Developer Surface

- Graphical app panel: `Memory Journal (read-only)`.
- Compact status line: memory record count, sealed patch row count, and
  `bias-only no-replay` boundary copy.
- Recent patch rows show stable IDs, tick/sequence, success state, and memory
  update counts.
- Recent memory rows show stable memory IDs, source tick/sequence, valence,
  affordance, danger, and observed action label as history only.
- Expectancy rows show bounded bias values and confidence.
- Save/load visibility states that stable memory IDs are visible; no engine
  IDs or Bevy `Entity` values are exposed.

## Focused Evidence

```powershell
cargo run -p alife_game_app --bin alife_game_app -- memory-history-journal-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Observed:

- `memories=5`
- `patches=5`
- `bias_rows=4`
- `action_replay_blocked=true`
- `direct_mutation=false`

Focused tests:

```powershell
cargo test -p alife_game_app --test app_shell ca29 -- --nocapture
```

The tests validate:

- schema/version,
- recent sealed patches,
- memory expectancy summaries,
- save/load visibility,
- read-only/no-replay boundary,
- no Bevy `Entity` leakage,
- no full action-authoritative claim.

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
- Memory expectancy is bias/context only, not action replay.
- The journal is read-only and cannot emit actions.
- CPU fallback and CPU shadow parity remain unchanged.
- Product GPU claim is unchanged; this plan does not prove full
  action-authoritative GPU runtime.

## Known Limitations

- The journal is a compact text panel, not a searchable history browser.
- It summarizes bounded recent rows; older memory remains in the core bank but
  is not fully displayed.
- Graphical layout evidence remains local machine evidence and should be
  rechecked during the CAR31 cognition inspection review.

## Next Plan

CA30 - Neural activity and lobe profiler view.
