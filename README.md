# A-Life

A-Life is a Rust, Bevy, wgpu, and WGSL artificial-life game and research project building persistent embodied organisms whose neural policy runs on the GPU and whose actions remain subject to an authoritative world.

`production-voxel` starts and ticks a GPU-authoritative cognition loop. The visible scene starts from the selected save, then projects live authoritative creature positions, adds newborns, and applies runtime retirement events.

## Run the current voxel frontend

Windows prerequisites:

- a current Rust toolchain;
- Git for Windows, used by the repository's validation wrappers;
- a Vulkan-capable adapter for GPU neural execution.

Start a fresh game with one creature:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -NewGame -Population 1
```

New Game creates a current-format creature and save. The launcher chooses a fresh seed; pass `-Seed` to choose one explicitly. Existing save files are preserved.

Check an existing save's manifest, assets, and GPU setup without opening a window (`-DryRun` does not exercise New Game):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun
```

Print the Cargo command without executing it:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -PreviewCommand
```

Build the local Windows package:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_production_voxel.ps1
```

`MinSpecComfort1080p` is the default presentation profile. `MinimumSettings30x30` is the graphics floor. A graphics setting is not a neural fallback: production cognition remains GPU-authoritative, and unavailable neural hardware is reported as unavailable.

## What exists today

- World perception, unscored candidate generation, action validation, outcomes, and sealed experience are implemented.
- WGSL selection, signed action/speech decoding, credit-gated fast plasticity, episodic memory, topology, sleep consolidation, and batched training state capture are implemented in the GPU runtime.
- The production shell constructs and ticks that runtime.
- The voxel UI provides camera, selection, care dock, inspector, naming, speech, save, load, pause, speed, and overlays.
- The renderer projects tick-bound runtime transforms, births, and retirement events without owning simulation truth.
- Read [current status](docs/STATUS.md) before treating a feature as integrated or proven.
- Read the [organism and gameplay design](docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md) before treating any current implementation choice as a permanent requirement.

## Project map

- [Vision](docs/VISION.md) — what the project is trying to become
- [Organism and gameplay design](docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md) — organism behavior, gameplay goals, and longer-term research
- [Status](docs/STATUS.md) — implemented, integrated, visible, and proven state
- [Current implementation map](docs/ARCHITECTURE.md) — ownership and production data flow
- [Roadmap](docs/ROADMAP.md) — the shortest path from here to the aspiration
- [Development](docs/DEVELOPMENT.md) — supported Windows workflows and gates
- [Evidence](docs/EVIDENCE.md) — receipts, source binding, EI0, and EI1
- [Current implementation reference](docs/REFERENCE.md) — technical conventions and evidence terms

## Current implementation boundaries

These boundaries describe the current codebase. They do not amend or narrow the v2.0 architecture.

- Production neural execution is GPU-authoritative WGSL. There is no automatic CPU neural fallback or live CPU parity shadow.
- The world owns legality, targets, action execution, and outcomes.
- Teachers communicate through normal perception. The private local SLM cannot act for an organism or write its mind.
- `Standard2048` is a reference profile, not the global brain topology. N4096 remains research-only.
- Missing evidence is `Unknown` or `Blocked`, never a pass.

Use the PowerShell wrappers in `scripts/` on Windows. They route shell checks through Git Bash rather than WSL.
