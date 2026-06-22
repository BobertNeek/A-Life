# S01 - Persistent graphical app window and launch stabilization

Branch: `codex/S01-graphical-window-stabilization`

Dependencies:
- R24 complete
- S00 evidence available

Recommended model/reasoning: GPT-5.5 High

Next plan(s): S02

## Purpose

Create the smallest persistent graphical app window that a normal player/tester can launch, observe, and close. S00 found the current graphical script exits after a smoke command; S01 must make that real window path persistent.

## Owned scope

- feature-gated graphical launcher/app shell
- `scripts/run_graphical_playground.ps1`
- `alife_game_app` graphical CLI/app command
- S01 evidence report
- CI-safe argument/config tests

## Likely files/crates to inspect or touch

- crates/alife_game_app/**
- crates/alife_bevy_adapter/**
- scripts/run_graphical_playground.ps1
- docs/productization/**
- Cargo.toml feature flags

## Forbidden scope

- full gameplay polish
- content expansion
- GPU performance claims
- release packaging/tagging
- `alife_core` changes
- committing screenshots/logs

## Implementation milestones

1. Audit current graphical launcher and Bevy feature path.
2. Add persistent interactive mode, timed smoke mode, and dry-run mode.
3. Display a minimal scene with creature/food/world placeholders and backend status.
4. Ensure the window stays open until user close or smoke timeout.
5. Use Computer Use to detect the window and capture screenshots where available.
6. Document launch commands and limitations.

## Required tests and evidence

- dry-run command test
- smoke-seconds argument/timeout test
- headless fallback still works
- all-features compile/test
- no `alife_core` dependency leak

## Acceptance criteria

- GUI command opens a persistent window when graphics are available.
- Timed smoke mode exits cleanly.
- Dry-run does not open a window.
- Screenshots or exact unavailable reason are recorded.
- No runtime claim exceeds measured evidence.

## Focused commands

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun
```
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 5
```
```powershell
cargo run -p alife_game_app --bin alife_game_app -- visible-world-smoke crates/alife_world/tests/fixtures/p34
```

## Computer-use / manual evidence

- Capture initial window screenshot if GUI available.
- Capture failure/terminal output if GUI unavailable.
- Create `docs/productization/S01_GRAPHICAL_STABILIZATION_REPORT.md`.

## Failure handling

- If window creation fails due environment, record exact reason and keep dry-run/smoke compile gates.
- If default headless path regresses, fix before proceeding.
- If Bevy feature causes dependency leak into core, remove leak and stop if unresolved.

## Review checklist

- The plan implemented only `S01` scope.
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
Plan: S01 - Persistent graphical app window and launch stabilization
Branch: codex/S01-graphical-window-stabilization
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s): S02
Stopped: yes
```
