# Creature Procedural World Chunks v1

Plan: direct post-CA44A goal continuation, not a new roadmap item.
Branch: `codex/creature-procedural-world-chunks-v1`

## Objective

Move the GPU alpha terrain surface from purely display-only local hashing toward
a creature-facing procedural world substrate. The world should be deterministic,
large, chunked, generated around stable-ID creature anchors, and usable without
requiring a rendered camera surface.

This does not start CA45 and does not change the Creatures-to-AGI manifest.

## Implementation

Added `alife_world::procedural_chunks`:

- deterministic `ProceduralWorldConfig`;
- stable `ProceduralTileCoord` and `ProceduralChunkCoord`;
- creature anchor contract using `WorldEntityId` plus `Vec3f`;
- chunk activation around creature anchors only;
- bounded active chunk cap;
- deterministic biome/material sampling;
- bounded creature neighborhood samples for sensory/context consumers;
- explicit flags that chunks are generated without rendering and cannot emit
  actions or rewrite weights.

Updated the graphical alpha Player View terrain rendering so Bevy samples
terrain material identity from `alife_world` instead of using a local
presentation-only material function. The graphical layer still renders only
visible active tiles; it is not authoritative and does not create action,
physics, sensory, or neural behavior by itself.

## Evidence

Focused tests:

```powershell
cargo test -p alife_world --test procedural_chunks -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_world_art_atlas_v3_breaks_up_debug_checkerboard -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell bevy_feature_ca37_world_art_props_are_display_only_and_stable_id_safe -- --nocapture
```

Results: passed.

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

Graphical focused smokes passed:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Graphify was refreshed with:

```powershell
C:\Users\PC\.local\bin\graphify.exe update .
```

## Boundary Checks

- No `alife_core` code or dependencies changed.
- No Bevy, wgpu, renderer, model-runtime, or UI dependency entered
  `alife_core`.
- World chunk activation uses stable IDs and engine-neutral math types.
- Bevy visuals mirror the world chunk sampler; Bevy is not authoritative.
- Procedural terrain output cannot emit actions, rewrite weights, bypass P09
  arbitration, or inject hidden vectors.
- CPU fallback and CPU shadow parity are unchanged.
- No full action-authoritative GPU runtime claim.
- No S12, G25, P37, CA45, or release tag.

## Known Limitations

- The new chunk substrate is deterministic and creature-anchored, but it is not
  yet fully integrated as the only source for all resource spawning,
  navigation, sensory ray queries, evolution pressure, or offscreen ecology.
- Graphical Player View renders active chunks from the substrate, but rendering
  still only materializes visible tiles for efficiency.
- The current local alpha fixture remains small. The procedural substrate has a
  larger virtual extent and can be sampled beyond the visible camera slice, but
  later roadmap work is still needed to turn that into complete Minecraft-like
  streamed ecology with persistence and creature migration across chunks.

## Main Status

Branch implementation validated. Final merge and post-merge validation status
will be recorded in the completion receipt/checkpoint.

Next roadmap plan remains: CA44 external tester evidence. CA45 was not started.
