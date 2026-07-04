# FVR01 Completion Receipt - Production Launcher, Dependency Cutover, and Frontend Demolition

Plan: FVR01 - Production launcher, dependency cutover, and frontend demolition

Branch: `codex/fvr01-production-voxel-frontend`

## Files Changed

- `Cargo.toml`
- `Cargo.lock`
- `crates/alife_game_app/Cargo.toml`
- `crates/alife_game_app/environment_manifest.json`
- `crates/alife_game_app/src/app_bundle_ingestion.rs`
- `crates/alife_game_app/src/app_shell.rs`
- `crates/alife_game_app/src/bevy_shell.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/src/lib.rs`
- `crates/alife_game_app/src/production_voxel_frontend.rs`
- `crates/alife_game_app/src/schema.rs`
- `crates/alife_game_app/src/tests.rs`
- `crates/alife_game_app/tests/app_shell.rs`
- `crates/alife_game_app/tests/fvr01_production_frontend.rs`
- `scripts/run_graphical_playground.ps1`
- `scripts/run_production_voxel_frontend.ps1`
- `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR01_COMPLETION_RECEIPT.md`

No `docs/progress/` index exists in this branch, so no progress/index file was
updated.

## Deleted/Replaced Old Frontend Files

| Surface | FVR01 result |
|---|---|
| `graphical-playground` CLI | Preserved only as a compatibility alias. It now routes to `production-voxel` and reports `legacy_alias=true`. |
| `scripts/run_graphical_playground.ps1` | Replaced with a compatibility wrapper that forwards to `scripts/run_production_voxel_frontend.ps1` and preserves CA42/CA43 safety hooks. |
| Environment default | Replaced `gpu-alpha` as default with `production-voxel`. The alpha scenario remains selectable only as legacy/regression. |
| App-bundle active visual wording | Quarantined alpha/True 2.5D status text as historical/reference instead of active player-facing product route. |

No runtime alpha history docs were rewritten broadly; they remain historical
receipts.

## Public APIs Changed

- Added `production-voxel`.
- Added `validate-production-save`.
- Added `record-production-performance` as a production command alias that
  enables the performance flag.
- Added `ProductionAppState` with `Boot`, `ValidateRuntime`, `LoadAssets`,
  `LoadOrCreateWorld`, `Running`, `Paused`, `Settings`, and `Shutdown`.
- Added `ProductionFrontendProfileId` and profile budgets for:
  `MinimumSettings30x30`, `MinSpecComfort1080p`, `Balanced1080p`,
  `HighSpecScaleUp`, and `ResearchScale`.
- Added `ProductionVoxelLaunchConfig`, `ProductionVoxelLaunchSummary`,
  `ProductionRuntimeDiagnostics`, and `ProductionSaveMetadata`.
- Added `scripts/run_production_voxel_frontend.ps1`.

## Saved-State/Schema Changes

- Added `alife.fvr01.production_voxel_frontend.v1`.
- Added FVR01 profile budget metadata version `1`.
- FVR01 does not mutate the P34 save schema. It records selected profile and
  budget version in production launch/save-validation summaries while consuming
  the existing real P34 config, asset manifest, and save file.
- No Bevy, wgpu, renderer, window, or asset-loader handles were added to
  `alife_core` or portable saves.

## Assets/Licenses Changed

- No new binary or generated media assets were committed.
- Production asset dependencies were wired behind features only.
- Existing alpha and True 2.5D manifests remain validated as historical
  regression/reference assets, not the FVR production default.

## Dependency/Feature Result

- Exact top-level Bevy pin: `bevy = "=0.18.0"`.
- Production voxel stack pins:
  `bevy_voxel_world = "=0.16.0"`, `block-mesh = "=0.2.0"`,
  `bevy_sprite3d = "=8.0.0"`, `bevy_asset_loader = "=0.26.0"`,
  `bevy_hanabi = "=0.18.0"`, `bevy_egui = "=0.39.0"`,
  `bevy-inspector-egui = "=0.36.0"`.
- Feature flags added:
  `voxel-backend`, `voxel-internal-mesh`, `creature-sprites`,
  `licensed-assets`, `vfx-hanabi`, `debug-tools`,
  `presentation-physics`, and `production-voxel-frontend`.
- `default = []` remains headless/CI-safe.

## Commands Run

Passed:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo check -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app
cargo tree -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" -i bevy
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --help
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --dry-run
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --smoke-seconds 1
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinimumSettings30x30
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinSpecComfort1080p
cargo test -p alife_game_app --test fvr01_production_frontend fvr01 -- --nocapture
C:\Users\PC\.local\bin\graphify.exe update .
```

An initial parallel run of the production help command timed out at 244 seconds
while waiting on the build lock. The same exact command was rerun with a
600-second timeout and passed.

## Hardware/GPU Evidence

Production dry-run and bounded graphical launch selected:

```text
adapter='NVIDIA GeForce RTX 3050'
backend_api=Dx12
selected_backend=GpuPlastic
fallback=None
graphics_backend=dx12
profile=MinSpecComfort1080p
population=30
states=Boot>ValidateRuntime>LoadAssets>LoadOrCreateWorld>Running>Shutdown
real_save_loaded=true
mock_data_source=false
```

`MinimumSettings30x30` save validation also selected `GpuPlastic` on the same
RTX 3050/Dx12 path with `fallback=None`.

## Results

- Default desktop graphical launch path is now `production-voxel`.
- Default environment scenario is now `production-voxel`, title
  `A-Life Voxel Frontend`.
- Default production profile is `MinSpecComfort1080p`.
- `MinimumSettings30x30` is present as the hard fallback floor.
- Legacy `graphical-playground` routes to production and no longer owns the
  product path.
- Production launch uses real P34 config/save/asset paths and CA42 runtime
  diagnostics. No mock simulation, fake backend, fake GPU availability, or
  smoke-only data source was added.

## Boundary Invariants

- `alife_core` was not changed.
- No Bevy, wgpu, Avian, renderer, UI, OS window, or asset-loader types were
  introduced into `alife_core`.
- Renderer/UI/debug surfaces still have no authority over actions, cognition,
  weights, rewards, or world legality.
- GPU runtime selection remains diagnostic/fallback-aware and does not claim
  full action-authoritative GPU behavior.

## Deviations

None.

## Known Limitations

None for FVR01 scope. FVR02-FVR08 remain responsible for the persistent voxel
world backend, finished voxel renderer, creature rendering, UX, full gameplay
GPU runtime hardening, production assets, packaging, and final acceptance.
