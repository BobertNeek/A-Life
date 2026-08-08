# A-Life

A-Life is a Rust, Bevy, wgpu, and WGSL artificial-life research project. It is building persistent embodied organisms whose neural policy runs on the GPU and whose actions remain subject to an authoritative world.

The current product boundary is narrower than the ambition: `production-voxel` starts and ticks a real GPU-authoritative headless cognition loop, while the visible voxel scene is still reconstructed from a save. Live world-to-voxel synchronization, autonomous birth and death, and a causally complete player loop remain open.

## Run the current voxel frontend

Windows prerequisites:

- a current Rust toolchain;
- Git for Windows, used by the repository's validation wrappers;
- a Vulkan-capable adapter for GPU neural execution.

Launch the frontend:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
```

Inspect the launch without opening a window:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun
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
- The renderer does not yet project live runtime transforms, births, or deaths.
- EI0 passed a bounded source-bound exit gate. EI1 produced a complete source-bound corpus but remains `Blocked`.
- This is a research alpha, not a release-ready autonomous simulation.

Read [current status](docs/STATUS.md) before treating a feature as integrated or proven.

## Project map

- [Vision](docs/VISION.md) — what the project is trying to become
- [Status](docs/STATUS.md) — implemented, integrated, visible, and proven state
- [Architecture](docs/ARCHITECTURE.md) — ownership and production data flow
- [Roadmap](docs/ROADMAP.md) — the shortest path from here to the aspiration
- [Development](docs/DEVELOPMENT.md) — supported Windows workflows and gates
- [Evidence](docs/EVIDENCE.md) — receipts, source binding, EI0, and EI1
- [Reference](docs/REFERENCE.md) — stable technical and research invariants

## Non-negotiable boundaries

- Production neural execution is GPU-authoritative WGSL. There is no automatic CPU neural fallback or live CPU parity shadow.
- The world owns legality, targets, action execution, and outcomes.
- Teachers communicate through normal perception. The private local SLM cannot act for an organism or write its mind.
- `Standard2048` is a reference profile, not the global brain topology. N4096 remains research-only.
- Missing evidence is `Unknown` or `Blocked`, never a pass.

Use the PowerShell wrappers in `scripts/` on Windows. They route shell checks through Git Bash rather than WSL.
