# CA37 Terrain, Props, and World Art Style Pass

## Plan

CA37 - Terrain, props, and world art style pass.

## Branch

`codex/CA37-terrain-props-world-art-style-pass`

## Files Changed

- `crates/alife_game_app/src/world_art_style.rs`
- `crates/alife_game_app/src/bevy_shell.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/src/lib.rs`
- `crates/alife_game_app/src/schema.rs`
- `crates/alife_game_app/placeholder_art_manifest.json`
- `crates/alife_game_app/tests/app_shell.rs`
- `docs/creatures_agi_roadmap_pack/status/CA37_TERRAIN_PROPS_WORLD_ART_STYLE.md`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`

## Runtime Code Changed

Yes. CA37 adds a headless-testable world-art summary and a Bevy presentation
layer that spawns a deterministic display-only procedural visual terrain map plus
dressing props behind the existing stable-ID creature, food, hazard, obstacle,
school, and overlay markers.
After local screenshot review, the default graphical surface was also tightened
so broad debug-zone blocks, legacy diagnostic panels, and large center-screen
action/debug labels do not dominate the first-view player surface.

## Core APIs Changed

No. `alife_core` was not changed.

## Docs Changed

Yes. This status document and the roadmap progress table were updated.

## Public APIs Changed

Yes, within `alife_game_app` only:

- `run_world_art_style_smoke`
- `ca37_world_art_style_summary`
- `ca37_material_palette`
- `ca37_default_world_dressing_props`
- `bevy_shell::ca37_world_art_overlay_text`
- `bevy_shell::build_graphical_playground_preview_app_shell`

These are app/product evidence and presentation helpers. They do not add
simulation authority.

## Tests Added/Changed

- Added a headless CA37 world-art smoke test for palette, prop, manifest, and
  large procedural visual-map boundary validation.
- Added a Bevy-feature preview-shell test proving CA37 props spawn as
  display-only components and use stable IDs rather than Bevy entity IDs.
- Extended the Bevy-feature preview-shell test to prove the terrain-tile canvas
  is larger than the initial viewport, display-only, and includes safe,
  resource, hazard, soil, and stone materials.

## Focused Evidence

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell ca37 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- world-art-style-smoke crates/alife_world/tests/fixtures/gpu_alpha
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Observed CA37 smoke summary:

```text
tiles=7081 span_world_units=62.0 large_world_exploration=true distributed_objects=true zones=4 resource_materials=4 hazard_materials=3
signature=alife.ca37.world_art_style.v1:1:seed=4242:palette=6:props=8:visual_tiles=7081:span=62.0:zones=4:resource=4:hazard=3:explore=true:display_only=true:claim=CpuShadowGuardedStaticPlusLiveHShadow
```

Local screenshot review found that the first CA37 rendering still looked like a
debug dashboard: broad red/green terrain-zone rectangles and always-on legacy
diagnostic panels dominated the world. The branch was corrected before merge by
adding a deterministic larger generated visual terrain map, reducing broad
terrain-zone alpha, shortening world labels/action badges, shrinking
controls/event panels, and hiding older subsystem diagnostic panels from the
default graphical first view.
Later user review found the alpha still read as a single-screen board. The
active CA38 branch corrects that stale CA37 surface by expanding the generated
terrain to a seeded `97x73` tile map, spreading stable-ID food, hazards,
obstacles, and ecology zones across the map, and requiring world-art smoke to
prove `large_world_exploration=true`.

Any local app-window captures remain under `target/` evidence folders and are
not tracked.

## Commands Run

Validation commands are recorded in the final CA37 receipt. Focused commands
include the CA37 app smoke and Bevy-feature CA37 test above. Graphical smoke
and forced fallback smoke are required because CA37 changes graphical behavior.

## Validation Results

Branch validation passed before merge. Final post-merge validation is recorded
in the CA37 receipt.

## Invariant Checks

- `alife_core` unchanged and remains engine-independent.
- CA37 props are Bevy/app presentation only.
- CA37 procedural visual terrain tiles are Bevy/app presentation only.
- No simulation, physics, sensory, navigation, topology, action arbitration,
  ExperiencePatch, CPU shadow, or GPU authority semantics were changed.
- CPU fallback remains available.
- CPU shadow parity remains the correctness gate.
- Product runtime claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- No full action-authoritative GPU runtime claim was added.
- No Bevy entity IDs are used in player-facing CA37 text.
- Placeholder art descriptors remain tiny JSON metadata.

## Known Limitations

- CA37/CA38 now provide a deterministic large alpha terrain presentation with
  distributed stable-ID resources, hazards, obstacles, and ecology zones; it is
  still not a full art-production pass.
- The generated map is the graphical alpha exploration surface, but terrain does
  not add new physics, navigation, sensory-backend, topology, or cognition
  authority. Existing world/core arbitration and stable-ID world objects remain
  the only gameplay authority.
- Props are flat/2.5D visual cues, not physics objects, navigation obstacles,
  sensory sources, or world-generation authority.
- Broad CA19 terrain-zone sprites remain as faint visual hints only; they no
  longer function as the dominant art layer.
- The generated design blueprint used during implementation is local working
  evidence and is not committed.

## Planet Smith Ideas Used

The Planet Smith context was provided in-prompt and used only as a CA37 visual
presentation reference:

- A compact palette/material language for safe grass, neutral soil, resource
  grove, hazard pressure, stone dressing, and school/cue accent.
- A deterministic larger visual terrain map that uses palette thinking for
  generated safe, resource, hazard, soil, and stone regions.
- A seeded alpha terrain surface large enough for camera panning and distributed
  stable-ID creature/resource/hazard/object placement.
- Lightweight terrain/prop dressing: soil paths, leaf patches, warning shards,
  stone chips, and cue accents.
- 2.5D-style visual depth cues through sprite size, placement, and layering.

## Planet Smith Ideas Deferred

The following runtime ideas were explicitly deferred:

- Neural, GPU, chunk, or tile compression.
- Custom algebraic raycasting or sensory/spatial-query rewrites.
- Icosahedral/spherical/planet topology.
- New physics, navigation, sensory, or world-generation authority.
- Runtime chunk streaming, terrain compression, pathfinding over generated
  topology, and physics/sensory systems derived from terrain cells.
- Async token/action validation or ExperiencePatch transaction changes.

## Artifacts Tracked

No screenshots, logs, target artifacts, model files, caches, or generated media
are tracked. The only asset change is the tiny textual placeholder-art manifest.

## Release/Tag Status

No release tag was created.

## alife_core Dependency Status

`alife_core` remains dependency-clean; CA37 does not add Bevy, wgpu, renderer,
windowing, model-runtime, or app dependencies to core.

## Main Status

Pending merge and post-merge validation.

## Next Plan

CA38 - Creature animation expression polish. CA38 was not started by CA37.
