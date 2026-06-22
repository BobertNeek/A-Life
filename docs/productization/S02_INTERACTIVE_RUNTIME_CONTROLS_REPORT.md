# S02 Interactive Runtime Controls Report

## Scope

S02 adds the smallest interactive runtime-control layer over the existing CPU
reference live brain loop. The controls are a product shell feature only:
`alife_core` remains engine-independent, the CPU oracle remains authoritative,
and graphical input maps to the existing sealed-patch tick loop.

## Controls

- Space: pause/resume.
- N: step exactly one live brain tick.
- 1/2/3: run at one, two, or three bounded ticks per update.
- Esc: request shutdown.

The Bevy overlay displays playback state, run speed, mind tick, world tick,
last selected action, target stable ID, sealed patch count, packed log count,
and CPU fallback/backend status.

## Evidence Commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- runtime-controls-smoke crates/alife_world/tests/fixtures/p34 5
```

```powershell
cargo run -p alife_game_app --bin alife_game_app -- live-brain-paused-smoke crates/alife_world/tests/fixtures/p34
```

```powershell
cargo run -p alife_game_app --bin alife_game_app -- live-brain-fixed-smoke crates/alife_world/tests/fixtures/p34 5
```

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 5
```

## Design Boundary

Runtime controls cannot issue actions, rewrite cognition state, or bypass action
arbitration. They only choose the live-loop cadence: paused, one sealed tick, or
a bounded fixed run. The selected action still comes from the existing CPU
reference arbitration path and every tick summary must report a sealed patch.

## Manual Evidence

Computer Use was available on this machine. Native
`get_window_state(... include_screenshot: true)` still failed on Windows 10 with
`SetIsBorderRequired failed: No such interface supported (0x80004002)`, so
window screenshots were captured through the verified Alt+PrintScreen clipboard
fallback.

Local untracked evidence paths:

- `target/playtest_evidence/S02/screenshots/s02_graphical_smoke_initial_window.png`
- `target/playtest_evidence/S02/screenshots/s02_interactive_paused.png`
- `target/playtest_evidence/S02/screenshots/s02_interactive_step_once.png`
- `target/playtest_evidence/S02/screenshots/s02_interactive_running_speed2.png`

Observed controls:

- Initial window opened in paused state.
- `N` advanced the live loop by one sealed tick.
- `Space` resumed running.
- `2` changed the run speed overlay to two ticks per update.
- `Esc` closed the window cleanly.

## Known Limitations

- This is still a minimal diagnostic playground, not full game UX polish.
- Save/load UI and camera polish are intentionally deferred to later product
  work.
- GPU acceleration is not required and CPU fallback remains the displayed
  backend status.
- The graphical smoke output reports deterministic S02 runtime smoke state
  after shutdown; the overlay itself updates during the Bevy run.
