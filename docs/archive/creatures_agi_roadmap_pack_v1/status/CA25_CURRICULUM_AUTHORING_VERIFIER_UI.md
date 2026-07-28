# CA25 - Curriculum Authoring And Verifier UI

Status: complete

Branch: `codex/CA25-curriculum-authoring-verifier-ui`

## Scope

CA25 adds a small, bounded lesson manifest format and validator for the school
mode path. The app-level smoke validates `examples/ca25/lesson_manifest.json`,
maps verifier conditions to sealed-patch checks, displays curriculum progress,
and roundtrips portable lesson save state.

## Evidence

Focused command:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- curriculum-authoring-smoke
```

Expected result includes:

- schema `alife.ca25.curriculum_authoring.v1`
- one active lesson with lesson ID `10100`
- sealed-patch verifier evidence
- completed progress `1/1`
- model inference required `false`
- fake model output used `false`
- action authority `false`
- weight rewrite authority `false`

## Boundary

This plan does not implement the CA26 semantic embedding provider or the CA27
local SLM prior. Lesson manifests are player/tooling configuration only. They
cannot emit actions, rewrite weights, inject hidden vectors, or store teacher
private state. Teacher and verifier behavior remains perception-only and
sealed-patch based.

Next executable plan: CA26.
