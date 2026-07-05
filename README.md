# A-Life

A-Life is a Rust + Bevy + wgpu/WebGPU + WGSL artificial-life simulation project. Its desktop product path is the Bevy 0.18 production voxel frontend backed by real A-Life saved world/core/runtime data.

Launch the production voxel frontend on Windows:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
```

Dry-run the launch without opening the window:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun
```

Build the local Windows production package:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_production_voxel.ps1
```

Default profile: `MinSpecComfort1080p` at 1920x1080. Minimum fallback profile:
`MinimumSettings30x30`, the 30-creature/30-FPS floor. Production launch uses
`bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi` and reports
GPU fallback diagnostics honestly.

Start with:

- `docs/master_spec.md`
- `docs/architecture_decisions.md`
- `docs/codex_handoff_prompt.md`
- `docs/release_checklist.md`
- `docs/final_status_report.md`

This repository should not use Unity, C#, or HLSL production shaders. `Standard2048` is only a reference brain class; the architecture is scalable.

On Windows, use the PowerShell validation wrappers in `scripts/check.ps1`,
`scripts/check_core_boundaries.ps1`, and `scripts/docs_check.ps1` so validation
uses Git Bash instead of accidentally invoking WSL.
