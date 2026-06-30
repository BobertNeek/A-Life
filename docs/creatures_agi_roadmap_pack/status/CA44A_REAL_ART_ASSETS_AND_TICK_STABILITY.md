# CA44A Real Art Assets And Tick Stability

Plan: CA44A
Branch: codex/CA44A-real-art-assets-and-tick-stability
Follow-up visual correction branch: codex/production-procedural-world-visual-fix
Status: implemented on branch; validation passed
Next plan: CA44

## Current Follow-up Status

The later procedural seeded Player View correction supersedes the final
post-feedback composition note below. Default Player View now uses a
runtime-generated seeded biome map with active creature chunk windows and fog of
war. `world-painted-viewport` is retained only for Full Debug/style-reference
presentation and must not be restored as the default player terrain surface.

The latest visual correction replaces the rejected dark/noisy swatches, the
muddy v25/v30 attempts, the still-insufficient v31 single-atlas attempt, and the
v32 stamped-blob runtime result with new image-generated v33 ground tiles and
sprites. The committed Player View assets now come from sliced generated PNGs
for grass, path, grove ground, hazard-pressure ground, stone, water, sand,
creatures, selection, food, hazard crystals, rocks, and props. The importer is
tracked at `scripts/import_alpha_art_imagegen_atlas.py` so the split terrain and
sprite sheets can be re-sliced repeatably, including terrain-only refreshes.
The runtime biome compositor now samples generated tiles as fine material
texture over a continuous seeded biome field, uses biome regions/trails for
large-scale composition, smooths fog-of-war reveal radially around active
creature chunks, avoids hard square chunk masks in default Player View, and
uses readable map-scale creature/object sprites instead of close-up stickers or
debug-map specks.

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
- `world_backdrop_gpu_alpha.png`
- `prop_grass_tuft.png`
- `prop_pebble_cluster.png`
- `prop_warning_shard.png`
- `prop_leaf_patch.png`
- `alpha_art_manifest.json`

The active manifest art direction is now
`production-alpha-generated-world-atlas-v21-distinct-biome-tiles`. Ordinary
sprite/tile PNGs remain 128x128 and below the 64 KB per-file cap.
`world_backdrop_gpu_alpha.png` is retained as a Full Debug/style-reference map
plate, capped by the stricter world-backdrop exception (`<= 768 KB`). Assets are
original project-generated sprites/tiles/backdrops, not third-party downloads.

The v21 refresh regenerated the committed terrain and object PNGs from two new
generated atlases:

- terrain atlas: grass, dirt path, resource grove, red hazard pressure, gray
  stone, teal water, and sand;
- object atlas: blue-green creature idle/hurt poses, selection ring, food
  sprout, red crystal hazard, rock cluster, and prop variants.

The live Player View capture at
`target/playtest_evidence/terrain_tiles/generated_v21_preview/player_view_v21_clean_dressing_capture.png`
confirmed that the default window uses the generated terrain/sprites instead of
rectangle placeholders. That evidence path remains local and untracked.

The v7 terrain refresh regenerated the committed terrain PNGs with organic
alpha masks, deterministic texture noise, softer edges, and stronger biome
color language:

- `terrain_safe_grass.png` is 22,846 bytes
- `terrain_soil_path.png` is 21,942 bytes
- `terrain_resource_grove.png` is 23,522 bytes
- `terrain_hazard_pressure.png` is 24,557 bytes
- `terrain_stone_rough.png` is 22,236 bytes

The manifest now contains 35 versioned art entries including creature poses,
selection assets, food/hazard/rock sprites, five terrain roles, prop dressing,
ambient layers, HUD skin assets, and the painted Player View world backdrop.

## Manifest Changes

Added strict alpha art manifest validation:

- schema/version checked
- required roles checked
- PNG signature checked
- PNG dimensions checked
- manifest dimensions/file sizes checked against disk
- per-file size cap enforced
- world-backdrop role uses its own manifest-validated size/dimension cap
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
- painted world backdrop
- prop dressing

Rectangle fallback remains available only for degraded diagnostics or non-player debug paths. Player View tests assert that required roles are backed by alpha art components and fallback rectangle components are absent.

The Player View terrain renderer now keeps the deterministic seeded terrain
field virtual while the default player presentation is driven by a generated
painted map surface, a runtime procedural biome texture, materialized
procedural chunk evidence, and chunk-generated content. Active local chunk
samples still carry asset-backed terrain sprites and chunk provenance, but
those tile sprites are intentionally near-transparent so they do not read as
square debug slabs. Offscreen terrain remains data-only until creature anchors
or the camera require materialization. The terrain/content layers remain
display-only; they are not physics, navigation, sensory, cognition, ecology,
neural, or action authority.

The v15 correction replaces the rejected v12/v13 compositions. The bad v12
plate had giant baked creatures, high-contrast blob fields, and black gaps. The
v13.1 plate fixed the square-placeholder problem but still read too sparse and
washed out compared with the target top-down game-world mockup. The generator
now always regenerates the committed backdrop instead of preserving an existing
stale file. The new v15 backdrop is a dense 1280x720 painted map plate with
small-scale terrain texture, narrow dirt trails, green resource groves, gray
highlands, red hazard pressure, small rocks/flowers/crystals/creature hints,
and no baked giant foreground actors. The Rust validation constant and manifest
now both require v15.

The post-feedback player-view correction keeps that v15 painted map as the
dominant visible surface. The fix changed only presentation composition:
default camera framing now starts with a world-establishing view, terrain masks
are subtle live-context overlays, and foreground creatures/props/selection
pulses are small map-scale sprites instead of giant close-up elements. Simulation
semantics, GPU/CPU correctness rules, and action authority are unchanged.

The final post-feedback correction previously made that generated painted map
fill the default Player View instead of appearing as a small plate inside a flat
field. That presentation was superseded by the seeded procedural Player View
follow-up: the default Player View now asserts zero `world-painted-viewport`
layers and one primary `runtime-procedural-biome-map` layer. The painted
viewport remains a Full Debug/style reference only. This is still a
graphical/presentation step toward the active goal, not a claim that procedural
terrain is authoritative ecology or sensory state.

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
  - high-resolution painted Player View map fit
  - procedural terrain/content ledger evidence without visible debug terrain tiles
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

Result: PASS, 6 tests.

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
requested=600 completed=600 first_invalid_tick=None diagnostic=None sealed=600 packed=600 topology=5/3/600/1 terminal_invalid=0 parity=true avg_ms_per_tick=2.508 ticks_per_second=398.71
```

App bundle smoke:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- app-bundle-smoke --manifest crates/alife_game_app/app_bundle_manifest.json
```

Result: PASS, `alpha_art=35`, `alpha_roles=true`, `production_alpha_art=true`, largest file evidence 236,817 bytes.

Production art / chunked terrain tests:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell production_world_art_atlas_v3_breaks_up_debug_checkerboard -- --nocapture
cargo test -p alife_game_app --all-features --test app_shell bevy_feature_ca37_world_art_props_are_display_only_and_stable_id_safe -- --nocapture
```

Result: PASS. These tests verify asset-backed terrain, opacity below
debug-block levels, chunk provenance, and virtual-map materialization near
active view/creature anchors.

Procedural Player View / target-match focused tests:

```powershell
cargo test -p alife_game_app --features bevy-app production_player_view_default_camera_is_world_establishing -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_starts_with_rendered_procedural_chunk_window -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_composition_layers_are_asset_backed_and_display_only -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell procedural_world_content_uses_alpha_art_and_no_action_authority -- --nocapture
```

Result: PASS. These tests verify the wide default camera, generated painted map
surface, runtime procedural biome texture, rendered procedural terrain chunk
window, generated content visual layer, display-only composition, and no action
authority.

Manual screenshot comparison used untracked local evidence:

```text
target/playtest_evidence/visual_fix/player_view_v15_actual_settled.png
target/playtest_evidence/visual_fix/fresh_window_capture.png
target/playtest_evidence/visual_fix/current_player_view_scaled_painted_surface.png
```

Result: the actual Bevy window now renders the target-style painted map plate
as a full Player View surface with small creatures, food, rocks, narrow paths,
dense resource greenery, gray rough terrain, and visible red hazard pressure
region. The prior flat-green fallback capture was traced to the v11/v12
manifest mismatch; the later noisy v12 composition, sparse/washed v13
composition, and blocky runtime-only terrain view were replaced by the scaled
v15 painted map plus procedural runtime evidence.

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
cargo test --workspace --all-features --all-targets --jobs 1
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
- `cargo test --workspace --all-features --all-targets --jobs 1`

The first all-features test attempt without `--jobs 1` hit a local Windows
resource failure (`C:` free space reached 0 bytes and the linker reported
paging/PDB allocation errors). After deleting generated `target/` build
artifacts with `cargo clean`, the same all-features target set passed with
`--jobs 1`. This was an environment resource workaround, not a test weakening
or code failure.

## Known Limitations

- The art pack is a small alpha pack, not a final production art direction.
- PNG sprites are intentionally small and stylized.
- Player View uses asset-backed sprites and a painted procedural viewport with
  low-opacity chunk masks. Future CA work still needs camera paging polish,
  long-lived offscreen region policy, and richer creature exploration across
  many chunks.
- Procedural terrain currently improves graphical presentation and active-chunk
  rendering. It is not yet an authoritative Minecraft-like biome/chunk substrate
  for creature sensory, navigation, resource spawning, learning, or offscreen
  ecology.
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
