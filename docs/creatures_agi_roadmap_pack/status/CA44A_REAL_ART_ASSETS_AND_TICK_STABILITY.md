# CA44A Real Art Assets And Tick Stability

Plan: CA44A
Branch: codex/CA44A-real-art-assets-and-tick-stability
Status: implemented and validated on branch; merge/post-merge validation pending
Next plan remains: CA44

## Scope

CA44A is a direct fix before continuing the roadmap. It does not start CA45 and does not request external CA44 tester evidence.

The pass addresses two problems found in manual player launch:

- the default `gpu_alpha` scenario stopped near tick 7 with `TerminalInvalidState`;
- default Player View still looked like programmer art/debug output instead of a game-world presentation.

## Tick-7 Reproduction And Root Cause

Pre-fix focused reproduction:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- gpu-alpha-stability-smoke crates/alife_world/tests/fixtures/gpu_alpha 64
```

Pre-fix result:

```text
requested=64 completed=6 first_invalid_tick=Some(8) diagnostic=TopologyCapacityExceeded sealed=5 packed=5 topology=5/3/5/1
```

Root cause: topology staging treated repeated dynamic observations of the same object concept as unique permanent bindings. The default alpha run rapidly exhausted the per-concept binding cap, then later the default simplex cap. CA44A fixes the summarization/capacity path while preserving strict invalid-state reporting.

`TerminalInvalidState` remains strict. Invalid states are not hidden, downgraded, or converted to warnings.

## Art Assets Generated

CA44A commits versioned original assets under:

```text
crates/alife_game_app/assets/alpha_art_v1/
crates/alife_game_app/assets/true_25d_alpha_v1/
```

The active default Player View target is the True 2.5D retro-futuristic biological presentation:

- low-poly `.glb` creatures/items/props/fog accents in `true_25d_alpha_v1`;
- fixed orthographic 2.5D Bevy camera;
- a single `Plane3d` seeded biome surface generated from a deterministic procedural sampler;
- seeded micro-ecology dressing generated around active creature/chunk anchors;
- offscreen regions represented by a data ledger/fog rather than rendered debug tiles;
- debug rectangles retained only for diagnostics/degraded paths, not the default Player View.

The `alpha_art_v1` PNG pack remains tracked and validated for HUD/debug/fallback/package surfaces, including `ground_tile_repeat.png`. It is not the primary terrain composition in default Player View.

The required visual roles are represented by assets:

- creature idle/hurt/motion/sleep/signal/eat
- selection ring/pulse
- food sprout/bloom
- hazard crystal/glow
- rock/obstacle cluster
- safe grass, soil/path, resource/grove, hazard-pressure, stone/rough, water, and sand tiles
- prop/dressing variants including grass tuft, leaf patch, mushroom cluster, pebble cluster, and warning shard
- low-poly 2.5D GLB roles for creature, food, hazard, rock, props, terrain ledgers, fog cells, and selection ring

No third-party copyrighted art was downloaded.

## Manifest Changes

Strict validation exists for both asset families:

- schema/version checked;
- required roles checked;
- PNG signature, dimensions, manifest file size, and per-role size caps checked;
- GLB files checked for existence, extension, required roles, size caps, local paths, and forbidden artifact/cache paths;
- forbidden target/log/cache/screenshot/capture/model-artifact paths rejected;
- package dry-run lists both `alpha_art_v1` and `true_25d_alpha_v1`.

## Rendering Changes

Default Player View now renders the world through a true-2.5D scene:

- camera uses fixed orthographic presentation with `FixedVertical(10.0)` and `(0, 12, 12)` looking at origin;
- world surface is one static Bevy plane using a generated seed biome map;
- creature/object/selection roles are spawned as normalized GLB scenes when available, with native low-poly fallback as degraded diagnostics;
- seeded chunks and active creature anchors drive procedural dressing and fog/ledger state;
- world labels, topology lines, stable ID spam, internal GPU/patch fields, teacher debug labels, and full event feed stay hidden in default Player View;
- Dev Overlay and Full Debug remain available for validation.

The 2.5D presentation is display-only. Bevy visuals do not become simulation, sensory, navigation, neural, ecology, or action authority.

The shader stack from the user brief remains a rendering contract only. CA44A does not claim implemented Sobel depth/normal outlines, pixel-step postprocessing, or full toon-shader postprocess.

## Tests Added/Changed

Focused coverage includes:

- alpha PNG manifest acceptance/rejection tests;
- true-2.5D GLB manifest acceptance/rejection tests;
- Player View tests proving asset-backed 2.5D rendering is default;
- tests proving the default view does not use fallback rectangles for required roles;
- fixed orthographic camera contract tests;
- seeded procedural biome map and chunk ledger tests;
- stable-ID/live creature anchor synchronization tests;
- Dev Overlay / Full Debug preservation via existing view-mode tests;
- 600-tick `gpu_alpha` stability smoke for the tick-7 regression.

## Focused Evidence

Alpha art validator:

```powershell
cargo test -p alife_game_app alpha_art_inner_validator -- --nocapture
```

Result: PASS, 8 tests.

True 2.5D asset validator:

```powershell
cargo test -p alife_game_app true_25d_assets -- --nocapture
```

Result: PASS, 2 tests.

Player View focused suite:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell player_view -- --nocapture
```

Result: PASS, 9 tests.

Production world-art regression:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell production_world_art -- --nocapture
```

Result: PASS, 1 test.

600-tick stability:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- gpu-alpha-stability-smoke crates/alife_world/tests/fixtures/gpu_alpha 600
```

Result:

```text
requested=600 completed=600 selected_creature=1 first_invalid_tick=None first_invalid_status=None action=None:None target=None diagnostic=None sealed=600 packed=600 topology=5/3/600/1 parity=true fallback=headless-cpu-oracle-stability-smoke terminal_invalid=0 recoverable_failures=0 avg_ms_per_tick=1.248 ticks_per_second=801.07
```

Default graphical Player View smoke:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Result: PASS. GPU selected `GpuPlastic`; fallback `None`; Player View acceptance true; smoke exited cleanly.

Forced CPU fallback graphical smoke:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Result: PASS. Fallback selected `CpuReference` with `HardwareUnavailable`; degraded fallback was visible; no GPU performance claim.

Package dry-run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_alpha.ps1 -DryRun
```

Result: PASS. Dry-run lists both asset directories and manifests.

Release/package cadence:

```powershell
cargo build -p alife_game_app --bin alife_game_app --features "bevy-app gpu-runtime" --release --jobs 1
```

Result: not completed in the local command window. The build exceeded 20 minutes and produced no release executable. CA44A therefore does not claim release/package runtime cadence evidence from this run. Debug/headless cadence and debug graphical smoke remain the only local runtime cadence evidence for this branch.

## Commands Run

Focused commands run before full validation:

```powershell
cargo test -p alife_game_app true_25d_assets -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell player_view -- --nocapture
cargo test -p alife_game_app alpha_art_inner_validator -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- gpu-alpha-stability-smoke crates/alife_world/tests/fixtures/gpu_alpha 600
cargo test -p alife_game_app --features bevy-app --test app_shell production_world_art -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_alpha.ps1 -DryRun
```

Full validation result: PASS on branch.

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
cargo test --workspace --all-features --all-targets --jobs 1
```

The all-features workspace test was run with `--jobs 1` after an unconstrained
all-features run exceeded the local command timeout. The serialized run passed.
Graphify update was attempted with the installed local executable, but it
refused to overwrite `graphify-out/graph.json` because the regenerated graph had
a different node count. No forced graph update was performed.

## Cadence Notes

The focused 600-tick headless stability smoke measured about 1.248 ms/tick in debug after compilation on this machine.

The graphical debug smoke is useful launch/playability evidence, but it includes debug compilation/runtime overhead and is not release performance evidence.

Release/package runtime cadence is not claimed because the local release build did not complete within the available command window.

## Known Limitations

- The world now reads as a seed-generated 2.5D alpha environment, but final art direction still needs real postprocess toon/Sobel/pixel-step rendering work.
- Procedural terrain is a graphical/display-context system for this pass. It is not yet an authoritative infinite ecology, sensory, navigation, resource-spawning, or offscreen simulation substrate.
- The world is generated from a deterministic seed and active creature/chunk anchors, but long-range creature exploration and streaming gameplay remain future work.
- Release/package runtime cadence remains unmeasured for this branch because the local release build did not complete in the command window.
- Local screenshots/logs under `target/playtest_evidence/` are evidence only and remain untracked.

## Invariant Checks

- No CA45 work started.
- No external CA44 tester evidence requested.
- No release tag created.
- No S12, G25, or P37 created.
- No semantic/SLM authority changes.
- No action authority changes.
- CPU fallback preserved.
- CPU shadow parity preserved.
- No full action-authoritative GPU runtime claim.
- No neural compression, custom sensory raycasting, planet topology, or ExperiencePatch transaction work.
- `alife_core` remains engine-independent; CA44A adds no Bevy/wgpu/app/model-runtime dependency to `alife_core`.

## Artifacts Tracked

Tracked: source code, docs, manifests, versioned PNG assets under `crates/alife_game_app/assets/alpha_art_v1/`, and versioned GLB assets under `crates/alife_game_app/assets/true_25d_alpha_v1/`.

Not tracked: screenshots, logs, target artifacts, model files, caches, captures, release zips, Blender temporary files, or generator scratch output.

## Release/Tag Status

No release tag was created. Release remains deferred.

## alife_core Dependency Status

`alife_core` remains dependency-clean. CA44A does not add Bevy, wgpu, renderer, app, model-runtime, GUI, or asset-pipeline dependencies to `alife_core`.

## Main Status

Branch implementation and validation are complete. Merge/post-merge validation
status is recorded in the final CA44A receipt.
