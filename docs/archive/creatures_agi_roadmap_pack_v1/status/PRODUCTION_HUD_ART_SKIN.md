# Production HUD Art Skin

Goal: move the default A-Life Player View HUD and inspector away from raw
developer text panels by adding committed, versioned alpha-art UI skin assets
that sit behind the existing read-only overlays.

Branch: `codex/production-hud-art-skin`

## Scope

This is a bounded production-art continuation after CA44A, production art
animation, and Player View composition polish. It is not roadmap continuation
and does not start CA45.

## Assets Added

New committed PNG assets under
`crates/alife_game_app/assets/alpha_art_v1/`:

- `ui_panel_frame.png`
- `ui_inspector_frame.png`
- `ui_status_chip.png`
- `ui_meter_bar.png`
- `ui_control_keycap.png`

The alpha art manifest now records `art_direction` as:

```text
production-alpha-organic-topdown-v5
```

The pack contains 30 PNG entries. The new files are original deterministic
project-generated assets, 128x128, listed in `alpha_art_manifest.json`, and
below the 64 KB per-file cap.

## Rendering Changes

Default Player View now spawns asset-backed Bevy UI image layers for:

- status panel frame;
- GPU status chip;
- creature inspector frame;
- inspector meter accent;
- controls panel frame;
- controls keycap accent;
- event-feed panel frame.

These layers are tagged as `GraphicalProductionHudSkinLayer` and
`display_only=true`. The existing HUD text remains read-only and still exposes
the CPU shadow gate and no-full-action-authoritative boundary. Text panel
background opacity is reduced in Player View so the committed UI art carries
the surface instead of solid debug rectangles.

Dev Overlay and Full Debug still keep their diagnostic panels and text.

## Tests Added Or Changed

- App bundle validation now expects 30 alpha-art entries.
- Alpha-art manifest required roles now include five UI skin roles.
- A Bevy feature-gated Player View test verifies the HUD skin layers are
  asset-backed Bevy UI image nodes and display-only.

## Focused Evidence

Commands run:

```powershell
python scripts/generate_alpha_art_v1.py
cargo fmt --all
cargo test -p alife_game_app alpha_art_inner_validator -- --nocapture
cargo test -p alife_game_app --test app_shell ca44a_committed_alpha_art_manifest_validates_required_roles_and_pngs -- --nocapture
cargo test -p alife_game_app ca12_app_bundle_manifest_discovers_assets_shaders_and_placeholder_art -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_hud_skin_uses_committed_ui_assets_in_player_view -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell ca44a_player_view_uses_alpha_art_sprites_not_default_rectangles -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell production_player_view_composition_layers_are_asset_backed_and_display_only -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- app-bundle-smoke --manifest crates/alife_game_app/app_bundle_manifest.json
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE='0'; powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Results: PASS. The first Bevy HUD-skin test invocation timed out while the
feature build was in progress, then passed on immediate rerun with a longer
timeout. The graphical GPU smoke selected `GpuPlastic`, kept CPU shadow parity,
reported Player View acceptance true, and exited cleanly. The forced fallback
smoke selected `CpuReference`, reported `HardwareUnavailable`, exposed degraded
fallback state, and exited cleanly.

## Validation Results

Full validation commands run:

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

- This is still an alpha HUD skin, not a finished commercial UI system.
- The pass adds art-backed UI frames and chips but does not implement full UI
  layout redesign, animated menu transitions, accessibility scaling, or a
  complete interaction tutorial.
- Computer Use screenshot comparison is not recorded in this status document;
  graphical smoke output remains the required local evidence.

## Artifacts

Tracked: source code, docs, manifest, and versioned product PNG assets under
`crates/alife_game_app/assets/alpha_art_v1/`.

Untracked/forbidden: screenshots, logs, target artifacts, model files, caches,
temporary generator output, and generated reference images.

## Main Status

Feature branch validation passed before merge. Final main merge/push status is
captured in the run receipt.

## Next Work

Continue the current production-art quality goal only after this slice is
validated and merged. Roadmap continuation remains stopped unless explicitly
authorized.
