# alife_bevy_adapter Instructions

Architecture authority:

- `../../docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`
  is the single normative source.
- This file records current implementation guardrails only. Existing Rust
  structures, processor placement, GPU layouts, constants, brain-size
  assumptions, adapters, tests, and fixtures do not amend v2.0.
- Earlier architecture documents are historical. Report conflicts as
  `AOA-*` gaps and do not start an unrequested repair pass.

This crate controls Bevy-specific app wiring, plugins, rendering, ECS
integration, debug UI, and eventual demo scenes.

Rules:

- Bevy is the host/game adapter, not the cognitive core.
- Do not move core cognitive contracts into ECS component definitions.
- Do not implement neural kernels here.
- Keep UI/debug surfaces consistent with the docs and verify screenshots when visual work is requested.
- Any teacher or semantic-prior interaction must pass through the appropriate crate boundary.
