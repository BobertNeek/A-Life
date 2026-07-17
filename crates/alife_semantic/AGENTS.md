# alife_semantic Instructions

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
- Keep provider traits vendor-neutral.
