# alife_school Instructions

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
- Keep provider integration vendor-neutral unless an explicit spec changes it.
