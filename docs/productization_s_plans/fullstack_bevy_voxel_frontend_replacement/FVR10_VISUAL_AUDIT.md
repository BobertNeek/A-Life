# FVR10 Visual Audit

Status: audit complete for FVR10 art-direction screenshot overhaul.

## Inputs Reviewed

- Root, docs, and crate `AGENTS.md` files after compaction.
- `docs/master_spec.md` and `docs/architecture_decisions.md`.
- FVR08 final acceptance and FVR09 completion receipts.
- Current production voxel screenshots under `target/artifacts/fvr03/`.
- `crates/alife_game_app/src/production_voxel_renderer.rs`.
- `crates/alife_game_app/src/production_assets.rs`.
- `crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json`.

The referenced FVR10 plan file is missing from the current worktree:

```text
docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_ART_DIRECTION_SCREENSHOT_OVERHAUL.md
```

The active goal text is therefore the controlling FVR10 plan for this pass.

## Screenshot Evidence

Pre-overhaul clean product screenshot path at audit time:

```text
target/artifacts/fvr03/MinSpecComfort1080p_runtime_screenshot.png
```

Pre-overhaul inspector screenshot path at audit time:

```text
target/artifacts/fvr03/MinSpecComfort1080p_runtime_screenshot_fvr05_world_inspector.png
```

Final FVR10 clean product screenshots regenerated after implementation:

```text
target/artifacts/fvr03/MinSpecComfort1080p_runtime_screenshot.png
target/artifacts/fvr03/MinimumSettings30x30_runtime_screenshot.png
```

The pre-overhaul screenshot files under `target/artifacts/` were local generated
artifacts and were overwritten by final acceptance captures. This audit records
the pre-overhaul findings; the final receipt records the post-overhaul evidence.

Visual blueprint generated for this pass:

```text
C:\Users\PC\.codex\generated_images\019f2a54-ead6-76d1-a32a-51fb7a56cc1a\ig_0b7e337a4edf39e4016a4b1a4f64548197b95b3b0e7cb15ea4.png
```

## Findings

| Area | Current Evidence | FVR10 Verdict |
|---|---|---|
| Terrain visible material output | The screenshot shows large flat slabs of green, tan, cyan, gray, purple, and red. Renderer mesh generation inserts positions, normals, and UVs only; there is no vertex color attribute and no texture/array/shader binding for the `top_texture` and `side_texture` labels. | Failed. Texture-slot labels are metadata only and do not produce visible natural terrain output. |
| Terrain chunk/grid presentation | Large flat merged quads plus visible seams make the world read as debug chunks. Chunk boundaries are disabled for MinSpecComfort1080p, so remaining seams come from flat material blocks and prism edges, not a deliberate art style. | Failed. Needs face variation, top/side separation, and less primary-color slab composition. |
| Creature visual mesh | `fvr09_cute_biped_mesh` is still assembled from cuboids through `fvr03_append_cuboid`. Eye and mouth markers exist, but the default screenshot scale makes creatures read as tiny primitive stacks rather than finished cute bipedal characters. | Failed. Needs generated rounded low-poly rig proportions and a closer/readable presentation. |
| Debug UI in product view | `Fvr05ProductionUxSettings::default_for_launch` starts with menu, settings, and overlays all visible. Record-performance screenshot sequencing explicitly enables FVR05 panels before capture. | Failed. Product screenshots need a clean capture/default view before diagnostic panels. |
| Asset manifest final-art policy | Descriptor JSON entries such as material atlas, terrain materials, water, creatures, UI, VFX config, and props are marked `final_art: true`. `ProductionAssetValidationSummary::validate` currently requires every entry to be final art. | Failed. Descriptor-only JSON must not be mislabeled as final visible art. |
| Simulation/backend boundaries | Current renderer reads stable world and creature state, keeps Bevy handles out of saves/core, and reports no renderer authority. | Preserve. FVR10 should not change this boundary. |

## Root Cause

FVR09 improved structural claims and metadata, but acceptance relied on labels and scene resources rather than pixel-level output. The actual render path still uses flat `StandardMaterial` base colors, cuboid terrain/creature mesh assembly, and default debug UI panels. The manifest also treats configuration descriptors as if they were final art assets, which hides the absence of visible texture/model output.

## Required FVR10 Fix Direction

- Add visible terrain output: generated vertex color/face variation or actual texture/shader binding. JSON `top_texture`/`side_texture` names alone do not satisfy FVR10.
- Replace the cuboid-stack creature mesh with a readable generated low-poly biped rig and set FVR10 visual/material version identifiers.
- Start production view clean and capture a clean product screenshot before enabling diagnostic FVR05 screenshots.
- Update manifest and validator policy so descriptor JSON remains generated/licensed metadata but is not counted as final visible art.
- Preserve `MinimumSettings30x30`, `MinSpecComfort1080p`, real save/load, real backend selection/fallback, no mocks, and no renderer authority over cognition/actions.

## Final FVR10 Resolution

FVR10 replaced the flat 4x4 product terrain slabs with denser 2x2 acceptance
profile terrain sampling, bound mesh vertex colors, stronger per-face
procedural color variation, and top/side color separation under
`fvr10-visible-surface-variation-v1`.

FVR10 replaced the screenshot-visible creature presentation with brighter
generated low-poly biped rigs, larger eyes/mouth, camera-facing facial markers,
closer default camera composition, and denser display-only hero dressing near
the real creature cluster. Product screenshots now start clean before diagnostic
FVR05 panels are captured.

Descriptor JSON entries in the production asset manifest are no longer counted
as final visible art. They remain generated/project-licensed metadata and are
validated for digest, license, source, and replacement policy without hiding the
need for actual renderer-bound visual output.

The final screenshots show the remaining art as intentionally procedural voxel
style rather than external hand-authored asset art. No renderer authority over
actions, cognition, rewards, weights, or save/world truth was added.
