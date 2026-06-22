# S05 - Player-facing save/load/menu UX

Branch: `codex/S05-save-load-menu-ux`

Dependencies:
- S04

Recommended model/reasoning: GPT-5.5 High or Extra High

Next plan(s): S06

## Purpose

Expose save/load/config through player-facing UX rather than only CLI smoke commands.

## Owned scope

- menu/start/load/save UX
- slot handling
- error display
- config menus/settings
- S05 report

## Likely files/crates to inspect or touch

- crates/alife_game_app/**
- crates/alife_world/src/persistence.rs
- docs/productization/**
- scripts/**

## Forbidden scope

- schema redesign unless direct bug
- engine-local ID persistence
- cloud saves
- release packaging

## Implementation milestones

1. Add a simple start/menu surface or command-accessible menu state.
2. Expose new/load/save/overwrite/cancel flow.
3. Show validation errors readably.
4. Restore visible world from saved slot.
5. Add autosave/manual save policy if minimal.
6. Use Computer Use to walk through menu if GUI available.

## Required tests and evidence

- save slot roundtrip
- overwrite guard
- invalid save/config error display
- stable IDs preserved
- headless save/load still works

## Acceptance criteria

- Player/tester can save and load a visible tiny world through a product-facing path.
- Errors are readable.
- No engine-local IDs enter save files.

## Focused commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- save-load-ux-smoke crates/alife_world/tests/fixtures/p34
```
```powershell
cargo run -p alife_tools --bin p35_playground -- save-load crates/alife_world/tests/fixtures/p34
```
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10
```

## Computer-use / manual evidence

- Screenshots of menu/save/load/error states if GUI available.
- Create `S05_SAVE_LOAD_UX_REPORT.md`.

## Failure handling

- If UI save/load cannot be completed in GUI, record blocker/high gap and do not claim player save UX.
- If persistence schema changes are needed, stop unless fix is tightly scoped and tested.

## Review checklist

- The plan implemented only `S05` scope.
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
Plan: S05 - Player-facing save/load/menu UX
Branch: codex/S05-save-load-menu-ux
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s): S06
Stopped: yes
```
