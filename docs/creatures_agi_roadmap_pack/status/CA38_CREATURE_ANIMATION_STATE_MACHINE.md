# CA38 Creature Animation State Machine

## Plan

CA38 - Creature animation state machine.

## Branch

`codex/CA38-creature-animation-state-machine`

## Scope Note

Before CA38 completion, user review found that the GPU alpha world still read
as a single-screen debug board rather than a procedurally generated world that
A-Life creatures can explore. This branch therefore includes a bounded
large-world correction to the CA37 terrain/art surface before recording the CA38
animation result.

This does not create a new roadmap plan. It corrects the active graphical alpha
surface while preserving manifest order.

## Files Changed

- `crates/alife_game_app/src/bevy_shell.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/src/creature_animation_style.rs`
- `crates/alife_game_app/src/lib.rs`
- `crates/alife_game_app/src/schema.rs`
- `crates/alife_game_app/src/world_art_style.rs`
- `crates/alife_game_app/tests/app_shell.rs`
- `crates/alife_world/tests/fixtures/gpu_alpha/tiny_save.json`
- `docs/creatures_agi_roadmap_pack/status/CA37_TERRAIN_PROPS_WORLD_ART_STYLE.md`
- `docs/creatures_agi_roadmap_pack/status/CA38_CREATURE_ANIMATION_STATE_MACHINE.md`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`

## Runtime Code Changed

Yes. CA38 adds a headless-testable creature animation/pose contract and Bevy
presentation components for readable creature pose changes.

The branch also promotes the CA37 GPU alpha terrain surface from a one-screen
visual board to a seeded large-world presentation:

- generated terrain map grows from `41x31` to `97x73` tiles;
- world-art smoke now proves `large_world_exploration=true`;
- the GPU alpha fixture has distributed stable-ID food, hazards, obstacles, and
  four ecology zones across the map;
- camera panning remains available for traversing the larger surface;
- terrain still does not become physics, navigation, sensory, cognition, or
  action authority.

## Core APIs Changed

No. `alife_core` was not changed.

## Docs Changed

Yes. CA37 status is updated to remove the stale single-screen limitation and
this CA38 status document records the correction.

## Public APIs Changed

Yes, within `alife_game_app` only:

- `run_creature_animation_state_machine_smoke`
- `ca38_creature_animation_summary`
- `ca38_creature_pose_for_state`
- `bevy_shell::GraphicalCreatureAnimationResource`
- `bevy_shell::GraphicalCreatureAnimationPose`

The CA37 world-art summary also reports seed, span, large-world exploration,
distributed stable objects, and camera-pan capability.

## Tests Added/Changed

- Added CA38 animation mapping tests for idle, move, eat, flee, sleep, pain,
  social, and inspector/fallback poses.
- Added Bevy-feature test coverage proving animation pose components are
  display-only, stable-ID based, and do not alter cognition.
- Updated CA37 tests to reject the old `true exploration worldgen=false`
  condition and require a larger seeded terrain map with distributed stable
  world objects.

## Focused Evidence

```powershell
cargo run -p alife_game_app --bin alife_game_app -- world-art-style-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo test -p alife_game_app --test app_shell ca37 -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell ca37 -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell ca38 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- creature-animation-state-smoke
```

Observed large-world smoke summary:

```text
tiles=7081 span_world_units=62.0 large_world_exploration=true distributed_objects=true zones=4 resource_materials=4 hazard_materials=3
```

## Validation Results

Focused validation is passing. Full branch validation and post-merge validation
are recorded in the final CA38 receipt.

## Invariant Checks

- `alife_core` unchanged and remains engine-independent.
- Bevy visual terrain and creature animation are not authoritative.
- Terrain does not emit actions or bypass world/core arbitration.
- Creature animation does not emit actions or mutate cognition.
- Stable IDs remain the player-facing and portable identity surface.
- No Bevy entity IDs are used in player-facing CA38 text.
- CPU fallback remains available.
- CPU shadow parity remains the gate.
- Product runtime claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- No full action-authoritative GPU runtime claim was added.

## Known Limitations

- The generated terrain is a deterministic bounded alpha map, not final art
  production.
- Terrain now gives the alpha a larger seeded exploration surface, but it still
  does not add new navigation, physics, sensory backend, or topology semantics.
- Only the current stable-ID objects and existing world actions participate in
  simulation.
- Creature animation is presentation-only and intentionally simple.

## Artifacts Tracked

No screenshots, logs, target artifacts, model files, caches, or generated media
are tracked.

## Release/Tag Status

No release tag was created.

## alife_core Dependency Status

`alife_core` remains dependency-clean; this branch does not add Bevy, wgpu,
renderer, windowing, model-runtime, or app dependencies to core.

## Main Status

Pending merge and post-merge validation.

## Next Plan

CA39 - Drive-coupled audio and VFX.
