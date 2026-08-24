# alife_world Instructions

Architecture authority:

- `../../docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`
  is the single normative source.
- This file records current implementation guardrails only. Existing Rust
  structures, processor placement, GPU layouts, constants, brain-size
  assumptions, adapters, tests, and fixtures do not amend v2.0.
- Earlier architecture documents are historical. Report conflicts as
  `AOA-*` gaps and do not start an unrequested repair pass.

This crate controls Bevy-independent world concepts: ecology, organisms,
resources, drives, lesson-world APIs, and sensory extraction contracts.

Rules:

- Do not depend on Bevy, wgpu, renderer types, or OS handles.
- Bevy ECS ownership belongs only to adapter/app layers.
- Keep the world layer authoritative for action legality and outcomes.
- Keep every candidate unscored and derived from the same authoritative world
  snapshot bound into promotion evidence.
- Use stable IDs and core ABI types rather than Bevy ECS internals.
- Do not store renderer, GPU backend, or teacher-private state here.
- Do not let neural outputs bypass world validation.
- Player, creature, and teacher speech are spatial world perception.
- `Vocalize` is an unscored opportunity whose payload is selected by the GPU
  brain.
- Death archiving completes before GPU retirement and despawn.
- Implement reviewed world behavior through focused modules and stable contracts.
