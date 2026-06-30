# Creature-Anchored Procedural World Streaming

Plan context: CA44A follow-up toward production-scale world presentation
Branch: codex/creature-anchored-procedural-world-streaming
Next plan: CA44

## Objective

The graphical alpha world must not behave like a fixed single-screen test map.
The player camera can show a local view, but generated terrain/content should
follow live stable-ID creature positions from the authoritative headless world.
Areas without creature presence may remain fogged or unmaterialized for render
efficiency.

## Implementation Summary

- `HeadlessWorld` now exposes stable-ID `WorldObject` snapshots without Bevy,
  wgpu, renderer, or engine-local IDs.
- `LiveBrainLoop` forwards those snapshots to the app bridge.
- The Bevy Player View syncs visible stable-ID markers, badges, selection
  state, and camera follow state from live headless object positions before
  recomputing procedural chunk anchors.
- The runtime procedural biome map records an active chunk signature,
  creature-anchor count, materialized tile count, and refresh count.
- The runtime biome map texture is regenerated when the active
  creature-anchored chunk window changes.
- The default Player View uses generated v21 terrain/object PNG assets as the
  runtime biome source: grass, dirt path, grove, hazard pressure, stone, water,
  sand, creatures, food, hazard crystals, rocks, and props. Region/trail/dressing
  paint is presentation-only and is kept below debug-dashboard opacity.
- A no-window runtime preview builder exercises the same live tick -> terrain
  field -> biome map core systems without initializing a desktop window in
  tests.

## Focused Evidence

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell player_view_streaming_keeps_live_creature_anchors_synced -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell runtime_biome_map_refreshes_when_active_chunk_window_changes -- --nocapture
```

Results:

- Stable-ID creature marker motion is mirrored from the live headless world into
  Player View.
- The procedural terrain field remains generated without rendering and remains
  materialized only near active creature anchors.
- Runtime biome map refresh metadata changes when the active chunk window
  changes.
- Fog-of-war remains active outside the creature-anchored window.
- Local untracked Alt+PrintScreen evidence confirmed the actual Bevy window uses
  the generated v21 assets in Player View:
  `target/playtest_evidence/terrain_tiles/generated_v21_preview/player_view_v21_clean_dressing_capture.png`.

## Invariant Checks

- `alife_core` was not changed.
- No Bevy, wgpu, renderer, or model-runtime dependency was added to
  `alife_core`.
- Stable IDs remain the bridge between world state and player-facing
  presentation.
- Procedural terrain/content remains display/context evidence only.
- Procedural terrain/content cannot emit actions or rewrite weights.
- P09 action arbitration remains the only action path.
- CPU fallback and CPU shadow parity are unchanged.
- No full action-authoritative GPU runtime claim was added.
- No S12, G25, or P37 was created.
- No release tag was created.
- No screenshots, logs, target artifacts, model files, or caches are intended
  for tracking.

## Known Limitations

- This closes the live-anchor streaming gap, not the full production art goal.
- Fog of war is still presentation-side; it is not an authoritative sensory
  visibility system.
- Offscreen ecology persistence and richer paging remain future CA work.
- The Blender/GLB or sprite-sheet asset pipeline requested for final-quality
  art remains future work.
