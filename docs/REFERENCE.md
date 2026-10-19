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

## Grounded terrain sight

`GroundedTerrainVisionV1` uses the existing bounded vision range, horizontal
gaze cone, and physical line-of-sight occlusion. An object at exactly the
observer's X/Z coordinates has no horizontal bearing, so it skips only the
azimuth rejection; vertical separation, range, occlusion, consumption, contact,
and action legality still follow their ordinary checks. Any nonzero horizontal
offset retains the gaze cone, even at contact distance. This is a general
geometry rule, without object-kind recognition or action scoring
(`AOA-WORLD-001`, `AOA-WORLD-002`, `AOA-SENSE-001`, `AOA-SENSE-002`).

Visibility does not authorize ingestion. For reachable, safe, unheld food,
selected Eat executes a real pickup using ordinary Grab reach, grip-path,
ownership, liveness, and manipulation-effector checks. Explicitly carried or
unknown legacy possession is protected. A temporary activity-held object may be
placed safely when switching to another legal hands-on target; this is a separate
preparation interval before pickup or use. A player-held target cannot be taken. "Safe" here means Food with no authored hazard pain; this is
world execution validation, never a candidate score or extra brain input.
Out-of-reach Eat fails without navigation. This first decision retains selected
Eat identity, reports Touch and zero nutrition, and leaves the exact target
unconsumed in the actor's grip. Only a later Eat while holding that same food
reports Consumed and provides ordinary nutrition/taste/ownership cleanup.
Explicit Grab retains its acquisition/release toggle (`AOA-WORLD-003`,
`AOA-MOTOR-002`).

Eat-selected physical acquisition is not selected Grab. Sealed decisions and
PPO labels retain the sampled Eat primitive. Explicit-Grab curriculum credit,
completion, and replay/rehearsal eligibility still require an actual selected
Grab of the same target; release/regrab provides one loss credit per life.
Successful Touch cannot establish eating completion or completed-action speech
labels. CPU geometry, world, and sealed-scaffold regressions do not establish
on-policy GPU behavior, learned preference, or founder promotion
(`AOA-TEACH-007`, `AOA-OBS-002`).

## Hands-on preparation and local movement

Eat and Play share bounded physical hand preparation. A mobile ball must be
acquired before actual Play; a fixed activity toy is used in place. Acquisition
and safe placement report `Preparation`, retain the selected activity identity,
and provide no nutrition, play stimulation, investigation novelty, completed-Play
speech label, or activity praise. Actual activation reports `Completed` and a
measured stimulation dose. App-only label evidence binds organism, sequence,
source/outcome ticks, selected manipulation primitive/target, and actual Touch;
missing or mismatched evidence supplies no completed-Play label. Adapted actuator
intensity need not equal the originally requested intensity.

Grip transfers name the affected object, old/new owner, old/new position, and
acquisition, release, placement, or consumption. Safe placement validates the
new target first, then checks a fixed set of nearby supported positions, reach,
terrain, swept object clearance, and occupancy. Failure keeps possession. Look,
Inspect, Approach, and movement never acquire or release an object implicitly.
Activity provenance persists; legacy held objects default to protected carrying.

Approach/entity-directed movement can take one bounded clearance step only when
the target appears in a fresh, calibrated, bounded canonical terrain-vision
snapshot. The selector receives nine nearby physical clearance ranges and the
observed bearing, never hidden target coordinates, an opening search, or a route.
Body size and a conservative carried-object envelope constrain clearance;
ordinary terrain/collision/grip checks still validate the selected step.
Transparent solid barriers remain physical obstacles.

A hidden destination permits no automatic corner detour. Explicit learned
intermediate movement can reveal it and restore assistance. Each decision
rechecks visibility; target occlusion pauses assistance on the next decision,
without completing a remembered multi-step maneuver. A persisted finite detour
budget replenishes only after net improvement beyond the best observed distance;
oscillating outward/inward steps, repeated commands, and reload cannot reset it.
Changing hidden distant geometry cannot select a route. Destination choice,
exploration, hidden-corner navigation, and persistence remain learned behavior
(`AOA-SENSE-001/002`, `AOA-MOTOR-002/005`, `AOA-EMB-001/003`,
`AOA-TIME-001/003`). These are physical execution conventions, not evidence of
learned competence or a settled training curriculum.

Care evaluation exposes prompted and silent conditions for held food, reachable
unheld food (`EatReachableFood`), and short Feeding approaches. Silence suppresses
initial and repeated spatial requests; absence of a demonstrator alone does not
make a probe silent. Care demonstrations select Eat for pickup and consumption.
Care success uses exact learner/target `FoundationFoodOutcome` nourishing-meal
evidence; blocked actions are reported separately. The existing explicit-Grab
experiment retains its distinct qualification and credit boundary. See the
[care-first next-run plan](training/care-first-next-run-2026-10-19.md).

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
