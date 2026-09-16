# Practical breed promotion

User-approved scope, 11 September 2026. Controlling requirements:
AOA-ASSIM-003 through AOA-ASSIM-009, revision 2.0.1 of the controlling architecture.

Get the everyday care loop playable first. Then let the player select a capable
individual and create a named breed from its compatible genome and learned
foundation. Keep useful abilities, start fresh personal records, and accept some
personal associations remaining in weights. Preserve the source individual.
Ordinary mating must not silently copy a parent's acquired brain.

## Existing code to reuse

- `crates/alife_core/src/foundation.rs`: `FoundationWeightAsset::from_trained_weights`,
  `from_promoted_weights`, and canonical encoding already package immutable neural
  weights with compatibility and provenance. These constructors do not extract
  learned weights from a live individual or reset its memories.
- `crates/alife_archive/src/bundle.rs`: `ResolvedGpuFounderCheckpoint` carries saved
  GPU state and assets. `resolve_founder_cohort` and `create_new_save_from_founders`
  already distinguish `GeneticFounder`, `GeneticOffspring`, and `MindStateClone`.
  `export_creature_bundle` supplies an existing portable packaging path.
- `crates/alife_world/src/organism.rs`: organism birth initializes fresh biological
  state. Reuse birth for the descendant rather than relabeling an adult save.

## Small implementation boundary

Add an explicit bridge from a stable saved individual's learned weights to a new
foundation and genetic founder. Reconstruct canonical weight order and verify
compatibility before export. Do not pass the complete adult checkpoint through
the mind-clone path. Existing Nano512 candidate admission is restricted; do not
weaken builtin identity checks or assume arbitrary weights are already admitted.

Inventory explicit memory fields once at this bridge. Omit episodes, personal
facts, relationships, individual/place references, and transient learning state.
Retain separable general concepts and language. If an explicit store mixes both
and cannot be filtered cheaply, omit it and preserve competence through weights.
Reuse existing asset digests, archive provenance, and fresh birth identities.
No new online learner, periodic promotion service, or mandatory distillation.

Use one focused export/load/reset check and one useful-skill comparison between
source and fresh descendant. Add further tests only for a concrete failure.
This note records scope and reuse points; breed promotion is not implemented by
this documentation change, and skill retention is not yet demonstrated.
