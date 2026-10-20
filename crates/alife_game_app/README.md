# alife_game_app

`alife_game_app` owns the application shell, launch policy, runtime scheduling, controls, foundation training runner, diagnostics, and voxel presentation for A-Life.

## Player controls

Normal play shows a population/pause indicator, the selected creature's physiology HUD, and the five-control care dock (`E` place food, `C` gentle touch, `J` place toy, `K` praise, `Enter` speech composer). `F2` names the selected creature. `F1` opens keyboard help. `F3` toggles debug mode (performance, brain and memory details, speech diagnostics, and world overlays). `R` restores the starting camera view.

For a bounded behavior trace, set `ALIFE_ACTION_TRACE_PATH` to a new JSONL file in an existing directory. It records up to 10,000 distinct presented world ticks, including actions, targets, outcomes, sleep state, positions, and food state, using existing presentation data without extra neural readback.

Sleep readiness follows fatigue, sleep pressure, and neural recovery. Deaths complete the archive transaction before GPU and visual retirement.

## Production path

The launcher uses an optimized release build by default (`-BuildProfile dev` selects an unoptimized build for iteration):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -NewGame -Population 1
```

Build the local Windows package with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_production_voxel.ps1
```

The production shell requires GPU-authoritative neural execution. A missing or failed GPU neural path is reported as unavailable; it does not silently switch to CPU neural math.

The shell restores and ticks a live `GpuLiveBrainRuntime`. The active renderer uses its layered-grid terrain meshes, starts creature presentation from the selected save, projects live authoritative positions, adds newborn presentation records, and consumes the runtime retirement queue. During foundation training (`foundation-training` feature), mutable GPU state before and after each tick is captured across all active creatures in two batched GPU submissions (`capture_training_state_batch`).

## Validation and diagnostic commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- validate-production-assets
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-closed-loop-acceptance --help
cargo run -p alife_game_app --features gpu-tests --bin alife_game_app -- gpu-closed-loop-soak --help
```

## Ownership

- `alife_world` owns perception facts, unscored candidates, legality, targets, action execution, outcomes, and stable world identity.
- `alife_runtime` and `alife_gpu_backend` own production neural state and GPU execution.
- This crate schedules those systems and translates read-only state for Bevy.
- UI and render code never own simulation truth.
