# alife_semantic Instructions

This crate controls internal semantic-prior interfaces and optional provider
stubs.

Rules:

- Internal SLM is a private subconscious semantic prior, not an actor.
- It may bias attention, lexicon/concept activity, recall, or bounded plasticity modulation.
- It may not issue actions, bypass action arbitration, directly rewrite weights, or act as a teacher.
- Keep provider traits vendor-neutral during scaffold work.
