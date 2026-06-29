# Creature Procedural World Content v1

Plan: direct post-CA44A goal continuation, not a new roadmap item.
Branch: `codex/procedural-world-content-v1`

## Objective

Extend the creature-anchored procedural chunk substrate beyond terrain
materials. The alpha world now generates deterministic food, hazard, obstacle,
and dressing-prop candidates around stable-ID creature anchors, without
requiring the camera/render surface to exist.

This does not start CA45, does not alter the roadmap manifest, and does not
change simulation action authority.

## Implementation

Added `alife_world` procedural content contracts:

- `ProceduralWorldContentKind`;
- `ProceduralWorldContentCandidate`;
- `ProceduralWorldContentReport`;
- `ProceduralCreatureContentNeighborhood`;
- `generate_procedural_world_content`;
- `sample_creature_procedural_content_neighborhood`.

The generated candidates are deterministic from seed, tile, chunk, and content
kind. They carry stable generated IDs, positions, art roles, bounded nutrition
or hazard values where applicable, and explicit boundary flags:

- generated without rendering: yes;
- rendering required: no;
- bounded for creature context: yes;
- can emit actions: no;
- can rewrite weights: no.

The Bevy Player View mirrors those candidates as asset-backed sprites using the
committed alpha art pack. Generated content has its own
`GraphicalProceduralWorldContentMarker` and is not inserted into portable saves,
not bound as Bevy-authoritative game state, and not allowed to bypass action
arbitration.

The default Player View now uses the committed
`world_backdrop_gpu_alpha.png` 1280x720 v15 painted map plate as the visible
terrain composition, while the procedural terrain/content substrate remains
available as engine-neutral generation and Bevy display metadata. This fixes
the earlier flat-green fallback view and replaces the noisy v12 backdrop with a
target-style top-down alpha map composition: green playable center, narrow dirt
paths, gray rough terrain, red hazard pressure, dense groves, rocks, food, and
small creatures.

## Assets Used

Generated procedural content uses the existing committed alpha art roles:

- food: `food_bloom.png` / food role;
- hazard: `hazard_crystal.png` / hazard role;
- obstacle: `rock_cluster.png` / rock-obstacle role;
- dressing prop: grass tuft, pebble, warning shard, leaf patch, or mushroom
  variants selected deterministically from material and label.
- backdrop: `world_backdrop_gpu_alpha.png` / world-backdrop role.

No new screenshots, generated captures, model files, caches, logs, or target
artifacts are tracked.

## Focused Evidence

Engine-neutral procedural content:

```powershell
cargo test -p alife_world --test procedural_chunks -- --nocapture
```

Result: PASS, 12 tests.

Player View generated content rendering:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell procedural_world_content_uses_alpha_art_and_no_action_authority -- --nocapture
```

Result: PASS.

Existing chunked terrain regression:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell production_world_art_atlas_v3_breaks_up_debug_checkerboard -- --nocapture
```

Result: PASS.

Painted Player View / v15 backdrop regression:

```powershell
cargo test -p alife_game_app --features bevy-app production_player_view_default_camera_is_world_establishing -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_starts_with_wide_painted_map_camera -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- app-bundle-smoke --manifest crates/alife_game_app/app_bundle_manifest.json
cargo run -p alife_game_app --bin alife_game_app -- world-art-style-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Result: PASS. The v15 alpha-art manifest and app bundle now validate, so the
painted backdrop is present in the actual graphical app instead of falling back
to the flat green ground plane, stale noisy v12 composition, or sparse/washed
v13 composition.

Graphical focused smokes:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Result: PASS. GPU path selected `GpuPlastic` in normal smoke; forced fallback
selected `CpuReference` with degraded mode explicit.

## Known Limitations

- Generated content is now an engine-neutral creature-context substrate and a
  Player View visual layer, but it is not yet the sole authoritative source for
  all saved resources, navigation, sensory ray queries, or offscreen ecology.
- Bevy mirrors generated candidates with art sprites. Bevy remains
  presentation, not authority.
- The painted backdrop is a target-style alpha map plate, not yet a streamed
  authoritative terrain renderer. The procedural chunk/content substrate remains
  the non-rendering generation layer behind the visible composition.
- Generated content IDs intentionally use a high stable-ID range to avoid
  colliding with fixture objects, but future persistence work should formalize
  generated-ID namespaces before saving long-lived worlds.

## Invariant Checks

- No CA45 work started.
- No release tag created.
- No S12, G25, or P37 created.
- No `alife_core` code or dependencies changed.
- No Bevy, wgpu, renderer, app, model-runtime, or GUI dependency entered
  `alife_core`.
- CPU fallback preserved.
- CPU shadow parity preserved.
- No full action-authoritative GPU runtime claim.
- No UI/teacher/semantic/SLM/GPU/memory/topology action authority added.
- No neural compression, custom sensory raycasting, planet topology, or
  ExperiencePatch transaction work.

## Main Status

Branch implementation in progress. Final merge and post-merge validation status
will be recorded in the completion receipt/checkpoint.

Next roadmap plan remains: CA44 external tester evidence. CA45 was not started.
