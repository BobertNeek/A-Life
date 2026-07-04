# FVR03 Completion Receipt - Finished Default Voxel World Renderer

Status: complete
Branch: `codex/fvr03-default-voxel-renderer`
Primary checkout: `D:\A life`

## Scope

FVR03 replaces the FVR01 text-only graphical production boot scene with the
default Bevy 0.18 voxel world renderer. The production app now opens a
stylized, selectable, persistence-backed voxel terrain view built from real
FVR02 `alife_world` persistent chunk snapshots.

The renderer remains a consumer. World truth, save validation, stable chunk
coordinates, stable tile coordinates, resource/hazard references, and dirty
chunk metadata remain owned by `alife_world`. No Bevy entity, renderer handle,
window handle, wgpu resource, or mesh/material handle is serialized into core or
world state.

## Files Changed

- `crates/alife_game_app/src/bevy_shell.rs`
- `crates/alife_game_app/src/lib.rs`
- `crates/alife_game_app/src/production_voxel_renderer.rs`
- `crates/alife_game_app/tests/fvr03_voxel_renderer.rs`
- `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR03_COMPLETION_RECEIPT.md`

## Deleted Or Replaced Old Frontend Files

- Replaced the FVR01 production voxel boot scene that spawned only a 2D camera
  and launch-diagnostics text.
- True2.5D and older graphical player-view code remains available only as
  historical/debug surface. It no longer owns the production voxel app path.

## Public APIs Changed

- Added `FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA`.
- Added `FVR03_RENDERER_BACKEND_ID`.
- Added `Fvr03ProductionVoxelRendererSettings`.
- Added `Fvr03ProductionVoxelSceneResource`.
- Added `Fvr03ProductionVoxelSelectionResource`.
- Added `Fvr03ProductionVoxelTerrainTile`.
- Added `Fvr03ProductionVoxelTerrainBatch`.
- Added `Fvr03ProductionVoxelChunk`.
- Added `Fvr03ProductionVoxelCamera`.
- Added `spawn_fvr03_production_voxel_scene`.

## Saved-State And Schema Changes

No save-file schema changed in FVR03. The renderer consumes the FVR02
`alife.fvr02.persistent_voxel_world.v1` saved backend and writes only runtime
diagnostic artifacts under `target/artifacts/fvr03/`.

FVR03 runtime diagnostics use schema
`alife.fvr03.production_voxel_renderer.v1` and include:

- selected frontend profile
- renderer backend id
- target FPS
- visible and resident chunk counts
- rendered tile count
- estimated resident renderer bytes
- local smoke measured FPS
- local smoke measurement frame count and seconds
- explicit non-broad-claim performance status

## Renderer Backend

`bevy_voxel_world` remains wired as the Bevy 0.18 voxel-stack integration point,
but the visible production renderer uses the FVR03 internal Bevy chunk-mesh
path for FVR02 snapshot correctness. Direct `VoxelWorldCamera` chunk generation
was not used for the final visible path because it duplicated terrain work and
did not preserve the FVR02 stable chunk/tile selection contract cleanly.

The final visible renderer:

- streams the active persistent chunk window returned by FVR02 snapshots
- materializes only resident chunks for the selected profile
- batches terrain into generated material meshes instead of per-tile draw calls
- keeps hidden per-tile entities for stable selection/query contracts
- renders water, sand, safe grass, soil, stone, resource, hazard, and decay
  materials
- renders chunk-boundary surfaces and stable creature markers
- supports orthographic isometric and orbit camera modes
- resolves selected tiles to stable chunk/tile coordinates without Bevy entity
  leakage
- uses conservative no-shadow lighting for minimum and comfort profiles, with
  shadow support left enabled for higher profiles

## Profile Results

Observed on local hardware:

- GPU: `NVIDIA GeForce RTX 3050`
- API: `Dx12`
- Selected backend: `GpuPlastic`
- Fallback: `None`
- Resolution: `1920x1080`
- Build: `cargo run --release`

| Profile | Chunks | Rendered tiles | Estimated bytes | Measured FPS | Target FPS | Screenshot |
|---|---:|---:|---:|---:|---:|---|
| `MinimumSettings30x30` | 35 | 560 | 256512 | 59.89 | 30 | `target/artifacts/fvr03/MinimumSettings30x30_runtime_screenshot.png` |
| `MinSpecComfort1080p` | 99 | 1584 | 485888 | 60.05 | 60 | `target/artifacts/fvr03/MinSpecComfort1080p_runtime_screenshot.png` |

Diagnostic JSON artifacts:

- `target/artifacts/fvr03/MinimumSettings30x30_renderer_diagnostics.json`
- `target/artifacts/fvr03/MinSpecComfort1080p_renderer_diagnostics.json`

Blueprint artifact used for the GUI design comparison:

- `target/artifacts/fvr03/fvr03_voxel_world_blueprint.png`

These artifacts are generated under `target/` and are not committed.

## App Save/Load Evidence

`validate-production-save` reports real FVR02 backend evidence for both primary
profiles:

- `real_save_loaded=true`
- `mock_data_source=false`
- `voxel_backend_schema=alife.fvr02.persistent_voxel_world.v1`
- `voxel_roundtrip=true`
- `voxel_renderer_tokens_saved=false`

Profile evidence:

- `MinimumSettings30x30`: 35 visible/materialized chunks, 39
  resource/hazard refs, 77 stable selection refs.
- `MinSpecComfort1080p`: 99 visible/materialized chunks, 71
  resource/hazard refs, 173 stable selection refs.

## Validation Receipt

Passing commands in this goal:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo test --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo check --workspace --all-targets --all-features`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`
- `cargo test -p alife_game_app --features "bevy-app voxel-backend" fvr03 -- --nocapture`
- `cargo test -p alife_game_app --features "bevy-app voxel-backend" voxel -- --nocapture`
- `cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinimumSettings30x30`
- `cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinSpecComfort1080p`
- `cargo run --release -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinimumSettings30x30 --population 30 --smoke-seconds 60 --record-performance`
- `cargo run --release -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinSpecComfort1080p --population 30 --smoke-seconds 60 --record-performance`
- `C:\Users\PC\.local\bin\graphify.exe update .`

## Boundary Invariants

- No Bevy or wgpu types were added to `alife_core`.
- No Bevy or wgpu types were added to `alife_world`.
- The renderer does not own actions, cognition, persistence truth, or world
  legality.
- Selection labels expose stable chunk/tile coordinates and do not expose Bevy
  `Entity` tokens.
- The production path uses real P34/FVR02 saves and real GPU/backend
  diagnostics, not mock simulation or fake backend data.
- No large generated artifacts are committed.

## Deviations

- `bevy_voxel_world` is kept as a wired integration dependency/config resource,
  but the final visible terrain path is the internal FVR03 chunk-mesh renderer.
  This was required to satisfy the FVR02 stable selection and persistence
  contract without duplicate terrain generation.

## Known Limitations

None for FVR03 scope. FVR04 owns production creature mesh/animation batching;
FVR03 provides stable visible creature markers on the voxel terrain.

## FVR04 Readiness

FVR04 can start without more renderer planning after this branch is merged.
