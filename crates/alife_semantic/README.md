# alife_semantic

Private semantic priors, speech translation and bounded context adapters.

- `local-llamacpp` provides local embeddings, asynchronous SLM priors, and speech
  translation with their own validated model/connection configurations.
- `gaussian-adapter` converts Gaussian observations and semantic bindings into
  bounded optional contexts.
- `fake-semantic-provider` supplies deterministic contexts for headless tests.

Recorded prior banks replay exact request contexts. Runtime queues bound pending
work. Developmental fading reduces assistance after successful unaided probes.
Providers return context hints or translations; the organism runtime owns world
actions and neural learning. Missing optional providers stay nonfatal.
