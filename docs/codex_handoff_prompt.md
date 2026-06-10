# Codex Handoff Prompt for A-Life

## Recommended `/goal`

Paste this first. It is deliberately short enough for `/goal`.

```text
/goal Scaffold A-Life according to docs/master_spec.md and docs/architecture_decisions.md: Rust+Bevy+wgpu/WebGPU+WGSL only; scalable brain classes; genetics/chemistry/evolution contracts; internal semantic prior separate from external teacher; docs + crate skeletons + tests only, no runtime neural kernels.
```

## Main Codex Prompt

Paste this after setting `/goal`.

```text
Read these files first:
- docs/master_spec.md
- docs/architecture_decisions.md
- docs/future_research_compatibility.md
- docs/schooling_and_teacher_architecture.md
- docs/codex_handoff_prompt.md

Task: update the repository scaffold to match the spec. Do not implement neural runtime kernels. Do not create Unity files. Do not author HLSL. Use Rust + Bevy + wgpu/WebGPU + WGSL only.

Create or update:
- Cargo workspace
- crates/alife_core
- crates/alife_world
- crates/alife_gpu_backend
- crates/alife_bevy_adapter
- crates/alife_school
- crates/alife_semantic
- crates/alife_tools
- scripts/setup.sh
- scripts/build.sh
- scripts/test.sh
- scripts/graphify.sh
- AGENTS.md plus local AGENTS.md files using DOX-style hierarchy
- README.md pointing to docs/master_spec.md

Implement only type skeletons and invariant tests:
- BrainScaleTier
- BrainClassSpec
- LobeLayout
- BrainGenome
- EndocrineProfile
- ExperiencePatchHeader
- ActionCommand
- SemanticPriorProvider
- NeuralComputeBackend
- LineageExportManifest

Important: 2048 is only Standard2048. Brains are scalable classes. The internal SLM is a private semantic prior. The teacher LLM is an external in-world teacher using hearing/vision/writing/gesture/object channels. Graphify and DOX are developer tooling only.

Before editing, produce a concise plan. After editing, run cargo fmt and cargo check --workspace if possible. Report any dependency or environment blockers exactly.
```

## Graphify Setup Prompt

```text
Install project-scoped Graphify guidance if available. Prefer uv tool install graphifyy, then graphify install --project --platform codex. Add scripts/graphify.sh but do not make Graphify required for cargo build.
```

## DOX Setup Prompt

```text
Initialize a DOX-style AGENTS.md tree: root AGENTS.md with project-wide instructions and child AGENTS.md files in docs/ and each crate. Local instructions must state what files in that area control and what architectural rules cannot be violated.
```
