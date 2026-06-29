# Production Player View Composition Polish

Goal: improve the default A-Life Player View composition with asset-backed
lighting/shadow dressing so the graphical alpha reads less like flat placed
sprites on a tile board, without changing simulation authority.

Branch: `codex/production-player-view-composition-polish`

## Scope

This is a bounded production-art continuation after CA44A and the production
art animation pass. It is not roadmap continuation and does not start CA45.

## Assets Added

New committed PNG assets under
`crates/alife_game_app/assets/alpha_art_v1/`:

- `ambient_canopy_shadow.png`
- `ambient_light_pool.png`
- `entity_shadow.png`

The alpha art manifest now records `art_direction` as:

```text
production-alpha-organic-topdown-v4
```

The pack contains 25 PNG entries. The new files are original deterministic
project-generated assets, 128x128, listed in
`alpha_art_manifest.json`, and below the 64 KB per-file cap.

## Rendering Changes

Default Player View now adds asset-backed composition layers:

- wide transparent canopy-shadow overlays across the generated map;
- soft light-pool overlays to break up the flat board composition;
- per-object entity shadows for creatures, food, hazards, and obstacles.

These layers are tagged as display-only production art. They do not emit
actions, alter sensory input, change navigation/physics, mutate cognition, or
make Bevy authoritative over the world model.

## Tests Added Or Changed

- App bundle validation now expects 25 alpha-art entries.
- CA44A manifest validation expects the expanded committed art pack.
- Default Player View asset-backed rendering checks include ambient and
  entity-shadow roles.
- A Bevy feature-gated composition-layer test verifies the new layers are
  present and display-only.

## Focused Evidence

Commands run:

```powershell
python scripts/generate_alpha_art_v1.py
cargo fmt --all -- --check
cargo test -p alife_game_app alpha_art_inner_validator -- --nocapture
cargo test -p alife_game_app --test app_shell ca44a_committed_alpha_art_manifest_validates_required_roles_and_pngs -- --nocapture
cargo test -p alife_game_app ca12_app_bundle_manifest_discovers_assets_shaders_and_placeholder_art -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell ca44a_player_view_uses_alpha_art_sprites_not_default_rectangles -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_composition_layers_are_asset_backed_and_display_only -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- app-bundle-smoke --manifest crates/alife_game_app/app_bundle_manifest.json
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE='0'; powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Results: PASS.

Notes:

- The first Bevy-focused Player View test invocation timed out while waiting on
  the build directory lock, then passed on immediate rerun with a longer
  timeout.
- The graphical GPU smoke selected `GpuPlastic`, kept CPU shadow parity, and
  exited cleanly.
- The forced fallback smoke selected `CpuReference`, reported
  `HardwareUnavailable`, exposed degraded fallback state, and exited cleanly.

## Full Validation

Commands run:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\docs_check.ps1
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

## Known Limitations

- This is still an alpha art pack, not a finished commercial-grade art set.
- The pass improves lighting/depth composition but does not add full VFX,
  biome sets, animated tile transitions, or artist-authored UI skinning.
- Computer Use screenshot capture was unavailable in this turn, so visual
  comparison relies on generated asset inspection and graphical smoke output.

## Artifacts

Tracked: source code, docs, manifest, and versioned product PNG assets under
`crates/alife_game_app/assets/alpha_art_v1/`.

Untracked/forbidden: screenshots, logs, target artifacts, model files, caches,
temporary generator output, and generated reference images.

## Main Status

Pending merge and post-merge validation.

## Next Work

Continue the current production-art quality goal only after this slice is
validated and merged. Roadmap continuation remains stopped unless explicitly
authorized.
