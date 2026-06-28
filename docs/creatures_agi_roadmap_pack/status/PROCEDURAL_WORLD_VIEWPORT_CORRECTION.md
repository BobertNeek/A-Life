# Procedural World Viewport Correction

## Scope

This is a bounded product-surface correction after CA39 and before CA40. It is
not a new roadmap item and does not change `plan_manifest.json`.

## Reason

User screenshot review showed the graphical alpha still read as a single dense
screen rather than a local camera view into a larger procedurally generated
terrain map.

## Branch

`codex/procedural-world-viewport-fix`

## Runtime Code Changed

Yes, in `alife_game_app` presentation code only.

## Core APIs Changed

No. `alife_core` was not changed.

## Change Summary

- CA37 world-art metadata now records an explicit generated-map to local-camera
  split: `97x73` generated terrain tiles and a `17x11` viewport slice.
- World-art validation rejects summaries where the viewport is not smaller than
  the generated map or where fewer than four stable world objects are off-screen.
- The graphical app starts zoomed into the local world slice when CA37 world art
  is available, so the terrain reads as a traversable map instead of a compressed
  whole-board view.
- Terrain tiles now use world-scale cell spacing with small jitter, preventing
  the full generated map from collapsing into overlapping translucent blocks.
- Player-facing legend and smoke output now call out the local viewport,
  off-screen stable objects, and pan/follow exploration path.

## Invariant Checks

- Terrain remains display-only.
- No physics, navigation, sensory-backend, topology, cognition, or action
  semantics changed.
- Stable IDs remain the player-facing identity surface.
- CPU fallback remains available.
- CPU shadow parity remains the correctness gate.
- Product runtime claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- No full action-authoritative GPU runtime claim was added.
- No Bevy, wgpu, renderer, windowing, or app dependencies were added to
  `alife_core`.

## Focused Evidence

```powershell
cargo test -p alife_game_app --test app_shell ca37 -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell ca37 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- world-art-style-smoke crates/alife_world/tests/fixtures/gpu_alpha
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
cargo run -p alife_game_app --bin alife_game_app -- graphical-controls-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Observed world-art smoke evidence:

```text
tiles=7081 viewport=17x11 ratio=37.9 offscreen_objects=6 span_world_units=96.0 large_world_exploration=true
```

Observed graphical smoke selected `GpuPlastic`, retained CPU shadow parity, and
closed cleanly. Forced fallback selected `CpuReference` with
`HardwareUnavailable` and made no GPU-performance claim.

The deterministic controls smoke passed for the GPU alpha fixture, including
run/pause, step, speed, follow, reset, and exit-request semantics.

## Artifacts

No screenshots, logs, target artifacts, model files, cache files, or generated
media are tracked.

## Next Plan

CA40 remains the next manifest item after this correction is merged and
validated.
