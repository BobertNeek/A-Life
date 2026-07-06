# FVR09 Completion

Status: FVR09 complete.

## Completion Receipt

Plan: FVR09 - Greedy Meshing, Natural World Textures, and Cute Bipedal Creature Redesign

Branch: `codex/FVR09-greedy-meshing-natural-world-cute-bipeds`

Files changed:

| Area | Files |
|---|---|
| Production voxel renderer | `crates/alife_game_app/src/production_voxel_renderer.rs` |
| Renderer tests | `crates/alife_game_app/tests/fvr03_voxel_renderer.rs` |
| Production voxel assets | `crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json`, `crates/alife_game_app/assets/production_voxel_v1/palette/*.json`, `crates/alife_game_app/assets/production_voxel_v1/creatures/creature_visuals.json` |
| Production save fixtures | `crates/alife_world/tests/fixtures/production_voxel/tiny_asset_manifest.json`, `crates/alife_world/tests/fixtures/production_voxel/tiny_save.json` |
| FVR docs | `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR09_COMPLETION.md`, `README.md`, `FINAL_ACCEPTANCE.md` |

Deleted/replaced crude visual files:

| Old/crude surface | FVR09 disposition |
|---|---|
| FVR03 terrain tile mesh path | Replaced in the production path by material-aware greedy terrain mesh generation. The old sampled-tile summary still exists as renderer input and stable selection data, but production terrain no longer spawns one cuboid mesh per sampled tile. |
| Primary-color production terrain palette | Replaced by generated natural material definitions with atlas version `fvr09-natural-materials-v1`, top/side texture slots, deterministic variation seeds, and explicit debug-color metadata for overlay use only. |
| Single-box creature body mesh | Replaced by a composite low-poly bipedal mesh with compact body, large head, legs, feet, side appendages, and face feature entities driven by real creature stable IDs and state. |
| External/large art artifacts | None added. FVR09 uses generated/project JSON material definitions and procedural mesh generation; large screenshots and performance receipts remain under ignored `target/artifacts/`. |

Implementation note from FVR08 inventory:

- The pre-FVR09 production path used an internal Bevy chunk-mesh backend. `bevy_voxel_world` remained available in the dependency stack, but FVR03/FVR08 product rendering was driven by app-owned chunk summaries and batched cuboid meshes.
- Terrain was sampled through profile-driven tile stride and chunk radius. Before FVR09, `MinimumSettings30x30` used stride 4, `MinSpecComfort1080p` used stride 16, `Balanced1080p` and `HighSpecScaleUp` used stride 2, and `ResearchScale` used stride 4. FVR09 keeps `MinimumSettings30x30` at stride 4 and raises the default comfort view to stride 4 so natural terrain remains dense enough to read at 1080p.
- Dirty-region data already existed in world snapshots as renderer-independent chunk/tile metadata. FVR09 adds renderer instrumentation for dirty, cached, and skipped remesh counts plus a profile/material/chunk-version cache key; it does not serialize Bevy entities or renderer handles.
- Selection and ray/picking still resolve camera hits to stable world tile coordinates and stable selectable refs. Saves/core state continue to store world and creature identifiers, not Bevy `Entity` values.
- FVR08 performance evidence established `MinimumSettings30x30` at 248.79 FPS for 30 creatures and `MinSpecComfort1080p` at 127.25 FPS from source / 214.68 FPS from package on the RTX 3050 evidence machine.

Mesher implementation:

| Requirement | FVR09 result |
|---|---|
| Binary/occupancy masks | Implemented CPU-side row occupancy masks with `u64` bitfields per material bucket and chunk-local tile coordinates. |
| Six-direction face visibility | The production stats report all six direction masks as active. Top, bottom, and side surfaces are emitted through merged rectangular prism meshes with neighbor-aware extents. |
| Material-aware greedy merge | Implemented by `fvr09_material_greedy_prisms`: only same material and compatible visual-height buckets merge. Natural top/side texture slots remain material-specific. |
| Neighbor seams | Chunk border extents are included in the mesh cache key and stats report `neighbor_seams_checked=true`; selection remains tile-coordinate based rather than mesh-entity based. |
| Dirty bounded remeshing | Stats record dirty regions, cached chunks, skipped chunks, and `remesh_budget_chunks_per_frame` from the active profile settings. |
| Cache invalidation | Cache key includes profile name, material palette version, chunk size, tile stride, active chunk count, dirty region count, chunk version sum, and coordinate sum. |
| Instrumentation | Receipts include visible voxels, naive visible faces, emitted quads, merge ratio, remesh time, dirty/cached/skipped chunk counts, active chunks, and material version. |

Terrain material/texture implementation:

| Material class | Production slot coverage |
|---|---|
| Grass | Natural green/brown material with `grass-moss-top` and `dirt-rooted-side` texture slots. |
| Soil/dirt | `soil-granular-top` and `soil-layered-side` texture slots with deterministic variation seed. |
| Stone/rock | `stone-speckled-top` and `stone-fracture-side` texture slots. |
| Sand/dry soil | `sand-ripple-top` and `sand-packed-side` texture slots. |
| Water/wet ground | `water-soft-ripple-top` and `wet-bank-side` texture slots. |
| Decay | `decay-leaf-mat-top` and `decay-root-side` texture slots. |
| Resource vegetation | `resource-sprout-top` and `resource-vine-side` texture slots. |
| Hazards | `hazard-bristle-top` and `hazard-warning-side` texture slots. |

Production colors were muted away from primary debug colors. Debug RGBA values remain recorded in the palette metadata for explicit debug modes, not as the default product look.

Creature mesh/material implementation:

- Creature visuals now use `fvr09-cute-biped-v1` with `fvr09-soft-biped-materials-v1`.
- The base creature mesh is a generated composite low-poly biped: rounded compact body, larger head, two legs, two feet, side appendages, and a face-facing silhouette.
- Eye and mouth meshes are spawned as separate lightweight visual entities using the same real creature stable ID and state marker. They follow animation through local offsets, not mock data.
- Creature state remains read-only from the renderer perspective. Hunger, fear/danger, fatigue/sleep, valence, reproduction, social, death, and interaction cues are mapped to material/emissive tint and animation scale without mutating cognition or actions.
- Profile behavior remains bounded: `MinimumSettings30x30` keeps cheap mesh/material counts, `MinSpecComfort1080p` uses the default cute biped profile, and higher profiles can keep additional detail while preserving real creature identity.

Asset/license changes:

- Production asset manifest version now records FVR09 generated material palettes and creature visual metadata.
- No external art assets were added. All FVR09 assets are generated/project JSON definitions under the existing production voxel asset directory.
- Manifest entries include digest, size, generated source, license, usage, replacement policy, and FVR09 metadata.
- Large generated artifacts were not committed; performance JSON and screenshots stay under ignored `target/artifacts/`.

Saved-state/schema changes:

| Schema/surface | FVR09 state |
|---|---|
| Production save schema | No save schema version change. Stable world, creature, runtime, and asset references are preserved. |
| Material IDs | Existing saved material IDs remain renderer-independent. FVR09 adds renderer palette and texture-slot metadata in assets/receipts, not renderer handles in saves. |
| Backend/runtime state | No neural/runtime schema redesign. GPU backend selection, CPU fallback diagnostics, residency slots, and no-bulk-readback receipts remain under the FVR06/FVR08 contracts. |
| Fixture digests | The tiny production voxel fixture manifest/save digest was corrected to match the committed production asset manifest bytes. |
| Renderer receipts | Performance artifacts now include FVR09 mesher stats, material atlas version, creature visual profile, and creature mesh/material version. |

Commands run:

| Command | Result |
|---|---|
| `cargo test -p alife_game_app --features "bevy-app voxel-backend production-assets vfx-hanabi" --test fvr03_voxel_renderer fvr09_ -- --nocapture` | First TDD run failed as expected before implementation because FVR09 types, mesh stats, material texture slots, and cute biped metadata did not exist. Reruns passed, 3 tests. |
| `cargo test -p alife_game_app production_asset_manifest_validates_license_digest_and_vfx_contract` | Passed after production asset manifest digest updates. |
| `cargo fmt --all -- --check` | Passed on the final tree. |
| `cargo check --workspace --all-targets` | Passed on the final tree. |
| `cargo test --workspace --all-targets` | Passed on the final tree. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed on the final tree. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1` | Passed on the final tree. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1` | Passed on the final tree; `alife_core boundary checks passed`. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1` | Passed on the final tree. |
| `cargo tree -p alife_core` | Passed. `alife_core` depends on `bitflags`, `bytemuck`, `serde`, `smallvec`, and `thiserror`; dev-dependency `serde_json` only. |
| `cargo check --workspace --all-features --all-targets` | Passed on the final tree. |
| `cargo test --workspace --all-features --all-targets` | Final rerun passed on the final tree. The first all-features attempt failed one stale FVR03 profile-scaling assertion after the FVR09 comfort-profile terrain density increase; the assertion was corrected and the command passed on rerun. |
| `cargo test -p alife_game_app --all-features --test fvr03_voxel_renderer fvr03_profiles_scale_renderer_residency_lod_and_camera_modes -- --nocapture` | Passed after correcting the stale profile-scaling assertion. |
| `cargo run --release -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinimumSettings30x30 --population 30 --record-performance` | Passed. |
| `cargo run --release -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinSpecComfort1080p --record-performance` | Passed. |
| `cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -- validate-production-save --profile MinimumSettings30x30` | Passed. Real save loaded; mock data source false. |
| `cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -- validate-production-save --profile MinSpecComfort1080p` | Passed. Real save loaded; mock data source false. |
| Population sweep for 1, 10, 30, 50, 100, 250, and 500 creatures | Passed. Receipts copied under `target/artifacts/fvr09/population_tiers/`. |

Hardware/GPU evidence:

- Target hardware class: NVIDIA RTX 3050 8 GB, Intel Core i7-3770K, 32 GB DDR3, Windows 10, 1920x1080.
- Measured adapter in FVR09 receipts: `NVIDIA GeForce RTX 3050 (Vulkan, DiscreteGpu, 581.80)`.
- Selected backend in production receipts: `GpuFull`.
- Fallback reason: `None`.
- Active bulk neural readback: blocked by the existing FVR06/FVR08 no-bulk-readback contract.
- Evidence artifacts copied under ignored `target/artifacts/fvr09/population_tiers/`.

Performance results:

| Profile/tier | Target FPS | Measured FPS | Creatures | Active chunks | Tile mesh count | Emitted quads | Merge ratio | Renderer backend |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| `MinimumSettings30x30`, 1 creature | 30 | 227.92 | 1 | 25 | 400 | 666 | 3.604 | `GpuFull` |
| `MinimumSettings30x30`, 10 creatures | 30 | 257.69 | 10 | 36 | 576 | 816 | 4.235 | `GpuFull` |
| `MinimumSettings30x30`, 30 creatures | 30 | 204.64 | 30 | 36 | 576 | 816 | 4.235 | `GpuFull` |
| `Balanced1080p`, 50 creatures | 60 | 146.72 | 50 | 144 | 2304 | 5898 | 9.375 | `GpuFull` |
| `HighSpecScaleUp`, 100 creatures | 60 | 156.81 | 100 | 396 | 6336 | 13062 | 11.642 | `GpuFull` |
| `HighSpecScaleUp`, 250 creatures | 60 | 153.49 | 250 | 554 | 8864 | 17298 | 12.298 | `GpuFull` |
| `HighSpecScaleUp`, 500 creatures | 60 | 152.34 | 500 | 768 | 12288 | 24486 | 12.044 | `GpuFull` |

MinimumSettings30x30 result:

- Required floor: 30 real creatures at 30 FPS with real world, real saves, real backend selection/fallback, visible voxel terrain, creature interaction, essential UI, and no mock simulation/backend.
- FVR09 result: 30 real creatures at 204.64 FPS in release at 1920x1080 on the RTX 3050 evidence machine.
- Mesh result: 36 active chunks, 576 sampled terrain tiles, 816 emitted quads, 4.235 merge ratio, 676 us remesh timing, 30 cute biped creature visuals.

MinSpecComfort1080p result:

- Required default comfort profile: smooth 1080p operation on RTX 3050 8 GB / i7-3770K / Win10 without manual feature-disablement.
- FVR09 result: 30 real creatures at 206.22 FPS in release at 1920x1080 on the RTX 3050 evidence machine.
- Mesh result: 100 active chunks, 1600 sampled terrain tiles, 1890 emitted quads, 5.079 merge ratio, 1615 us remesh timing, 30 cute biped creature visuals.

Visual validation:

- A generated blueprint image was used as the visual target for natural voxel terrain and cute biped silhouettes: `C:\Users\PC\.codex\generated_images\019f2a54-ead6-76d1-a32a-51fb7a56cc1a\ig_03d93ae6ec538596016a4aef93d9d8819aac30ac05c1164924.png`.
- The refreshed production runtime screenshot is `target/artifacts/fvr03/MinSpecComfort1080p_runtime_screenshot_fvr05_world_inspector.png`.
- The screenshot shows FVR09 diagnostics, denser default comfort terrain sampling, muted natural material palette, and biped creature silhouettes with face/feet cues. The implementation is intentionally procedural/material-slot based rather than external hand-authored texture art.

Boundary invariants:

- `alife_core` remains engine-independent. No Bevy, Avian, wgpu, renderer, UI, OS window, asset handle, or Bevy `Entity` dependency was added.
- `alife_world` remains renderer-independent. It owns saved world/chunk truth and stable IDs; it does not persist Bevy entities or renderer handles.
- Bevy renderer, VFX, UI, and debug overlays remain display/read-only for cognition and cannot issue hidden actions, rewrite weights, inject rewards, bypass arbitration, or mutate hidden state.
- `alife_gpu_backend` remains responsible for neural/runtime wgpu/WGSL work and separate from Bevy renderer internals.
- Active gameplay still avoids bulk neural, per-synapse, per-lobe, and weight readback.
- No mock simulation, fake backend, fake GPU availability, fake population, or visual-only stand-in path was added.
- No external unlicensed assets or large generated artifacts were committed.

Deviations:

- Development-mode renderer runs passed but measured lower FPS than release, so release receipts are used for performance acceptance. The target profile gates are release/product gates.
- FVR09 uses generated material palette definitions and texture-slot IDs rather than importing external hand-authored texture images. This keeps licensing clean and preserves deterministic procedural asset generation.
- The committed tiny fixture asset digest was stale before FVR09; it was corrected to match the committed production asset manifest so real-save validation exercises the current manifest bytes.
- The first `cargo test --workspace --all-features --all-targets` run failed `fvr03_profiles_scale_renderer_residency_lod_and_camera_modes` because the pre-FVR09 assertion expected `MinimumSettings30x30` to have an equal-or-higher tile budget than `MinSpecComfort1080p`. FVR09 intentionally made default comfort terrain denser while keeping the minimum floor conservative; the test now asserts comfort and balanced budgets scale upward. The focused rerun and final all-features rerun passed.

Known limitations:

None for FVR09 owned scope.

FVR09 acceptance statement:

FVR09 replaces the crude production voxel terrain and creature visuals with material-aware greedy terrain meshing, natural generated material slots, and real-state cute bipedal creature visuals while preserving the minimum floor, default comfort profile, GPU backend integration, save boundaries, and future scale-up profiles.
