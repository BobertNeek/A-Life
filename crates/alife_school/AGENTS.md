# alife_school Instructions

Architecture authority:

- `../../docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`
  is the single normative source.
- This file records current implementation guardrails only. Existing Rust
  structures, processor placement, GPU layouts, constants, brain-size
  assumptions, adapters, tests, and fixtures do not amend v2.0.
- Earlier architecture documents are historical. Report conflicts as
  `AOA-*` gaps and do not start an unrequested repair pass.

This crate controls external teacher LLM roles, lesson APIs, verifiers,
curricula, school progress, and in-world teaching object contracts.

Rules:

- External teachers must teach through ordinary perception: speech, writing,
  gesture, demonstrations, objects, and feedback.
- Teach vocabulary through spatial hearing, visible objects, demonstrations,
  and sealed outcomes.
- Run language mastery gates with SLM translation disabled.
- Do not inject hidden vectors, direct rewards, or weight edits into creatures.
- Keep teacher-private planning state separate from creature memory.
- Teacher requests never become scored candidates, selected actions, or hidden
  semantic activations.
- `language_nursery.rs` is the canonical vocabulary-exposure path and must keep
  player, teacher, and peer speech on the same spatial hearing and grounded
  perception route.
- Keep provider integration vendor-neutral unless an explicit spec changes it.
