# FVR00 Replacement Blueprint

Status: FVR00 complete, planning-only.

Scope: this file is the single scaffolding/review pass for replacing the old
graphical frontend with a finished Bevy 0.18 voxel fullstack frontend. It does
not implement the replacement. Later FVR plans must implement against this map
without another broad planning pass.

No ADR update is required for FVR00. This blueprint applies existing ADRs,
especially ADR-001, ADR-019, ADR-020, and ADR-023. It does not change the
project architecture: `alife_core` stays engine-independent, `alife_world` stays
renderer-independent, `alife_gpu_backend` owns neural wgpu/WGSL work, and
Bevy/voxel/UI work stays in `alife_bevy_adapter` and `alife_game_app`.

## Controlling Inputs

- Root `AGENTS.md`, `docs/AGENTS.md`, `docs/master_spec.md`, and
  `docs/architecture_decisions.md`.
- FVR plan pack `README.md` and `ACCEPTANCE_AND_VALIDATION.md`.
- Current crates: `alife_game_app`, `alife_bevy_adapter`, `alife_world`, and
  `alife_gpu_backend`.
- Existing GPU-alpha, True 2.5D, asset-pipeline, status, launcher, and save
  surfaces.

Non-negotiables carried forward:

- No mock simulation, fake backend, fake GPU availability, or placeholder
  art-as-final.
- No new production naming that says alpha, smoke, or contract-only.
- No Bevy, Avian, wgpu, renderer, UI, OS window, or asset-loader types in
  `alife_core`.
- No renderer, UI, VFX, debug, or inspector authority over actions, rewards,
  learning, cognition, or world legality.
- No large generated artifacts committed. Generated or external assets require
  manifests, digests, source/origin, license, and replacement policy.

## Current Frontend Inventory

### Workspace and Crates

| Surface | Current state | FVR consequence |
|---|---|---|
| Root workspace | Bevy is pinned as `bevy = "0.18.0"` with `default-features = false`. `wgpu = "29.0.3"` is a workspace dependency for `alife_gpu_backend`. | Keep Bevy 0.18 for the production app. Do not share Bevy renderer wgpu types with neural backend wgpu contracts. |
| `alife_game_app` | Owns app shell, launch policy, player-facing commands, asset manifests, True 2.5D and GPU-alpha presentation, saved UX smoke commands, packaging commands, and Bevy feature gating. | FVR01 starts here by adding production voxel launch/profile wiring and retiring old graphical paths from the product route. |
| `alife_bevy_adapter` | Bevy/Avian boundary. Converts Bevy ECS observations into stable core sensory contracts and converts core actions into engine-side plans/outcomes. | Reuse boundary patterns. Add voxel/player presentation adapters here only when they need Bevy ECS integration. |
| `alife_world` | Bevy-independent headless world, ecology, persistence, stable IDs, procedural chunk/context reports, action legality/outcome authority. | FVR02 extends this crate with saved voxel chunk truth and adapter snapshots. It must not import renderer types. |
| `alife_gpu_backend` | Owns wgpu/WGSL neural contracts, GPU runtime probe/fallback, no-active-readback guard, CPU-shadow parity, post-seal H_shadow path, and performance-tier reports. | FVR06 extends production backend descriptors and RTX 3050 receipts. It does not become world or renderer authority. |
| `alife_core` | Engine-agnostic IDs, brain classes, action/sensory contracts, ExperiencePatch, sparse neural schemas, lifetime delta contracts. | No FVR production renderer dependency may be added here. |

### Current Feature Flags

| Feature | Current owner | Current meaning | FVR disposition |
|---|---|---|---|
| `bevy-app` | `alife_game_app` | Enables `alife_bevy_adapter`, `bevy`, and Bevy app/render/UI/sprite/glTF features. | Reuse as the base graphical feature. Add Bevy picking and production voxel features under this path. |
| `gpu-runtime` | `alife_game_app` | Enables `alife_gpu_backend`. | Reuse. Production launch should request it and emit explicit fallback diagnostics. |
| `avian3d` | `alife_bevy_adapter` | Optional Bevy/Avian presentation-side physics/spatial adapter. | Keep optional. It may assist presentation collision/spatial queries but cannot become ecology truth. |
| `gpu-tests` | `alife_gpu_backend` | GPU backend test feature. | Keep backend-local. |
| `default = []` | Workspace app crates | CI-safe/headless by default. | Keep default headless/CI-safe. Production scripts pass explicit features. |

### Current App Command Surface

The current `alife_game_app` binary exposes a broad command surface. Important
groups for FVR are:

| Group | Current commands | Product status |
|---|---|---|
| Headless/config | `headless-smoke`, `headless-paused-smoke`, `validate-config`, `list-environments`, `environment-launch-smoke` | Reuse focused validation. Not the product graphical route. |
| Current graphical launch | `graphical-playground`, `graphical-playground-smoke`, `runtime-prereq-smoke`, `tester-feedback-smoke` | Legacy GPU-alpha/True 2.5D route. Preserve only as regression/diagnostic until FVR08 retires or quarantines it. |
| Current visible world/UI | `visible-signature`, `visible-world-smoke`, `graphical-controls-smoke`, `creature-visual-smoke`, `creature-inspector-smoke`, `advanced-gameplay-ux-smoke` | Replace product acceptance with production voxel commands. Reuse tests only when they validate real stable-ID/UI contracts. |
| GPU runtime | `gpu-product-smoke`, `full-gpu-runtime-smoke`, `batched-gpu-runtime-smoke`, `sampled-gpu-runtime-smoke`, `gpu-longrun-soak`, `gpu-sustained-learning-soak`, `gpu-graphics-performance-smoke` | Reuse backend evidence, but FVR06 must add production profile receipts and 30-creature tier coverage. |
| True 2.5D and alpha art | `true25d-headless-continuity-smoke`, `true25d-launch-baseline-smoke`, `world-art-style-smoke`, `production-asset-pipeline-smoke`, `drive-coupled-audio-vfx-smoke` | Historical/reference only for FVR. Do not use these as production voxel acceptance. |
| Gameplay/status slices | survival, ecology, social, lifecycle, school, teacher, semantic, editor, save/load, product QA, release candidate commands | Reuse real backend semantics where useful. FVR must replace the graphical shell around them rather than re-plan gameplay. |

Missing production commands that FVR01-FVR08 must introduce or map to exact
equivalents:

```text
production-voxel
validate-production-save
validate-production-assets
record-production-performance
```

The required public launch script by FVR08 is:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
```

### Current Launchers and Packaging

| File | Current behavior | FVR disposition |
|---|---|---|
| `scripts/run_graphical_playground.ps1` | Starts `graphical-playground`, prints `A-Life GPU Alpha Playground`, defaults to `gpu-alpha`, uses `bevy-app gpu-runtime`, runs `runtime-prereq-smoke`, supports timed window smoke. | Preserve as legacy regression only. FVR01 adds `run_production_voxel_frontend.ps1` and stops using this script for product acceptance. |
| `scripts/run_headless_playground.ps1` | Runs P35 headless playground through `alife_tools`. | Reuse as headless validation context only. |
| `scripts/package_windows_alpha.ps1` | Builds and packages `alife-gpu-alpha-windows`, copies alpha/True 2.5D assets and GPU shader files to `target/artifacts`. | Replace with production package flow by FVR08. Keep historical package script until replacement is proven. |
| `scripts/run_windows_alpha_package.ps1` | Runs packaged GPU-alpha executable with alpha diagnostics. | Legacy only after FVR08. |
| `scripts/render_alpha_art_blender_sprites.ps1` | Dev/art helper for alpha PNG generation, with local Blender discovery. | Historical/dev tooling only. FVR07 production voxel assets require a new asset/license manifest path. |
| `scripts/normalize_true25d_gltf_assets.ps1` | Dev/art helper for True 2.5D glTF normalization. | Historical/dev tooling only unless a production voxel asset uses the same generic license/digest helper pattern. |

### Current Visual and Status Surfaces

| Surface | Current status | FVR consequence |
|---|---|---|
| GPU-alpha player baseline | Current status files describe a GPU-first alpha path using `A-Life GPU Alpha Playground`, `gpu_alpha` fixtures, read-only inspector, controls, CPU fallback, and CPU-shadow-guarded `GpuPlastic` evidence. | Useful backend evidence, not finished product. FVR must not claim alpha path as production. |
| True 2.5D runtime | True 2.5D status files describe locked orthographic presentation, low-poly glTF assets, stylization postprocess, render-bypass proofs, endocrine visual feedback, and no action authority. | Preserve as historical visual evidence/reference. Replace default player view with voxel terrain and production naming. |
| Procedural world presentation | Current procedural chunks are Bevy-independent and creature-anchored, with render layers mirroring reports. They are not a full saved voxel substrate. | FVR02 promotes world-owned saved voxel chunk truth and adapter snapshots. |
| Asset bundle ingestion | App bundle validation currently requires placeholder, alpha art, True 2.5D assets, shader assets, and no committed large generated media. | FVR07 creates production voxel asset manifest rules and moves alpha/True 2.5D requirements out of the production route. |
| Status/progress docs | Existing status docs are historical receipts for CA/S/productization lanes. There is no `docs/progress/` directory in this branch. | No progress/index file needs updating for FVR00. This blueprint is the FVR00 authoritative artifact. |

### Current Save and Runtime Contracts

| Contract | Current state | FVR consequence |
|---|---|---|
| P34 runtime config | `RuntimeConfig` stores deterministic seed, brain class, benchmark population tier, backend selection, feature flags, school/semantic config, GPU limits, logging, asset root, and save root. | FVR02/FVR06 add production profile, voxel renderer backend, quality budgets, backend validation receipt references, and profile budget version. |
| P34 save file | `PortableSaveFile` stores stable IDs, config, asset manifest, world state, creatures, school state, adapter remap, generated weights, ETF prototypes, and packed log schema. | FVR02/FVR05/FVR06 add voxel chunk store, frontend state, selected stable ID, camera/settings, backend descriptor, performance receipt metadata, and asset license manifest refs. |
| Asset manifest | `AssetManifestEntry` stores asset id, kind, relative path, digest, presence, schema version, optional size, and provenance. | FVR07 extends or wraps entries with source/origin, license name/ref/text, author, usage category, replacement policy, and generated-asset metadata. |
| Renderer-handle rejection | P34 rejects Bevy, Avian, wgpu, renderer, window-handle style tokens in portable payloads. | Preserve and expand tests for voxel save fields. |
| GPU runtime | Backend status supports `CpuReference`, `GpuStatic`, `GpuPlastic`, and `GpuFull`, with typed fallback reasons and no active bulk readback. | FVR06 adds production profile receipts, 30-creature tier, RTX 3050 validation, and saved backend descriptors. No fake `GpuFull` claim. |

## Delete, Replace, Reuse Table

| Area | Delete from production route | Replace with | Reuse |
|---|---|---|---|
| App title/naming | `A-Life GPU Alpha Playground` as product title. | Production title such as `A-Life Voxel Frontend` or final product title selected in FVR01. | Historical alpha title only in legacy regression commands. |
| Main graphical command | `graphical-playground` as user-facing launch. | `production-voxel` with profile, population, resolution, save path, backend, and performance flags. | The existing parser style and preflight diagnostics. |
| Graphical launcher | `scripts/run_graphical_playground.ps1` as product entry. | `scripts/run_production_voxel_frontend.ps1`. | Legacy launcher as regression evidence until FVR08. |
| Windows package | `package_windows_alpha.ps1`, `run_windows_alpha_package.ps1`, `alife-gpu-alpha-windows`. | Production Windows desktop package scripts and metadata. | Safe artifact-root checks and crash/feedback receipt pattern. |
| Alpha fixtures | `gpu_alpha` as production default scenario. | Production voxel save/world fixture with real chunk, creature, ecology, backend, profile, and asset manifest data. | Existing P34 fixture validation patterns. |
| Alpha art | `alpha_art_v1` as final production visuals. | Production voxel material, creature, UI, VFX, and audio-visual asset pack. | Digest checks, small committed manifest pattern, generated-art target exclusion. |
| True 2.5D GLB path | `true_25d_alpha_v1` as default product view. | Voxel terrain and production creature renderer. | Historical visual reference and optional regression tests. |
| True 2.5D shaders | `true25d_stylization_postprocess.wgsl` as product style target. | Voxel/material/VFX shader stack selected by FVR03/FVR07. | WGSL-only source rule and shader manifest validation style. |
| Procedural chunks | Creature-anchored presentation/context reports as the sole world surface. | Saved, world-owned voxel chunk truth with materialized edits, signatures, ecology/resource layers, and adapter snapshots. | Bevy-independent sampling and no-authority flags. |
| `visible_world` placeholders | Marker/shape-focused visible world acceptance. | Streamed selectable voxel world and production creature batches. | Stable-ID mapping and small contract tests when still relevant. |
| `alife_bevy_adapter` action/sensory | Nothing should be deleted from the boundary pattern. | Add voxel/player presentation systems without moving core contracts into ECS. | `BevyEntityMap`, stable-ID conversion, sensory/action adapter contracts. |
| `alife_world` headless/ecology | No world authority deletion. | Extend save/world contracts for voxel chunks and production profile snapshots. | Headless world action legality, ecology, stable IDs, persistence validation. |
| `alife_gpu_backend` P29/P34 evidence | No backend evidence deletion. | Add production runtime descriptors, profile timing receipts, and 30-tier measurement. | Probe/fallback/no-readback/CPU-shadow/H_shadow contracts. |
| `alife_core` | No production renderer work belongs here. | No FVR renderer dependencies or handles. | Stable ID, brain class, action, sensory, and lifetime delta contracts. |
| Historical status docs | Do not rewrite broad CA/S history. | New FVR receipts supersede product route claims. | Historical evidence for regression and context. |

## Exact Bevy 0.18 Voxel Stack

Version verification was done against the current workspace and crates.io
metadata on July 3, 2026. FVR01 must pin Bevy-compatible versions, not latest
Bevy 0.19 ecosystem crates.

| Crate | Exact version | Owner | Required feature/use | License | Bevy compatibility | FVR rule |
|---|---:|---|---|---|---|---|
| `bevy` | `0.18.0` | workspace / `alife_game_app` / `alife_bevy_adapter` | Desktop app, renderer, PBR, UI, text, glTF, picking, input, app lifecycle. | MIT OR Apache-2.0 | Native target. | Already pinned. Keep `default-features = false`; add only explicit Bevy features needed for production. |
| `wgpu` | `29.0.3` | `alife_gpu_backend` | Neural runtime probe, buffers, WGSL compute. | MIT OR Apache-2.0 | Independent from Bevy renderer internals. | Keep backend-local. Do not expose wgpu types to `alife_core`, `alife_world`, or renderer save schema. |
| `naga` | `29.0.3` | `alife_gpu_backend` dev-dep | WGSL validation/tests. | MIT OR Apache-2.0 | Matches workspace wgpu line. | Keep dev/backend-local. |
| `bevy_voxel_world` | `0.16.0` | `alife_game_app` or `alife_bevy_adapter` | Default voxel terrain backend. | MIT OR Apache-2.0 | Depends on Bevy 0.18. | Primary FVR03 terrain backend unless FVR01 proves an integration blocker, in which case FVR03 must finish the internal fallback in the same plan. |
| `block-mesh` | `0.2.0` | `alife_game_app` or `alife_bevy_adapter` | Internal chunk-mesh fallback or mesh generation support. | MIT OR Apache-2.0 | Renderer-agnostic mesh helper. | Add only if implementing internal mesh fallback or supplementing voxel backend. |
| `bevy_sprite3d` | `8.0.0` | `alife_game_app` or `alife_bevy_adapter` | Optional simple 3D sprites/billboards for markers, not terrain truth. | MIT | Depends on Bevy 0.18. | Use this version only if FVR04 chooses it. `9.0.0` is Bevy 0.19 and rejected. |
| Custom instanced mesh renderer | local module | `alife_bevy_adapter` / `alife_game_app` | Preferred production path for creatures if sprite crates are limiting. | Project MIT | Uses Bevy 0.18 APIs. | FVR04 may implement instead of `bevy_sprite3d`; must be feature-gated and profile-driven. |
| `bevy_asset_loader` | `0.26.0` | `alife_game_app` | Production loading states and asset collections. | MIT OR Apache-2.0 | Depends on Bevy 0.18. | Use this version. `0.27.0` is Bevy 0.19 and rejected. |
| `bevy_hanabi` | `0.18.0` | `alife_game_app` or `alife_bevy_adapter` | Optional GPU VFX for spores, pheromones, dust, sleep/consolidation signals. | MIT OR Apache-2.0 | Depends on Bevy 0.18. | Optional behind `vfx-hanabi`. It may pull renderer-side wgpu/naga versions; keep isolated from `alife_gpu_backend`. `0.19.0` is Bevy 0.19 and rejected. |
| `bevy_egui` | `0.39.0` | `alife_game_app` | Debug/settings/perf UI if Bevy UI alone is not enough. | MIT | Depends on Bevy 0.18. | Optional behind `debug-ui` or production settings feature. `0.40.0+` is Bevy 0.19 and rejected. |
| `bevy-inspector-egui` | `0.36.0` | `alife_game_app` | Developer inspector only. | MIT OR Apache-2.0 | Depends on Bevy 0.18 and `bevy_egui 0.39`. | Debug-only. Must not be compiled into default production profile unless explicitly requested. |
| `avian3d` | `0.6.1` | `alife_bevy_adapter` | Optional presentation collision/spatial query layer. | MIT OR Apache-2.0 | Depends on Bevy 0.18. | Already optional. It cannot become ecology or action legality truth. |
| `image` | `0.25.10` | `alife_game_app` | PNG validation/loading helpers. | MIT OR Apache-2.0 | Renderer-adjacent utility. | Existing dependency. Keep for asset validation as needed. |
| `serde` | `1.0.228` | workspace | Save/profile/manifest schemas. | MIT OR Apache-2.0 | Not Bevy-specific. | Continue for versioned FVR schemas. |
| `serde_json` | `1.0.145` | workspace | Save/profile/manifest JSON. | MIT OR Apache-2.0 | Not Bevy-specific. | Continue for portable saves and receipts. |

Rejected latest versions for FVR01: `bevy_sprite3d 9.0.0`,
`bevy_asset_loader 0.27.0`, `bevy_hanabi 0.19.0`, `bevy_egui 0.40.0+`, and
`bevy-inspector-egui 0.37.0` because they target Bevy 0.19.

## Feature Flag Map

FVR01 must add or confirm these flags without making graphical dependencies part
of the default headless build.

| Feature | Owner | Dependencies | Purpose | Acceptance rule |
|---|---|---|---|---|
| `bevy-app` | `alife_game_app` | Existing Bevy 0.18 app/render/UI/glTF/PBR/sprite/png set, plus Bevy picking features if required. | Base graphical app. | Existing commands keep compiling; no default feature change. |
| `gpu-runtime` | `alife_game_app` | `dep:alife_gpu_backend` | Enables neural GPU runtime/probe/fallback path. | Production launch passes it. CPU fallback is explicit, never silent success. |
| `voxel-backend` | `alife_game_app` | `bevy-app`, `bevy_voxel_world = 0.16.0` | Default production voxel terrain backend. | Required by production launch unless FVR01 documents and wires the internal fallback as default. |
| `voxel-internal-mesh` | `alife_game_app` or `alife_bevy_adapter` | `bevy-app`, `block-mesh = 0.2.0` | Production internal chunk mesh fallback. | If the external voxel crate is unsuitable, this must be finished in FVR03, not deferred. |
| `production-voxel-frontend` | `alife_game_app` | `bevy-app`, `voxel-backend` or `voxel-internal-mesh`, `production-assets`, optional `gpu-runtime` in scripts. | Convenience feature for the finished desktop frontend. | Does not replace precise lower-level flags in tests. |
| `production-assets` | `alife_game_app` | `bevy_asset_loader = 0.26.0`, asset manifest modules. | Production loading states and licensed asset collections. | Required by FVR07/FVR08 production launch. |
| `presentation-physics` | `alife_bevy_adapter` | `avian3d = 0.6.1` | Optional presentation collision/spatial query. | Must not own world/ecology/action legality. |
| `vfx-hanabi` | `alife_game_app` or `alife_bevy_adapter` | `bevy_hanabi = 0.18.0` | Optional GPU VFX. | Adaptive caps are profile-driven; feature may be disabled by minimum settings without removing core gameplay. |
| `debug-ui` | `alife_game_app` | `bevy_egui = 0.39.0`, optional `bevy-inspector-egui = 0.36.0` | Debug/perf/inspector UI. | Debug controls are read-only for cognition/action authority unless routed through normal player commands. |

No new feature or command should include `alpha` in production naming. Existing
alpha-named features/commands may remain only for regression and history.

## App Command Cutover Map

| Current command/script | FVR route | Cutover plan |
|---|---|---|
| `graphical-playground` | `production-voxel` | FVR01 adds production parser, help text, profile registry, and launcher. FVR08 removes this command from user docs/product acceptance. |
| `graphical-playground-smoke` | `production-voxel --profile MinimumSettings30x30 --population 30 --record-performance --smoke-seconds <N>` if a bounded mode remains useful | Replace smoke naming in acceptance. Bounded runs can exist as test harness options, not product proof names. |
| `runtime-prereq-smoke` | `production-voxel --preflight-only` or `record-production-performance --preflight` | Keep hardware probe/fallback details, rename production-facing surface. |
| `gpu-product-smoke` | `record-production-performance --backend` | Reuse backend status format; add profile, hardware, frame, chunk, creature, and saved descriptor data. |
| `full-gpu-runtime-smoke`, `batched-gpu-runtime-smoke`, `sampled-gpu-runtime-smoke` | FVR06 backend validation subcommands or focused tests | Keep focused backend tests. Production acceptance goes through `production-voxel` and `validate-production-save`. |
| `gpu-graphics-performance-smoke` | `record-production-performance` | Replace unknown/manual graphics status with measured production profile receipts. |
| `visible-world-smoke`, `visible-signature` | FVR03/FVR04 renderer and snapshot tests | Keep stable-ID contract assertions. Product visual proof moves to production voxel screenshots/perf receipts. |
| `save-load-ux-smoke`, `graphical-save-load-menu-smoke` | `validate-production-save` plus production menu tests | Preserve P34-style validation; expand to voxel chunks, profile, UI settings, backend descriptor. |
| `app-bundle-smoke`, `production-asset-pipeline-smoke` | `validate-production-assets` | Replace alpha/True 2.5D asset requirements with production voxel asset license manifest. |
| `platform-package-smoke` | FVR08 production package validation | Rename package metadata away from alpha; preserve safe `target/artifacts` output policy. |
| `product-qa-smoke`, `release-candidate-smoke` | FVR08 final acceptance commands | Keep as internal QA helpers only if they call production voxel commands. |
| `scripts/run_graphical_playground.ps1` | `scripts/run_production_voxel_frontend.ps1` | FVR01 adds new script. FVR08 updates docs/default command references. |
| `scripts/package_windows_alpha.ps1` | Production package script | FVR08 adds/replaces with production package builder. |

Required production command shape by FVR08:

```powershell
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinimumSettings30x30 --population 30 --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinSpecComfort1080p
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinimumSettings30x30
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinSpecComfort1080p
```

## Saved-State Schema Change Map

FVR02-FVR07 must migrate P34 through tested schema changes. Names below are
implementation targets; exact Rust type names may change if the committed code
uses clearer local naming.

| Schema area | Current P34 field/type | Required FVR change | Owner plan | Boundary rule |
|---|---|---|---|---|
| Runtime profile | `RuntimeConfig::benchmark_population_tier`, GPU limits | Add `frontend_profile`, `profile_budget_version`, `population_target`, `resolution`, `internal_render_scale`, `quality_overrides`, and selected production profile. | FVR01/FVR02 | Profile data is plain serde data, not Bevy settings resources. |
| Renderer backend | None in P34 | Add renderer backend descriptor: `voxel-backend`, `voxel-internal-mesh`, material palette id, LOD policy, lighting/shadow profile, VFX profile. | FVR03/FVR07 | No Bevy entities, handles, pipelines, materials, images, meshes, or windows in save files. |
| Voxel world | `WorldSaveState` plus procedural reports outside main save | Add world-owned voxel chunk store: world seed, chunk schema version, chunk size, material palette version, materialized chunk signatures, dirty edits, ecology/resource overlays, and migration hooks. | FVR02 | `alife_world` owns truth. Renderer receives snapshots only. |
| Chunk snapshots | Current procedural chunks are creature-anchored context reports | Add adapter snapshot structs for active chunk windows, visible chunk IDs, chunk material summaries, selectable tile coordinates, and stable content IDs. | FVR02/FVR03 | Snapshots may be consumed by Bevy, but remain serializable without Bevy types. |
| Frontend state | Save/load UX exists outside P34 schema | Add camera pose, selected stable ID, UI panel/settings state, accessibility/readability settings, profile override state, and last validated resolution. | FVR05 | Selected entity is a stable world ID, never Bevy `Entity`. |
| Backend state | `BackendConfig`, `GpuLimitsConfig` | Add saved backend descriptor: requested/selected backend, adapter identity, validation receipt id, CPU parity status, fallback reason, compact readback bytes, timing sample summary, no-readback policy version. | FVR06 | Persist descriptors and receipts only, never raw wgpu resources. |
| Population tiers | `GpuTierPopulation` has 1, 10, 50, 100, 250, 500 | Add population tier 30. Record 1, 10, 30, 50, 100, 250, 500 in production receipts. | FVR06/FVR08 | 30 is a floor tier, not a ceiling. |
| Asset manifest | `AssetManifestEntry` has id, kind, path, digest, presence, schema, size, provenance | Add or wrap production asset manifest fields: source URL/origin note, license name, license text/ref, author, usage category, replacement policy, generated source/config/seed when applicable. | FVR07 | Asset handles stay in app/adapter. Saves store refs/digests. |
| Save migration | P34 rejects mismatched schemas unless migration exists | Add tested migration from current P34 examples to FVR schema, or explicitly reject older saves with a user-facing reason until migration is implemented. | FVR02/FVR08 | No silent reinterpretation. |

## GPU and Runtime Integration Map

| Layer | Production role | FVR implementation rule |
|---|---|---|
| CPU world oracle | Owns action legality, ecology/resource outcomes, save truth, stable IDs, creature existence, and ExperiencePatch sealing. | Renderer/GPU proposals cannot bypass world validation. |
| Bevy renderer | Displays voxel world, creatures, UI, overlays, VFX, selection, and settings. | Consumes world/adapter snapshots. It does not create truth, issue hidden actions, rewrite weights, or inject rewards. |
| `alife_bevy_adapter` | Converts between Bevy ECS and stable core/world contracts. | Bevy `Entity` values stay inside the adapter. |
| `alife_gpu_backend` | Owns neural wgpu/WGSL runtime, hardware probe, CPU-shadow parity, no-readback guard, compact action summaries, H_shadow conversion. | Production selects GPU only after real probe/validation. Fallback is explicit. |
| GPU renderer stack | Bevy renderer, voxel terrain, instancing, VFX, materials. | Keep renderer-side wgpu implementation details separate from `alife_gpu_backend` public contracts. |
| Active readback | Compact action summaries may be read during active tick when allowed by backend guard. | Bulk neural, per-synapse, per-lobe, and weight readback remain forbidden in active gameplay. |
| Debug/inspector | Shows selected creature/world/backend/profile state. | Read-only by default for cognition. Any user command must go through normal world/player action paths. |
| Save/perf receipts | Persist selected backend descriptor, validation receipt, timing summaries, frame/chunk/creature budgets, and fallback/adaptation actions. | Receipts are evidence. They are not GPU success claims unless measured on the real path. |

GPU runtime target for the production desktop app:

- Request the strongest validated backend that is genuinely implemented.
- On Cassidy's RTX 3050 8 GB, the expected production path should select the
  GPU backend when real hardware probe and validation pass.
- If `GpuFull` is not validated by FVR06, the production app must not pretend it
  is full action-authoritative. It must choose the strongest real mode and show
  the exact fallback/unsupported reason.
- CPU fallback is allowed only with visible diagnostics and must still preserve
  the `MinimumSettings30x30` floor through conservative budgets.

## Asset and License Policy

FVR07 owns the production asset pack, but FVR01-FVR06 must avoid adding assets
that violate this policy.

Allowed license families for committed external assets:

- CC0 or public domain.
- MIT, Apache-2.0, BSD, Zlib, or similarly permissive licenses approved in the
  asset manifest.

Rejected asset inputs:

- Unclear, missing, non-redistributable, copyleft-incompatible, or source-less
  external assets.
- Large generated media committed directly to git.
- Assets that require a private runtime tool to load the production build.
- Placeholder assets claimed as final.

Every production asset manifest entry must record:

- source URL or origin note;
- license name;
- committed license text path or license reference;
- author/creator when available;
- local path;
- digest;
- size when useful for validation;
- usage category;
- replacement policy;
- generated-asset tool, model, prompt/config, seed, and date when applicable.

The alpha PNG pack and True 2.5D GLB lane remain historical/reference assets.
FVR production validation must use the production voxel asset manifest.

## Target Hardware and Profiles

### Minimum Supported Hardware

```text
GPU: NVIDIA RTX 3050
VRAM: 8 GB
CPU: Intel Core i7-3770K
RAM: 32 GB DDR3
OS: Windows 10
Resolution: 1920x1080
Platform: desktop only
Renderer: Bevy 0.18 desktop
```

The i7-3770K is a primary risk. FVR implementation must prefer chunk batching,
dirty-region updates, GPU instancing, compact snapshots, bounded UI sampling,
async/background preparation where safe, and profile-driven budgets.

### `MinimumSettings30x30`

Hard playable floor on the minimum hardware class.

| Budget | Required value |
|---|---|
| Population | 30 real creatures, not impostors or fake population counters. |
| FPS target | 30 FPS minimum playable floor. Target frame budget: 33.3 ms. |
| Resolution | 1920x1080 output with configurable reduced internal render scale. Internal scale floor: 0.67. |
| World | Real saved `alife_world` voxel chunks, visible terrain, selectable tile/chunk coordinates, ecology/resources/hazards visible at conservative density. |
| Backend | Real backend selection/fallback diagnostics. No fake GPU path. |
| Saves | Save/load preserves chunks, creatures, selected ID, profile, UI settings, backend descriptor, and asset refs. |
| UI | Essential menus, settings, selection, backend status, frame/profile status, and readable overlays. |
| Visual quality | Low shadows or shadow substitutes, conservative VFX, low label density, bounded chunk radius. |
| Starting chunk budget | Chunk tile size 16, activation/view radius 2 chunks, active chunk cap 128, dirty mesh rebuilds only. FVR08 may raise this with evidence but may not lower below playable floor without failing acceptance. |
| Starting creature render budget | 30 visible creatures, GPU-instanced or otherwise batched, selected/hover labels only. |
| Starting neural residency budget | Profile records hot/warm/cold slots. Initial target: 4 hot, 12 warm, 14 cold for the 30-creature floor, with sensory/motor/homeostasis protected. |
| Forbidden reductions | No removal of real simulation, real saves, creature interaction, backend diagnostics, voxel terrain, or essential overlays. |

### `MinSpecComfort1080p`

Default comfortable profile for Cassidy's minimum supported machine.

| Budget | Required value |
|---|---|
| Default population | 30 real creatures unless FVR08 evidence proves a higher default is comfortable on the target machine. |
| FPS target | Smooth 60 FPS target at 1920x1080. Target frame budget: 16.7 ms. |
| Resolution | Native 1920x1080 internal scale 1.0 by default, adaptive only when required and recorded. |
| World | Real saved voxel chunks, readable biome/material variation, view radius 4 chunks, active chunk cap 256 starting target. |
| Backend | GPU path selected on RTX 3050 when validation passes. CPU fallback is explicit degraded mode. |
| UI | Production menus, settings, overlays, selection, selected-creature inspector, and performance/backend status. |
| Visual quality | Coherent stylized lighting/shadow substitute, medium VFX caps, readable creature expressions, compact overlays. |
| Starting neural residency budget | Initial target: 8 hot, 16 warm, remaining default population cold/sleeping, profile-controlled and recorded. |
| Core feature posture | All core gameplay/inspection systems enabled. The user should not manually disable core features to get the default experience. |

The exact numeric budgets above are starting implementation contracts. Later
plans may raise them with measured evidence. They may only lower a budget if
the FVR completion receipt records the measured reason and still satisfies the
named profile semantics.

### Future Scale-Up Profiles

| Profile | Intended machine | Population target | World/render target | Neural/runtime target |
|---|---|---:|---|---|
| `Balanced1080p` | RTX 3050 class with measured headroom or modestly stronger desktop | 50 real creatures | 1080p, radius 5 chunks, denser materials/VFX, active cap 384 | More warm slots and higher nonessential update cadence when timing allows. |
| `HighSpecScaleUp` | Stronger desktop GPU/CPU | 100 default, 250 and 500 benchmark tiers | Larger chunk radius, richer shadows/materials/VFX, denser overlays | More hot/warm slots, larger brain-class budgets, adaptive timing receipts. |
| `ResearchScale` | Non-default experiment mode | 250 to 500+ as explicit run configuration | Large worlds and long soaks; comfort FPS may be missed honestly | Schema-compatible experiments, no-readback rules preserved, no product comfort claim unless measured. |

## Exact Validation Plan for FVR01-FVR08

Every FVR01-FVR08 receipt must run the standard boundary commands unless the
plan-specific command set clearly requires more. If a command is unavailable,
record the exact command, exit/failure text, and why it could not prove a pass.

Baseline commands for every implementation plan:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
```

### FVR01 - Production Launcher, Dependency Cutover, and Frontend Demolition

Must prove:

- Bevy 0.18-compatible voxel dependencies are pinned.
- No Bevy 0.19 ecosystem crate enters the production feature set.
- `production-voxel` and `scripts/run_production_voxel_frontend.ps1` exist.
- Old alpha/graphical commands are legacy/regression only.
- Profile registry exposes all named profiles.

Required focused validations:

```powershell
cargo check -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app
cargo tree -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" -i bevy
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --help
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun
```

Receipt must list old frontend files/commands deleted, replaced, or preserved
as regression.

### FVR02 - Real Persistent World Backend and Chunk/Snapshot Contracts

Must prove:

- `alife_world` owns saved voxel chunk truth and materialized edits.
- Adapter snapshots are renderer-independent serde/core data.
- Existing P34 saves either migrate through a tested path or reject with a
  clear version reason.
- No renderer/GPU/window/Bevy handles enter `alife_world` saves.

Required focused validations:

```powershell
cargo test -p alife_world persistence -- --nocapture
cargo test -p alife_world procedural_chunks -- --nocapture
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinimumSettings30x30
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinSpecComfort1080p
```

Receipt must include schema names/versions, migration stance, and rejected
engine-token proof.

### FVR03 - Finished Default Voxel World Renderer

Must prove:

- The default player view shows real saved voxel chunks from `alife_world`.
- Terrain/chunks/tiles are selectable through stable coordinates/IDs.
- `MinimumSettings30x30` can launch the production voxel world with the real
  backend path and 30 real creatures configured.
- Renderer consumes snapshots and does not own world truth.

Required focused validations:

```powershell
cargo test -p alife_game_app --features "bevy-app voxel-backend" voxel -- --nocapture
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinimumSettings30x30 --population 30 --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinimumSettings30x30
```

Receipt must include a screenshot/performance artifact path under
`target/artifacts/` and state whether GPU, CPU fallback, or both were used.
Artifacts are not committed.

### FVR04 - GPU-Scaled Creatures, Animation, Selection, and Expression

Must prove:

- 30 visible real creatures render from stable world IDs under
  `MinimumSettings30x30`.
- Creature batching/instancing or equivalent GPU-friendly rendering is active.
- Selection maps Bevy presentation back to stable IDs.
- Animation/expression state comes from real world/core state, not fake display
  data.

Required focused validations:

```powershell
cargo test -p alife_game_app --features "bevy-app voxel-backend" creature -- --nocapture
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinimumSettings30x30 --population 30 --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinSpecComfort1080p --record-performance
```

Receipt must include rendered creature count, selected stable ID proof, batching
path, and profile budgets.

### FVR05 - Production Game UX, Overlays, and Inspectors

Must prove:

- Production menus, settings, profile selection, camera controls, overlays,
  backend status, save/load flow, and selected-creature inspector are usable.
- UI/debug surfaces are read-only for cognition/action authority unless routed
  through normal player commands.
- Minimum settings remain readable and playable.

Required focused validations:

```powershell
cargo test -p alife_game_app --features "bevy-app voxel-backend debug-ui" ux -- --nocapture
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinimumSettings30x30 --population 30 --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinSpecComfort1080p
```

Receipt must include screenshot paths for menu/settings/inspector states and an
authority statement for every debug control.

### FVR06 - Full Gameplay GPU Backend Integration and Saved Runtime State

Must prove:

- Production launch requests the strongest real validated backend.
- RTX 3050 path selects GPU when probe and validation pass.
- CPU fallback records typed degradation reason and still preserves the
  `MinimumSettings30x30` floor.
- Saved runtime state includes backend descriptor and validation receipt.
- Active gameplay does not bulk-read neural buffers.
- Population tiers include 1, 10, 30, 50, 100, 250, and 500.

Required focused validations:

```powershell
cargo test -p alife_gpu_backend -- --nocapture
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --profile MinimumSettings30x30 --population 30 --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --profile MinSpecComfort1080p --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinimumSettings30x30
```

If testing forced fallback:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"; cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --profile MinimumSettings30x30 --population 30 --record-performance; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Receipt must include adapter name/API/driver, selected backend, fallback reason,
CPU parity status, compact readback bytes, and saved descriptor version.

### FVR07 - Art, Assets, VFX, Audio-Visual Polish, and License Manifest

Must prove:

- Production voxel assets are coherent and licensed/generated with manifest
  source, license, digest, local path, usage category, and replacement policy.
- No placeholder art is claimed as final.
- VFX/audio-visual polish is profile-controlled and can degrade for
  `MinimumSettings30x30`.
- No large generated artifacts are committed.

Required focused validations:

```powershell
cargo test -p alife_game_app production_asset -- --nocapture
cargo run -p alife_game_app --features "bevy-app voxel-backend production-assets" --bin alife_game_app -- validate-production-assets
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -- production-voxel --profile MinimumSettings30x30 --population 30 --record-performance
```

Receipt must include license manifest path, rejected/missing asset count,
committed asset size summary, generated-art target path, and VFX budget state.

### FVR08 - Final Replacement Hardening, Packaging, and Acceptance

Must prove:

- The old ugly frontend is fully replaced in the default desktop product path.
- A clean checkout can launch production voxel frontend, load/create a real
  saved world, render voxel terrain, interact with real creatures, save/reload
  backend state, and produce performance/diagnostic receipts.
- `MinimumSettings30x30` meets 30 real creatures at 30 FPS.
- `MinSpecComfort1080p` is the default comfortable profile on the target class.
- Scale-up profiles run without schema/architecture rewrites.
- Old alpha/smoke/contract-only commands are not product acceptance.

Required focused validations:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinimumSettings30x30 --population 30 --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --resolution 1920x1080 --profile MinSpecComfort1080p --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- production-voxel --profile HighSpecScaleUp --population 500 --record-performance
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinimumSettings30x30
cargo run -p alife_game_app --features "bevy-app gpu-runtime voxel-backend" --bin alife_game_app -- validate-production-save --profile MinSpecComfort1080p
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
```

If all-features validation is required by the final receipt, run:

```powershell
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

If the known local MSVC all-features linker/access-violation issue appears,
record the exact failure and rerun with a narrower or serialized equivalent.
Do not claim the original command passed unless it actually passed.

Receipt must include final package path, production command outputs, minimum
profile FPS/frame timing, comfort profile timing, scale-up behavior, save/load
proof, GPU/backend evidence, asset/license proof, and a statement that known
limitations are empty or platform-only.

## FVR00 Completion Notes

- No runtime code was changed by FVR00.
- No production dependency was added by FVR00.
- No generated asset or target artifact is part of FVR00.
- No progress/index document required an update in this branch; the FVR plan
  pack has no existing progress index, and historical CA/S progress files are
  not the authoritative index for this new FVR pack.
- FVR01 can start from this blueprint without more planning.
