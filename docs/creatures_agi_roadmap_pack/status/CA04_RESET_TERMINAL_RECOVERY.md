# CA04 Reset And Terminal Recovery

Status: CA04 complete.

## Summary

CA04 makes the graphical alpha reset and terminal recovery path explicit for
players and deterministic tests. The existing `R` restart control now records a
visible reset event, and runtime failures or terminal-invalid state are surfaced
as a readable recovery line instead of silently leaving the run ambiguous.

## Player-Facing Behavior

- `R` resets the alpha fixture, preserves stable-ID presentation, clears stale
  action/patch state, and records:
  `Alpha fixture reset; stable IDs preserved. Press Space or N to continue.`
- Terminal or runtime command failure displays:
  `Simulation stopped: <cause>. Press R to restart.`
- The controls line continues to show:
  `Space run/pause | N step | R reset | Esc quit`.
- The deterministic `graphical-controls-smoke` path now verifies reset and
  terminal recovery guidance without relying on Computer Use keyboard injection.

## Boundaries

- No `alife_core` changes.
- No Bevy entity IDs are exposed in player-facing overlay text.
- CPU shadow remains the gate; no full action-authoritative GPU claim was made.
- Reset/recovery is presentation/runtime-control behavior only and does not
  mutate cognition directly.

## Focused Evidence

Focused commands for CA04:

```powershell
cargo test -p alife_game_app --test app_shell ca04 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- graphical-controls-smoke crates/alife_world/tests/fixtures/gpu_alpha
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Forced fallback remains the expected degraded-mode check:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

## Next

CAR04 is the next manifest step and is a hard-stop review gate before UI/content
expansion.
