# S07 - Social, lifecycle, school, and semantic gameplay UX

Branch: `codex/S07-social-school-semantic-ux`

Dependencies:
- S06

Recommended model/reasoning: GPT-5.5 High

Next plan(s): S08

## Purpose

Turn social, lifecycle, school, and semantic systems into visible optional gameplay modes, not just smoke tests.

## Owned scope

- multi-creature social UX
- lifecycle/lineage display
- school lesson UI
- semantic context display
- S07 report

## Likely files/crates to inspect or touch

- crates/alife_game_app/**
- crates/alife_school/**
- crates/alife_semantic/**
- docs/productization/**

## Forbidden scope

- real LLM dependency
- semantic direct actions
- teacher action bypass
- weight rewrites
- unbounded population

## Implementation milestones

1. Show multiple creatures and social/vocal events in the graphical or product-facing UI.
2. Show lifecycle/death/reproduction/lineage status.
3. Expose school lesson cues/verifier results.
4. Expose semantic provider disabled/fake state and bounded context when enabled.
5. Keep all advanced systems optional/non-authoritative.
6. Capture screenshots/evidence.

## Required tests and evidence

- social perception-only
- lineage/genetic separation
- school no-bypass
- semantic provider non-authoritative
- all advanced systems optional

## Acceptance criteria

- A player/tester can observe social/lifecycle/school/semantic state without hidden authority paths.
- Missing semantic provider remains nonfatal.
- No action/weight bypass.

## Focused commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- population-social-loop-smoke
```
```powershell
cargo run -p alife_game_app --bin alife_game_app -- lifecycle-lineage-smoke
```
```powershell
cargo run -p alife_game_app --bin alife_game_app -- school-mode-smoke
```
```powershell
cargo run -p alife_game_app --bin alife_game_app -- semantic-provider-smoke
```

## Computer-use / manual evidence

- Screenshots of social/lineage/school/semantic panels if GUI available.
- Create `S07_ADVANCED_GAMEPLAY_UX_REPORT.md`.

## Failure handling

- If advanced modes remain CLI-only, report as product UX gap and do not overclaim.
- If provider/teacher can influence action directly, block and fix.

## Review checklist

- The plan implemented only `S07` scope.
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
Plan: S07 - Social, lifecycle, school, and semantic gameplay UX
Branch: codex/S07-social-school-semantic-ux
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s): S08
Stopped: yes
```
