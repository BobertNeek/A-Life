# A-Life

A-Life is a Rust + Bevy + wgpu/WebGPU + WGSL artificial-life simulation project. It uses scalable sparse Hebbian/Oja brain classes, genome-controlled brain topology, neurochemistry, evolution, internal semantic priors, and future-compatible schooling/teacher architecture.

Start with:

- `docs/master_spec.md`
- `docs/architecture_decisions.md`
- `docs/codex_handoff_prompt.md`
- `docs/release_checklist.md`
- `docs/final_status_report.md`

The current codebase is a scaffold: it defines crate boundaries, public contract
types, and invariant tests. It does not implement neural runtime kernels.

This repository should not use Unity, C#, or HLSL production shaders. `Standard2048` is only a reference brain class; the architecture is scalable.

On Windows, use the PowerShell validation wrappers in `scripts/check.ps1`,
`scripts/check_core_boundaries.ps1`, and `scripts/docs_check.ps1` so validation
uses Git Bash instead of accidentally invoking WSL.
