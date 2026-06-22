# S07 Advanced Gameplay UX Report

Status: implemented on `codex/S07-social-school-semantic-ux`.

S07 exposes the existing social, lifecycle, school, and semantic systems as a
single product-facing advanced gameplay panel. The change does not add a real
LLM, does not let semantic context issue actions, does not let teacher metadata
bypass arbitration, and does not inherit lifetime learning into genetic state.

## Scope

- Owned path: `alife_game_app` display summaries, optional Bevy overlay, smoke
  command, tests, and this productization report.
- No changes to `alife_core`.
- No GPU, save/schema, release-tag, or new roadmap work.
- Advanced systems stay optional and display-only.

## Player-Facing Surface

S07 adds:

- `advanced-gameplay-ux-smoke` for CI-safe product-facing evidence.
- An optional Bevy overlay titled `Advanced Systems (S07)` when the graphical
  playground is built with the `bevy-app` feature.
- A compact social panel showing population order, social samples, vocal tokens,
  and collision feedback.
- A lifecycle panel showing living population, births, deaths, blocked
  reproduction, selected stable ID, and lineage count.
- A school panel showing current curriculum/lesson, teacher cue count, verifier
  result, sealed patch count, and perception-only channel status.
- A semantic panel showing disabled-provider tolerance, fake/local provider
  context, bounded display lines, and explicit no-action/no-weight boundaries.

Focused command:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- advanced-gameplay-ux-smoke
```

Expected fast smoke signature shape:

```text
S07 advanced gameplay UX schema=alife.s07.advanced_gameplay_ux.v1 version=1 social='2:4:4:3:0' lifecycle='3:1:1:1:true' school='g10-grounded-object-food:10100:3:true:true' semantic='true:true:3:true:true' display_only=true optional=true bypass_blocked=true ...
```

## Evidence Matrix

| Area | Evidence | Boundary |
|---|---|---|
| Social UX | G08 population summary is shown as schedule, social samples, vocal tokens, and collisions. | `direct_action_bypass_count=0`; social signals remain perception/modulatory. |
| Lifecycle UX | G09 lineage summary is shown as living/birth/death/lineage state. | Birth weight assets are initializers; lifetime state is not inherited. |
| School UX | G10 lesson cue and verifier status are shown. | Teacher cues enter perception and verifier reads sealed patches. |
| Semantic UX | G11 disabled/fake provider state and bounded context are shown. | Semantic provider cannot act or rewrite weights; absence remains nonfatal. |

## Manual Graphical Evidence

The S07 overlay is wired into the feature-gated Bevy graphical shell, but manual
screenshots are evidence only when a graphical run is actually captured.
This branch captured a local Computer Use screenshot with the S07 panel visible:

```text
target/playtest_evidence/S07/screenshots/s07_advanced_gameplay_overlay_banner_tight.png
```

The screenshot remains an untracked local artifact and is not committed. Do not
treat dry-run, CPU fallback, or CI text output as proof of a full player-facing
graphical playtest.

## Invariant Status

- `alife_core` remains engine-independent and untouched by S07.
- Headless CPU remains the correctness oracle.
- School/teacher signals remain perception/context/feedback only.
- Semantic context remains optional, bounded, non-authoritative, and unable to
  act or rewrite weights.
- Lifecycle display preserves genetic/lifetime separation.
- No P37, G25, or S12 was created.

## Known Limitations

- S07 turns existing bounded smoke systems into an observable panel; it does not
  make social behavior, schooling, or semantic context fully rich gameplay.
- The fake semantic provider is still deterministic test scaffolding, not a real
  SLM.
- The school lesson is one grounded smoke lesson, not a broad curriculum.
- Screenshots remain manual evidence unless captured during a local graphical
  run.

## Recommendation

Proceed to S08 only after S07 is reviewed, merged, and main validation passes.
Future work should improve measured GPU/graphics/performance evidence without
converting optional semantic or school systems into hidden authority paths.
