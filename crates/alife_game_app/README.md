# alife_game_app

`alife_game_app` owns the application shell, launch policy, runtime scheduling,
controls, diagnostics, and voxel presentation for A-Life.

## Player controls

Normal play shows a small population/pause indicator and the selected creature's
needs. F1 opens the controls. F3 toggles debug mode, which contains performance,
brain and memory details, speech diagnostics, and world overlays. Debug mode is
off by default; `--developer-overlay` enables it at launch. M/G/H and the overlay
shortcuts operate only in debug mode. Live FPS appears in the debug bar and uses
real frame time, including while paused. R restores the player view.

## Production path

The launcher uses an optimized release build by default. The first build compiles
the engine dependencies; subsequent launches reuse them. Pass `-BuildProfile dev`
for quick unoptimized iteration. `-Manifest PATH` selects a compatible saved
environment. Hidden debug panels skip text construction and
inspector work, then refresh from current state when reopened.

Launch the current player-facing frontend on Windows:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
```

The bundled default save currently fails migration because it lacks the genetic
biochemical graph. New Game also needs a compatible environment manifest for
its configuration and assets. With one available, build and run:

```powershell
cargo build -p alife_game_app --features production-voxel-frontend --bin alife_game_app
.\target\debug\alife_game_app.exe production-voxel --manifest <compatible-environment-manifest.json> --new-game --seed 96002 --population 1 --graphics-backend vulkan --require-gpu
```

New Game accepts 1â€“8 creatures, including a single-creature care test. Windows MSVC builds reserve a 16 MiB
main-thread stack for loading the production world. Dev builds omit debug
symbols for `wgpu-hal` and this crate to avoid observed Rust 1.96/LLVM 22 Windows
code-generation crashes; debug assertions remain enabled.

The 2026-09-12 playtest found and repaired an omitted cognitive-record size in
GPU upload preparation. The single-creature Vulkan run then executed neural
actions, moved, saved at tick 49, reloaded through the game control, and continued
running. Help, pause/resume, ground selection, and debug controls also worked.
Results were checked through in-engine captures because Windows capture failed.
Normal play now reports simulation failure instead of hiding it in debug mode.

The renderer maps simulation XY to Bevy XZ without snapping movement to tiles.
E places food on selected ground, including a selected creature's tile. Food
appears from authoritative world objects even while paused and disappears on
consumption. Decorative oak trees hide when their canopy blocks a creature from
the camera. Repeated placements receive distinct identities. Placement uses the
editor's supported 512-unit bound instead of its old 12-unit demo default.

The September 12 focused checks passed for XY movement, repeated placement, and
food visual removal. Recorded single-creature play verified placement while
paused, starting-food consumption, and save/reload of placed food. The full care
loop remains unaccepted. Without continuous capture, the development build showed
roughly 9–11 FPS running versus 37 paused on the RTX 3050. The main-thread tick
path synchronously waits for GPU readback; its share of frame cost still needs
measurement. Speech is untested. The retained September 8 learned save still fails
cognitive-snapshot decoding (`motor_condition_magnitude` is missing).

Build its local package with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_production_voxel.ps1
```

The production shell requires GPU-authoritative neural execution. A missing or
failed GPU neural path is reported as unavailable; it does not silently switch
to CPU neural math.

The shell restores and ticks a real `GpuLiveBrainRuntime`. The active renderer
uses its internal layered-grid terrain meshes as the sole terrain drawing path.
It starts creature presentation from the selected save, then projects live
authoritative positions, adds newborn presentation records, and consumes the
runtime retirement queue. The renderer does not own lifecycle decisions. See
`docs/STATUS.md` and `docs/ROADMAP.md` for the exact evidence boundary.

Graphics work still open:

- dynamic overlay contents are snapshot-derived, although their GPU meshes are
  created only when first shown;
- creature-attached effects follow live positions, but effect creation and
  removal are not yet driven by live tick events;
- camera-distance creature LOD is not implemented;
- current-source Vulkan performance and rendered lifecycle evidence must be
  refreshed after graphics work.

## Validation and diagnostic commands

The current binary exposes production asset validation plus source-bound GPU
acceptance and evidence commands. Use `--help` for their current arguments.
These commands do not make a CPU helper a product fallback.

```powershell
cargo run -p alife_game_app --bin alife_game_app -- validate-production-assets
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-closed-loop-acceptance --help
cargo run -p alife_game_app --features gpu-tests --bin alife_game_app -- gpu-closed-loop-soak --help
```

Historical milestone smoke runners and command catalogs are archived under
`archive/legacy_app_milestones`. They are not supported CLI commands.

## Ownership

- `alife_world` owns perception facts, unscored candidates, legality, targets,
  action execution, outcomes, and stable world identity.
- `alife_runtime` and `alife_gpu_backend` own production neural state and GPU
  execution.
- This crate schedules those systems and translates read-only state for Bevy.
- UI and render code must not become simulation authority.

Current pause and load controls do not yet fully control or replace the live GPU
runtime. Treat them as partial until the causal gates in `docs/ROADMAP.md` pass.

## Validation

Use the repository's Windows wrappers and the smallest focused test that can
falsify a change. Current commands and evidence rules are in
`docs/DEVELOPMENT.md` and `docs/EVIDENCE.md`.
