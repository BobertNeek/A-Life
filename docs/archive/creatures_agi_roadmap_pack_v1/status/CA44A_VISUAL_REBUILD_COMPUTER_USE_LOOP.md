# CA44A Visual Rebuild Computer Use Loop

Plan: CA44A extension
Branch: codex/CA44A-visual-rebuild-computer-use-loop
Status: branch validated; fallback app-window capture available
Next plan remains: CA44 after visual acceptance and external tester evidence

## Scope

This is a CA44A visual-quality blocker fix. It does not advance the CA roadmap,
does not start CA45, and does not request external CA44 tester evidence.

The work is scoped to the graphical Player View presentation, committed
alpha-art assets, and app-shell tests. It does not change simulation semantics,
action authority, CPU fallback, CPU shadow parity, semantic/SLM authority, or
`alife_core` dependencies.

## Baseline And Target Evidence

Local untracked evidence directory:

```text
target/playtest_evidence/visual_rebuild/
```

Baseline and target files:

```text
target/playtest_evidence/visual_rebuild/baseline_user_reported_current.png
target/playtest_evidence/visual_rebuild/blueprint_target.png
```

Iteration captures:

```text
target/playtest_evidence/visual_rebuild/iteration12_crisp_texture_reduced_routes.png
target/playtest_evidence/visual_rebuild/current_branch_player_view_window.png
```

The baseline/user evidence showed the Player View still reading as a debug or
prototype scene: noisy grass carpet, weak biome composition, tiny props,
unconvincing world scale, and debug-like shape language.

The target blueprint is a stylized orthographic 2.5D ecosystem with large
readable biome masses, softened terrain transitions, recognizable creature,
food, hazard, rock, plant, water, sand, path, and fog-of-war cues, plus a small
HUD that does not dominate the world.

## Changes Made

The Player View now uses a deterministic seeded runtime biome map instead of a
single repeated grass tile. The generated map combines committed alpha terrain
tiles with continuous material fields for:

- safe grass;
- soil/path;
- resource/grove;
- hazard-pressure ground;
- stone/rough ground;
- water;
- sand.

The runtime map is display-only and mirrors world/context state. It is not
simulation, sensory, resource-spawn, navigation, or action authority.

The latest visual hierarchy pass changed the remaining presentation issues:

- added the committed `world_backdrop_gpu_alpha.png` as a 3D painted biome art
  substrate under the True 2.5D runtime objects;
- kept the seeded runtime biome map and procedural chunk ledger active beneath
  the art substrate for deterministic terrain evidence;
- switched the generated biome image sampler to nearest filtering for crisper
  terrain detail;
- strengthened water, sand, stone, resource, and hazard biome region contrast;
- hid token/school cue entities from the default Player View while preserving
  their stable-ID runtime mapping;
- removed token-like reed/plant prop dressing from the default Player View;
- rebalanced object scale so food, hazard, and rock accents read more clearly;
- added display-only contact shadows under world objects so they sit on the
  map instead of floating as flat cutouts.

The current visual-quality loop adds a second Player View entity pass:

- default True 2.5D Player View objects now use committed alpha-art PNGs as
  camera-facing 3D billboards for creature, food, hazard, rock, and selection
  roles when the alpha art pack is loaded;
- the low-poly/glTF placeholder bodies remain available for diagnostics or
  degraded paths, but they are no longer the primary Player View entity
  surface when alpha art is present;
- selected creature posture/learning feedback keeps its display-only
  endocrine state while preserving the billboard's isometric facing;
- small stable-ID-derived display offsets separate overlapping creature
  markers so the selected cluster reads as multiple organisms instead of a
  single blob;
- selection-ring scale was reduced to avoid dominating the creature cluster.

The validation loop also exposed a Windows all-features test harness issue:
the `app_shell` Bevy-heavy test binary passed when run serially, but the exact
workspace all-features command repeatedly exited with `STATUS_ACCESS_VIOLATION`
while `player_view_streaming_keeps_live_creature_anchors_synced` was running
in parallel with other Bevy app-shell tests. The repo Cargo test environment
now sets `RUST_TEST_THREADS=1` in `.cargo/config.toml`. This preserves all test
assertions and the exact validation command while avoiding parallel Bevy
resource teardown races in the test harness. It does not change runtime code,
simulation behavior, CPU fallback, CPU shadow parity, or action authority.

The `alpha_art_v1` terrain PNGs were regenerated as small original project
assets and the manifest file sizes were updated. The strict manifest validator
still rejects malformed PNGs, missing roles, dimension mismatches, oversized
non-backdrop assets, flat terrain swatches, and forbidden artifact paths.

## Procedural World Status

The visual terrain is generated from a deterministic seed and active
creature/chunk anchors. It presents a larger world window than a single screen
and records procedural chunk/materialization state in the existing app ledger.

Current limitation: this remains a graphical presentation/context layer. It is
not yet an authoritative infinite ecology or navigation substrate. Long-range
creature exploration and true streamed simulation beyond the active visual
window remain future CA work.

## Fog-Of-War Status

Fog/unexplored dimming exists in the runtime biome generator and GLB fog-cell
presentation. The latest pass reduces fog-cell scale to avoid opaque blobs.
Fog remains display-only and cannot alter creature perception, actions, or
world state.

## Computer Use / Screenshot Evidence

Previous app-window captures were saved under the local evidence directory.

Native Computer Use remains unavailable in this thread because the Computer Use
JS bootstrap fails before app enumeration with:

```text
failed to write kernel assets: The system cannot find the path specified. (os error 3)
```

The failure occurs before `sky.list_apps()` can run. Therefore the latest image
comparison is not claimed as native Computer Use screenshot evidence.

The current loop retried the official Computer Use bootstrap after tool
discovery exposed the Node-backed execution tool. It failed with the same
kernel asset-path error, so native Computer Use remains unavailable for this
thread.

Fallback app-window capture succeeded using the local screenshot helper:

```text
target/playtest_evidence/visual_rebuild/current_branch_player_view_window.png
```

That fallback capture shows a materially cleaner player view than the user
baseline: coherent roads, groves, rocks, hazard/resource zones, selected
creature ring, and compact HUD. It still does not fully match the generated
blueprint quality and remains a CA44A visual-quality iteration rather than
external alpha evidence.

## Focused Evidence

Strict PNG manifest validation:

```powershell
cargo test -p alife_game_app alpha_art_inner_validator -- --nocapture
```

Result: PASS, 8 tests.

True 2.5D focused tests:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d -- --nocapture
```

Result: PASS, 10 tests.

Player View art-substrate regression:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell ca44a_player_view_uses_true_25d_world_assets_not_default_rectangles -- --nocapture
```

Result: PASS. The default Player View includes the painted biome art substrate
and hides token-like reed props. After the current entity pass, it also proves
default True 2.5D Player View uses committed alpha art for creature, food,
hazard, rock, and selection roles.

Production/player-view focused tests:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell production_ -- --nocapture
```

Result: PASS, 8 tests.

Default graphical Player View smoke:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Result: PASS. GPU selected `GpuPlastic`, fallback `None`, Player View
acceptance true, `mind_tick=19`, `world_tick=18`, `sealed_patches=16`, and no
terminal invalid state was reported.

Forced CPU fallback smoke:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Result: PASS. Fallback selected `CpuReference` with
`HardwareUnavailable`; degraded fallback remained explicit and no GPU
performance claim was emitted.

## Previous Full Validation Results

Full branch validation passed:

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

Current result: PASS. The final all-features test initially reproduced a
Windows `STATUS_ACCESS_VIOLATION` in the all-features `app_shell` binary under
parallel harness execution. The isolated suspect test passed, the serial
`app_shell` binary passed all 210 active tests, and the exact workspace command
then passed after the repo test environment was made single-threaded for Rust
test binaries.

Final all-features verification:

```powershell
$env:CARGO_BUILD_JOBS="1"
cargo test --workspace --all-features --all-targets
Remove-Item Env:\CARGO_BUILD_JOBS -ErrorAction SilentlyContinue
```

Result: PASS. The command used the repo `RUST_TEST_THREADS=1` setting from
`.cargo/config.toml`; no manual `--test-threads=1` argument was needed.

## Known Limitations

- Native Computer Use app enumeration/screenshot capture remains blocked in
  this thread by the node kernel asset-path error, so visual evidence is
  fallback app-window capture, not native Computer Use evidence.
- The scene is materially improved but still not at the blueprint quality bar:
  entity sprites are now art-backed, but the world still depends on an authored
  painted substrate over the deterministic terrain map and does not yet have a
  full dynamic art-quality chunk compositor.
- The terrain generation is display/context-only and not a full streamed
  authoritative ecology substrate.
- The renderer still does not claim full action-authoritative GPU runtime.
- Rust test binaries now run single-threaded by repo configuration because
  Bevy-heavy app-shell tests can crash under parallel harness execution on this
  Windows machine. This is validation-stability infrastructure, not a runtime
  or gameplay change.

## Invariant Checks

- No CA45 work started.
- No external CA44 tester evidence requested.
- No release tag created.
- No S12, G25, or P37 created.
- No action authority changes.
- CPU fallback preserved.
- CPU shadow parity preserved.
- No full action-authoritative GPU claim.
- No semantic/SLM authority changes.
- No active bulk neural readback added.
- No neural compression, custom sensory raycasting, planet topology, or
  ExperiencePatch transaction work.
- `alife_core` remains engine-independent.

## Artifacts Tracked

Tracked: source code, tests, docs, manifests, and versioned alpha-art PNG
assets.

Not tracked: screenshots, logs, target artifacts, model files, caches, captures,
or temporary generated files.

## Release/Tag Status

No release tag was created. Release remains deferred.

## Main Status

Branch validation passed after the current alpha-art billboard entity pass.
Main has not been advanced by this extension document.
