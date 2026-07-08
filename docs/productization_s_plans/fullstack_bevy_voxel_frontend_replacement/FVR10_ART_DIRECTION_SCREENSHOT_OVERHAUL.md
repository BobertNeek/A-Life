# FVR10 - Art Direction and Screenshot-Driven Visual Overhaul

Branch: `codex/FVR10-art-direction-screenshot-overhaul`

Prerequisites: FVR09 completed technically but the live result still looks bad.

Concurrency: Serial visual-quality correction. Do not run concurrently with other renderer/material/creature branches.

Recommended model/reasoning: GPT-5.5 Pro High.

## Problem statement

FVR09 technically satisfied many acceptance checks, but the game still looks visually bad. Treat that as a failed art-direction outcome, not as user preference noise.

Evidence from the FVR09 result:

- FVR09 declared completion while relying on generated/project JSON material definitions rather than actual hand-authored or generated texture images.
- The production asset manifest marks procedural JSON configs as `final_art=true`, which is not sufficient visual proof.
- Creature visuals are described by metadata such as `fvr09-cute-biped-v1`, but metadata is not a finished character design.
- The renderer path includes material entries with texture-slot names, but the visible `StandardMaterial` construction still uses RGBA base colors and roughness rather than bound terrain texture handles.
- A screenshot path was recorded, but the visual result is still judged unacceptable by Cassidy.

This goal fixes the actual visible output. Passing tests, manifests, and performance receipts is not enough. FVR10 must produce a screenshot-visible improvement.

## Non-negotiable constraints

- Preserve `MinimumSettings30x30`: 30 real creatures at 30 FPS on the minimum hardware class.
- Preserve `MinSpecComfort1080p`: default comfortable profile on RTX 3050 8 GB / i7-3770K / Windows 10 / 1080p.
- Preserve `Balanced1080p`, `HighSpecScaleUp`, and `ResearchScale`.
- Preserve real simulation, real saves, real backend selection/fallback, real creatures, stable IDs, and no-mock product paths.
- Do not add Bevy, wgpu, renderer, asset, OS window, or entity-handle dependencies to `alife_core`.
- Do not let visual systems mutate cognition, actions, learning, reproduction, backend selection, or world truth.
- Do not mark JSON-only palette labels as final art unless they drive actual visible shader/texture/mesh output that passes screenshot review.
- Do not use unlicensed external assets.
- Do not claim visual completion without committed screenshot evidence or a clearly documented local screenshot artifact path plus exact command to reproduce it.

## Owned scope

Expected touch areas:

- `crates/alife_game_app/src/production_voxel_renderer.rs`
- current terrain material/shader modules
- current creature renderer modules
- production asset manifest(s)
- actual asset directories under `crates/alife_game_app/assets/**` or the current asset root
- texture/image generation scripts if procedural assets are generated
- Bevy material/shader setup
- production screenshot/performance commands
- FVR docs under `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/`

Do not assume exact paths. Inspect current code first.

## Required visual target

The final scene should look like a small stylized voxel diorama / cozy ALife ecosystem, not a debug visualization.

Target qualities:

- muted natural palette, not primary colors;
- textured or shader-detailed terrain, not flat color blocks;
- grass/soil blocks with top/side distinction;
- visible small-scale ground variation;
- water/wet/decay/resource/hazard materials that are visually distinct but not neon debug colors;
- cute bipedal creatures with readable face, head, body, legs, feet, and expression;
- creatures should look like simple living characters, not diagnostic primitives;
- lighting and camera should make silhouettes readable;
- UI/debug overlays should not dominate the default screenshot;
- the default view should be pleasant enough that a user would not describe it as programmer art.

## Implementation requirements

### 1. Visual failure audit

Before editing, inspect and document why FVR09 still looks bad.

Required audit outputs in `FVR10_VISUAL_AUDIT.md`:

- current runtime screenshot path(s);
- whether actual image textures are loaded or only material slot names exist;
- whether `StandardMaterial` uses texture handles, custom material/shader texture sampling, or only flat RGBA;
- creature mesh part count, actual geometry, actual material usage, and screenshot-visible silhouette;
- whether debug overlays/chunk boundaries are visible in default product screenshot;
- lighting/camera weaknesses;
- exact reasons the result still reads as crude.

Do not stop at audit. Implement the overhaul.

### 2. Replace fake texture slots with visible texture/shader output

Terrain materials must visibly use one of:

- actual generated PNG/WebP texture atlas loaded through Bevy assets;
- actual array texture loaded through Bevy assets;
- custom procedural shader that visibly creates non-flat texture/noise/variation;
- generated mesh vertex colors with high-enough variation and face distinction to no longer read as flat debug color.

A JSON field named `top_texture` or `side_texture` is not enough unless it is actually bound into rendering.

Required terrain visual features:

- grass top and soil side distinction;
- stone with visible speckle/fracture/noise;
- dirt/soil variation;
- water/wet material with transparent or reflective/stylized surface behavior if water exists;
- decay material that reads organic/rotting without neon debug colors;
- resource vegetation or food material that looks like plants/food, not a green square label;
- hazard material that reads as dangerous/noxious but not raw red primary-color debug;
- distance-readable biome tinting;
- texture/material resolution and sampling chosen to preserve performance on minimum spec.

If using generated textures, commit the generator source/config and generated assets if they are small enough. If generated assets are too large, document the regeneration path and keep manifests precise. No large generated artifacts should be committed unless project policy allows them.

### 3. Replace procedural-description creatures with actual character design

Creature visuals must be improved at the actual mesh/material level, not just JSON metadata.

Required visible creature design:

- cute bipedal silhouette;
- large head, small rounded body, short legs, visible feet;
- face with eyes visible from production camera angle;
- side appendages/arms if feasible;
- softened geometry: bevels, rounded cuboids, low-poly spheres, or stylized capsule approximations;
- non-primary body colors;
- expression material/mesh changes visible in screenshot: sleep, fear/danger, hunger/low-energy, selected/focused;
- movement animation that reads as bipedal locomotion or bobbing/stepping, not sliding cubes;
- lower LOD that remains cute and bipedal rather than reverting to crude primitives.

Acceptable approaches:

- generated low-poly mesh assets;
- simple glTF assets if license is clean;
- procedural mesh using rounded-cube/sphere/capsule approximations;
- instanced creature mesh with per-instance color/expression attributes;
- billboard impostor only for distant LODs, not default near creature view.

Unacceptable:

- boxes plus flat plane as default;
- metadata-only creature design;
- hidden creature details that are too tiny to read in the default camera;
- external unlicensed character models;
- performance-only excuse for ugly default assets.

### 4. Default camera, lighting, and presentation pass

Make the default view read as a game scene.

Required:

- default product screenshot should hide or minimize debug panels/chunk boundaries unless explicitly in debug mode;
- camera angle and zoom should show terrain texture and creature silhouettes clearly;
- lighting should avoid flat unshaded primary-color look;
- add ambient/fill light or stylized shadowing appropriate for readability;
- add subtle fog/depth/color grading if cheap;
- selected creature highlight should be readable but not visually ugly;
- screenshots must be captured from the actual production app path.

### 5. Screenshot-driven acceptance

FVR10 must produce screenshot evidence.

Required artifacts:

- before screenshot path or reference from FVR09 if available;
- after screenshot path generated by the production app;
- visual audit notes explaining what changed;
- explicit statement whether terrain uses real textures/shaders/vertex-color variation or still only flat RGBA;
- explicit statement whether creatures are actual bipedal character meshes or only metadata descriptions.

The screenshot artifact may stay under ignored `target/artifacts/`, but the report must include exact path and exact command to regenerate it. Do not commit large screenshot artifacts unless repo policy allows it.

### 6. Performance preservation

Visual quality must not break performance gates.

Validate at least:

- `MinimumSettings30x30 --population 30 --record-performance`;
- `MinSpecComfort1080p --record-performance`;
- `validate-production-save --profile MinimumSettings30x30`;
- `validate-production-save --profile MinSpecComfort1080p`.

If screenshots require a graphics machine and current Codex environment cannot generate them, record the exact command and require Cassidy-run screenshot evidence before declaring visual acceptance. However, do not mark FVR10 visually complete without screenshot evidence from a real graphical run.

### 7. Manifest truthfulness

Update asset manifests so they distinguish:

- `final_art`: actual visible texture/model/shader asset used in production;
- `material_metadata`: JSON descriptors that are not themselves visible art;
- `generated_source`: source/config used to generate visible assets;
- `debug_visual`: debug-only colors/overlays;
- `placeholder`: anything not acceptable as production final art.

Do not mark descriptor-only JSON as final art unless it directly drives visible shader/mesh output and the screenshot proves acceptable appearance.

## Required validation

Run, as applicable:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

Run current production graphical commands, adjusted to actual FVR08/FVR09 command names if different:

```powershell
cargo run --release -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinimumSettings30x30 --population 30 --record-performance
cargo run --release -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinSpecComfort1080p --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -- validate-production-save --profile MinimumSettings30x30
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -- validate-production-save --profile MinSpecComfort1080p
```

Add the actual screenshot-generation command used by the current app. If no screenshot command exists, add one.

## Acceptance criteria

FVR10 is complete only when:

- default production screenshot no longer looks like primary-color programmer art;
- terrain visibly uses real texture/shader/variation output, not only flat RGBA labels;
- descriptor-only JSON is no longer misrepresented as final visible art;
- creatures are visibly cute and bipedal in the actual production screenshot;
- creature geometry/materials are actual visible assets or procedural meshes, not just metadata descriptions;
- default product view minimizes debug visual clutter;
- `MinimumSettings30x30` still meets 30 real creatures at 30 FPS on target hardware class;
- `MinSpecComfort1080p` remains comfortable;
- save/load validates;
- no mocks/fakes are added;
- `alife_core` remains renderer-free;
- FVR10 visual audit and completion docs include screenshot paths and exact reproduction commands.

## Required docs

Add:

- `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_VISUAL_AUDIT.md`
- `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_COMPLETION.md`

`FVR10_COMPLETION.md` must include:

```text
Completion receipt
Plan: FVR10 - Art Direction and Screenshot-Driven Visual Overhaul
Branch:
Files changed:
Crude visual causes found:
Terrain texture/shader implementation:
Creature mesh/material implementation:
Camera/lighting/presentation changes:
Asset manifest truthfulness changes:
Before screenshot path:
After screenshot path:
Screenshot reproduction command:
Commands run:
Hardware/GPU evidence:
MinimumSettings30x30 result:
MinSpecComfort1080p result:
Save/load result:
Boundary invariants:
Deviations:
Known limitations:
```

`Known limitations` must not contain missing actual textures, crude creatures, no screenshot evidence, descriptor-only final art, or failed minimum-performance floor.

## Paste-ready Codex goal prompt

```text
Goal: Complete FVR10 for A-Life. FVR09 technically completed, but the game still looks bad. Treat this as a failed visible art-direction result, not as subjective noise. Fix the actual production screenshots.

Read AGENTS.md, docs/master_spec.md, docs/architecture_decisions.md, FVR08/FVR09 completion docs, current production voxel renderer/material/creature code, current production asset manifests, and docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_ART_DIRECTION_SCREENSHOT_OVERHAUL.md.

Audit why the game still looks bad. Specifically determine whether terrain is still flat RGBA despite texture-slot labels, whether JSON descriptors are being marked as final art without visible texture/model output, whether creatures are actual character meshes or only metadata/procedural primitives, and whether debug UI/chunk boundaries/camera/lighting are making the default view ugly. Write FVR10_VISUAL_AUDIT.md.

Implement a visible art overhaul. Terrain must visibly use real texture images, array textures, custom procedural shader output, or strong generated vertex-color/face variation. JSON fields named top_texture/side_texture are not enough unless they actually bind into rendering. Creatures must become actual cute bipedal visible meshes or generated low-poly rigs with readable head/body/legs/feet/face/expression in the production camera. Replace crude primitive defaults; keep old crude forms only as explicit debug fallback.

Improve default camera, lighting, and presentation so the default production screenshot reads as a stylized voxel ALife scene rather than debug programmer art. Hide/minimize debug clutter in product screenshots.

Preserve MinimumSettings30x30, MinSpecComfort1080p, real saves, real backend selection/fallback, no mocks, no visual authority over simulation, no alife_core renderer leaks, and external asset license safety. Update asset manifests so descriptor-only JSON is not mislabeled as final visible art.

Generate or require a real production screenshot. Add screenshot command if missing. Write FVR10_COMPLETION.md with before/after screenshot paths, screenshot reproduction command, terrain implementation, creature implementation, performance results, save/load results, and boundary invariants. Do not declare FVR10 complete without screenshot evidence or explicit Cassidy-run screenshot command plus honest incomplete visual acceptance status.

Validate with cargo fmt/check/test/clippy, check scripts, core boundary scripts, docs check, all-features check/test where possible, MinimumSettings30x30 --population 30 --record-performance, MinSpecComfort1080p --record-performance, and save validation for both profiles.

Completion requires the default production screenshot to stop looking like primary-color programmer art, terrain to visibly use real texture/shader/variation output, creatures to be visibly cute and bipedal, performance floors to hold, save/load to pass, and no deferred owned visual work.
```
