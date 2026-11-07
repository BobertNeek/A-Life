# Current implementation reference

> This guide describes current implementation conventions and evidence terminology. See the [organism and gameplay design](architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md) for the intended behavior.

It is not a claim that every feature is integrated or proven. See
[status](STATUS.md).

## Production authority

- Gameplay neural policy: GPU-authoritative WGSL.
- Perception, candidates, legality, targets, execution, and outcomes: `alife_world`.
- Scheduling and translation: `alife_game_app`.
- Presentation: read-only projection.
- Gameplay CPU neural helpers: reference, test, or developer use only.
- Explicit offline CPU cloud training: opt-in `alife_training/cpu-backend`, with
  canonical biology, distinct sealed CPU evidence and native optimizer handoffs.
  See the [backend report](performance/20261019-native-cpu-training-backend.md).
  GPU hardware and accepted-source qualification require their own evidence.
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

Terrain-vision object chemistry and temperature remain contact observations;
anonymous bilateral smell is separate. Under danger urgency the inherited Avoid
readout uses observed bearing before contact, without identifying which object
is hazardous; contact retains the chemical cue.
Hunger still primes non-contact investigation and contact/reach-gated tasting.
Fresh procedural candidate distance coefficients are negative. Trained
foundations replace them with their stored signed coefficients; recurrent
excitatory/inhibitory sign rules do not constrain mathematical decoder weights.
Play uses the existing Other family and Grab uses Contact, preserving every
physical feature, including angular speed in lane 20. This changes the learned
meaning of Play; older policies and replay do not automatically carry that meaning.
See the [founder regression report](performance/20261020-founder-brain-corrections.md).

Visibility does not authorize ingestion. For reachable, unheld food,
selected Eat executes a real pickup using ordinary Grab reach, grip-path,
ownership, liveness, and manipulation-effector checks. If another object is
held, the exact requested target and a local placement for the old object are
validated before either possession changes. Placement and pickup commit together,
even for deliberate or legacy holds. A failed swap preserves the old hold.
A player-held target cannot be taken.
Acquisition validates physical capability, not future benefit or harm. Authored
hazard pain does not veto Eat/Play preparation through an otherwise legal grip;
the same target can already be acquired by explicit Grab. Ingesting harmful food
still produces its ordinary nutrition, injury and taste. Aversion belongs to
grounded sensing, physiological consequences and the creature's learning, rather
than a hidden zero-harm acquisition rule (`AOA-WORLD-002`, `AOA-BODY-007/008`).
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
acquisition, release, placement, or consumption. A swap reports both `Placed`
and `Acquired`, while the action result retains the exact acquired target. For
placement-only preparation of a fixed toy, the command still names the toy but
the physical Touch and transfer name only the old object actually placed. Eat
and mobile Play report `Preparation`, with no consumption or activation yet.
The compound transition occupies one selected manipulation interval and retains
the existing single-Grab effort profile (physical energy cost 0.06). It does not
charge or advance a second hidden action. This intentionally replaces the former
two-interval place-then-acquire schedule; biology across those different schedules
is not claimed to be identical. Legacy Grab still releases on the same target; another target requests a swap.

Placement checks eight local ring positions followed by the actor's feet. It
retains bounded reach, terrain support, an item-radius map footprint, swept solid
obstacle clearance, and conservative spacing from unrelated items and peers.
The held path is airborne: its start need not have ground beneath it, but its
endpoint must be supported and the full path must clear actual solids. The
carrier and newly acquired target are excluded from this spacing: the latter
vacates its ground slot atomically. Contact radii are not authored mesh volumes;
existing carry poses already overlap the carrier's contact envelope. This does
not make real terrain or obstacle solids passable.

Non-Agent items use the existing grounded-anchor convention, including underwater
terrain. Actor wading and walking-slope limits are not item-placement rules. This
adds no buoyancy or sliding simulation. Item-sized obstacle checks and map bounds
still apply, so an actor standing somewhere does not prove an arbitrary large
item fits there. A player-lifted actor cannot use a floating feet fallback.

All fallible receipt construction precedes publishing the staged old/new items.
Registered commands retain their existing outer transaction for biology and joint
receipts; a compound swap does not advance biology or clocks twice. Look, Inspect,
Approach, and movement never acquire or release an object implicitly. Activity
provenance remains saved historical evidence, not a release permission flag.

These world-rule changes require native qualification on their exact source.
They do not establish a learned preference or revise previous frozen brain-study
baselines; matched brain comparisons must use the same world source in all arms.

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
experiment retains its distinct qualification and credit boundary.

## Brain classes

| Class | Status |
| --- | --- |
| N512 | Production-admissible class |
| N1024 | Production-admissible class |
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

Neurons, projections, synapses, and decoders use persistent logical addresses. Packed GPU offsets are runtime-local. Research migration maps old state by persistent identity and initializes expansion dormant. Its handoff checks preserve a sealed rollback boundary and compare behavior on the same adapter; this does not require brain-growth experiments for ordinary gameplay changes.

## Language

`LanguageCodebookV1` contains 256 stable logical codes independent of neuron indices and packed offsets. It defines pronounceable symbols and grammatical roles, not inherited object meanings.

Player, peer, creature, and teacher speech are spatial world events. Hearing depends on range, noise, attention, and ability. Player text maps to bounded tokens; it does not create an action score, target, or reward.

Self-narration is a neural action. The world exposes an unscored `Vocalize` opportunity. Only after GPU arbitration selects it may the speech head emit a bounded raw token sequence for world validation and broadcast.

## Teacher

Teacher content enters through ordinary hearing, vision, glyphs, gestures, demonstrations, objects, social feedback, and world consequences.

The language nursery and training demonstrations use world speech, perception, and actions. Teaching does not write private concepts, weights, action scores, targets, or reward. A private lesson planner may choose a later exercise without changing the learner's evaluated action or outcome.

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
