# CAR12 - Persistence and Content Review Report

Verdict: `PASS`

Reviewed tranche: CA09 through CA12.

## Scope Reviewed

- CA09 player-facing graphical save/load menu.
- CA10 configuration-driven environment launcher.
- CA11 player sandbox editor v1.
- CA12 asset bundle automation and ingestion.

This review checked the Phase C persistence/content surface before the roadmap
continues into CA13 scheduler work. The tranche stayed inside the expected
`alife_game_app`, launcher script, fixture/config, and roadmap documentation
surfaces.

## Files Inspected

Plans and receipts:

- `docs/creatures_agi_roadmap_pack/plans/CA09_player-facing-save-load-menu-in-graphical-app.md`
- `docs/creatures_agi_roadmap_pack/plans/CA10_configuration-driven-environment-launcher.md`
- `docs/creatures_agi_roadmap_pack/plans/CA11_player-sandbox-editor-v1.md`
- `docs/creatures_agi_roadmap_pack/plans/CA12_asset-bundle-automation-and-ingestion.md`
- `docs/creatures_agi_roadmap_pack/status/CA09_SAVE_LOAD_MENU.md`
- `docs/creatures_agi_roadmap_pack/status/CA10_ENVIRONMENT_LAUNCHER.md`
- `docs/creatures_agi_roadmap_pack/status/CA11_PLAYER_SANDBOX_EDITOR.md`
- `docs/creatures_agi_roadmap_pack/status/CA12_ASSET_BUNDLE_INGESTION.md`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`

Changed tranche files:

- `crates/alife_game_app/app_bundle_manifest.json`
- `crates/alife_game_app/environment_manifest.json`
- `crates/alife_game_app/placeholder_art_manifest.json`
- `crates/alife_game_app/src/app_bundle_ingestion.rs`
- `crates/alife_game_app/src/app_shell.rs`
- `crates/alife_game_app/src/bevy_shell.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/src/environment_launcher.rs`
- `crates/alife_game_app/src/lib.rs`
- `crates/alife_game_app/src/save_load_ux.rs`
- `crates/alife_game_app/src/schema.rs`
- `crates/alife_game_app/src/tests.rs`
- `crates/alife_game_app/src/world_editor.rs`
- `crates/alife_game_app/tests/app_shell.rs`
- `docs/productization/FIRST_GRAPHICAL_ALPHA_PLAYTEST_CHECKLIST.md`
- `docs/productization/GPU_FIRST_PLAYABLE_ALPHA_REPORT.md`
- `scripts/run_graphical_playground.ps1`

## Commands Run

Focused tranche validation:

```powershell
cargo test -p alife_game_app --test app_shell ca09 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- graphical-save-load-menu-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo test -p alife_game_app ca10 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- list-environments
cargo run -p alife_game_app --bin alife_game_app -- environment-launch-smoke --scenario gpu-alpha
cargo run -p alife_game_app --bin alife_game_app -- environment-launch-smoke --scenario p34
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun -Scenario gpu-alpha -GpuMode static-plastic-cpu-shadow-guarded
cargo test -p alife_game_app ca11 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- player-sandbox-editor-smoke --scenario gpu-alpha
cargo run -p alife_game_app --bin alife_game_app -- player-sandbox-editor-smoke --scenario p34
cargo run -p alife_game_app --bin alife_game_app -- graphical-controls-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo test -p alife_game_app ca12 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- app-bundle-smoke
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
cargo run -p alife_game_app --bin alife_game_app -- content-authoring-smoke
```

Standard validation, run on main after CA12 merge before this review branch:

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

## Findings By Severity

BLOCKER: none.

HIGH: none.

MEDIUM: none.

LOW:

- P34 remains a legacy compatibility fixture with `hazards=0` and
  `obstacles=0`. This is acceptable because CA10 identifies it as invisible
  legacy validation data, the GPU alpha scenario contains the player-facing
  hazard/obstacle markers, and CA11 proves sandbox hazard/obstacle placement for
  both scenarios.

INFO:

- `platform-package-smoke` reports an output path under `target/artifacts/`;
  this remains generated validation output and is not tracked.
- CA12 validates committed WGSL shader discovery and tiny placeholder-art
  metadata. It does not produce a release package or tag.

## Invariant Status

- `alife_core` dependency tree remains engine-independent: no Bevy, wgpu, GPU,
  renderer, school UI, or game-app dependency leaked into core.
- Save/load remains stable-ID based; invalid loads report schema errors and
  `partial_load=false`.
- Sandbox edits are pause-gated and use portable save roundtrips.
- The environment launcher selects versioned manifest scenarios and reports
  player-readable bad-selection guidance.
- Asset bundle ingestion validates small committed config/save/shader/art
  references and rejects missing required entries.
- CPU fallback and CPU shadow parity remain intact.
- No full action-authoritative GPU runtime claim was added.
- No S12, G25, P37, release tag, screenshots, logs, captures, or target
  artifacts were created as tracked outputs.

## User-Facing Status

Phase C improves player-facing persistence and content access:

- Save/load is exposed in the graphical app through player-visible controls.
- Known environments are launchable by scenario ID through a versioned manifest.
- Sandbox editor smoke proves food, hazard, and obstacle placement/removal plus
  edited-save roundtrip.
- App bundle ingestion gives the app a central inventory of runtime configs,
  saves, shaders, and placeholder art without packaging or release claims.

## Evidence Gaps

- The review did not perform a fresh manual interactive graphical playtest.
  Existing graphical smoke and dry-run launcher evidence cover command wiring;
  future playability gates should continue to use Computer Use or human tester
  evidence when visual UX changes.
- CA12 validates bundle discovery and small placeholder metadata; it does not
  prove distributable release packaging.

## Fix Prompt If Needed

No fix prompt is required. There are no BLOCKER, HIGH, or MEDIUM findings.

## Next Plan Recommendation

Next manifest plan: CA13 - Double-buffered graphical/game tick scheduler.

CAR12 is a hard-stop review gate. Stop here for user/ChatGPT consultation before
starting CA13.
