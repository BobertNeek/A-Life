# alife_semantic

Internal private semantic-prior boundary.

This crate owns optional semantic prior providers and modulation packet adapters. The semantic prior is private to the organism and may bias perception or salience, but it cannot issue actions, bypass arbitration, or mutate weights.

Plan P22 adds adapter-side Gaussian/semantic context conversion as optional functionality behind feature flags:

- `gaussian-adapter`: exposes stable Gaussian cluster and semantic conversion helpers plus optional provider bundle traits.
- `fake-semantic-provider`: enables a deterministic fake provider for headless, offline, and test usage when no renderer/splat source exists.

Builds without these features remain valid; absence of a semantic/Gaussian source is non-fatal and yields `None` optional contexts.
