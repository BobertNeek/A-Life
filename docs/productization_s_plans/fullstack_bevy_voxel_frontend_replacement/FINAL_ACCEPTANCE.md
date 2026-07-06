# FVR08 Final Acceptance

Status: FVR08 complete.

## Completion Receipt

Plan: FVR08 - Final replacement hardening, packaging, and acceptance

Branch: `codex/fvr08-final-product-cutover`

Files changed:

| Area | Files |
|---|---|
| Product launch docs | `README.md`, `crates/alife_game_app/README.md`, `docs/release_candidate.md`, `docs/playable_sim_spec/platform_packaging.md`, `docs/playable_sim_spec/product_qa_hardening.md`, `docs/playable_sim_spec/FINAL_PLAYABLE_SIM_STATUS_REPORT.md`, `docs/playable_sim_spec/POST_RELEASE_BACKLOG.md`, `docs/playable_sim_spec/known_issues.md`, `docs/productization/S10_EXTERNAL_PLAYTEST_CANDIDATE_REPORT.md`, `docs/productization/S10_EXTERNAL_TESTER_CHECKLIST.md`, `docs/productization/S11_FINAL_PRODUCTIZATION_REPORT.md` |
| App command/product wiring | `crates/alife_game_app/Cargo.toml`, `crates/alife_game_app/environment_manifest.json`, `crates/alife_game_app/app_bundle_manifest.json`, `crates/alife_game_app/src/bin/alife_game_app.rs`, `crates/alife_game_app/src/app_bundle_ingestion.rs`, `crates/alife_game_app/src/ecological_soak.rs`, `crates/alife_game_app/src/gpu_graphics_performance.rs`, `crates/alife_game_app/src/onboarding_tutorial.rs`, `crates/alife_game_app/src/packaging_platform.rs`, `crates/alife_game_app/src/production_assets.rs`, `crates/alife_game_app/src/production_voxel_frontend.rs`, `crates/alife_game_app/src/release_candidate.rs`, `crates/alife_game_app/src/soak_isolation.rs`, `crates/alife_game_app/src/tests.rs` |
| Product package and launcher scripts | `scripts/run_production_voxel_frontend.ps1`, `scripts/package_windows_production_voxel.ps1`, `scripts/run_windows_production_voxel_package.ps1`, `scripts/run_graphical_playground.sh`, `scripts/package_windows_alpha.ps1`, `scripts/run_windows_alpha_package.ps1` |
| Tests and fixtures | `crates/alife_game_app/tests/app_shell.rs`, `crates/alife_game_app/tests/fvr08_final_cutover.rs`, `crates/alife_world/tests/fixtures/production_voxel/` |

Deleted/replaced old frontend files:

| Old surface | FVR08 disposition |
|---|---|
| `production-voxel` app command | Production path. It now owns finished launch, profile, save, renderer, GPU, asset, and performance receipt behavior. |
| `scripts/run_production_voxel_frontend.ps1` | Product Windows source launcher. Defaults to `MinSpecComfort1080p`, requests `auto-with-cpu-fallback`, includes `production-assets` and `vfx-hanabi`, and records performance by default. |
| `scripts/package_windows_production_voxel.ps1` and `scripts/run_windows_production_voxel_package.ps1` | New FVR08 Windows package builder and package-local runner. |
| `graphical-playground` command and `scripts/run_graphical_playground.ps1/.sh` | Compatibility aliases to the production voxel frontend. They are not product acceptance commands. |
| `scripts/package_windows_alpha.ps1` and `scripts/run_windows_alpha_package.ps1` | Preserved only as explicit legacy GPU Alpha regression package commands. |
| True 2.5D and alpha asset/status files | Preserved as historical/regression/reference surfaces. They do not own the desktop product path or FVR08 acceptance. |
| Commands named `smoke`, `alpha`, or `contract-only` | Preserved only as focused validation/regression commands where still valuable. Product acceptance uses production voxel commands and receipts. |

Public APIs changed:

- `alife_game_app` production feature stack now exposes `production-voxel-frontend` through `bevy-app`, `gpu-runtime`, `voxel-backend`, `production-assets`, and `vfx-hanabi`.
- `production-voxel` is the public production desktop command. `graphical-playground` is an alias with legacy compatibility diagnostics.
- Packaging APIs expose `fvr08-windows-production-voxel-package-dry-run` and `fvr08-windows-production-voxel-launcher-dry-run` through platform package smoke metadata.
- No public API in `alife_core` was changed for renderer, Bevy, wgpu, UI, window, or asset-loader types.

Saved-state/schema changes:

| Schema | FVR08 state |
|---|---|
| `alife.p34.save_file.v1` | Production voxel fixture save remains stable-ID based and stores world, creature, asset, and runtime references without Bevy entity or wgpu handles. |
| `alife.fvr06.gpu_runtime_state.v1` | Save state records selected backend mode, adapter identity, validation profile, residency slots, active profile caps, shader/ABI versions, CPU shadow parity, checkpoint, fallback reason, selected scale profile, compact readback bytes, and no-active-bulk-readback flag. |
| `alife.fvr05.production_ux_settings.v1` | Profile, UI, overlay, save/load, backend status, and inspector settings are persisted as portable JSON under the launch artifact root. Package runs now write these under the package-local `target/artifacts` tree. |
| Production voxel fixture | Added `crates/alife_world/tests/fixtures/production_voxel/` with small committed JSON fixture files and digest references. |

Assets/licenses changed:

- Production asset manifest path: `crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json`.
- Validation result: 16 production voxel assets, 16 generated assets, 0 external assets, 0 unknown licenses, 0 placeholder-final entries, 0 rejected entries, committed bytes 7310, largest asset 937 bytes, 5 VFX profiles, `no_large_artifacts=true`.
- License posture: generated/project assets only for FVR08; no unclear external license accepted.
- Large generated artifacts remain under `target/artifacts/` and are not committed.

Commands run:

| Command | Result |
|---|---|
| `cargo test -p alife_game_app --test fvr08_final_cutover -- --nocapture` | Passed, 3 tests. |
| `cargo test -p alife_game_app fvr08_package_launch_artifacts_are_rooted_next_to_packaged_manifest -- --nocapture` | Passed after package-local artifact root fix. |
| `cargo test -p alife_game_app --test app_shell ca42_launcher_scripts_run_preflight_and_keep_artifacts_untracked -- --nocapture` | Passed after restoring the CA42 legacy `runtime-prereq-smoke` doc reference as compatibility diagnostic. |
| `cargo test -p alife_game_app --features "bevy-app voxel-backend production-assets vfx-hanabi" --test fvr03_voxel_renderer -- --nocapture` | Passed, 3 tests, on rerun with longer compile timeout. |
| `cargo build --release -p alife_game_app --bin alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi"` | Passed. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_production_voxel.ps1 -SkipBuild` | Passed. Package path: `target/artifacts/fvr08_windows_production/alife-production-voxel-windows`. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File target/artifacts/fvr08_windows_production/alife-production-voxel-windows/run_windows_production_voxel_package.ps1 -RecordPerformance` | Passed. Wrote package-local FVR03/FVR05/FVR06 artifacts and selected `GpuFull`. |
| `target\release\alife_game_app.exe production-voxel --resolution 1920x1080 --profile MinimumSettings30x30 --population 30 --record-performance` | Passed. |
| `target\release\alife_game_app.exe production-voxel --resolution 1920x1080 --profile MinSpecComfort1080p --record-performance` | Passed. |
| `target\release\alife_game_app.exe production-voxel --profile HighSpecScaleUp --population 500 --record-performance` | Passed. |
| `cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -- validate-production-save --profile MinimumSettings30x30` | Passed. Real save loaded; no mock data source. |
| `cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -- validate-production-save --profile MinSpecComfort1080p` | Passed. Real save loaded; no mock data source. |
| `cargo run -p alife_game_app --features "bevy-app voxel-backend production-assets" --bin alife_game_app -- validate-production-assets` | Passed. |
| Population sweep, `production-voxel --resolution 1920x1080 --record-performance` for 1, 10, 30, 50, 100, 250, and 500 creatures | Passed. Receipts copied under `target/artifacts/fvr08/population_tiers`. |
| `cargo fmt --all -- --check` | Passed after formatting. |
| `cargo check --workspace --all-targets` | Passed. |
| `cargo test --workspace --all-targets` | Passed after the CA42 compatibility doc fix. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed after replacing approximate constants with `to_radians()` / `std::f32::consts::FRAC_PI_6`. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1` | Passed. |
| `cargo check --workspace --all-features --all-targets` | Passed. |
| `cargo test --workspace --all-features --all-targets` | First attempt timed out at 20 minutes without output; rerun with a longer timeout passed. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1` | Passed after this receipt was added. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1` | Passed after this receipt was added. |
| `cargo tree -p alife_core` | Passed. `alife_core` depends on `bitflags`, `bytemuck`, `serde`, `smallvec`, and `thiserror`; dev-dependency `serde_json` only. |

Hardware/GPU evidence:

- Target hardware class: NVIDIA RTX 3050 8 GB, Intel Core i7-3770K, 32 GB DDR3, Windows 10, 1920x1080.
- Measured adapter: `NVIDIA GeForce RTX 3050 (Vulkan, DiscreteGpu, 581.80)`.
- Production GPU mode: `auto-with-cpu-fallback`.
- Selected backend in source and package receipts: `GpuFull`.
- Fallback reason: `None`.
- CPU shadow parity failures: 0 in recorded production GPU gameplay receipts.
- Compact action readback: 1920 bytes for 30-creature validation batches.
- Active bulk neural readback: blocked, `no_active_bulk_readback=true`.
- Product runtime claim in GPU receipts: `CpuShadowGuardedStaticPlusLiveHShadow`; `full_action_authoritative_claim=false`, so no fake full-authority claim is made beyond the guarded production integration.

Performance results:

| Run | Target FPS | Measured FPS | Creatures | Visible chunks | VFX budget | Backend evidence |
|---|---:|---:|---:|---:|---|---|
| Package `MinSpecComfort1080p` | 60 | 214.68 | 30 | 100 | medium | `GpuFull`, RTX 3050, 0 parity failures, no bulk readback |
| Source `MinimumSettings30x30` | 30 | 248.79 | 30 | 36 | conservative | `GpuFull`, RTX 3050, 0 parity failures, no bulk readback |
| Source `MinSpecComfort1080p` | 60 | 127.25 | 30 | 100 | medium | `GpuFull`, RTX 3050, 0 parity failures, no bulk readback |
| Source `HighSpecScaleUp` | 60 | 100.52 | 500 | 768 | high | `GpuFull`, RTX 3050, 0 parity failures, no bulk readback |

Population tier results:

| Profile/tier | Target FPS | Measured FPS | Creatures | Visible chunks | VFX budget | GPU receipt |
|---|---:|---:|---:|---:|---|---|
| `MinimumSettings30x30`, 1 creature | 30 | 272.81 | 1 | 25 | conservative | No gameplay GPU receipt by design because the FVR06 batch receipt requires at least 2 creatures; command stdout selected `GpuFull`. |
| `MinimumSettings30x30`, 10 creatures | 30 | 127.88 | 10 | 36 | conservative | `GpuFull`, RTX 3050, 0 parity failures, no bulk readback |
| `MinimumSettings30x30`, 30 creatures | 30 | 248.79 | 30 | 36 | conservative | `GpuFull`, RTX 3050, 0 parity failures, no bulk readback |
| `Balanced1080p`, 50 creatures | 60 | 118.11 | 50 | 144 | balanced | `GpuFull`, RTX 3050, 0 parity failures, no bulk readback |
| `HighSpecScaleUp`, 100 creatures | 60 | 109.69 | 100 | 396 | high | `GpuFull`, RTX 3050, 0 parity failures, no bulk readback |
| `HighSpecScaleUp`, 250 creatures | 60 | 107.79 | 250 | 554 | high | `GpuFull`, RTX 3050, 0 parity failures, no bulk readback |
| `HighSpecScaleUp`, 500 creatures | 60 | 100.52 | 500 | 768 | high | `GpuFull`, RTX 3050, 0 parity failures, no bulk readback |

Profile and residency proof:

| Profile | Population posture | Residency/budget proof |
|---|---|---|
| `MinimumSettings30x30` | Hard floor: 30 real creatures at 30 FPS. Measured 248.79 FPS for 30 creatures. | Hot 4, warm 12, cold 14, neural heap 256 MB, chunk radius 2, active chunk cap 128, conservative VFX. |
| `MinSpecComfort1080p` | Default comfort profile for RTX 3050 / i7-3770K / Win10 / 1080p. Measured 127.25 FPS from source and 214.68 FPS from package. | Hot 8, warm 16, cold 6, neural heap 512 MB, chunk radius 4, active chunk cap 256, medium VFX. |
| `Balanced1080p` | Scale-up profile with 50 creatures. Measured 118.11 FPS. | Hot 12, warm 24, cold 14, neural heap 768 MB, active chunk cap 384, balanced VFX. |
| `HighSpecScaleUp` | Scale-up benchmark profile with 100, 250, and 500 creatures. The 500 tier measured 100.52 FPS. | Hot 24, warm 64, cold 412, neural heap 1536 MB, active chunk cap 768, high VFX. |
| `ResearchScale` | Non-default experimental mode preserved for future large-world/long-soak work. | Hot 32, warm 128, cold 340, neural heap 2048 MB, schema-compatible and no-readback rules preserved. |

Product launch instructions:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -Profile MinimumSettings30x30 -Population 30 -RecordPerformance
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_production_voxel.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File target/artifacts/fvr08_windows_production/alife-production-voxel-windows/run_windows_production_voxel_package.ps1
```

Package path:

```text
target/artifacts/fvr08_windows_production/alife-production-voxel-windows
```

Package contents include:

- `alife_game_app.exe`
- `run_windows_production_voxel_package.ps1`
- `scripts/run_production_voxel_frontend.ps1`
- `crates/alife_game_app/environment_manifest.json`
- `crates/alife_game_app/assets/production_voxel_v1`
- `crates/alife_world/tests/fixtures/production_voxel`
- `crates/alife_gpu_backend/shaders`
- `LICENSE`

Results:

- The desktop product path is the Bevy 0.18 production voxel frontend.
- The Windows source launcher and package runner default to `MinSpecComfort1080p`.
- `MinimumSettings30x30` remains available as the hard 30-creature / 30-FPS floor.
- Real production save validation loads the production voxel fixture and reports `real_save_loaded=true`, `mock_data_source=false`.
- Real production asset validation reports no unlicensed, placeholder-final, rejected, or large committed assets.
- The RTX 3050 path selects `GpuFull` only after real local probe/validation, with CPU fallback diagnostics available and visible.
- Population tiers 1, 10, 30, 50, 100, 250, and 500 run through the production voxel command and write honest receipts under ignored `target/artifacts/`.

Boundary invariants:

- `alife_core` remains engine-independent. No Bevy, Avian, wgpu, renderer, UI, OS window, or asset-loader dependency was added.
- `alife_world` owns saved world/chunk truth and uses stable IDs; it does not persist Bevy entities, wgpu handles, renderer handles, or window handles.
- Bevy renderer, UI, VFX, and inspector surfaces are display/read-only for cognition and cannot issue hidden actions, rewrite weights, inject rewards, bypass action arbitration, or mutate hidden state.
- `alife_gpu_backend` owns neural wgpu/WGSL runtime work and remains separate from Bevy renderer internals.
- Active gameplay does not perform bulk neural, per-synapse, per-lobe, or weight readback.
- No mock simulation, fake backend, fake GPU availability, or fake population path is used for product acceptance.
- `git ls-files target/artifacts target/generated_art` returned no tracked generated artifacts; `target/` remains ignored.

Deviations:

- The first `cargo test -p alife_game_app --features "bevy-app voxel-backend production-assets vfx-hanabi" --test fvr03_voxel_renderer -- --nocapture` attempt timed out while compiling. The rerun with a longer timeout passed.
- The first `cargo test --workspace --all-targets` failed one CA42 launcher-doc regression because `docs/playable_sim_spec/platform_packaging.md` no longer mentioned the legacy `runtime-prereq-smoke` diagnostic. The doc was corrected to preserve CA42 compatibility, and the full command passed on rerun.
- The first `cargo clippy --workspace --all-targets -- -D warnings` failed on approximate constants in `production_voxel_frontend.rs`. The code now uses `to_radians()` and `std::f32::consts::FRAC_PI_6`; clippy passed on rerun.
- The first `cargo test --workspace --all-features --all-targets` attempt hit a 20-minute command timeout without output. No Cargo/test processes remained afterward. The same exact command passed on rerun with a longer timeout.
- Population 1 has no copied FVR06 gameplay GPU receipt because the production GPU gameplay receipt intentionally skips batches below 2 creatures. The production command still ran that tier and selected `GpuFull`; tiers 10 and above have GPU gameplay receipts.

Known limitations:

None for FVR08 owned scope.

FVR08 acceptance statement:

The old ugly frontend has been replaced by the production Bevy 0.18 voxel frontend as the desktop product path. `MinimumSettings30x30` establishes the 30-creature / 30-FPS hard floor, `MinSpecComfort1080p` establishes the default minimum-spec comfort path on the RTX 3050 evidence machine, and profile-driven scale-up remains preserved for later work.

## FVR09 Post-Acceptance Addendum

Status: FVR09 complete.

FVR09 keeps the FVR08 production desktop path and upgrades the visual/performance implementation in place:

- Production terrain now uses a material-aware greedy mesh path with chunk-local occupancy masks, six-direction face-mask instrumentation, material-compatible rectangle merging, remesh budget diagnostics, cache-key diagnostics, and stable tile-coordinate selection.
- Production terrain materials now use generated natural material definitions and texture-slot metadata under `fvr09-natural-materials-v1`; primary colors are retained only as explicit debug-color metadata.
- Production creature visuals now use `fvr09-cute-biped-v1` and `fvr09-soft-biped-materials-v1`, with a composite low-poly biped mesh, eye/mouth features, and real creature stable IDs/state driving animation and expression.
- No external art assets, mock simulation, fake backend, fake GPU path, fake population path, or renderer authority over cognition/actions were added.

FVR09 measured release performance on the RTX 3050 evidence machine:

| Profile/tier | Target FPS | Measured FPS | Creatures | Active chunks | Emitted quads | Merge ratio | Backend |
|---|---:|---:|---:|---:|---:|---:|---|
| `MinimumSettings30x30`, 30 creatures | 30 | 204.64 | 30 | 36 | 816 | 4.235 | `GpuFull` |
| `MinSpecComfort1080p`, 30 creatures | 60 | 206.22 | 30 | 100 | 1890 | 5.079 | `GpuFull` |
| `HighSpecScaleUp`, 500 creatures | 60 | 152.34 | 500 | 768 | 24486 | 12.044 | `GpuFull` |

The detailed completion receipt is `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR09_COMPLETION.md`.
