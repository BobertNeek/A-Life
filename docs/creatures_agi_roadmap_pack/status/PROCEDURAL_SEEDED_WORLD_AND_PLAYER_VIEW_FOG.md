# Procedural Seeded World And Player View Fog

Plan context: CA44A follow-up visual/product correction
Branch: codex/procedural-biome-primary-player-view
Status: implemented on branch
Next plan: CA44

## User Correction

The default graphical alpha must not be a single baked screen. The player-facing
world should be generated from a seed, generate additional world around
creature travel, and may use fog of war where no creatures are present.

## Implementation Summary

- Default Player View now uses a runtime-generated `runtime-procedural-biome-map`
  as the primary terrain surface.
- `world-painted-viewport` is no longer spawned in default Player View. It
  remains available only as Full Debug/style-reference presentation.
- The runtime map is generated from the scenario seed and the active
  creature-anchored procedural chunk field.
- The app records the seed, texture dimensions, active chunk count, and
  fogged-pixel count on `GraphicalRuntimeProceduralBiomeMap`.
- Fog of war darkens generated terrain outside active creature chunk windows.
- Foreground sprites were reduced back to map scale so creatures, food,
  hazards, rocks, selection rings, and generated content no longer cover the
  world surface.
- The generated surface now includes seed-derived biome blending for safe
  grass, soil/path, resource grove, hazard pressure, and stone/rough terrain.
- `alife_world` now has a regression test proving that moving a creature to a
  distant tile activates a different deterministic chunk window from the same
  seed instead of reusing one fixed screen.

## Focused Evidence

```powershell
cargo test -p alife_world --test procedural_chunks -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_starts_with_rendered_procedural_chunk_window -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_world_art_atlas_v3_breaks_up_debug_checkerboard -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_composition_layers_are_asset_backed_and_display_only -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_uses_runtime_map_and_tiny_foreground_sprites -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Results: focused tests passed. The 30-second graphical smoke selected GPU mode
and exited cleanly. Local screenshot evidence was captured under
`target/playtest_evidence/procedural_biome_player_view/` and remains untracked.

## Invariant Checks

- No CA45 work started.
- No S12, G25, or P37 created.
- No release tag created.
- No screenshots, logs, target artifacts, model files, or caches are intended
  for tracking.
- `alife_core` was not changed and remains free of Bevy/wgpu/app dependencies.
- CPU fallback and CPU shadow parity remain unchanged.
- No full action-authoritative GPU runtime claim was added.
- Procedural terrain/content remains presentation/context evidence only and
  does not emit actions, rewrite weights, or bypass P09 action arbitration.

## Known Limitations

- This is still alpha procedural art, not final production art.
- Runtime generation is visually much closer to a map than the prior rectangle
  view, but it is not yet a polished Blender/asset-pipeline world.
- Fog of war is currently a display-layer effect tied to active chunk windows;
  it is not an authoritative sensory visibility system.
- Chunk generation follows creature anchors through `alife_world`; richer
  persistent offscreen ecology and paging polish remain future CA work.

