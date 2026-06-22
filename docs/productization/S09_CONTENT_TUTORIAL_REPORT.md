# S09 Content, Tutorial, Scenario, And World Authoring Report

Status: implemented on `codex/S09-content-tutorial-world-authoring`.

S09 adds a small first-run content pack and a product-facing smoke command that
validates the authored world, lesson, creature preset, scenario pack, P34
asset/config/save references, and onboarding tutorial path. This is a content
and validation slice only; it does not add a full modding platform or new
runtime cognition.

## Player/Test Command

```powershell
cargo run -p alife_game_app --bin alife_game_app -- content-authoring-smoke
```

Expected successful output includes:

```text
S09 content tutorial authoring schema=alife.s09.content_tutorial_authoring.v1 version=1 pack=s09-first-run-tutorial-pack worlds=1 lessons=1 creatures=1 scenarios=1 ...
```

The existing onboarding and manifest commands remain current:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- onboarding-help-smoke
cargo run -p alife_tools --bin p35_playground -- validate-manifest examples/p35/playground_manifest.json
```

## Content Pack

Committed S09 files:

```text
content/fixtures/s09/content_pack_manifest.json
content/fixtures/s09/worlds/first_run_meadow_world.json
content/fixtures/s09/lessons/food_hazard_school_lesson.json
content/fixtures/s09/creatures/nano_forager_preset.json
content/fixtures/s09/scenarios/first_run_tutorial_scenario.json
```

The pack is intentionally tiny and uses versioned JSON. It references existing
P34 fixtures and the existing tiny generated-weight reference rather than
committing bulk tensors or large assets.

## Scenario Coverage

The first-run meadow includes:

- player creature marker
- peer creature marker for bounded social context
- food marker
- hazard marker
- rest/obstacle marker
- teacher word/token marker
- resource, hazard, and school perception zones

The lesson pack includes food naming, hazard warning, and peer context steps.
Every lesson step is perception-only:

- `perception_only=true`
- `direct_motor_bypass=false`
- `hidden_vector_injection=false`

## Validation Behavior

The S09 smoke validates:

- current content-pack schema and version
- P34 runtime config, asset manifest, asset root, and save references
- stable world IDs and finite object values
- food, hazard, peer/social, school token, and resource-zone coverage
- creature preset birth-only generated-weight reference
- tutorial scenario recommended commands
- missing required content rejection
- small committed fixture size

The app-side S09 smoke deliberately does not add an `alife_tools` dependency to
`alife_game_app`; it validates the same committed content boundary directly so
runtime crates do not depend on offline tools.

## Manual / Graphical Evidence

S09 content validation is headless and deterministic. The scenario pack lists a
manual graphical command for testers, but graphical launch evidence is not
required to prove the content pack itself:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1
```

Graphics remain optional/manual evidence and should not be represented as
content validation unless a graphical run is actually captured.

## Known Limitations

- This is not a full modding platform.
- Scenario packs are small, curated fixtures rather than arbitrary user content.
- The tutorial still relies on smoke commands and overlays; it is not a polished
  in-window quest UI.
- Graphics/GPU evidence remains governed by the S08 report.

## Next Step

Proceed to S10 only after S09 review, merge, and main validation. S10 should use
this content pack as one input to packaging/QA/external playtest candidate work,
without creating S12, G25, or P37.
