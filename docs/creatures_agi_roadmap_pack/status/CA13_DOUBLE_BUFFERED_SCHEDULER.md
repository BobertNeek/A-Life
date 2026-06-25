# CA13 - Double-buffered graphical/game tick scheduler

Status: implemented on `codex/CA13-double-buffered-graphical-game-tick-scheduler`.

## Scope

CA13 adds a Bevy-free app-layer scheduler for the graphical playground. It
aligns render frames against a fixed simulation cadence and records compact
A/B buffer state for player-facing overlays and smoke evidence.

## Behavior

- Fixed simulation cadence: 20 Hz.
- Target render cadence label: 60 Hz.
- Render interpolation state: `alpha` in the CA13 scheduler summary.
- Double buffer state: front/back `A` and `B` labels.
- Catch-up bound: at most 4 simulation ticks per rendered frame.
- Pause semantics: render frames do not accumulate hidden cognition ticks.
- Step semantics: `N`/step advances exactly one sealed live tick.
- Run semantics: graphical runtime advances only when accumulated fixed-tick
  time is due.

## Evidence command

```powershell
cargo run -p alife_game_app --bin alife_game_app -- double-buffered-scheduler-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Expected evidence includes:

- `fixed_hz=20`
- `render_hz=60`
- `paused=0`
- `sub_tick=0`
- `fixed_tick=1`
- `step=1`
- `catch_up=4`
- `frame_drift_prevented=true`

## Invariants

- `alife_core` is unchanged.
- Bevy remains feature-gated.
- CPU fallback and CPU shadow parity are unchanged.
- The scheduler is presentation/app orchestration only; it does not make Bevy
  authoritative over cognition.
- Player-facing output uses stable IDs only.
- No active bulk neural readback is introduced.

## Limitations

The scheduler exposes timing evidence and bounded catch-up behavior. It does not
claim full action-authoritative GPU runtime and does not change GPU neural
semantics.
