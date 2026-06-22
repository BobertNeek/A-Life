# S03 Camera Inspector Playtest Report

## Scope

S03 adds camera navigation and a read-only creature inspector to the
feature-gated graphical playground. The graphical path remains a product shell
over the existing CPU reference/headless fixtures: `alife_core` is unchanged,
CPU fallback is still displayed as the backend status, and the inspector does
not issue actions or mutate cognition.

## Visible Surface

The graphical window now shows:

- a stable-ID creature marker with a selection ring
- the existing food/world marker labels
- a left runtime status overlay with playback, tick, sealed patch, log, and CPU
  fallback status
- a right read-only inspector overlay with the selected stable ID, adapter-local
  presence, organism/action/target summary, sealed patch status, drives,
  hormones, sleep/visual state, memory/topology summary, and camera state
- camera controls for pan, zoom, orbit, and follow

The overlay intentionally says `adapter-local` instead of printing raw Bevy
entity IDs. Portable save/model state remains stable-ID based.

## Evidence Commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- creature-inspector-smoke crates/alife_world/tests/fixtures/p34
```

```powershell
cargo run -p alife_game_app --bin alife_game_app -- creature-visual-smoke crates/alife_world/tests/fixtures/p34
```

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun
```

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10
```

## Manual Evidence

Computer Use was available on this machine. Native
`get_window_state(... include_screenshot: true)` remains unreliable on this
Windows 10 host, so screenshots were captured through the verified
Alt+PrintScreen clipboard fallback.

Local untracked evidence paths:

- `target/playtest_evidence/S03/screenshots/s03_initial_selected_inspector.png`
- `target/playtest_evidence/S03/screenshots/s03_camera_pan_zoom_orbit_follow.png`
- `target/playtest_evidence/S03/logs/graphical_smoke_10s_after_fix.log`
- `target/playtest_evidence/S03/logs/interactive_graphical_launch.log`

Observed interaction:

- The A-Life graphical window opened with title `A-Life Graphical Playground`.
- The selected creature marker used stable ID `1` and displayed a selection
  ring.
- The inspector overlay reported `Selected stable:1`, sealed patch status,
  drives, hormones, sleep/visual state, memory/topology updates, camera state,
  and `read_only=true`.
- The runtime overlay reported `Backend: CPU Reference fallback`.
- Keyboard input through Computer Use exercised right/up pan, zoom in, orbit
  right, and follow selected stable ID.
- The second screenshot showed the camera zoom/yaw changed to `zoom=1.25` and
  `yaw=15.0`.
- `Esc` closed the window and no A-Life app window remained.

## Screenshot Index

| path | what it proves | result |
| --- | --- | --- |
| `target/playtest_evidence/S03/screenshots/s03_initial_selected_inspector.png` | Persistent graphical window with stable-ID creature selection, food marker, runtime overlay, read-only inspector, and CPU fallback status. | pass |
| `target/playtest_evidence/S03/screenshots/s03_camera_pan_zoom_orbit_follow.png` | Camera navigation input changed zoom/yaw while preserving the stable-ID inspector overlay. | pass |

## Design Boundary

The inspector is presentation-only. It is derived from existing app-shell,
visible-world, live-brain, creature-visual, and creature-inspector summaries.
It does not add fields to `alife_core`, does not serialize engine-local IDs, and
does not mutate brain, memory, topology, or action arbitration state.

## Known Limitations

- Mouse picking and object cycling are not implemented in S03. The graphical
  playground starts with the fixture creature selected by stable ID and supports
  keyboard camera/follow controls.
- This is still a diagnostic playground. It is not full visual polish, menu UX,
  or save/load UX.
- GPU acceleration is not required. The captured evidence shows CPU fallback,
  not measured GPU performance.
- The graphical smoke log still uses the existing S02 runtime-smoke wording for
  the live-loop summary after shutdown; the S03 inspector evidence is visible in
  the window screenshots.
