# Current implementation reference

> **NON-NORMATIVE:** This file records current implementation conventions and
> evidence terminology. The sole controlling architecture is
> `architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`.
> If this file conflicts with v2.0, v2.0 controls and this file records an
> implementation gap.

It is not a claim that every feature is integrated or proven. See
[status](STATUS.md).

## Production authority

- Neural policy: GPU-authoritative WGSL.
- Perception, candidates, legality, targets, execution, and outcomes: `alife_world`.
- Scheduling and translation: `alife_game_app`.
- Presentation: read-only projection.
- CPU neural helpers: reference, test, or developer use only.
- GPU unavailable: typed unavailable result, never silent CPU substitution.

## Brain classes

| Class | Status |
| --- | --- |
| N512 | Production-admissible class; requires its own evidence |
| N1024 | Production-admissible class; requires its own evidence |
| N2048 | Production-admissible and current trained foundation class |
| N4096 | Research-only migration/equivalence class |
| Larger legacy classes | Inspection, export, or research only |

`Standard2048` is a reference profile. It is not the global brain shape. Runtime population limits come from an explicit neural-heap profile, not from the class name.

## Foundation and inheritance

The production genetic policy is:

```text
W_genetic = immutable curated foundation + compiled genome deltas
```

Foundation bands are `Fixed`, `Slow`, or `Fast` for lifetime plasticity.

Ordinary offspring inherit:

- foundation identity and compatible ABI/codebook family;
- structural genes and sparse genetic deltas;
- endocrine and morphology traits;
- lineage and provenance.

They do not inherit:

- lifetime-consolidated weights;
- episodic or semantic memories;
- learned vocabulary, aliases, or dialect;
- eligibility, activations, working memory, injuries, age, targets, or conversations;
- teacher-private or SLM-authored state;
- raw world or GPU handles.

An explicit durable learned founder is a separate, provenanced clone mode.

## Persistent neural identity

Neurons, projections, synapses, and decoders use persistent logical addresses. Packed GPU offsets are runtime-local. Migration maps old state by persistent identity, initializes expansion dormant, and requires a sealed rollback boundary plus same-adapter behavioral equivalence before handoff.

## Language

`LanguageCodebookV1` contains 256 stable logical codes independent of neuron indices and packed offsets. It defines pronounceable symbols and grammatical roles, not inherited object meanings.

Player, peer, creature, and teacher speech are spatial world events. Hearing depends on range, noise, attention, and ability. Player text maps to bounded tokens; it does not create an action score, target, or reward.

Self-narration is a neural action. The world exposes an unscored `Vocalize` opportunity. Only after GPU arbitration selects it may the speech head emit a bounded raw token sequence for world validation and broadcast.

## Teacher

Teacher content enters through ordinary hearing, vision, glyphs, gestures, demonstrations, objects, social feedback, and world consequences.

The production teacher API has no hidden concept vector, direct lexicon write, weight write, plasticity delta, selected candidate, action score, entity target, private reward injection, or legality bypass. Private grading may choose a future lesson; it cannot change the evaluated action or outcome.

## Private local SLM

The SLM may:

- provide a weak, fading developmental prior through a bounded schema;
- translate human surface text to bounded tokens;
- render raw creature tokens for the player with model identity, confidence, and assistance state.

It may not:

- author creature thought or raw speech;
- issue an action or choose a target;
- inject reward or hidden concepts;
- mutate weights or arbitration state;
- bypass perception or world authority.

Unaided and SLM-assisted evidence remain separate.

## Persistence modes

- `GeneticRebuild` — foundation plus genome, no acquired cognition.
- `DurableLearnedFounder` — selected consolidated learning and durable content, with transient/world-local state cleared.
- `ExactResume` — sealed runtime continuation bound to save and world tick.

Save validation is fail-stop for invalid neural state. Checkpoint and save identities are digest-bound.

## Archive rules

- Every organism receives an immutable genetic archive before GPU insertion.
- A dying organism seals its final outcome and life statistics before retirement.
- Optional learned-state capture and the immutable life manifest precede GPU-handle scrub and world despawn.
- Pinned learned checkpoints are not automatically evicted.
- Import/export bundles are bounded, traversal-safe, digest-checked, and provenance-complete.

## Evidence and status terms

- Missing exposure is `Unknown`, not zero.
- Missing hardware is `Unavailable`, not fallback success.
- Failed prerequisites are `Blocked`, not partial promotion.
- `Implemented`, `Integrated`, `Player-visible`, and `Proven` are distinct labels.
- Current-source proof requires current-source receipts. Historical reports retain their original scope.

## Superseded policy

The current GPU-authority model supersedes old production CPU consolidation authority, GPU parity-gated CPU shadows, and automatic CPU neural fallback. Compatible save safety, sparse-layout, world-authority, sealed-patch, and evidence-honesty rules remain in force.

Retired implementation snapshots live under `archive/legacy_true25d` and
`archive/legacy_app_milestones`. They are source history only. Active code,
commands, manifests, packages, and tests must not depend on those directories.
