# AGENTS.md — A-Life Root Instructions

Read `docs/master_spec.md` and `docs/architecture_decisions.md` before edits.

Non-negotiable rules:

- Rust + Bevy + wgpu/WebGPU + WGSL only.
- No Unity.
- No HLSL production shaders.
- No fixed global 2048-neuron brain assumption.
- `Standard2048` is a reference tier only.
- Use scalable brain classes and sparse class-bucketed storage.
- Internal SLM is a private subconscious semantic prior.
- External teacher LLM teaches through ordinary perception.
- Do not implement neural runtime kernels during scaffold phase.
- Keep docs and local AGENTS.md files updated after meaningful changes.
- Prefer Graphify queries for architecture questions when installed.
