# A-Life

A-Life is a Rust, Bevy, wgpu, and WGSL artificial-life research project. It is building persistent embodied organisms whose neural policy runs on the GPU and whose actions remain subject to an authoritative world.

The current product boundary is narrower than the ambition. `production-voxel` starts and ticks a GPU-authoritative cognition loop. The visible scene starts from the selected save, then projects live authoritative creature positions, adds newborns, and applies runtime retirement events. Fresh rendered lifecycle proof and the full player loop remain open.

## Run the current voxel frontend

Windows prerequisites:

- a current Rust toolchain;
- Git for Windows, used by the repository's validation wrappers;
- a Vulkan-capable adapter for GPU neural execution.

Launch the frontend:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
```

Run the application's manifest, asset, save, and GPU preflight without opening a window:

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
- WGSL selection, learning, memory, topology, sleep, and checkpoint paths are implemented in the GPU runtime.
- The production shell constructs and ticks that runtime.
- The voxel UI provides camera, selection, inspector, speech, save, load, pause, speed, and overlays.
- The renderer projects tick-bound runtime transforms, births, and retirement events without owning simulation truth.
- The production graphics path uses one layered-grid terrain renderer and one lighting authority. Overlay geometry is created on demand.
- EI0 passed a bounded source-bound exit gate. EI1 produced a complete source-bound corpus but remains `Blocked`.
- This is a research alpha, not a release-ready autonomous simulation.

Read [current status](docs/STATUS.md) before treating a feature as integrated or proven.
Read the [v2.0 controlling architecture](docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md)
before treating any current implementation choice as a permanent requirement.

## Project map

- [Vision](docs/VISION.md) — what the project is trying to become
- [Controlling architecture](docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md) — the single normative source and `AOA-*` compliance standard
- [Status](docs/STATUS.md) — implemented, integrated, visible, and proven state
- [Current implementation map](docs/ARCHITECTURE.md) — non-normative ownership and production data flow
- [Roadmap](docs/ROADMAP.md) — the shortest path from here to the aspiration
- [Development](docs/DEVELOPMENT.md) — supported Windows workflows and gates
- [Evidence](docs/EVIDENCE.md) — receipts, source binding, EI0, and EI1
- [Current implementation reference](docs/REFERENCE.md) — non-normative technical conventions and evidence terms

## Current implementation boundaries

These boundaries describe the current codebase. They do not amend or narrow
the v2.0 architecture.

- Production neural execution is GPU-authoritative WGSL. There is no automatic CPU neural fallback or live CPU parity shadow.
- The world owns legality, targets, action execution, and outcomes.
- Teachers communicate through normal perception. The private local SLM cannot act for an organism or write its mind.
- `Standard2048` is a reference profile, not the global brain topology. N4096 remains research-only.
- Missing evidence is `Unknown` or `Blocked`, never a pass.

Use the PowerShell wrappers in `scripts/` on Windows. They route shell checks through Git Bash rather than WSL.

Retired renderer assets and milestone smoke/report code are preserved under
`archive/legacy_true25d` and `archive/legacy_app_milestones`. Nothing under
`archive/` is compiled, packaged, or treated as current product guidance.
