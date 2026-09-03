# alife_game_app

`alife_game_app` owns the application shell, launch policy, runtime scheduling,
controls, diagnostics, and voxel presentation for A-Life.

## Production path

Launch the current player-facing frontend on Windows:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
```

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
