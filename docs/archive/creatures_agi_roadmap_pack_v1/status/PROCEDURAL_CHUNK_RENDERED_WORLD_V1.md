# Procedural Chunk Rendered World v1

Plan: direct goal continuation, not a new roadmap item.
Branch: `codex/procedural-chunk-rendered-world-v1`

## Objective

Move the graphical alpha away from debug rectangles and toward a large
creature-anchored procedural world. This document initially described a
painted-map primary surface, but that approach was superseded after user
feedback that the world must be generated from a seed rather than presented as
a single screen. Current default Player View uses a runtime-generated seeded
biome map with active creature chunk windows and fog of war. The painted
viewport is retained only for Full Debug/style-reference presentation.

This does not start CA45, does not alter the roadmap manifest, and does not
change simulation action authority.

## Implementation

- Player View terrain chunk materialization now records and spawns alpha-art
  backed material masks instead of suppressing chunk evidence whenever art
  handles exist.
- The earlier 1280x720 `world-painted-viewport` player surface is no longer the
  default. Player View now uses the runtime `runtime-procedural-biome-map`
  layer as its primary terrain surface.
- Terrain masks use deterministic material samples from `alife_world`, chunk
  provenance components, organic jitter, size variation, rotation, and
  alpha-art terrain/edge-blend sprites, but their opacity is capped below
  debug-card visibility.
- The rendered local slice remains bounded by active creature/camera
  materialization. The full virtual world remains larger than the visible slice
  and is not fully rendered.

## Boundary

- `alife_world` remains the Bevy-independent generator for terrain and
  procedural content.
- Bevy mirrors the active slice for presentation only.
- Generated terrain/content cannot emit actions, rewrite weights, bypass P09
  arbitration, or become save/physics/neural authority.
- CPU fallback and CPU shadow parity are unchanged.

## Focused Evidence

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell ca44a_player_view_uses_alpha_art_sprites_not_default_rectangles -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_world_art_atlas_v3_breaks_up_debug_checkerboard -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_starts_with_rendered_procedural_chunk_window -- --nocapture
```

Result: PASS at the time for the painted-surface implementation. Current
follow-up tests now prove that default Player View uses asset-backed rendering,
has no fallback rectangles, uses a seeded runtime procedural biome map as the
primary game-map layer, applies fog outside active creature chunks, retains
active chunk provenance, keeps generation independent of rendering, and keeps
the world-art layers display-only.

## Known Limitations

- The current visible window is still a bounded active slice, not a fully paged
  Minecraft-like streaming renderer with eviction, async loads, or persistent
  region aging.
- Creature movement and long-term learning across many generated chunks still
  needs deeper runtime integration.
- The art pack remains alpha-quality generated art. It is closer to a game
  world than the debug dashboard, but it is not final production art.

## Invariant Checks

- No CA45 work started.
- No release tag created.
- No S12, G25, or P37 created.
- No `alife_core` code or dependency change.
- No Bevy/wgpu/app dependency entered `alife_core`.
- CPU fallback preserved.
- CPU shadow parity preserved.
- No full action-authoritative GPU runtime claim.
- No UI/teacher/semantic/SLM/GPU/memory/topology action authority added.

## Main Status

Branch implementation in progress. Final merge and post-merge validation status
will be recorded in the completion receipt/checkpoint.

Next roadmap plan remains: CA44 external tester evidence. CA45 was not started.
