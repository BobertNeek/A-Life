# Fullstack Bevy Voxel Frontend Replacement - Codex Goal Mode Prompts

Use these prompts one at a time. FVR00 is the only scaffolding/review pass. Do not split FVR01-FVR08 into alpha/smoke/practice plans. Each implementation goal must finish the owned subsystem completely before moving on.

Cassidy's RTX 3050 8 GB / i7-3770K / 32 GB DDR3 / Windows 10 / 1080p machine is the minimum comfortable supported spec, not a stretch benchmark. The hard minimum-settings floor is `MinimumSettings30x30`: 30 real creatures at 30 FPS with real simulation, real saves, visible voxel terrain, creature selection, essential overlays, essential UI, and real backend selection/fallback diagnostics. Future scale-up must remain profile-driven and must not require architecture rewrites.

## FVR00 - One-pass repo audit and replacement blueprint

```text
Goal: Complete FVR00 for A-Life. Perform the only scaffolding/review pass for replacing the old graphical frontend with a finished Bevy 0.18 voxel fullstack frontend. Do not implement the replacement yet except for documentation/plan-pack wiring needed to make later goals unambiguous.

Read AGENTS.md, docs/master_spec.md, docs/architecture_decisions.md, docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/README.md, docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/ACCEPTANCE_AND_VALIDATION.md, and this prompt. Inspect current alife_game_app, alife_bevy_adapter, alife_world, alife_gpu_backend, existing True2.5D/graphical/status files, scripts, features, and app commands.

Produce docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR00_REPLACEMENT_BLUEPRINT.md. It must contain: current frontend inventory; delete/replace/reuse table; exact crate dependency/version table for Bevy 0.18 voxel stack; feature flag map; app command cutover map; saved-state schema change map; GPU/runtime integration map; asset/license policy; target hardware assumptions RTX 3050 8GB/i7-3770K/Win10/1080p; `MinimumSettings30x30` floor definition; `MinSpecComfort1080p` default comfort definition; future scale-up profile map; exact validations for FVR01-FVR08. Update docs/progress/index files only if they already exist and require it.

Non-negotiables: no mock sim, no fake backend, no alpha naming for new production work, no Bevy/wgpu/renderer types in alife_core, no renderer authority over actions or cognition, no large generated artifacts committed. This is the only goal allowed to stop after planning.

Validation: cargo fmt --all -- --check; cargo check --workspace --all-targets; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1. If commands are unavailable, record exact failure and why without fabricating pass claims.

Finish with a completion receipt and explicit statement that FVR01 can start without more planning.
```

## FVR01 - Production launcher, dependency cutover, and frontend demolition

```text
Goal: Complete FVR01 for A-Life. Replace the old ugly graphical frontend entry path with a production Bevy 0.18 voxel frontend launcher and dependencies. This is not an alpha shell. At completion, the default desktop graphical launch must enter the new production app state pipeline, even if later visual systems are added in their own finished goals.

Read the FVR00 blueprint. Implement the dependency and launch cutover in Cargo.toml, crates/alife_game_app, crates/alife_bevy_adapter, app config, scripts, docs, and manifests. Add Bevy 0.18-compatible dependencies for voxel terrain, sprite/instanced creatures, asset loading, picking, debug UI/perf UI, VFX, and presentation physics as selected in FVR00. Feature flags must be product-grade: default graphical desktop path, gpu-runtime, debug-tools, voxel-backend, licensed-assets, and any necessary narrow optional flags. Avoid feature flag tangles that make all-features fail.

Retire old graphical frontend commands by routing them to the new production app or removing stale commands if safe. Preserve legacy commands only as compatibility aliases with warnings, not as the default player path. Remove or quarantine old placeholder/manifest-only visual assertions that would conflict with the new finished app. Do not touch alife_core except if a boundary test needs a new stable non-renderer contract already approved by FVR00.

Implement production app states: Boot, ValidateRuntime, LoadAssets, LoadOrCreateWorld, Running, Paused, Settings, Shutdown. Wire Windows scripts for default graphical launch and production validation. Add runtime diagnostics that show selected GPU/backend, adapter name, renderer profile, save path, asset manifest, and fallback reason.

Implement production profile plumbing from the start: `MinimumSettings30x30`, `MinSpecComfort1080p`, `Balanced1080p`, `HighSpecScaleUp`, and `ResearchScale`. Store selected profile in config/save metadata. Use profile values for budgets; do not hard-code min-spec limits or the 30-creature floor as architectural ceilings.

No mockups, no fake data source, no new smoke-only app. Use existing real save/config/world paths. If a required real backend is incomplete, implement the minimal real backend bridge here rather than adding a mock.

Validation: cargo fmt; cargo check --workspace --all-targets; cargo check --workspace --all-features --all-targets; cargo test --workspace --all-targets; clippy -D warnings if current repo expects it; check_core_boundaries.ps1; docs_check.ps1; run the new production graphical launch on Windows if hardware is available and record actual result.

Completion requires old default frontend no longer being the product path and the new app starting through production states with `MinSpecComfort1080p` selected by default on the target machine and `MinimumSettings30x30` available as the hard fallback floor.
```

## FVR02 - Real persistent world backend and chunk/snapshot contracts

```text
Goal: Complete FVR02 for A-Life. Build the real renderer-independent persistent world backend required by the voxel frontend. This is not a mock world, not a fixture harness, and not a visual-only map. The saved backend must own procedural voxel chunk truth, ecology/resource layers, stable IDs, profile metadata, and adapter snapshots.

Read FVR00/FVR01 outputs and current alife_world/alife_core save contracts. Implement or complete alife_world contracts for chunk coordinates, chunk keys, terrain/material IDs, biome/ecology zones, resources, hazards, creature anchors, dirty regions, materialized chunk cache metadata, chunk seed derivation, profile-driven active areas, and save/load schema migration. Chunks must be deterministic from seed plus saved edits, but saved edits and materialized backend state must persist. Renderer-specific fields are forbidden in alife_world.

Expose production snapshot APIs for alife_bevy_adapter: visible chunk stream, creature stream, field overlay stream, resource/hazard stream, selection lookup, and stable object references. Snapshots must be compact and dirty-region aware. Include active-area/chunk residency policy appropriate for `MinimumSettings30x30`, comfortable `MinSpecComfort1080p`, and larger scale-up profiles. Keep CPU work bounded: do not allocate a huge contiguous world, do not initialize far chunks, do not run procedural decoration every frame.

Integrate ghx_proc_gen or a simpler deterministic internal generator only if it produces real saved world content. No mock generator. If external/procedural decoration is included, save seed, ruleset version, output digest, and edits.

Update app save/load so backend work is saved: world seed, chunk edits, materialized chunk metadata, creatures, resources, selected backend mode, visual/profile reference, minimum-settings budget overrides, scale-up budget overrides, and asset manifest references. Add validation that reloading the same save reconstructs the same visible chunk signatures and stable IDs.

Validation: full cargo checks/tests, boundary scripts, docs check, save/load roundtrip for real world backend, deterministic chunk signature test using production generator, no Bevy/wgpu types in alife_world/core, no committed target artifacts.

Completion requires alife_game_app to load/create a real persistent world backend that later renderers consume without mock fixtures and without baking the minimum spec into world architecture.
```

## FVR03 - Finished default voxel world renderer

```text
Goal: Complete FVR03 for A-Life. Implement the finished default voxel world renderer. The player view must be a polished stylized voxel world backed by real alife_world chunks, not a tilemap placeholder, manifest claim, or isolated graphics demo.

Use Bevy 0.18. Integrate bevy_voxel_world if compatible with the repo and FVR02 contracts. If it cannot satisfy production requirements after direct integration attempts, implement an internal Bevy chunk-mesh voxel backend in the same goal; do not defer. The backend must stream chunks around player/selected creature anchors, despawn distant chunks, use dirty-chunk updates, material palettes, LOD/draw-distance settings, and memory-budgeted residency.

Implement production isometric/orthographic and orbit camera modes for voxel terrain at 1080p. Add lighting, shadows or stylized fake shadows, fog/depth cues, water/decay/resource materials, biome material variation, outlines/silhouette style if supported, and readable height/occlusion handling. Terrain must be selectable: clicking/hovering resolves to stable chunk/tile/world coordinates without exposing Bevy Entity to core.

Wire renderer settings through named profiles: `MinimumSettings30x30`, `MinSpecComfort1080p`, `Balanced1080p`, `HighSpecScaleUp`, and `ResearchScale`. `MinimumSettings30x30` must keep the game playable at 30 creatures / 30 FPS while preserving visible voxel terrain, real creature selection, essential overlays, essential UI, and save/backend diagnostics. `MinSpecComfort1080p` must run comfortably on RTX 3050/i7 hardware with all core features enabled. Higher profiles must raise budgets without code rewrites.

Remove or downgrade old True2.5D player-view code if it conflicts. It may remain as a debug view only if it does not own the product path. The voxel backend must be the default graphical frontend.

Validation: cargo fmt/check/test/clippy; all-features check; launch graphical production app under `MinimumSettings30x30` with 30 creatures and under `MinSpecComfort1080p`; verify real chunk load/save/reload; verify selection returns stable coordinates; record adapter/backend/FPS/memory diagnostics if local GPU available. Do not claim performance if not measured.

Completion requires a visible, streamed, selectable, stylized voxel world as the default app view, meeting the 30-creature/30-FPS minimum floor, comfortable on minimum spec, and budget-scalable on stronger hardware.
```

## FVR04 - GPU-scaled creatures, animation, selection, and expression

```text
Goal: Complete FVR04 for A-Life. Implement finished creature rendering and interaction on top of the real voxel world. The system must handle 1, 10, 30, 50, 100, 250, and 500 real creatures using GPU-friendly rendering, with 30 creatures at 30 FPS as the hard minimum-settings floor, comfortable default play tuned for RTX 3050/i7, and higher tiers available through scale-up profiles.

Use real creature/world/core state. Do not create a fake population generator except through the production world population system and real save/config knobs. Implement GPU-instanced mesh or billboard/sprite3d rendering for creature bodies, orientation, movement interpolation, sleep/death/reproduction/resource states, and selection markers. If bevy_sprite3d is used, ensure batching/caching; if it cannot hit budgets, build custom instanced mesh/billboard buffers in alife_bevy_adapter.

Map core/world neurochemical and drive state into a compact visual expression buffer: hunger, fatigue, fear/cortisol, dopamine/valence, reproductive state, sleep/consolidation, social signal. Use shader/material parameters or instance attributes so expressions do not cause CPU material churn. Do not let visual expression mutate cognition.

Implement selection, hover, camera follow, selected-creature panel hooks, stable ID lookup, and visual affordance cues. Add simple production animations: locomotion, idle, eating, fleeing, sleeping, death/corpse, reproduction/offspring marker, teacher/social cue if available. Animation must be data-driven enough to handle many creatures cheaply.

Integrate creature rendering with voxel occlusion/readability: outlines, height offset, billboards facing orthographic camera, label culling, selected entity highlight, and profile-based label/detail/VFX density.

Validation: cargo checks/tests; graphical run with real population tiers 1/10/30/50/100/250/500 where supported by current production configs; record FPS/frame time/backend selection on RTX 3050 if available; verify `MinimumSettings30x30` hits 30 creatures / 30 FPS; verify `MinSpecComfort1080p` default tier is comfortable; verify higher tiers can adapt or scale without schema/code rewrites; verify save/load preserves selected creature and visible creature signatures; verify no Bevy Entity leaks into saved/core state.

Completion requires the default voxel app to show real animated selectable creatures with GPU-scaled rendering, the 30-creature/30-FPS minimum floor, minimum-spec comfort, and future scale-up profiles.
```

## FVR05 - Production game UX, overlays, and inspectors

```text
Goal: Complete FVR05 for A-Life. Build the finished production UX and debug/inspection layer for the voxel frontend. This is not a developer-only overlay and not a mock inspector. It must make the game usable, inspectable, and debuggable without opening source code.

Implement main menu, load/create world, pause/resume, speed controls, save/load, settings, quality/profile selection, backend selection display, camera controls, selected creature panel, selected chunk/tile panel, world/ecology panel, and GPU/runtime status panel. Use Bevy UI/egui/debug tools as appropriate but keep debug authority read-only unless an existing production editor command path authorizes a world edit.

Implement overlays: resource/food, danger/hazard, pheromone/chemistry, creature energy, age/lifecycle, fertility/reproduction, territory/social, neural activity summary, brain residency, backend timing, chunk boundaries, draw/LOD budget, and save/persistence state. Overlays must be generated from real world/core/gpu summaries and rendered efficiently over voxel terrain. They may use heatmap textures, chunk overlay meshes, GPU particles, labels, or instanced markers.

Implement input model: mouse picking, hover details, selected creature follow, camera pan/orbit/zoom, keyboard shortcuts, controller-safe future mapping if trivial, and UI focus handling. Add production error UI for GPU fallback, missing assets, incompatible saves, and validation failure.

Expose profile controls for `MinimumSettings30x30`, `MinSpecComfort1080p`, `Balanced1080p`, `HighSpecScaleUp`, and `ResearchScale`. The UI must show what was reduced or increased by the selected profile. Profile changes must persist in user config/save metadata without serializing engine-local IDs.

Prevent debug bypasses: UI cannot emit direct actions, rewrite weights, inject rewards, or mutate core hidden state except through existing authorized world/editor APIs. All debug snapshots must be bounded and avoid active bulk neural readback.

Validation: cargo checks/tests; run production app and exercise menu/load/save/pause/settings/profile/selection paths; verify overlay toggles do not change sim signatures; verify `MinimumSettings30x30` keeps essential UI/overlays readable; verify core boundary scripts; verify UI state persists in user config without serializing engine-local IDs.

Completion requires a usable finished frontend UX over the voxel world and real creatures, including minimum-settings floor controls, minimum-spec comfort controls, and future scale-up controls.
```

## FVR06 - Full gameplay GPU backend integration and saved runtime state

```text
Goal: Complete FVR06 for A-Life. Convert the GPU backend from diagnostic/parity-only behavior into a real selectable gameplay backend for the production voxel app on RTX 3050, while preserving CPU oracle semantics, no-active-bulk-readback rules, the 30-creature/30-FPS minimum floor, minimum-spec comfort, and future scale-up.

Read current alife_gpu_backend README, WGSL shaders, runtime selection code, P25-P29 docs, and graphical app GPU mode code. Implement the missing full-stack integration so the production app can select GPU static/plastic/full as configured, validate hardware limits, allocate persistent GPU resources, upload real brain/world sensory batches, dispatch separated WGSL passes, consume compact action summaries, stage H_shadow/lifetime deltas through core-owned validated batches, and save backend/runtime state needed to resume safely.

Finish backend persistence: selected backend mode, adapter identity, validation profile, brain residency slots, class bucket allocations, active profile caps, shader/ABI versions, CPU shadow parity status, last safe checkpoint, fallback reason, and selected scale profile. Save files must not serialize wgpu handles; they store stable descriptors and recreate resources on load.

Implement `MinimumSettings30x30`: 30 real creatures at 30 FPS with strict renderer reserve, strict neural heap, strict staging/readback budget, limited hot/warm slots, low chunk radius, conservative VFX, and adaptive throttling before hitches. Implement `MinSpecComfort1080p`: renderer reserve, neural heap, staging/readback budget, max hot/warm/cold creatures, chunk radius coupling, VFX budget, frame timing thresholds, and adaptive throttling order for RTX 3050/i7. Protect sensory, metabolic, motor, and homeostatic lobes before decimating association/lexicon/memory cadence. GPU timing overruns must degrade gracefully rather than hitch.

Implement scale-up profile support: stronger hardware may increase hot/warm brain slots, brain class budgets, population tiers, chunk radius, VFX density, and diagnostic depth without changing save schemas or core/world ABI. ResearchScale may exceed comfort targets but must keep honest diagnostics and no-readback rules.

Remove fake availability toggles from product decisions. Environment flags may force diagnostics but cannot fake hardware proof. The app must report GPU selected only after real probe and validation succeed. CPU fallback remains allowed but visible.

Validation: cargo checks/tests; ignored/manual GPU tests on RTX 3050 if available; production graphical app with gpu-runtime under `MinimumSettings30x30` and `MinSpecComfort1080p`; save/load with GPU backend state; no active bulk readback; CPU shadow parity for bounded validation; performance receipt written under target/artifacts only, not committed.

Completion requires real gameplay path integration, not just benchmark smoke, backend work saved/resumable, the 30-creature/30-FPS floor, minimum-spec comfort, and scale-up profile support.
```

## FVR07 - Art, assets, VFX, audio-visual polish, and license manifest

```text
Goal: Complete FVR07 for A-Life. Finish the visual/audio polish and asset pipeline so the voxel frontend looks like a real stylized game, not programmer art. External assets are allowed only with committed license/source/digest metadata.

Implement a production asset pipeline using bevy_asset_loader or equivalent. Add asset manifest schema fields for source, license, digest, author, local path, generated-vs-external flag, and replacement policy. Validate licenses and reject missing/unknown licenses. Do not commit large packs, caches, captures, or generated target artifacts.

Create or ingest the finished visual set: voxel material atlas/palette, terrain/water/decay/resource/hazard materials, creature sprites/meshes/texture atlases, props, nests, corpses, food/resource objects, selection/hover effects, UI icons, fonts if licensed, and environment dressing. Use external CC0/permissive assets when useful; otherwise generate simple coherent assets procedurally and commit source generation scripts/configs if needed.

Implement GPU VFX: pheromone trails, spores, sleep/consolidation glows, danger/hazard particles, eating/resource effects, birth/death effects, water/decay ambient motion, and selected-creature neural pulse. VFX must be budgeted and adaptive on RTX 3050. It must never drive sim state.

Implement final stylization: toon/low-poly/voxel-consistent lighting, outlines, fog, color grading, height readability, night/day or biome mood if production-ready, and screenshot-free validation receipts. Ensure art loads through production states and missing assets produce clear errors or licensed generated fallbacks.

Tune assets and VFX for `MinimumSettings30x30` first, preserving readability and essential feedback at 30 creatures / 30 FPS. Tune `MinSpecComfort1080p` as the default polished minimum-spec experience. Higher profiles may increase texture resolution, particle density, shadow quality, foliage/prop density, and animation richness without requiring asset pipeline rewrites.

Validation: cargo checks/tests; asset manifest validation; launch production app from clean checkout; verify no placeholder art entries are marked final; verify license metadata for every external asset; verify VFX budget toggles and quality profiles; verify the minimum-settings floor remains readable; boundary scripts.

Completion requires a coherent polished visual presentation and licensed asset pipeline that meets the minimum floor, runs comfortably at the minimum spec, and scales upward.
```

## FVR08 - Final replacement hardening, packaging, and acceptance

```text
Goal: Complete FVR08 for A-Life. Finish the replacement. Remove or quarantine obsolete ugly frontend code, harden desktop packaging, run final validation, and make the voxel frontend the finished product path.

Audit all old graphical/player/True2.5D/alpha/smoke frontend paths. Delete, replace, or demote them to explicit legacy regression commands if still valuable. New docs, commands, menus, and status files must describe the production voxel frontend, not alpha/practice slices. Do not leave duplicate player paths that confuse Codex or users.

Implement final desktop packaging for Windows 10: production run script, config defaults, save directory policy, asset inclusion, license bundle, GPU fallback diagnostics, crash/error reporting text, README launch instructions, clean first-run behavior, `MinimumSettings30x30` fallback, and default `MinSpecComfort1080p` profile selection on Cassidy's RTX 3050/i7 machine.

Implement final acceptance validation: 1080p default launch; load/create/save/reload; voxel chunks stream and persist; creatures visible/selectable/animated; overlays and UI work; GPU backend selected on RTX 3050 when validated; adaptive quality profiles work; profile-driven scale-up paths exist; 1/10/30/50/100/250/500 population tiers run with honest measured receipts; `MinimumSettings30x30` meets 30 creatures at 30 FPS; core boundary scripts pass; all docs updated; no large artifacts committed; no unlicensed assets; no mock sim/backends remain in product path.

Performance acceptance on Cassidy's machine: `MinimumSettings30x30` must be playable at 30 real creatures / 30 FPS. `MinSpecComfort1080p` targets comfortable 60 FPS at 1080p for the default population and world settings. For higher tiers including 500 agents, either sustain 60 FPS or automatically enter a visible adaptive profile that preserves playability and records the exact degraded budgets. Do not fabricate numbers; record real adapter/FPS/frame-time/VRAM diagnostics when available.

Update docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FINAL_ACCEPTANCE.md with commands run, results, hardware, selected backend, measured performance, saved-state proof, deleted old frontend files, remaining legacy regression commands, minimum-settings floor evidence, min-spec comfort evidence, scale-up profile evidence, and exact product launch instructions.

Completion requires the old ugly frontend to be replaced by the finished voxel frontend with no owned work deferred, the 30-creature/30-FPS floor established, minimum-spec comfort established, and later scale-up preserved.
```
