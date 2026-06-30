# Production Terrain Tile Assets V2

Plan context: direct post-CA44A visual correction, not roadmap continuation.

Branch: `codex/production-terrain-tile-assets-v2`

## Objective

Replace the weak terrain-art failure mode where alpha-masked blobs and flat
swatches could pass manifest validation. The default Player View should be
built from committed, opaque, role-specific PNG terrain tiles and recognizable
sprite silhouettes rather than debug rectangles, transparent stains, or
single-color placeholders.

## Blueprint

A new image-generation visual blueprint was created for this pass: a wide
top-down 2.5D A-Life GPU alpha world with organic grass, soil paths, stone,
water, sand, resource grove, red hazard biome, small props, distinct creatures,
and minimal corner HUD. It is used as a qualitative target only and is not
committed as a product asset.

## Assets Changed

Regenerated `crates/alife_game_app/assets/alpha_art_v1/` with art direction:

`production-alpha-generated-world-atlas-v22-opaque-ground-tiles`

The v22 pack keeps required CA44A roles and specifically corrects:

- `terrain_safe_grass.png`
- `terrain_soil_path.png`
- `terrain_resource_grove.png`
- `terrain_hazard_pressure.png`
- `terrain_stone_rough.png`
- `terrain_water.png`
- `terrain_sand.png`
- creature/food/hazard/rock/prop sprites regenerated with compact silhouettes

Terrain PNGs are now full-cell opaque material tiles intended for sampled
procedural biome-map construction. They are not transparent daubs and are not
fallback rectangles.

## Manifest And Quality Gate

`alpha_art_manifest.json` now records the v22 art direction and updated file
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
seeded procedural biome map. Rectangle fallback remains available only as a
degraded diagnostics path when alpha-art handles are unavailable or in
diagnostic overlays.

This pass does not change simulation semantics, action authority, GPU/CPU
correctness, CPU fallback, CPU shadow parity, semantic/SLM authority, or
`alife_core`.

## Focused Evidence

Commands run:

```powershell
python scripts/generate_alpha_art_v1.py
cargo test -p alife_game_app alpha_art_inner_validator -- --nocapture
cargo test -p alife_game_app --test app_shell ca44a_committed_alpha_art_manifest_validates_required_roles_and_pngs -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell ca44a_player_view_uses_alpha_art_sprites_not_default_rectangles -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_uses_runtime_map_and_tiny_foreground_sprites -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE='0'; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Results:

- Alpha-art validator: PASS, including flat terrain and square sprite rejection.
- Manifest role/PNG validation: PASS.
- Player View asset-backed rendering: PASS.
- Runtime-map/tiny foreground sprite test: PASS.
- Default GPU graphical smoke: PASS; GPU selected `GpuPlastic`, fallback `None`,
  Player View acceptance true.
- Forced fallback graphical smoke: PASS; selected `CpuReference`, fallback
  `HardwareUnavailable`, degraded fallback visible, Player View acceptance true.

The first attempted graphical smoke was incorrectly run in parallel with forced
fallback and timed out on Cargo/Bevy window contention. Leftover `cargo`/`rustc`
validation processes were stopped, and both smokes passed when rerun
sequentially.

## Known Limitations

- This is still a committed 2D PNG tile/sprite pack, not a Blender-authored GLB
  or full material pipeline.
- The asset contact sheet is untracked under `target/playtest_evidence/` and is
  evidence only.
- The generated image blueprint is not committed.
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
