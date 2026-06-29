# Production World Art Atlas v2

Goal: continue the post-CA44A production-art quality pass by making the default
Player View terrain and props read less like a debug checkerboard and more like
a coherent stylized game-world surface.

Branch: `codex/production-world-art-atlas-v2`

## Scope

This is a bounded visual presentation slice. It is not roadmap continuation and
does not start CA45. It does not change simulation semantics, action authority,
CPU fallback, CPU shadow parity, semantic/SLM authority, save/load contracts, or
`alife_core`.

## Blueprint

The design target for this pass was a top-down production-game alpha view:
organic grass/soil/grove/hazard/stone patches, distinct creature/food/hazard/
rock silhouettes, small environmental dressing, and compact HUD surfaces at the
screen edges. The key correction from the previous screenshots was to stop the
world from reading as repeated translucent square blocks.

## Assets Changed

The committed alpha art pack remains:

```text
crates/alife_game_app/assets/alpha_art_v1/
```

The manifest art direction is now:

```text
production-alpha-organic-topdown-v6
```

Changes:

- regenerated the five terrain material PNGs as transparent organic patches
  instead of opaque square fills;
- added `terrain_edge_blend.png` as a committed display-only blend overlay for
  tile seams;
- added `prop_mushroom_cluster.png` as an additional organic dressing prop;
- kept the pack inside the existing 32-entry bundle cap;
- all PNGs remain 128x128 and below the 64 KB per-file cap.

The art generator remains deterministic and project-owned:

```text
scripts/generate_alpha_art_v1.py
```

No third-party art was downloaded.

## Rendering Changes

Default Player View now renders terrain material sprites as overlapping,
transparent, rotated organic patches. Terrain edge-blend sprites are inserted as
display-only `GraphicalProductionArtLayer` entries on a deterministic subset of
tiles. Prop dressing selection now uses the prop id as well as material, so
resource areas can show mushroom/leaf/grass variation without changing world
state.

The debug/fallback rectangle path remains available only when alpha-art handles
are unavailable or diagnostics are requested. Dev Overlay and Full Debug remain
available.

## Tests Added Or Changed

- App bundle validation now expects 32 alpha-art entries.
- Alpha art required roles now include `terrain-edge-blend`.
- Manifest validation requires at least five prop-dressing variants.
- A Bevy feature-gated Player View test verifies:
  - at least 100 terrain tiles in the visible map slice;
  - terrain art is display-only;
  - Player View terrain has nonzero organic rotation;
  - terrain opacity stays below debug-block levels;
  - terrain edge-blend layers exist and are display-only.
- Existing Player View tests still verify required roles are asset-backed and
  rectangle fallback sprites are absent in default Player View.

## Focused Evidence

Commands run:

```powershell
python scripts/generate_alpha_art_v1.py
cargo fmt --all
cargo test -p alife_game_app alpha_art_inner_validator -- --nocapture
cargo test -p alife_game_app --test app_shell ca44a_committed_alpha_art_manifest_validates_required_roles_and_pngs -- --nocapture
cargo test -p alife_game_app ca12_app_bundle_manifest_discovers_assets_shaders_and_placeholder_art -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_world_art_atlas_v2_breaks_up_debug_checkerboard -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell ca44a_player_view_uses_alpha_art_sprites_not_default_rectangles -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_composition_layers_are_asset_backed_and_display_only -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_hud_skin_uses_committed_ui_assets_in_player_view -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- app-bundle-smoke --manifest crates/alife_game_app/app_bundle_manifest.json
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE='0'; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Results: PASS.

Notes:

- Four Bevy feature tests first timed out when run in parallel because Cargo
  feature builds contended on the build lock. They passed when rerun with a
  serial/longer budget.
- The 30-second graphical smoke selected `GpuPlastic`, kept CPU shadow parity
  true, reported Player View acceptance true, showed 12 objects with 3
  creatures, 3 food, and 3 hazards, and exited cleanly.
- The forced fallback smoke selected `CpuReference`, reported
  `HardwareUnavailable`, kept degraded fallback visible, and exited cleanly.

## Validation Results

Full validation commands run:

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

Results: PASS.

## Invariant Checks

- No CA45 work started.
- No S12, G25, or P37 created.
- No release tag created.
- No `alife_core` dependency changes.
- CPU fallback preserved.
- CPU shadow parity preserved.
- No full action-authoritative GPU runtime claim.
- No semantic/SLM authority change.
- No neural compression, custom sensory raycasting, planet topology, or
  ExperiencePatch transaction work.
- Terrain and prop changes are display-only.

## Known Limitations

- This remains a deterministic procedural alpha-art pack, not a final
  professional artist-authored production set.
- This pass improves terrain/prop composition; it does not add new gameplay,
  physics, sensory maps, navigation, topology, or animation systems.
- Fresh Computer Use app-window screenshot capture was unavailable in this
  thread; visual review used the generated asset contact sheet and graphical
  smoke evidence.

## Artifacts

Tracked: source code, docs, manifest, generator, and versioned PNG art assets
under `crates/alife_game_app/assets/alpha_art_v1/`.

Untracked/forbidden: screenshots, logs, target artifacts, model files, caches,
temporary generator output, generated reference images, and contact sheets.

## Main Status

Feature branch validation and final main merge/push status are recorded in the
run receipt.

## Next Work

Continue only the current production-art quality goal if explicitly requested.
Roadmap continuation remains stopped unless explicitly authorized.
