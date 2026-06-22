# S09 - Content, tutorial, scenario, and world authoring pass

Branch: `codex/S09-content-tutorial-world-authoring`

Dependencies:
- S08

Recommended model/reasoning: GPT-5.5 High

Next plan(s): S10

## Purpose

Build a small coherent content/tutorial/world-authoring slice: enough player-facing content to explain what the sim is and demonstrate core loops.

## Owned scope

- tutorial script/content pack
- sample world/scenario pack
- lesson pack
- creature preset pack
- authoring validation
- S09 report

## Likely files/crates to inspect or touch

- crates/alife_game_app/**
- crates/alife_tools/**
- docs/productization/**
- examples/**
- assets/ or fixtures if already used

## Forbidden scope

- huge assets
- unversioned content
- content requiring hidden provider/model
- full modding platform

## Implementation milestones

1. Create or refine one coherent tutorial scenario.
2. Create a small world pack with food/hazard/resource/social/school features.
3. Validate content manifests and assets.
4. Add docs for authoring small scenarios.
5. Run content-authoring smoke and graphical/headless smoke.
6. Record content gaps.

## Required tests and evidence

- content pack schema validation
- missing asset rejection
- tiny committed fixture size
- tutorial commands current
- school cues perception-only

## Acceptance criteria

- A new tester can run one coherent scenario/tutorial without hand-editing JSON.
- Content assets are tiny/versioned.
- No huge artifacts committed.

## Focused commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- content-authoring-smoke
```
```powershell
cargo run -p alife_game_app --bin alife_game_app -- onboarding-help-smoke
```
```powershell
cargo run -p alife_tools --bin p35_playground -- validate-manifest examples/p35/playground_manifest.json
```

## Computer-use / manual evidence

- Tutorial walkthrough report and content manifest list.
- Create `S09_CONTENT_TUTORIAL_REPORT.md`.

## Failure handling

- If content depends on unavailable graphics, keep headless content smoke and report graphics gap.
- If assets grow large, stop and convert to refs or generated fixtures.

## Review checklist

- The plan implemented only `S09` scope.
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
Plan: S09 - Content, tutorial, scenario, and world authoring pass
Branch: codex/S09-content-tutorial-world-authoring
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s): S10
Stopped: yes
```
