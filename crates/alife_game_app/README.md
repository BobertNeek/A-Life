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

The shell restores and ticks a real `GpuLiveBrainRuntime`. The active voxel
renderer still builds creature records from the selected save and animates
saved base positions. It does not yet project live runtime transforms, births,
or deaths. See `docs/STATUS.md` and `docs/ROADMAP.md` for the exact boundary.

## Headless and diagnostic commands

The crate retains headless smokes for contracts and developer diagnosis. They
are not production-authority evidence and do not make a CPU helper a product
fallback.

```powershell
cargo run -p alife_game_app --bin alife_game_app -- headless-smoke crates/alife_world/tests/fixtures/p34
cargo run -p alife_game_app --bin alife_game_app -- visible-signature crates/alife_world/tests/fixtures/p34
cargo run -p alife_game_app --bin alife_game_app -- live-brain-tick-smoke crates/alife_world/tests/fixtures/p34
cargo run -p alife_game_app --bin alife_game_app -- creature-inspector-smoke crates/alife_world/tests/fixtures/p34
cargo run -p alife_game_app --bin alife_game_app -- school-mode-smoke
cargo run -p alife_game_app --bin alife_game_app -- semantic-provider-smoke
cargo run -p alife_game_app --bin alife_game_app -- gpu-product-smoke
```

The school smoke proves only its bounded perception and arbitration contracts.
The semantic smoke uses disabled or deterministic fake providers. Neither is a
live GPU school or private-SLM cognition proof.

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
