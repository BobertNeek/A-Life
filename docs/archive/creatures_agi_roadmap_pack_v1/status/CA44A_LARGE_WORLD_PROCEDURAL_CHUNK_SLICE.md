# CA44A Large World Procedural Chunk Slice

Plan: CA44A follow-up visual correction
Branch: codex/large-world-procedural-chunk-slice
Next plan: CA44

## Problem

Manual screenshots after CA44A still did not match the intended top-down game-world mockup. The committed art pack contained a target-style painted world plate, but the default Player View composition still read like a cropped single-screen board with oversized foreground sprites and debug-scale selection effects.

The user also clarified that the world must be a large procedurally generated terrain map explored through creature-local materialization, not a single visible board.

Current follow-up: the later seeded Player View correction replaces the
painted-backdrop default with a runtime-generated procedural biome map. The
painted world plate remains available only as Full Debug/style-reference
presentation.

## Fix

This branch keeps simulation semantics unchanged and fixes the presentation/scale contract:

- `alife_world` now exposes a validated `ProceduralWorldScaleReport` proving a large virtual procedural terrain domain.
- The default procedural world remains creature-anchored: no chunks exist without creature anchors, active chunks are bounded, and generated terrain/content remains non-authoritative.
- The CA37 visual map constants now describe a large virtual map instead of a 97x73 visible board.
- The original branch scaled the Player View painted world backdrop to the
  zoomed-out player camera view. This is now superseded: default Player View
  uses the runtime-generated seeded biome map, and the painted backdrop is not
  spawned in Player View.
- Foreground creatures, selection rings, food, hazards, rocks, and generated dressing props are rendered at map scale so they read like small agents/objects in a larger environment.

## Procedural World Scale

- Virtual terrain map: 4112x4112 tiles.
- Potential chunk domain: more than 60,000 chunks.
- Active chunk window: bounded by creature anchors and `max_active_chunks`.
- Materialized chunks: only near active creature/view context.
- Offscreen world: represented by deterministic procedural sampling, not pre-rendered or allocated as a giant texture.

## Boundaries

- No simulation semantics changed.
- No action authority changed.
- No CPU fallback or CPU shadow parity rule changed.
- No full action-authoritative GPU runtime claim added.
- No Bevy/wgpu/app dependency added to `alife_core`.
- No neural compression, custom sensory raycasting, planet topology, semantic/SLM authority, or ExperiencePatch transaction work added.

## Tests Added/Changed

- `procedural_world_scale_is_large_virtual_and_creature_anchored`
- strengthened `procedural_chunks_do_not_exist_without_creature_anchors`
- strengthened CA37 world-art smoke thresholds for large virtual map scale
- `production_player_view_uses_full_map_scale_backdrop_and_tiny_foreground_sprites`
- tightened Player View selection-ring size assertions
- tightened CA37 dressing prop size so default Player View does not reintroduce oversized prop overlays

## Focused Evidence

Focused commands:

```powershell
cargo test -p alife_world --test procedural_chunks -- --nocapture
cargo test -p alife_game_app --test app_shell ca37_world_art_style_smoke_validates_palette_props_and_manifest -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_uses_full_map_scale_backdrop_and_tiny_foreground_sprites -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_starts_with_rendered_procedural_chunk_window -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell ca44a_player_view_uses_alpha_art_sprites_not_default_rectangles -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_composition_layers_are_asset_backed_and_display_only -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_world_art_atlas_v3_breaks_up_debug_checkerboard -- --nocapture
```

Result: PASS for all focused tests above.

Graphical smoke:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Result: PASS. GPU selected `GpuPlastic` on local RTX 3050/DX12; Player View acceptance reported debug overlays hidden and stable labels hidden.

Forced CPU fallback smoke:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Result: PASS. Fallback was explicit as `CpuReference` / `HardwareUnavailable`.

Manual screenshot comparison used untracked local evidence:

```text
target/playtest_evidence/visual_fix/large_world_slice_actual.png
target/playtest_evidence/visual_fix/large_world_slice_full_surface_actual.png
```

The first capture showed the painted map as a small centered plate surrounded by a flat field. The second capture, after scaling the painted surface to the camera framing, shows a broad target-style top-down map: paths, groves, stones, red hazard pressure, and small map-scale creature/object sprites.

Later follow-up evidence captured the runtime-generated seeded biome map as the
default Player View surface with small map-scale sprites and fog outside active
creature chunk windows. Those screenshots remain local untracked evidence under
`target/playtest_evidence/procedural_biome_player_view/`.

## Validation Results

Full validation passed:

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

Optional Graphify update was attempted with the installed local `graphify.exe` and timed out after two minutes. Graphify remains optional project tooling and is not a Cargo/check/test prerequisite.

## Known Limitations

This is still a graphical/presentation slice. The procedural terrain map is deterministic, large, and creature-anchored, but it is not yet a fully authoritative offscreen ecology, navigation, sensory, or resource simulation substrate. Future roadmap work can make exploration deeper; this slice fixes the default player-facing scale/readability mismatch.

The current default visual surface is generated from the seed at runtime. It is
still alpha art and not final production rendering.

## Artifact Status

No screenshots, logs, target artifacts, model files, cache files, or generated captures are intended to be tracked.

## Release/Tag Status

No release tag.
