# alife_world Instructions

This crate controls Bevy-independent world concepts: ecology, organisms,
resources, drives, lesson-world APIs, and sensory extraction contracts.

Rules:

- Keep the world layer authoritative for action legality and outcomes.
- Use stable IDs and core ABI types rather than Bevy ECS internals.
- Do not store renderer, GPU backend, or teacher-private state here.
- Do not let neural outputs bypass world validation.
- Keep scaffold code minimal until the spec asks for runtime behavior.
