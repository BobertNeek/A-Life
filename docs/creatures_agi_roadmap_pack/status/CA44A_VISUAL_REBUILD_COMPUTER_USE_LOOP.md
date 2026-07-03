# CA44A Visual Rebuild Computer Use Loop

Plan: CA44A extension
Branch: codex/CA44A-visual-rebuild-computer-use-loop
Status: branch validated; latest Computer Use screenshot capture blocked
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

Iteration captures before this final smoothing pass:

```text
target/playtest_evidence/visual_rebuild/iteration12_crisp_texture_reduced_routes.png
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

The final smoothing pass changed the remaining presentation issues:

- switched the generated biome image sampler to linear filtering to reduce
  nearest-neighbor carpet artifacts;
- reduced terrain-tile blend strength so macro biome shapes carry the scene
  instead of per-pixel texture noise;
- reduced per-pixel micro-noise;
- thinned and softened procedural tile-edge accents so they no longer draw a
  visible grid over most tiles;
- slightly enlarged 2.5D dressing props so food, hazard, rock, and plant cues
  are easier to read;
- reduced broad fog-cell GLB scale so fog-of-war does not obscure the whole
  play surface.

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

## Computer Use Evidence

Previous app-window captures were saved under the local evidence directory.

This thread could not capture the latest app-window screenshot after the final
smoothing pass because the Computer Use JS bootstrap failed before app
enumeration with:

```text
failed to write kernel assets: The system cannot find the path specified. (os error 3)
```

The failure occurred before `sky.list_apps()` could run, and remained after two
supported setup attempts with a JS kernel reset between them. Therefore the
latest final image comparison is not claimed as Computer Use screenshot
evidence in this document.

The graphical smoke itself was run in the foreground and passed.

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

## Known Limitations

- The latest final screenshot could not be captured in this thread because the
  Computer Use JS runtime failed before app enumeration.
- The scene is improved toward a seed-generated 2.5D ecosystem, but final
  acceptance still needs a fresh app-window screenshot compared to the
  blueprint.
- The terrain generation is display/context-only and not a full streamed
  authoritative ecology substrate.
- The renderer still does not claim full action-authoritative GPU runtime.

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

Branch validation passed. Main has not been advanced by this extension document
because the latest Computer Use screenshot comparison could not be captured in
this thread.
