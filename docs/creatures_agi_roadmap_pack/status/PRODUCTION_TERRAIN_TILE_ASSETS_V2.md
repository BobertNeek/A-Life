# Production Terrain Tile Assets V2

Plan context: direct post-CA44A visual correction, not roadmap continuation.

Branch: `codex/production-procedural-world-visual-fix`

## Objective

Replace the weak terrain-art failure mode where alpha-masked blobs and flat
swatches could pass manifest validation. The default Player View should be
built from committed, opaque, role-specific PNG terrain tiles and recognizable
sprite silhouettes rather than debug rectangles, transparent stains, or
single-color placeholders.

Superseded status: after further user review, this PNG tile/sprite lane is no
longer the active default Player View art direction. It remains versioned for
HUD/debug/fallback validation and as historical evidence. The active default
Player View target is now the True 2.5D glTF/orthographic presentation documented
in `CA44A_REAL_ART_ASSETS_AND_TICK_STABILITY.md`.

## Blueprint

A new image-generation visual blueprint and follow-up source sheets were
created for this pass. The rejected v32 pass used generated art but allowed
large baked terrain shapes to become visible as repeated blobs in the runtime
compositor. The v33 pass still read too noisy and sticker-like in the live
Player View, and the v40 pass was still too dark/noisy in the final window. The
active v41 pass uses one polished generated tile/sprite sheet. The ground row
contains clean, readable
grass, soil paths, resource grove ground, red hazard pressure ground, stone,
water, and sand with no baked creatures, UI, labels, or landmarks. The sprite
sheet contains separate creature, food, hazard, rock, selection, and prop
silhouettes. The blueprint/source sheets remain local evidence under
`.codex/generated_images/`; only the sliced, versioned product PNGs are
committed.

## Assets Changed

Regenerated `crates/alife_game_app/assets/alpha_art_v1/` with art direction:

`production-alpha-imagegen-ground-tiles-v41`

The v41 pack supersedes the rejected v25/v30/v31 visual attempts, the
insufficient v32 stamped-blob runtime result, the still-unsatisfactory v33
screen, the v34 foreground-readability miss, and the v35 noisy/blob-like
terrain result. It also supersedes the insufficient v36-v39 refinement line
and the too-dark/noisy v40 refinement with cleaner hand-painted terrain tiles
and stronger object silhouettes. It
keeps required CA44A roles and specifically corrects:

- `terrain_safe_grass.png`
- `terrain_soil_path.png`
- `terrain_resource_grove.png`
- `terrain_hazard_pressure.png`
- `terrain_stone_rough.png`
- `terrain_water.png`
- `terrain_sand.png`
- creature/food/hazard/rock/prop sprites sliced from the new v41 generated
  sheet with transparent backgrounds and compact readable silhouettes.
- entity and terrain crops are imported through
  `scripts/import_alpha_art_imagegen_atlas.py` so the source sheets can be
  re-sliced repeatably instead of manually copied. The importer supports
  terrain-only replacement for future ground-tile refreshes.

Terrain PNGs are now full-cell opaque material tiles intended for sampled
procedural biome-map construction. They are not transparent daubs and are not
fallback rectangles.

## Manifest And Quality Gate

`alpha_art_manifest.json` now records the v41 art direction and updated file
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

Default Player View no longer uses this PNG tile/sprite compositor as its
primary world-art path. The active branch uses committed low-poly `.glb` assets
from `crates/alife_game_app/assets/true_25d_alpha_v1/`, a locked orthographic
3D camera, and a procedural micro-ecology ledger. The v41 PNGs remain available
for HUD/debug/fallback diagnostics and packaging validation. Rectangle fallback
remains available only as a degraded diagnostics path when art handles are
unavailable or in diagnostic overlays.

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
cargo run -p alife_game_app --bin alife_game_app -- graphical-controls-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --bin alife_game_app -- gpu-alpha-stability-smoke crates/alife_world/tests/fixtures/gpu_alpha 600
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE='0'; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Results:

- Alpha-art validator: PASS, including flat terrain and square sprite rejection.
- Manifest role/PNG validation: PASS.
- Runtime-map/readable foreground sprite test: PASS.
- Rendered procedural chunk-window test: PASS.
- World-art checkerboard rejection test: PASS.
- Graphical controls smoke: PASS.
- 600-tick `gpu_alpha` stability smoke: PASS, `completed=600`,
  `first_invalid_tick=None`, `sealed=600`, `parity=true`.
- Default GPU graphical smoke: PASS; GPU selected `GpuPlastic`, fallback `None`,
  Player View acceptance true.
- Forced fallback graphical smoke: PASS; selected `CpuReference`, fallback
  `HardwareUnavailable`, degraded fallback visible, Player View acceptance true.

The current Windows smoke launcher also starts a smoke-only watchdog. It waits
for the exact A-Life smoke window title and sends a normal window close after
the requested smoke duration plus slack if the in-app Bevy timer has not already
returned the runner. Persistent launches are unchanged.

Local untracked visual evidence includes:

```text
target/playtest_evidence/visual_fix/alpha_art_v32_contact_sheet.png
target/playtest_evidence/visual_fix/player_view_v32_actual_app_window.png
target/playtest_evidence/visual_fix/alpha_art_v33_ground_contact_sheet.png
target/playtest_evidence/visual_fix/alpha_art_v33_full_contact_sheet.png
target/playtest_evidence/visual_fix/alpha_art_v34_contact_sheet.png
target/playtest_evidence/visual_fix/alpha_art_v40_contact_sheet.png
target/playtest_evidence/visual_fix/player_view_v40_render_window.png
target/playtest_evidence/visual_fix/alpha_art_v41_contact_sheet.png
target/playtest_evidence/visual_fix/player_view_v31_actual_app_window.png
target/playtest_evidence/visual_fix/player_view_v31_zoom_texture_settled_app_window.png
```

The v41 contact sheet confirms that the committed terrain PNGs are clean
ground-only material tiles and the committed sprite PNGs are separate
recognizable silhouettes. The previous v32/v33/v34/v35 captures are retained
only as evidence of the rejected stamped-blob/noisy-sticker, foreground-scale,
blob-terrain, and dark/noisy v40 failure modes.

## Known Limitations

- This is a superseded committed 2D PNG tile/sprite pack, not the current
  default Player View target.
- The active target is now True 2.5D low-poly glTF presentation with shader
  contracts for toon bands, Sobel outlines, and pixel-step filtering.
- The asset contact sheet is untracked under `target/playtest_evidence/` and is
  evidence only.
- The generated image blueprint/source sheets are not committed.
- Runtime terrain generation is seeded and viewport-driven, but terrain remains
  presentation/context-only; it is not yet an authoritative infinite-world
  sensory/navigation substrate.
- App-window captures are local/untracked evidence only. The validated pass
  uses focused Bevy tests and graphical smoke; Windows capture tooling can still
  target the wrong foreground window unless the render-window handle is selected
  explicitly.
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
