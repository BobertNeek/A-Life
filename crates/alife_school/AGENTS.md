# alife_school Instructions

This crate controls external teacher LLM roles, lesson APIs, verifiers,
curricula, school progress, and in-world teaching object contracts.

Rules:

- External teachers must teach through ordinary perception: speech, writing,
  gesture, demonstrations, objects, and feedback.
- Do not inject hidden vectors, direct rewards, or weight edits into creatures.
- Keep teacher-private planning state separate from creature memory.
- Do not bind the scaffold to a specific LLM vendor without an explicit spec update.
