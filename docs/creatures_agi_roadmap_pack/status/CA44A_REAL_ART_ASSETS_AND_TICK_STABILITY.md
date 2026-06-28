# CA44A Real Art Assets And Tick Stability

Plan: CA44A
Branch: codex/CA44A-real-art-assets-and-tick-stability
Status: implemented on branch; validation passed
Next plan: CA44

## Reproduction Summary

The default `gpu_alpha` player path previously stopped almost immediately with `TerminalInvalidState`.
Focused reproduction used:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- gpu-alpha-stability-smoke crates/alife_world/tests/fixtures/gpu_alpha 64
```

Pre-fix result:

```text
requested=64 completed=6 first_invalid_tick=Some(8) diagnostic=TopologyCapacityExceeded sealed=5 packed=5 topology=5/3/5/1
```

This matched the manual observation that the graphical alpha stopped around tick 7.

## Root Cause

The first stop was a real `alife_core` topology rejection, not a GPU/CPU parity failure and not presentation code. `ConceptCell::observe` treated changing drive and location samples as unique permanent bindings. Repeated observations of the same object concept filled the per-concept binding cap rapidly, causing `TopologyCapacityExceeded` during sealed-patch topology staging.

After fixing dynamic binding summarization, the run advanced to 256 sealed patches and then hit the default simplex storage cap. Existing topology tests and world tests expect sealed patches to bind into simplexes, so CA44A preserves that contract and raises the bounded default simplex capacity from 256 to 1024 for the default `CreatureMind` topology map.

`TerminalInvalidState` remains strict. Invalid states are not hidden, downgraded, or converted into warnings.

## Art Assets Generated

Original generated PNG assets were committed under:

```text
crates/alife_game_app/assets/alpha_art_v1/
```

Assets:

- `creature_idle.png`
- `creature_hurt.png`
- `selection_ring.png`
- `food_sprout.png`
- `hazard_crystal.png`
- `rock_cluster.png`
- `terrain_safe_grass.png`
- `terrain_soil_path.png`
- `terrain_resource_grove.png`
- `terrain_hazard_pressure.png`
- `terrain_stone_rough.png`
- `prop_grass_tuft.png`
- `prop_pebble_cluster.png`
- `prop_warning_shard.png`
- `prop_leaf_patch.png`
- `alpha_art_manifest.json`

Each PNG is 64x64 and below the 64 KB per-file cap. Assets are original project-generated sprites/tiles, not third-party downloads.

## Manifest Changes

Added strict alpha art manifest validation:

- schema/version checked
- required roles checked
- PNG signature checked
- PNG dimensions checked
- manifest dimensions/file sizes checked against disk
- per-file size cap enforced
- at least three prop/dressing variants required
- forbidden artifact paths rejected

The app bundle manifest now references the alpha art manifest, and package inputs include the alpha art directory.

## Rendering Changes

Default Player View now uses asset-backed sprites for required visual roles:

- creature idle/hurt
- selection ring
- food
- hazard
- rock/obstacle
- primary terrain/material tiles
- prop dressing

Rectangle fallback remains available only for degraded diagnostics or non-player debug paths. Player View tests assert that required roles are backed by alpha art components and fallback rectangle components are absent.

## Tests Added/Changed

- Core topology regression for repeated dynamic observations.
- Alpha art unit tests for:
  - complete manifest acceptance
  - missing role rejection
  - dimension mismatch rejection
  - malformed PNG rejection
  - forbidden artifact path rejection
- App integration tests for:
  - committed alpha art manifest validation
  - 600-tick `gpu_alpha` stability regression
  - Bevy Player View alpha-art backed rendering
  - Dev Overlay / Full Debug preservation through existing view-mode tests

## Focused Evidence

Core topology regression:

```powershell
cargo test -p alife_core --test topological_map repeated_dynamic_observations_summarize_without_binding_capacity_failure -- --nocapture
```

Result: PASS.

Alpha art validator tests:

```powershell
cargo test -p alife_game_app alpha_art_inner_validator -- --nocapture
```

Result: PASS, 5 tests.

CA44A app tests:

```powershell
cargo test -p alife_game_app --test app_shell ca44a -- --nocapture
```

Result: PASS, 2 tests.

Bevy Player View art-backed rendering:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell ca44a_player_view_uses_alpha_art_sprites_not_default_rectangles -- --nocapture
```

Result: PASS.

600-tick stability:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- gpu-alpha-stability-smoke crates/alife_world/tests/fixtures/gpu_alpha 600
```

Result:

```text
requested=600 completed=600 first_invalid_tick=None diagnostic=None sealed=600 packed=600 topology=5/3/600/1 terminal_invalid=0 parity=true avg_ms_per_tick=1.293 ticks_per_second=773.14
```

App bundle smoke:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- app-bundle-smoke --manifest crates/alife_game_app/app_bundle_manifest.json
```

Result: PASS, `alpha_art=15`, `alpha_roles=true`, largest file evidence 15135 bytes.

Default graphical Player View smoke:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Result: PASS. GPU selected `GpuPlastic` on RTX 3050/DX12; Player View acceptance true; smoke exited cleanly after wall-clock timeout.

Forced CPU fallback graphical smoke:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Result: PASS. Fallback was explicit: `CpuReference`, `HardwareUnavailable`, degraded visible.

Package dry-run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_alpha.ps1 -DryRun
```

Result: PASS. Dry-run listed alpha art manifest and alpha art directory.

No-zip release package build:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_alpha.ps1 -NoZip
```

Result: PASS. Release build completed and package root was written under `target/artifacts/`.

Packaged graphical smoke:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File target/artifacts/ca41_windows_alpha/alife-gpu-alpha-windows/run_windows_alpha_package.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
```

Result: PASS. GPU selected `GpuPlastic`; Player View acceptance true; packaged smoke exited cleanly.

## Cadence Notes

Debug cargo-run 600-tick headless stability smoke measured roughly 0.9-1.3 ms/tick on this machine after compilation. Graphical debug cargo-run includes compile time and is not performance evidence.

Release/package smoke built with `cargo build --release` and launched the packaged executable successfully. The packaged 10-second smoke completed cleanly with GPU selected. CA44A does not claim full product performance or full action-authoritative GPU runtime.

## Commands Run

Focused commands are listed above. Full validation was run before merge:

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

## Validation Results

Focused validation passed. Full validation passed:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo test --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`
- `cargo tree -p alife_core`
- `cargo check --workspace --all-features --all-targets`
- `cargo test --workspace --all-features --all-targets`

## Known Limitations

- The art pack is a small alpha pack, not a final production art direction.
- PNG sprites are intentionally small and stylized.
- Player View uses asset-backed sprites, but future CA work may still improve animation, composition, and larger-world exploration.
- Package artifacts and diagnostics are generated under `target/artifacts/` and must remain untracked.

## Invariant Checks

- No CA45 work started.
- No release tag created.
- No S12, G25, or P37 created.
- No semantic/SLM authority changes.
- No action authority changes.
- CPU fallback preserved.
- CPU shadow parity preserved.
- No full action-authoritative GPU runtime claim.
- No neural compression, custom sensory raycasting, planet topology, or ExperiencePatch transaction work.
- `alife_core` remains engine-independent; CA44A changed bounded topology summarization/capacity only and added no Bevy/wgpu/app dependency.

## Artifacts Tracked

Tracked: source code, docs, manifest files, and versioned PNG art assets under `crates/alife_game_app/assets/alpha_art_v1/`.

Not tracked: screenshots, logs, target artifacts, model files, caches, captures, release zips, or temporary generator outputs.

## Release/Tag Status

No release tag was created. Release remains deferred.

## alife_core Dependency Status

`alife_core` remains dependency-clean. CA44A does not add Bevy, wgpu, renderer, app, model-runtime, or GUI dependencies to `alife_core`.

## Main Status

Branch implementation validated and ready for review/merge. Final main merge and post-merge validation status is recorded in the CA44A completion receipt.
