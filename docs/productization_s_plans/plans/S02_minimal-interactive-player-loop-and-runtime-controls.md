# S02 - Minimal interactive player loop and runtime controls

Branch: `codex/S02-interactive-player-loop`

Dependencies:
- S01

Recommended model/reasoning: GPT-5.5 High

Next plan(s): S03

## Purpose

Turn the persistent window into a minimal interactive loop: pause, step, run, tick state display, visible updates from the existing CPU/headless brain loop, and clean shutdown.

## Owned scope

- interactive runtime controls
- keyboard/control mapping
- tick scheduler bridge in graphical mode
- overlay/status text
- S02 interaction evidence report

## Likely files/crates to inspect or touch

- crates/alife_game_app/**
- crates/alife_bevy_adapter/**
- scripts/run_graphical_playground.ps1
- docs/productization/**

## Forbidden scope

- new cognition model
- save/load UI
- full camera polish
- large UI framework rewrite
- action arbitration bypass

## Implementation milestones

1. Add pause/resume/step/run speed controls.
2. Wire controls to existing live brain loop without bypassing sealed patch order.
3. Display tick count, selected action, backend status, and pause state.
4. Add timed smoke mode that advances a known number of ticks.
5. Use Computer Use to exercise controls if GUI available.
6. Record evidence and screenshots.

## Required tests and evidence

- headless tick loop unchanged
- pause does not advance state
- step advances exactly one tick
- run advances bounded ticks
- controls cannot mutate cognition directly
- smoke command logs sealed patches

## Acceptance criteria

- Player/tester can pause, step, resume/run, and exit.
- Visible status updates after ticks.
- No direct control path bypasses arbitration.
- CI-safe smoke covers control semantics.

## Focused commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- live-brain-paused-smoke crates/alife_world/tests/fixtures/p34
```
```powershell
cargo run -p alife_game_app --bin alife_game_app -- live-brain-fixed-smoke crates/alife_world/tests/fixtures/p34 5
```
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 5
```

## Computer-use / manual evidence

- Screenshots for paused, stepped, running, and exit states if GUI available.
- Report controls attempted and observed result.

## Failure handling

- If input cannot be observed through Computer Use, keep deterministic CLI tests and record manual evidence missing.
- If control changes affect headless tick determinism, revert or isolate graphical layer.

## Review checklist

- The plan implemented only `S02` scope.
- Runtime/code changes match the plan's owned scope.
- `alife_core` remains engine-independent.
- Headless CPU path remains green.
- Optional graphics/GPU/semantic/school systems remain optional unless explicitly hardened.
- No P37/G25/new automatic chain was created.
- Product claims match actual evidence.
- Reports under `docs/productization/` are honest about unavailable manual evidence.



## Global invariants

Read and obey:

- `docs/productization_s_plans/GLOBAL_INVARIANTS.md` if imported there, or the imported equivalent under the productization plan pack.
- Existing repo invariants in `AGENTS.md`.
- Existing P36/R24 validation discipline.

## Standard validation

Use Windows wrappers. Do not run plain `bash scripts/check.sh`.

Run the standard validation set from `VALIDATION_PROTOCOL.md`, plus each plan's focused commands.

## Completion receipt

```text
Completion receipt
Plan:
Branch:
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s):
Stopped:
```


## Required receipt override

```text
Completion receipt
Plan: S02 - Minimal interactive player loop and runtime controls
Branch: codex/S02-interactive-player-loop
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s): S03
Stopped: yes
```
