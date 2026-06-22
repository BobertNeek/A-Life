# S03 - Camera, selection, creature inspector, and screenshot evidence

Branch: `codex/S03-camera-inspector-screenshots`

Dependencies:
- S02

Recommended model/reasoning: GPT-5.5 High

Next plan(s): S04

## Purpose

Make the player able to see and inspect the world: camera movement, creature selection, inspector panel, screenshots, and clear evidence that presentation remains read-only.

## Owned scope

- camera controls
- selection input
- creature inspector overlay
- screenshot/evidence harness
- S03 report

## Likely files/crates to inspect or touch

- crates/alife_game_app/**
- crates/alife_bevy_adapter/**
- docs/productization/**

## Forbidden scope

- save/load UX
- new ecosystem tuning
- new core fields just for UI
- engine-local IDs in saves

## Implementation milestones

1. Add/verify camera pan/zoom/orbit/follow controls.
2. Implement stable-ID based creature/object selection.
3. Show read-only inspector: drives, hormones, action, target, sleep, sealed patch status.
4. Add screenshot capture instructions and index.
5. Use Computer Use to click/select and record observed UI.
6. Document manual gaps.

## Required tests and evidence

- camera controls validate bounded values
- selection uses stable IDs
- inspector read-only
- no Bevy entity IDs in model/persistence
- screenshot index docs path checks

## Acceptance criteria

- A tester can select a creature and read its state in a graphical window.
- Inspector is read-only and stable-ID based.
- Computer Use screenshots or exact missing evidence are recorded.

## Focused commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- creature-inspector-smoke crates/alife_world/tests/fixtures/p34
```
```powershell
cargo run -p alife_game_app --bin alife_game_app -- creature-visual-smoke crates/alife_world/tests/fixtures/p34
```
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10
```

## Computer-use / manual evidence

- Screenshots: camera view, selected creature, inspector overlay, status panel.
- Create `S03_CAMERA_INSPECTOR_PLAYTEST_REPORT.md`.

## Failure handling

- If selection cannot be interacted with via Computer Use, mark manual evidence missing and keep CLI tests.
- If UI requires engine IDs in portable state, stop and redesign adapter boundary.

## Review checklist

- The plan implemented only `S03` scope.
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
Plan: S03 - Camera, selection, creature inspector, and screenshot evidence
Branch: codex/S03-camera-inspector-screenshots
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s): S04
Stopped: yes
```
