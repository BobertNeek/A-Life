# alife_world Instructions

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
