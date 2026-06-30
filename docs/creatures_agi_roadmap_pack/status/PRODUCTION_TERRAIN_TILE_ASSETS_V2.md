# Production Terrain Tile Assets V2

Plan context: direct post-CA44A visual correction, not roadmap continuation.

Branch: `codex/production-procedural-world-visual-fix`

## Objective

Replace the weak terrain-art failure mode where alpha-masked blobs and flat
swatches could pass manifest validation. The default Player View should be
built from committed, opaque, role-specific PNG terrain tiles and recognizable
sprite silhouettes rather than debug rectangles, transparent stains, or
single-color placeholders.

## Blueprint

A new image-generation visual blueprint and follow-up source sheets were
created for this pass. The rejected v32 pass used generated art but allowed
large baked terrain shapes to become visible as repeated blobs in the runtime
compositor. The active v33 pass uses a new terrain-only ground sheet plus a new
chroma-keyed sprite sheet. The ground sheet contains grass, soil paths, resource
grove ground, red hazard pressure ground, stone, water, and sand with no baked
creatures, crystals, trees, UI, labels, or landmarks. The sprite sheet contains
separate creature, food, hazard, rock, selection, and prop silhouettes. The
blueprint/source sheets remain local evidence under
`.codex/generated_images/`; only the sliced, versioned product PNGs are
committed.

## Assets Changed

Regenerated `crates/alife_game_app/assets/alpha_art_v1/` with art direction:

`production-alpha-imagegen-ground-tiles-v33`

The v33 pack supersedes the rejected v25/v30/v31 visual attempts and the
insufficient v32 stamped-blob runtime result. It keeps required CA44A roles and
specifically corrects:

- `terrain_safe_grass.png`
- `terrain_soil_path.png`
- `terrain_resource_grove.png`
- `terrain_hazard_pressure.png`
- `terrain_stone_rough.png`
- `terrain_water.png`
- `terrain_sand.png`
- creature/food/hazard/rock/prop sprites sliced from the new v33 chroma-keyed
  generated sprite sheet with transparent backgrounds and compact readable
  silhouettes.
- entity and terrain crops are imported through
  `scripts/import_alpha_art_imagegen_atlas.py` so the source sheets can be
  re-sliced repeatably instead of manually copied. The importer supports
  terrain-only replacement for future ground-tile refreshes.

Terrain PNGs are now full-cell opaque material tiles intended for sampled
procedural biome-map construction. They are not transparent daubs and are not
fallback rectangles.

## Manifest And Quality Gate

`alpha_art_manifest.json` now records the v33 art direction and updated file
sizes. The validator now decodes PNG pixels and rejects:

- flat terrain swatches;
- transparent terrain blobs;
- malformed PNGs;
- dimension or manifest/file-size mismatches;
- oversized non-backdrop sprites;
- opaque square entity sprites;
- forbidden artifact paths.

The quality gate requires role-appropriate pixel coverage, visible color/luma
variation, edge opacity for terrain/backdrops, and bounded non-square
silhouettes for creature/food/hazard/rock/prop roles.

## Rendering Boundary

Default Player View continues to use asset-backed sprites and the runtime
seeded procedural biome map. The final compositor samples the v33 terrain PNGs
as fine material texture over a continuous seeded biome field instead of
stretching them into giant repeated stamps. Biome regions and trails provide the
large-scale map composition; tiles provide local ground texture. Fog-of-war
reveal is radial around active creature chunks rather than hard square chunk
masks. Foreground creatures, food, hazards, rocks, props, and the selection ring
are map-scale sprites instead of close-up stickers or tiny debug-map specks.
After all-features validation caught a regression in the default establishing
camera contract, the Player View opening zoom was restored to the wider
world-establishing value so the default view reads as a generated terrain field
rather than a close-up debug crop.
Rectangle fallback remains available only as a degraded diagnostics path when
alpha-art handles are unavailable or in diagnostic overlays.

This pass does not change simulation semantics, action authority, GPU/CPU
correctness, CPU fallback, CPU shadow parity, semantic/SLM authority, or
`alife_core`.

## Focused Evidence

Commands run:

```powershell
cargo test -p alife_game_app alpha_art_inner_validator -- --nocapture
cargo test -p alife_game_app --test app_shell ca44a_committed_alpha_art_manifest_validates_required_roles_and_pngs -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_uses_runtime_map_and_readable_foreground_sprites -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_starts_with_rendered_procedural_chunk_window -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_world_art_atlas_v3_breaks_up_debug_checkerboard -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE='0'; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Results:

- Alpha-art validator: PASS, including flat terrain and square sprite rejection.
- Manifest role/PNG validation: PASS.
- Runtime-map/readable foreground sprite test: PASS.
- Rendered procedural chunk-window test: PASS.
- World-art checkerboard rejection test: PASS.
- Default GPU graphical smoke: PASS; GPU selected `GpuPlastic`, fallback `None`,
  Player View acceptance true.
- Forced fallback graphical smoke: PASS; selected `CpuReference`, fallback
  `HardwareUnavailable`, degraded fallback visible, Player View acceptance true.

Local untracked visual evidence includes:

```text
target/playtest_evidence/visual_fix/alpha_art_v32_contact_sheet.png
target/playtest_evidence/visual_fix/player_view_v32_actual_app_window.png
target/playtest_evidence/visual_fix/alpha_art_v33_ground_contact_sheet.png
target/playtest_evidence/visual_fix/alpha_art_v33_full_contact_sheet.png
target/playtest_evidence/visual_fix/player_view_v31_actual_app_window.png
target/playtest_evidence/visual_fix/player_view_v31_zoom_texture_settled_app_window.png
```

The v33 contact sheets confirm that the committed terrain PNGs are ground-only
material tiles and the committed sprite PNGs are separate recognizable
silhouettes. The previous v32 app-window capture is retained only as evidence of
the rejected stamped-blob failure mode. Automated app-window capture for the
v33 smoke could not reacquire a targetable Bevy window from the shell-launched
process, so final visual evidence combines the v33 contact sheets,
deterministic render tests, and passing direct default/fallback graphical
smokes. The capture miss was local screenshot orchestration only; the graphical
smokes themselves passed.

## Known Limitations

- This is still a committed 2D PNG tile/sprite pack, not a Blender-authored GLB
  or full material pipeline.
- The asset contact sheet is untracked under `target/playtest_evidence/` and is
  evidence only.
- The generated image blueprint/source sheets are not committed.
- Runtime terrain generation is seeded and viewport-driven, but terrain remains
  presentation/context-only; it is not yet an authoritative infinite-world
  sensory/navigation substrate.
- Full production-level game art still needs a dedicated art pipeline and likely
  larger authored source assets.

## Invariant Checks

- No `alife_core` dependency changes.
- No Bevy/wgpu/model-runtime dependencies moved into `alife_core`.
- No S12/G25/P37 created.
- No release tag created.
- No screenshots, logs, target artifacts, model files, caches, or generated
  scratch assets are tracked.
- CPU fallback preserved.
- CPU shadow parity preserved.
- No full action-authoritative GPU runtime claim.
- No action authority changes.

## Next Step

Remain stopped before CA45 unless explicitly authorized. If continuing visual
quality work, the next bounded slice should be a real production art pipeline
decision: Blender-authored sprite/GLB sources, a better AI-generated atlas
workflow, or a tile-set authoring tool with preview comparison.
