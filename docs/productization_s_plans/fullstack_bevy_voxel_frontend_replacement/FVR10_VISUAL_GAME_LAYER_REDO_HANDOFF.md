# FVR10 Visual Game Layer Redo Handoff

Date: 2026-07-09

Status: handoff for a replacement visual pass. The current procedural creature
visual result is rejected by the user. Do not treat green renderer tests or the
current screenshots as visual acceptance.

## Copy-Paste Prompt For The Next AI

You are taking over A-Life visual game-layer work in `D:\A life`.

Goal: replace the rejected production Bevy voxel creature/terrain visuals with a
usable player-facing visual layer. The game and persistence path must remain
real: no mock sim, no fake backend, no renderer authority over actions or
cognition, and no Bevy/wgpu/renderer types in `alife_core`.

Read first:

- `AGENTS.md`
- `docs/AGENTS.md`
- `docs/master_spec.md`
- `docs/architecture_decisions.md`
- `crates/alife_game_app/AGENTS.md`
- `crates/alife_world/AGENTS.md`
- `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_VISUAL_GAME_LAYER_REDO_HANDOFF.md`

Current art direction from the user:

- The current visuals "still look like shit" and should not be polished as-is.
- Creatures should be cute, mammalian, bipedal, caveman-furry style.
- Less teletubby, less teddy bear, less dinosaur, less pastel.
- More bold contrast, visible fur/markings/accessories.
- Use all 16 selected creature archetypes, not one species with color swaps.
- Appearance must be heritable and allow mutation-driven visual changes.
- Terrain voxel blocks need visible texture/detail, not flat color tiles.

Reference concept image selected by the user:

```text
C:\Users\PC\.codex\generated_images\019f2a54-ead6-76d1-a32a-51fb7a56cc1a\ig_08002c310560b237016a4efc73d39c819a9ccd53bdfcdf7c74.png
```

Current rejected runtime screenshots:

```text
D:\A life\target\artifacts\fvr03\MinimumSettings30x30_runtime_screenshot.png
D:\A life\target\artifacts\fvr03\MinSpecComfort1080p_runtime_screenshot.png
```

Treat those screenshots as failure evidence, not as an acceptable baseline.

## Current Worktree Warning

The branch `codex/FVR10-texture-detail-polish` contains uncommitted changes from
a failed procedural visual attempt. Do not blindly commit or merge them. Inspect
the current diff and decide what to keep, replace, or revert with explicit
intent.

Likely useful current changes:

- `crates/alife_world/src/appearance.rs`: renderer-neutral
  `CreatureAppearanceGenome` for 16 heritable appearance archetypes.
- `CreatureSaveState.appearance`: saved-state appearance gene field.
- Lifecycle lineage inheritance/mutation of appearance genes.
- Production population slot assignment that covers all 16 archetypes.
- Terrain face/detail color variation in the Bevy renderer.
- Tests that enforce 16 archetypes, heritable surface detail, and renderer
  authority boundaries.

Likely rejected current changes:

- Procedural ellipsoid creature mesh in
  `crates/alife_game_app/src/production_voxel_renderer.rs`.
- Current creature faces/details/VFX composition if it still reads as
  round mascot/teletubby/ugly placeholder.

Do not assume the current procedural mesh can be salvaged. A more usable pass
may need a different rendering strategy.

## Recommended Implementation Direction

Prefer an asset-driven or authored-silhouette layer over more procedural
ellipsoid stacking.

Good options:

1. Use a small committed, source-authored voxel/pixel creature asset set.
   - 16 species silhouettes.
   - Each species has a compact front-facing or 3/4-facing mesh/sprite/billboard.
   - Palette and markings are driven by `CreatureAppearanceGenome`.
   - Assets must be small and hand-authored or generated into lightweight
     repo-friendly source data, not large generated artifacts.

2. Use Bevy sprite/atlas billboards for creatures and keep terrain voxel mesh.
   - This may be the fastest way to get readable creatures.
   - Billboards can still be real production visuals if they are driven by real
     save data and stable IDs.
   - Keep selection, overlays, VFX, and inspector data separate from the art.

3. Use simple low-poly GLB/mesh assets generated offline from compact source.
   - Commit only small optimized assets or source definitions.
   - Do not commit large Blender renders, caches, or generated dumps.
   - If Blender is used, keep the runtime independent from Blender.

Avoid:

- More top-knob/blob procedural mascot rigs.
- One universal mesh with only color/palette swaps.
- Big square overlays or state markers covering the creature art.
- Diagnostic overlays in default product screenshots.
- Renderer-owned cognition/action shortcuts.

## Architecture Boundaries That Must Not Break

Hard boundaries:

- `alife_core` must stay renderer-free.
- `alife_world` may store renderer-neutral appearance genes and stable IDs, but
  must not store Bevy entities, wgpu handles, renderer handles, materials,
  meshes, sprites, windows, or GPU resources.
- `alife_game_app` owns Bevy projection, rendering, screenshots, UX, and
  production app launch policy.
- Renderer visuals are display-only projections of saved/runtime state.
- Renderer must not authorize actions, cognition, rewards, weights, or learning.
- Production runs must use real save/load and backend selection/fallback.

Keep or strengthen these markers/tests:

- `Fvr04ProductionCreatureSceneResource.no_renderer_authority_over_actions_or_cognition`
- `Fvr04ProductionCreatureSceneResource.expression_buffer_is_read_only_projection`
- `Fvr10CreatureSpeciesMarker.heritable_appearance`
- `Fvr10CreatureSurfaceDetailMarker.no_renderer_authority_over_actions_or_cognition`
- UX default product view tests that hide debug panels/overlays.

## Files To Inspect

Core app visual path:

```text
crates/alife_game_app/src/production_voxel_renderer.rs
crates/alife_game_app/src/production_voxel_frontend.rs
crates/alife_game_app/src/creature_visuals.rs
crates/alife_game_app/src/lifecycle_lineage.rs
crates/alife_game_app/tests/fvr03_voxel_renderer.rs
crates/alife_game_app/tests/app_shell.rs
```

World/persistence path:

```text
crates/alife_world/src/appearance.rs
crates/alife_world/src/persistence.rs
crates/alife_world/src/lib.rs
crates/alife_world/tests/save_load_roundtrip.rs
crates/alife_world/tests/fvr02_persistent_voxel_backend.rs
crates/alife_world/tests/ecology_resource_cycles.rs
```

Assets/manifests:

```text
crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json
crates/alife_world/tests/fixtures/production_voxel/tiny_save.json
crates/alife_world/tests/fixtures/production_voxel/tiny_asset_manifest.json
target/artifacts/fvr03/
target/artifacts/fvr05/
target/artifacts/fvr06/
```

## Specific Visual Requirements For The Redo

Creature visual requirements:

- 16 visibly distinct archetypes from the selected concept batch.
- Bipedal stance visible at gameplay camera distance.
- Cute mammalian/caveman-furry read at 1080p without zooming in.
- No teletubby silhouette: no single rounded blob body with tiny side nubs and
  a top knob.
- Recognizable animal features: ears, muzzle/snout, paws/hands, feet, tail or
  equivalent species feature, fur/ruff/hair silhouette, and high-contrast eyes.
- Bold but not garish colors; avoid broad pastel body fills.
- Each species needs at least one non-color body-plan difference.
- State/VFX markers must not cover or replace the creature body silhouette.
- Default product screenshot must show art, not debug overlays.

Terrain visual requirements:

- Voxel block surfaces need visible texture/detail variation.
- Detail should help readability, not become noisy snow/confetti.
- Top/side material differences should remain clear.
- Stone, soil, hazard, decay, grass/resource, water/sand should be distinct.
- Large generated terrain texture artifacts must not be committed.

Heritable appearance requirements:

- Appearance genes stay renderer-neutral.
- Founders cover all 16 species for the default production population.
- Offspring inherit from parents and mutate some appearance fields.
- Save/load roundtrip preserves appearance data.
- Renderer projects genes into visuals, but does not own genetic truth.

## Suggested Technical Shape

A robust replacement can use this shape:

```text
alife_world::CreatureAppearanceGenome
    -> alife_game_app::CreatureVisualSnapshot
    -> alife_game_app visual projection table
    -> Bevy mesh/sprite/material handles
```

Where the projection table is app-only:

```rust
struct CreatureSpeciesVisualDef {
    species_archetype: u8,
    label: &'static str,
    body_plan: CreatureBodyPlanVisual,
    palette_slot: CreaturePaletteSlot,
    marking_slots: &'static [CreatureMarkingSlot],
}
```

Do not put this table in `alife_core`. If it includes Bevy handles or renderer
types, it belongs only in `alife_game_app`.

If using billboards:

- Keep billboard texture/atlas references app-local.
- Billboard orientation follows camera, but actions/cognition remain unchanged.
- Add a small shadow/contact marker to ground the creature on its tile.
- Use one asset set per archetype, then palette/marking overlays for heredity.

If using meshes:

- Prefer authored low-poly/voxel meshes per archetype over procedural ellipsoid
  part stacking.
- If procedural generation remains, generate actual species silhouettes from
  explicit body-plan definitions and test their bounding boxes/features.
- Avoid putting state/VFX panels above or on top of heads in screenshots.

## Tests To Add Or Keep

Keep these current focused tests and strengthen them as needed:

```text
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer -j 1 -- --nocapture
cargo test -p alife_game_app --test app_shell lifecycle_lineage_birth_inherits_and_mutates_appearance_genes -j 1 -- --nocapture
```

Important tests in `fvr03_voxel_renderer.rs`:

- `fvr10_creature_mesh_is_readable_low_poly_rig_not_cuboid_stack`
- `fvr10_creatures_use_all_selected_bipedal_caveman_species_not_color_swaps`
- `fvr10_creatures_have_high_contrast_heritable_surface_markings`
- `fvr10_terrain_meshes_have_bound_visible_face_variation_not_texture_labels_only`
- `fvr10_default_product_view_starts_clean_without_debug_panels_or_overlays`
- `fvr10_product_camera_and_faces_are_composed_for_readable_creatures`

Add new tests only when they prove actual failure modes. Useful checks:

- default screenshot capture hides all diagnostic overlay batches and any
  creature-attached VFX larger than a small marker.
- every visible creature species has a distinct mesh handle or atlas frame.
- every visible species has nonzero ear/muzzle/tail/paw or equivalent feature
  markers.
- visual assets are under a small size limit and are referenced through a
  production manifest.
- appearance save/load roundtrip preserves new visual genes.

## Required Runtime Screenshot Evidence

Do not claim the redo is acceptable without fresh screenshots.

Regenerate both:

```powershell
cargo run -p alife_game_app --features "bevy-app voxel-backend production-assets vfx-hanabi" --bin alife_game_app -j 1 -- production-voxel --profile MinimumSettings30x30 --resolution 1920x1080 --record-performance
cargo run -p alife_game_app --features "bevy-app voxel-backend production-assets vfx-hanabi" --bin alife_game_app -j 1 -- production-voxel --profile MinSpecComfort1080p --resolution 1920x1080 --record-performance
```

Then inspect:

```text
D:\A life\target\artifacts\fvr03\MinimumSettings30x30_runtime_screenshot.png
D:\A life\target\artifacts\fvr03\MinSpecComfort1080p_runtime_screenshot.png
```

The screenshots must show:

- no large square/flat state panels covering creatures,
- readable biped mammal/furry silhouettes,
- visible species diversity,
- bold contrast and high-contrast eyes/markings,
- terrain surface texture that is visible but not overwhelming,
- no debug dashboard by default.

## Required Validation Gate

Before committing or handing back as complete, run:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets -j 1
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer -j 1 -- --nocapture
cargo test -p alife_game_app --test app_shell lifecycle_lineage_birth_inherits_and_mutates_appearance_genes -j 1 -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
```

If a Windows Bevy-heavy compile/test command fails with
`STATUS_ACCESS_VIOLATION` and no Rust diagnostic, rerun the exact same command
once with `-j 1`. Record both results honestly.

Do not claim GPU performance if the run selected `CpuReference` fallback. That
is acceptable for visual validation if the receipt says fallback honestly.

## Git Guidance

Do not commit the rejected visual result just because tests pass.

Safe sequence for the next AI:

1. `git status --short --branch`
2. Inspect the current dirty diff.
3. Preserve only architecture-safe useful pieces.
4. Replace rejected creature visuals with the new visual layer.
5. Regenerate screenshots.
6. Run validation gate.
7. Commit only after the screenshots are visually acceptable.
8. Push/merge only after validation and user acceptance, without wiping
   unrelated work in progress.

## Completion Receipt Shape

When done, report:

- changed files,
- visual strategy used,
- screenshots inspected with exact paths,
- exact validation command results,
- whether backend was GPU or CPU fallback,
- explicit statement that renderer visuals remain display-only and did not gain
  action/cognition authority.

No completion claim is valid without screenshot evidence.
