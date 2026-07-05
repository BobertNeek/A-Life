# FVR04 Completion Receipt - Creature Rendering And Interaction

Status: complete
Branch: `codex/fvr04-creature-rendering-interaction`
Primary checkout: `D:\A life`

## Scope

FVR04 adds finished production creature rendering and interaction on top of the
FVR03 voxel world renderer. The production voxel app now shows real creatures
from the prepared P34/FVR02 save path, with stable selection, hover, camera
follow hooks, selected-creature panel text, expression colors, profile-based
LOD/detail density, and simple production animation.

The renderer remains a projection of world state. It does not own action
authority, cognition, world legality, save truth, or neural state.

## Files Changed

- `Cargo.lock`
- `crates/alife_game_app/Cargo.toml`
- `crates/alife_game_app/src/bevy_shell.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/src/creature_visuals.rs`
- `crates/alife_game_app/src/production_voxel_frontend.rs`
- `crates/alife_game_app/src/production_voxel_renderer.rs`
- `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR04_COMPLETION_RECEIPT.md`

## Production Population Path

FVR04 uses `production_voxel_save_with_population` to prepare a real portable
save for the requested production population before FVR02 voxel migration. This
path trims or materializes real save/world/creature records for the requested
population and then validates/remigrates the voxel backend. It is not a
renderer-side fake population generator and it does not write renderer handles
into the save file.

Validated population tiers:

| Tier | Profile | Result |
|---:|---|---|
| 1 | `MinimumSettings30x30` | release graphical smoke passed |
| 10 | `MinimumSettings30x30` | release graphical smoke passed |
| 30 | `MinimumSettings30x30` | release graphical smoke passed |
| 50 | `Balanced1080p` | release graphical smoke passed |
| 100 | `Balanced1080p` | release graphical smoke passed |
| 250 | `HighSpecScaleUp` | release graphical smoke passed |
| 500 | `HighSpecScaleUp` | release graphical smoke passed |

## Renderer Backend

FVR04 runtime backend id:

`bevy_voxel_world+fvr03_chunk_mesh+fvr04_creature_interaction`

The creature renderer uses shared Bevy meshes plus bounded material buckets
keyed by expression, animation state, and LOD. Expression state is projected
into `Fvr04ProductionCreatureSceneResource.expression_buffer`; material churn is
bounded by bucket count rather than per-creature material creation. Cue meshes
and chunk-boundary overlays are profile gated so minimum and scale-up tiers do
not pay for unnecessary visual detail.

Production benchmark runs use Vulkan by default on Windows after DX12 produced
a blank screenshot on this host. Normal interactive launches keep vsync;
`--record-performance` uses immediate present and Bevy wgpu 27 instance flags
without debug validation for cleaner local performance evidence.

## Expression And Interaction

The renderer maps creature/world state into:

- hunger
- fatigue
- fear/cortisol
- dopamine/valence
- reproductive drive
- sleep pressure
- social signal
- animation state
- expression state

Implemented interaction hooks:

- stable creature hover and click selection
- selected creature highlight marker
- selected creature panel
- world label for hovered/selected creature
- camera follow toggle with `F`
- orthographic/orbit camera mode keys retained from FVR03

## Saved-State And Schema Changes

No committed save-file schema changed in FVR04. The production save preparation
path expands/trims the in-memory P34 save and remigrates the existing
`alife.fvr02.persistent_voxel_world.v1` backend.

Added renderer diagnostic fields:

- `creature_render_count`
- `creature_material_bucket_count`
- `creature_lod`

Added renderer-side schema:

`alife.fvr04.production_creature_renderer.v1`

Roundtrip evidence:

- `fvr04_save_roundtrip_preserves_selected_creature_and_visible_signature`
  proves the selected creature anchor and visible creature-anchor signature
  survive JSON save/load roundtrip.
- The same test checks the serialized save JSON does not contain `Entity(`,
  `bevy`, `wgpu`, or `renderer`.

## Profile Results

Observed local production release evidence:

| Profile | Population | Chunks | Tiles | Creature LOD | Creatures | Material Buckets | Measured FPS | Target FPS |
|---|---:|---:|---:|---|---:|---:|---:|---:|
| `MinimumSettings30x30` | 30 | 36 | 576 | `compact-voxel` | 30 | 8 | 264.41 | 30 |
| `MinSpecComfort1080p` | 30 | 100 | 100 | `compact-voxel` | 30 | 8 | 209.34 | 60 |

Artifacts:

- `target/artifacts/fvr03/MinimumSettings30x30_renderer_diagnostics.json`
- `target/artifacts/fvr03/MinimumSettings30x30_runtime_screenshot.png`
- `target/artifacts/fvr03/MinSpecComfort1080p_renderer_diagnostics.json`
- `target/artifacts/fvr03/MinSpecComfort1080p_runtime_screenshot.png`
- `target/artifacts/fvr04/fvr04_creature_rendering_blueprint.png`

Artifacts are under `target/` and are not committed.

## Boundary Invariants

- No Bevy or wgpu types were added to `alife_core`.
- No renderer authority was added over actions or cognition.
- Renderer selection uses stable world IDs and tile/chunk refs, not saved Bevy
  entities.
- The production path uses real P34/FVR02 saves and real launch/profile knobs.
- No mock sim, fake backend, alpha naming for new production work, or large
  generated artifacts were introduced.

## Validation Receipt

Passing commands already run in this goal:

- `cargo check -p alife_game_app --features production-voxel-frontend --all-targets`
- `cargo test -p alife_game_app --lib fvr04 -- --nocapture`
- `cargo build --release -p alife_game_app --features production-voxel-frontend`
- `target\release\alife_game_app.exe production-voxel --profile MinimumSettings30x30 --population 30 --record-performance --smoke-seconds 30`
- `target\release\alife_game_app.exe production-voxel --profile MinSpecComfort1080p --population 30 --record-performance --smoke-seconds 30`
- release graphical smokes for populations `1`, `10`, `30`, `50`, `100`, `250`, and `500`

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`
- `C:\Users\PC\.local\bin\graphify.exe update .`

## Known Limitations

FVR04 uses Bevy shared meshes and bounded material buckets rather than a custom
single draw-call instance buffer. The scale-up profiles avoid per-creature
material churn and gate optional cue meshes, but a custom instance buffer is
still a valid future optimization if later profiles need more headroom.

## FVR05 Readiness

FVR05 can start without more creature-rendering planning after this branch is
merged.
