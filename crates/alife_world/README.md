# alife_world

Bevy-independent world and ecology contracts.

This crate should define authoritative world-side legality, sensory extraction interfaces, organism/world concepts, and future headless harness boundaries. It may depend on `alife_core` but should not become the Bevy ECS adapter.

Habitat authority stores one deterministic habitat membership per creature and
an append-only transfer ledger with typed provenance. Wild, Reserve, Managed,
and School change world permissions only. They all retain
`PolicyBackend::NeuralClosedLoopGpu` as the cognition identity.

`HeadlessWorld::habitat_presentation_projection` is a read-only presentation
boundary. It returns stable organism/world IDs, literal creature utterance token
IDs only when backed by utterance-level grounding evidence, and only relationship
evidence present in current world snapshots. Emission alone does not prove
grounding. Missing trust, fear, grounded speech, or entity evidence is `Unknown`
or `None`; the projection does not infer it and exposes no mutation path.
