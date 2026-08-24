# alife_semantic Instructions

Architecture authority:

- `../../docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`
  is the single normative source.
- This file records current implementation guardrails only. Existing Rust
  structures, processor placement, GPU layouts, constants, brain-size
  assumptions, adapters, tests, and fixtures do not amend v2.0.
- Earlier architecture documents are historical. Report conflicts as
  `AOA-*` gaps and do not start an unrequested repair pass.

This crate controls bounded internal semantic-prior and speech-translation
provider interfaces.

Rules:

- Internal SLM is a private subconscious semantic prior, not an actor.
- It may bias attention, lexicon/concept activity, recall, or bounded plasticity modulation.
- It may not issue actions, bypass action arbitration, directly rewrite weights, or act as a teacher.
- `SemanticPriorRequest` and `SpeechTranslationRequest` remain separate schemas.
- Translation may map or render bounded raw tokens; it may not author creature
  thought or speech.
- Unknown concepts remain novel tokens, and uncertain rendering remains visibly
  uncertain.
- Deterministic tokenization may never label a receipt SLM-assisted. Assisted
  receipts require a successful bounded provider response that passes content
  validation.
- Keep provider traits vendor-neutral.
