# alife_bevy_adapter Instructions

Root [AGENTS.md](../../AGENTS.md) applies. These local rules are current
implementation guardrails; they do not amend the controlling v2.0 architecture.

This crate controls Bevy-specific app wiring, plugins, rendering, ECS
integration, debug UI, and eventual demo scenes.

Rules:

- Bevy is the host/game adapter, not the cognitive core.
- Do not move core cognitive contracts into ECS component definitions.
- Do not implement neural kernels here.
- Keep UI/debug surfaces consistent with the docs and verify screenshots when visual work is requested.
- Any teacher or semantic-prior interaction must pass through the appropriate crate boundary.
