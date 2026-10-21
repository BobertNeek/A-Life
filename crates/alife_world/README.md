# alife_world

Engine-independent world simulation and ecology.

Canonical New Game accepts 1–8 founders. A single founder supports a focused
creature-care playtest; the default population remains six.

`HeadlessWorld` runs world physics, perception, physiology, ecology, and action
validation for both graphical and headless hosts. It uses `alife_core` and keeps
rendering and engine handles outside saved world state.

Habitat authority stores one deterministic habitat membership per creature and
a transfer ledger with typed provenance. Retirement removes that creature's
entries while preserving surviving sequence IDs. Wild, Reserve, Managed,
and School change world permissions only. They all retain
`PolicyBackend::NeuralClosedLoopGpu` as the cognition identity.

`HeadlessWorld::habitat_presentation_projection` is a read-only presentation
boundary. It returns stable organism/world IDs, literal creature utterance token
IDs only when backed by utterance-level grounding evidence, and only relationship
evidence present in current world snapshots. Emission alone does not prove
grounding. Missing trust, fear, grounded speech, or entity evidence is `Unknown`
or `None`; the projection does not infer it and exposes no mutation path.
