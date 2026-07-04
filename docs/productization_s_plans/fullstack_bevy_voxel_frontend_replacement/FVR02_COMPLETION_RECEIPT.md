# FVR02 Completion Receipt - Persistent Voxel World Backend

Status: complete
Branch: `codex/fvr02-persistent-voxel-backend`
Primary checkout after repo move: `D:\A life`

## Scope

FVR02 replaced the frontend-only procedural chunk truth with a renderer-independent
persistent voxel world backend owned by `alife_world`. The new backend stores the
procedural ruleset identity, stable chunk signatures, bounded materialized chunk
metadata, compact tile edits, dirty regions, ecology/resource overlays, and
stable object references needed by later voxel renderer goals.

The renderer remains a consumer. No Bevy, wgpu, renderer handle, window handle,
or engine-local entity type is serialized into the FVR02 world backend.

## Files Changed

- `crates/alife_world/src/persistent_voxel.rs`
- `crates/alife_world/src/lib.rs`
- `crates/alife_world/src/persistence.rs`
- `crates/alife_world/tests/fvr02_persistent_voxel_backend.rs`
- `crates/alife_game_app/src/lib.rs`
- `crates/alife_game_app/src/production_voxel_frontend.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/tests/fvr02_production_save.rs`
- `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR02_COMPLETION_RECEIPT.md`

## Deleted Or Replaced Frontend Files

No old renderer files were deleted in FVR02. This goal added the backend truth
needed before renderer deletion/cutover goals can safely remove legacy visual
surfaces.

## Public APIs

- Added `alife_world::persistent_voxel`.
- Exported `FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA`,
  `FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA_VERSION`,
  `FVR02_GENERATOR_RULESET_ID`, and `FVR02_GENERATOR_RULESET_VERSION`.
- Added `PersistentVoxelWorldBackend` with deterministic construction from a
  world seed and `PersistentVoxelProfileId`.
- Added `PersistentVoxelWorldSaveState` and save validation for schema,
  generator version, profile budget, materialized chunk caps, dirty regions,
  stable references, and compact tile edits.
- Added deterministic `chunk_signature` and `snapshot_for_anchors` contracts.
- Added `PortableSaveFile::require_voxel_backend` and
  `PortableSaveFile::with_migrated_voxel_backend`.

## Saved-State And Schema Changes

- `WorldSaveState` now contains optional
  `voxel_backend: Option<PersistentVoxelWorldSaveState>`.
- Existing P34 saves remain readable as legacy saves.
- FVR02 migration attaches a generated backend using the existing save seed and
  requested profile.
- `WorldSaveState::validate` validates present FVR02 backend state and rejects
  seed mismatches.
- The production save preflight now roundtrips the FVR02 backend JSON and checks
  chunk signatures before reporting success.

## Backend Contract

- `alife_world` owns voxel world truth and derives chunks from the existing
  deterministic procedural chunk rules rather than a mock simulation.
- Far chunks remain virtual; `allocates_far_chunks()` returns `false`.
- Materialized chunks are bounded by the selected profile budget.
- Snapshots contain only compact streaming data and stable references for
  renderer/adapter consumers.
- Dirty regions are explicit, tile-scoped, and save-backed.
- Resource hazards and selectable objects use stable IDs and do not contain
  renderer authority over actions or cognition.

## Profiles

- `MinimumSettings30x30`: 30 creatures, 1080p, 30 FPS target, chunk radius 2,
  active chunk cap 128, hot/warm/cold budget 4/12/14.
- `MinSpecComfort1080p`: 30 creatures, 1080p, 60 FPS target, chunk radius 4,
  active chunk cap 256, hot/warm/cold budget 8/16/6.
- Scale-up profiles increase chunk radius and materialization caps without
  treating the minimum profile as a ceiling.

## App Save/Load Evidence

`validate-production-save` now reports FVR02 backend evidence:

- `voxel_backend_schema`
- `voxel_chunks`
- `voxel_materialized`
- `voxel_resource_hazard_refs`
- `voxel_selection_refs`
- `voxel_dirty_regions`
- `voxel_roundtrip`
- `voxel_renderer_tokens_saved`

Hardware path observed from the `D:\A life` checkout:

- Adapter: `NVIDIA GeForce RTX 3050`
- API: `Dx12`
- Selected backend: `GpuPlastic`
- Fallback: `None`

Profile results observed:

- `MinimumSettings30x30`: 35 visible chunks, 35 materialized chunks, 39
  resource/hazard refs, 77 stable selection refs, backend roundtrip true,
  renderer tokens saved false.
- `MinSpecComfort1080p`: 99 visible chunks, 99 materialized chunks, 71
  resource/hazard refs, 173 stable selection refs, backend roundtrip true,
  renderer tokens saved false.

## Validation Receipt

Passing commands in this goal:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo test --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo check --workspace --all-features --all-targets`
- `cargo test --workspace --all-features --all-targets`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`
- `cargo test -p alife_world --test fvr02_persistent_voxel_backend -- --nocapture`
- `cargo test -p alife_world --test save_load_roundtrip -- --nocapture`
- `cargo test -p alife_world procedural_chunks -- --nocapture`
- `cargo test -p alife_game_app --test fvr02_production_save --features "bevy-app gpu-runtime voxel-backend" -- --nocapture`
- `cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinimumSettings30x30`
- `cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinSpecComfort1080p`

Additional command outcome:

- `cargo test --workspace --all-features --all-targets` was attempted before
  the repo move and failed during third-party Bevy/physics compilation because
  the C: drive ran out of disk space. The failure was an OS write failure:
  `There is not enough space on the disk. (os error 112)`, not a test failure.
- After moving the checkout to `D:\A life`, the same command was rerun with a
  longer timeout and passed.

The final validation receipt was run from the `D:\A life` checkout after
generated build artifacts and merged dead branches were cleaned.

## Boundary Invariants

- No Bevy or wgpu type is introduced into `alife_core`.
- No Bevy or wgpu type is introduced into `alife_world`.
- `alife_world` remains the owner of saved voxel world truth.
- `alife_game_app` reports backend facts but does not own cognition or action
  authority.
- Renderer tokens are rejected from the FVR02 backend save JSON.
- No large generated artifacts are committed.

## Branch And Disk Hygiene

- Removed generated `target/` output from the old C: checkout.
- Deleted merged local branches.
- Removed clean, merged worktrees that were blocking branch deletion.
- Preserved the active `codex/fvr02-persistent-voxel-backend` branch.
- Preserved unmerged local branch `codex/P22-semantic-gaussian-adapter`.
- Copied the repo to `D:\A life` without `target/`.
- Removed the old C: checkout contents. The empty old root folder remains locked
  by Windows and can be removed after the owning process releases it.

## FVR03 Readiness

FVR03 can start without more backend planning after this branch is merged.
