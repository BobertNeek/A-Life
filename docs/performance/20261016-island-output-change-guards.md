# Island output-change guards

## Scope and baseline

Baseline: `624bb5b355aa93bfd2c986351e556810b5b1dd25`.
The baseline island renderer blob is `a6a6eeeeaeb816bee2737b1bf7f760bae48180a6`.
The shipping placements file contains 3,716 props.

This change keeps the existing 0.2-second update throttle, asset readiness and
deferred spawning, terrain occupancy checks, visibility thresholds, and terrain
and prop LOD policies. It changes when equal output values are written.

## Changes

- Set prop `Visibility` with Bevy's `set_if_neq` helper
- Assign `DirectionalLight.shadows_enabled` only when its value differs
- Keep a bounded, single-entry system-local cache of the desired shadow config,
  keyed by cascade count and exact near/far distances. These are all variable
  builder inputs; first-cascade distance and overlap retain the existing formula
  and constant
- Use Bevy's original `CascadeShadowConfigBuilder`, without duplicating its
  cascade spacing calculation
- Compare every actual config field against the desired config on every eligible
  pass: the entire bounds vector, minimum distance, and overlap proportion
- Mutate an unequal config only, reusing its bounds-vector capacity where possible

The cache does not skip entities or depend on camera change flags. New lights,
new props, external component edits, camera changes, sun policy changes, and
moving or removed creatures still reach their existing reconciliation paths.
Differing additional suns can reduce single-entry cache reuse but retain correct
outputs.

## Source-supported work reduction

In a settled scene, unchanged shipping props avoid 3,716 `Visibility` assignments
and component change marks per eligible update. The throttle allows at most
approximately five such updates per second, or up to 18,580 avoided assignments
per second; actual frame timing can reduce this rate.

Each unchanged matching sun also avoids one light-component write per eligible
update. An enabled sun with a reused desired configuration avoids rebuilding its
bounds vector and replacing an equal config.

The pinned component crates are Bevy 0.18.1, with a Bevy 0.18.0 facade. Mutable
component dereferences mark changes without an equality comparison. Bevy's
visibility propagation consumes `Changed<Visibility>`, then already guards
inherited-visibility writes itself. Its directional cascade processing and light
extraction are not filtered by these component change flags. These are avoided
CPU/ECS operations, not measured GPU, frame-time, or FPS improvements.

## Regression coverage

`production_voxel_renderer::island::update_tests` contains eight CPU/ECS tests:

1. Stationary outputs retain their change ticks and existing mesh guards
2. The desired-config cache reuses bounds and matches Bevy's builder
3. External visibility, light and config edits are repaired, including NaNs,
   while unrelated light fields survive
4. Zoom thresholds, throttling and LOD transitions retain their behavior
5. Camera distance and sun policy inputs refresh cascades; same-count updates
   retain the actual bounds allocation when capacity suffices
6. New props and suns are reconciled with a stationary camera
7. Creature movement, addition and removal still change terrain occupancy LOD
8. The actual deferred 3,716-prop spawn is reconciled, then stops visibility writes

Run the focused application tests with:

```sh
cargo test --locked -p alife_game_app --features production-voxel-frontend --lib production_voxel_renderer::island::update_tests:: -- --test-threads=1 --color=never
```

## Validation at draft preparation

- Passed: Rust formatting for the changed Rust files, using edition 2021
- Passed: source comparisons showing unchanged prop policy, view/focus helpers,
  readiness/spawn/throttle logic, terrain occupancy/LOD loop, camera constraints,
  and existing tests
- Passed: metadata-only typecheck of an isolated harness containing exact source
  spans for the modified island definitions, policies, view/focus helpers and
  complete update body, plus the actual eight test functions
- The harness reused previously compiled Bevy and local world/core dependencies;
  it does not establish an exact-head application build or test result
- Runtime tests were unexecuted locally: the isolated test-binary attempt reached
  its virtual-memory limit before execution, and full application linking was
  avoided under the host's memory/disk constraints
- A dedicated frontend CPU CI job runs the eight tests on the published head;
  its result must be checked separately
