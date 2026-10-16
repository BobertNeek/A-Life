# Island camera change guards

## Scope and source baseline

Baseline: `624bb5b355aa93bfd2c986351e556810b5b1dd25`.
The baseline island renderer blob is `a6a6eeeeaeb816bee2737b1bf7f760bae48180a6`.

The camera constraint system still runs on every eligible camera each frame.
It still reads the current selected terrain, computes the current view height,
finds the surface focus, clamps the horizontal focus, and applies the existing
clearance/backoff calculation. `view_height`, `focus_on_surface`, and the entire
`constrain_to_surface` helper are unchanged.

Only writes to ECS components change:

- Read projection first. Mutably borrow and correct an orthographic projection
  only when its near or far clipping plane differs from `0.1` or `10000.0`
- Compute the existing camera constraint on a stack copy of `Transform`
- Write translation only when the resulting three floating-point bit patterns
  differ from the actual component. Rotation and scale are not reassigned

Bit comparison preserves signed-zero changes, exact finite changes, infinities,
and NaN payload changes produced by the existing arithmetic. It also avoids
treating an unchanged NaN bit pattern as a new translation. There is no epsilon,
camera-state cache, change-flag gate, or assumption that the terrain is unchanged.
New cameras, external edits, projection variant switches, zoom, and terrain
replacement therefore reach the same reconciliation path immediately.

## Source-supported work reduction

The pinned facade is Bevy 0.18.0; component crates are Bevy 0.18.1 from upstream
revision `f667c282dad2c1419afb5836ded22a3ec263970e`.

The baseline unconditionally mutably dereferences every camera's `Projection`,
including perspective/custom variants, and passes every `Transform` through a
mutable dereference. Bevy marks these components changed even if their values
are equal. A settled camera now avoids both component change marks. A settled
orthographic camera also avoids its repeated near/far assignments.

These flags are consumed by Bevy's transform synchronization/propagation,
camera-projection update, and frustum update paths:

- [Mutable dereference marks the component changed](https://github.com/bevyengine/bevy/blob/f667c282dad2c1419afb5836ded22a3ec263970e/crates/bevy_ecs/src/change_detection/traits.rs#L419-L428)
- [Simple transform synchronization filters changed transforms](https://github.com/bevyengine/bevy/blob/f667c282dad2c1419afb5836ded22a3ec263970e/crates/bevy_transform/src/systems.rs#L13-L33)
- [Camera updates consume projection change flags](https://github.com/bevyengine/bevy/blob/f667c282dad2c1419afb5836ded22a3ec263970e/crates/bevy_render/src/camera.rs#L337-L350)
- [Frustum updates filter changed global transforms or projections](https://github.com/bevyengine/bevy/blob/f667c282dad2c1419afb5836ded22a3ec263970e/crates/bevy_camera/src/visibility/mod.rs#L488-L500)

Other systems can still legitimately mark these components changed. This does
not remove surface raycasts, camera-clamp arithmetic, camera queries, rendering,
or the independent island LOD updates. It establishes avoided CPU/ECS writes and
false invalidations, with no measured GPU, frame-time, or FPS claim.

## CPU regression coverage

`production_voxel_renderer::island::camera_tests` contains eight tests:

1. Settled orthographic and perspective cameras keep their component change ticks
2. Newly spawned cameras and projection-variant switches are reconciled
3. First-pass clamping retains rotation, scale and unrelated projection fields,
   while only orthographic near/far planes are corrected
4. External camera edits, zoom changes, and selected-terrain replacement match
   the original, unchanged clamp helper's exact output bits
5. External near/far edits, including NaNs and infinities, are repaired without
   dirtying a settled transform
6. Missing selected terrain and entities without the camera marker stay untouched
7. Signed-zero, nonfinite translation and nonfinite view-height inputs preserve
   the original clamp helper's exact output bits
8. Stable nonfinite fields do not force unrelated translation writes

Run the full focused application tests with:

```sh
cargo test --locked -p alife_game_app --features production-voxel-frontend --lib production_voxel_renderer::island::camera_tests:: -- --test-threads=1 --color=never
```

The workflow runs those tests in a CPU frontend lane. It does not
start a graphical app or a neural GPU session.

## Validation at patch preparation

- Passed: Rust formatting, static core boundaries, documentation assertions
  (77/77), and parsed workflow checks preserving unrelated jobs
- Passed: exact patch replay against the source baseline, plus a separate
  PR66-compatible replay preserving its output guards and reusing its frontend lane
- Passed: metadata-only typecheck of exact production camera-system, view/focus,
  and clamp-helper source spans with all eight actual CPU/ECS test functions
- Passed: combined metadata-only typecheck with PR66's complete island-update
  source and its eight tests alongside these eight camera tests
- Passed: five tiny runtime tests of the exact production write predicate using
  the pinned glam `Vec3`: equal finite values, one-ULP changes, signed zero,
  equal/different NaN payloads, and infinities
- The predicate harness uses a field-only `Transform` stand-in. It does not
  execute Bevy ECS, the full clamp, or a graphical application
- The metadata harness reuses existing compiled Bevy and local world/core
  libraries. It does not establish an exact-head full-application build
- Full CPU/ECS runtime tests and full application checks remain unexecuted
  locally under host memory/disk constraints. Exact-head hosted results are
  required before treating the runtime regression suite as passed

This is a presentation-only implementation change. It does not alter organism
behavior, cognition, neural execution, world physics, save state, or architecture.
