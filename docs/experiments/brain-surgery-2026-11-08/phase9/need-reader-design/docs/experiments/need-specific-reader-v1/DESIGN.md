# Need-specific consequence reader: detached candidate V1

## Recommendation and status

Build the smallest **detached neural arithmetic fixture**, not another autonomous
life sweep or another persistent decoder, next. Give it eleven separate measured
consequence lanes, the current eleven biological readings, and explicit evidence
status. Its shared, monotone neural circuit compares current need pressure with
a bounded, hypothetical successor pressure. It returns eleven signed associations
and a small optional readout. It does not choose a goal, create an action, issue a
motor command, manufacture a reward, or identify an action's causal effect.

The accompanying `reader_algebra.rs` is uncompiled, test-only source outside every
Cargo target. It models the proposed arithmetic and includes synthetic tests. It
does not implement source validation, retrieval, a store adapter, learning,
persistence, CPU integration, or GPU integration. No checkpoint, private memory,
acquired array, device, or world run was used to prepare this design. Nothing is
attached to ordinary main, and no new genetic asset has been created.

This is a more informative next step than raising V4's gain: that experiment's
pooled family valence was mostly common-mode, and its recurrent need neuron could
remain positive at raw loneliness zero. A need-specific source and a bounded
current-state comparison address those two different problems independently.
They do not solve the existing operating-point, retrieval, or action-margin
problems automatically.

## Source boundary and what exists

Base: ordinary main `550f79ad1f765132c634e7a272da56b521b57ec0`, tree
`c4e5230799be59df95d468be1343c4549892851d`. The design branch changes only this
documentation directory. Relevant source traces:

- `crates/alife_core/src/chemistry.rs:17-102`: inherited
  `BiologicalValueProfile`; reference drive coefficients are negative for hunger,
  fatigue, fear, pain relief, loneliness, temperature stress; positive for ATP;
  zero for curiosity, reproductive arousal and untyped extensions. Injury is
  separately valued, and positive pain change is excluded from this reward sum
  to avoid duplicate fresh-injury credit. The reader is not this reward sum.
- `chemistry.rs:106-153`: the actual eleven-lane `DriveSnapshot` order. ATP is
  availability; baseline ATP is .75, curiosity .45, loneliness .15. A nonzero
  baseline is not evidence that every action should reduce every channel.
- `biochemical_graph.rs:993-1036`: the explicit play-stimulation phenotype binds
  drive extension0 to boredom, species22, with value coefficient -.08, and
  separately wires novelty/investigation to curiosity. Unknown extensions do
  not automatically have this interpretation.
- `biochemical_graph.rs:1120-1148`: nutrition, social contact, sleep recovery,
  damage, thermal stress and mating opportunity are different body-to-chemical
  pathways. Sleep can decrease fatigue/pain and raise ATP. Mating opportunity
  raises reproductive arousal; that increase is not itself damage or a prize.
- `biochemistry.rs:540-573,602-650`: developmental expression and puberty are
  explicit; reproduction also requires health, energy, hormones, opportunity and
  fertility. A fresh organism initialized at age 600 is not one that has lived
  600 intervals, and is not necessarily mature.
- `crates/alife_training/src/cpu/input.rs:24-58,87-115`: canonical homeostasis
  supplies raw drives and hormones. Current replay input is a fixed `[f32;54]`:
  24 candidate, 8 target-memory, 4 family-memory and 18 cognitive lanes. Family memory is
  confidence-weighted before decoding. No spare lane may be reinterpreted.
- `crates/alife_core/src/phenotype/memory_channel.rs`: V1 fixes target lanes24–31,
  family lanes32–35, stride36 and gain ceiling .5.
- `crates/alife_core/src/phenotype/compiled.rs:109-128`: ordinary main has decoder
  heads 1–4. V4's separate candidate adds head 5, source-specific recipe identity,
  persistent role and eligibility semantics. Head 5 is not a generic need port.
  The read-only V4 source inspected here was `nano-direct-need-gate-v4` at
  `8d814864cdc32f246355ea7e6e86c67f1776601d`: see `nano512_readout_candidate.rs`,
  `phenotype/record.rs:240-308`, `cpu/organism.rs:299-317`, and
  `shaders/closed_loop_memory_context.wgsl:88-130` for its .35, lane32,
  positive-source-gated signed family-memory contribution.
- `crates/alife_training/src/cpu/replay.rs:148-167`: existing dendritic response is
  `tanh(max(sum - threshold, 0))`, followed by neuron dynamics. The proposed
  detached basis uses that response, but does not claim to reproduce all those
  neuron dynamics or to be an already compiled lobe.
- `docs/ARCHITECTURE.md:44-104`: the neural policy sees unscored legal candidates;
  memory does not select actions; realized translation cancels sleep recovery,
  whereas selected-but-blocked movement can lawfully coexist with rest. Body
  turning currently has separate limitations. Selection is not motion.
- Controlling architecture §§11.2,15–17: physiology changes salience without
  making each drive a reward; lobe functions remain neural; memory is evidence,
  not action replay; provenance and uncertainty are part of the interface.

Separate store component, read only:
`need-consequence-main-v1` at `493bb172da46add5f6d5da43dd375d5387116473`.
`crates/alife_core/src/need_consequence.rs:60-186` makes exact keys from complete
observed context, excluding only four outer clocks. Lines194–237 expose raw
eleven-lane deltas, confidence, source IDs/ticks/digests and joint-association
attribution. Lines358–395 return individual exact matches, or Unknown.
Lines513–580 require measured, sealed, neural, observed-target evidence and
retain one representative association per interval. **Targetless Rest/Idle are
Unknown, and the public observation does not expose source-before drives.**

Known completed findings, not rerun here: peer-contact interval loneliness delta
was -.007499993 at 20 Hz; toy/separated were 0; passive clones matched, so this is
not isolated Inspect causality. Exact next-world recall missed all three. The
separate experimental weighted-cosine query admitted next-state scores near
.99988 but also artificial loneliness 1 around .973–.975. Cosine .72 is not a
pressure-support guarantee. V4's actual 512-neuron CPU trials did not change sampled
choices. Those findings constrain the design; they are not new reader results.

## Biological roles, signs and initial scale

All eleven raw current readings and signed deltas are retained as neural inputs.
The following is an explicit **reference species recipe**, not a global relabeling
of the existing ABI or a claim about every evolved chemistry graph.

| Lane | Meaning | Reader pressure p(x) | Initial magnitude |
|---|---|---|---:|
|0|Hunger|x|1/7|
|1|Fatigue|x|1/7|
|2|Fear|x|1/7|
|3|Pain|x|1/7|
|4|Loneliness|x|1/7|
|5|Curiosity|Informational only|0|
|6|ATP availability|max(.75-x,0)|1/7|
|7|Temperature stress|x|1/7|
|8|Reproductive arousal|Informational only|0|
|9|Extension0|Informational unless typed boredom binding|0, or .08|
|10|Extension1|Informational until specified|0|

The ATP target .75 is a proposed inherited reference setpoint, grounded in the
reference graph baseline in `evolutionary_genetics.rs:989-992`; it is not proof
that .75 is optimal in every context. Do not divide by a small target or normalize
one-step deltas to unit effect. Availability above the target still enters raw
interoception and computation budgets but supplies no extra replenishment prize.
Future evolved chemistry must provide its own typed target rather than silently
using .75. An explicit target of1 is a matched alternative, not a reinterpretation
of stored ATP. This reference avoids making complete ATP saturation compulsory.
Both this .75 target and the .35 reader gain are **proposed inherited candidate
priors**. Neither was fitted to the closed observations or biologically established
by them. Reusing the earlier gain permits a bounded comparison; it does not imply
that the gain is sufficient for behavior or optimal for this new representation.

For burden channels, a negative delta can be favorable and a positive delta
unfavorable. For ATP, the directions reverse below its target. Fear/pain reduction
can reflect decay, not successful escape/healing; retain that distinction.
Positive pain change may supply prospective caution here but must never be
added to measured injury reward again. Temperature-stress relief does not reveal
whether the organism should go toward heat or cold.

Curiosity is exploratory motivation and an information signal. Decreasing it can
mean resolved investigation, decay or suppression; increasing it can mean useful
novelty. Neither sign is universally good. Reproductive arousal similarly needs
readiness and opportunity rather than a generic deficit penalty. These lanes are
connected observationally with zero *innate signed-consequence* readout, not
disabled throughout the brain. Their established lobe and biochemical pathways
remain. A later learned contextual reader can use them; this candidate does not
invent completion labels or reward arousal. Extension0 gets boredom's sign only
with a validated, versioned species/phenotype binding. Extension1 remains unknown.

## A concrete small neural circuit

For one candidate-associated, measured interval, let x be the current normalized
reading and d the unmodified historical `after-before` delta for each lane.
Compute a **local hypothetical successor**, not a forecast:

```
x_local = clamp(x + d, 0, 1)
p = role_pressure(x)
q = role_pressure(x_local)
phi(z) = tanh(max(z, 0))
B(z) = .125*phi(z) + .25*phi(z-.25)
     + .25*phi(z-.50) + .25*phi(z-.75)
u_i = B(p_i) - B(q_i)
r = gain * sum_i(magnitude_i * u_i)
```

Each lane uses the **same** monotone basis and weights on its current and local
successor branches. There is no independent intercept, constant-reference neuron,
positive activation floor or reward for the existence of a memory. Zero delta
therefore produces exactly zero signed contribution. The basis has progressively
recruited thresholds, giving large pressures more influence while keeping small
needs relevant. It uses existing dendritic arithmetic but is evaluated directly
in this detached model. A compiled recurrent realization requires its own proof.

Use gain 0 for the matched ablation and .35 for the first enabled fixture. There
are seven reference magnitudes of 1/7. Even including typed boredom .08, `B(1)<.5`
bounds the unweighted reference maximum below .54 and the .35 readout below .189.
Require nonnegative magnitudes with L1 sum≤2, nonnegative shared basis weights
with sum≤1, and a final signed ceiling .35. These are new candidate limits, not
changes to the old .5 memory ceiling. The reader must report clipping, if any.

The positive sign means a locally favorable **association**, not causal credit.
The eleven components can be correlated consequences of one event; they are not
eleven independent samples or independent causal proofs. Their bounded sum is a
reference motivational weighting, not added measured reward or source confidence.
The initial weights are equal across action families, targets and identities.
No coefficient says “peer→social,” “food→eat,” “toy→play” or “fatigue→Rest.” Only
the observed candidate association supplies different deltas. Family-common
consequences remain common-mode; do not manufacture contrast with hidden labels.

Current conditioning is hard at the biological boundary rather than delegated to
a recurrent neuron that happens to correlate with need. At loneliness 0 and
d<0, both p and q are 0, so u=0 regardless of neural bias elsewhere. At very low
positive loneliness, favorable contribution is at most B(current loneliness).
It cannot consume nonexistent remaining loneliness. At loneliness 0 and d>0, q>0
and u<0: anticipated worsening can matter even before distress is high. ATP gains
are capped by the present shortfall, and ATP loss can matter when it crosses the
target. These properties survive any allowed shared nonnegative basis weights.

The exact loneliness example at .15 with d=-.007499993 lies entirely in the first
segment; its algebraic readout is
`.35/7 * .125 * (tanh(.15)-tanh(.142500007))`, about 4.6e-5 before confidence.
This is deliberately not inflated into a full-scale benefit. It may be too small
to change a saturated 512-neuron policy. The first useful result is correct differential
features and sign/zero behavior, not changed sampled actions. Measure margins and
branch saturation before proposing gain or neuron changes.

This fixed-width reference performs 88 saturating basis responses per candidate
(11 lanes × 2 states × 4 knots), plus affine/clamp/readout work, whether gain is 0
or .35. Unknown rows also use the same scratch shape, then a known mask. That is
small for a standalone arithmetic test, not a claim of negligible production
cost: 32 candidates would mean 2816 such responses before additional retrieval.
No source JSON is copied into these neural lanes; the store's roughly 16 KB/record
and 1 MiB bound remain a separate unresolved storage cost.

### Reference math versus actual engine operators

The standalone file calls Rust `clamp`, `max`, `tanh`, sums and multiplication
directly. It has no actual `NeuronDynamics`, synapse arrays, microsteps, receptor
effects, recurrence, eligibility or decoder. A passing standalone test therefore
qualifies only that arithmetic, not a native neural circuit.

- **Affine sums and signed differences already exist:**
  `neural.rs::cpu_spmv_projection` applies signed sparse weights.
  `ActivationFunction::{Identity,Relu,Tanh,Logistic}` already exist at
  `neural.rs:644-664`. No new arithmetic opcode is needed for x+d or B(p)-B(q).
  They still need new cells/routes/identities in an actual candidate phenotype.
- **Clipping and availability can be composed from existing primitives:**
  `clamp(z,0,1)=ReLU(z)-ReLU(z-1)` and ATP pressure is `ReLU(target-x)`.
  `finalize_cpu_activations` also has explicit clamp bounds. The current native
  training recurrence does not automatically insert either construction. A
  feed-forward realization needs distinct stages, with aligned current/successor
  paths; replacing that staging with a host-computed pressure is not qualification
  of the neural path.
- **Each basis response is an existing dendrite:**
  `dendritic.rs:210-248::apply_dendritic_conjunctions` implements the exact
  `tanh(max(weighted_sum-threshold,0))*output_gain` response. There is a maximum
  of four branches per target neuron. B(current) and B(successor) therefore use
  two targets with four branches each, then a signed comparison target; eight
  branches on one target would violate the existing limit.
- **The full neuron is not that dendrite alone:** the native training path adds
  bias, encoded input, projection gain, metabolic/homeostatic subtraction and
  threshold shift, applies the target activation, then leak, EMA and metabolic
  updates. Its synchronous recurrent timing reads previous activations.
  `cpu/input.rs` also transforms raw homeostasis by receptor/sensor effects.
  Matching this reference requires an explicit test recipe, not the assumption
  that the current raw drive equals an existing neuron's final activation. A
  neutral-effects, identity-target, leak=1, homeostatic_gain=0 fixture can isolate
  primitives, but is not evidence about an unchanged living creature.
- **Candidate-specific data are missing:** existing recurrent neurons have one
  organism activation state, whereas deltas differ per candidate. Running the
  same scratch network serially for each candidate is possible in a test; it is
  not the current runtime architecture. Production needs a versioned candidate
  scratch/evidence port and explicit scheduling/budget, or a newly compiled
  candidate-conditioned decoder. Neither the old family-memory coordinates nor
  the fixed54 replay lanes provide this port.
- **Exact dynamic confidence gating is not a free recurrent edge:** an ordinary
  synapse multiplies a dynamic activation by a stored weight, not by another
  dynamic confidence. Existing action decoders multiply a source by a candidate
  feature, and V4 head 5 gates memory by a positive source, but neither owns this
  new evidence confidence/negative signed association. An exact new readout needs
  a typed gate/port and distinct coordinates, or a separately qualified network
  approximation. Do not rename sensor confidence, force the association positive,
  or apply confidence before the nonlinear comparison and call it equivalent.
- **Shared learnable coordinates are new:** the model stores one basis weight
  for both sides. The current `DendriticInputRef` stores per-branch weights and
  ordinary persistent addresses identify separate neurons/synapses; there is no
  demonstrated tied-parameter optimizer contract for this pair. A first native
  fixture can freeze identical inherited branch parameters. Any learned version
  needs an explicit shared parameter group, matching gradients/moments/ages and
  restore validation. Similarly, a new reader role/head/input-lane namespace
  needs a new optional ABI rather than overloading old coordinates.

Thus the arithmetic is expressible with familiar neural primitives, but its
candidate dataflow, exact gate, budget and learned/persistent identity are not
implemented. The direct scalar file must not be described as native qualification.

## Unknown, measured zero, confidence and current support

Three statuses must remain distinct: Unknown, measured but not supported here,
and measured supported. A measured all-zero row has known=1 and all eleven
values 0. Unknown has known=0 and zero scratch lanes solely for safe execution;
it is never a zero-effect training target. Unsupported historical evidence stays
visible to diagnostics, with its own reason, but does not enter this local reader.
Mixed zero/nonzero lanes in one observation remain measured independently.

Do not pool, average, multiply together or rename raw delta, evidence confidence,
age, context similarity and pressure distance in the transport. The store's
confidence is the minimum of decision and candidate sensor confidence, **not**
an estimate that this action caused the effect. Inside the detached neural
readout only, an explicit confidence gate c∈[0,1] may scale r. Report both r and
c*r, and do not multiply c again during encoding or learning. Confidence 0 is
still a known observation. Similarity never becomes reward or measurement
confidence. No record-age decay is proposed in this first age ≤ 1 fixture.

The scalar kernel alone is not a retrieval policy. Exact store V1 is the only
implemented production query; use it unchanged for any real-source bridge.
The first synthetic circuit tests can deliberately hold x and d independently to
test arithmetic, but must be labeled algebra-only and must not be reported as
evidence that the animal has experienced those combinations.

A later **detached-only** admission protocol may test local generalization:

1. Match organism, world/life epoch, genome and expressed chemistry identity,
   checkpoint lineage, sensor profile, action primitive/kind/family, tracked
   object and entity, source interval duration, and timebase. One 20 Hz interval
   is not a rate that may be multiplied across arbitrary durations.
2. Keep attribution `JointIntervalAssociation`; require the actual full bundle
   identity for any proposed joint-context use, not just a channel/family name.
   The existing V1 key lacks a current joint-bundle match. Until candidate-bundle
   representation exists, allow neural exposure only for verified singleton
   source/current contexts; otherwise diagnostics only. Here a current singleton
   context means a pre-decision fixture with every other motor factor fixed to
   no-op, not a promise based on the eventual sampled bundle. No circular
   lookahead to the selected action is allowed. Never duplicate the
   representative's effect onto other selected channels.
3. Start with age after observation≤1, no backward/future evidence, no source
   replay, same developmental category/puberty, and an explicit, small continuous
   developmental difference bound. Exact category equality is not enough to
   establish biochemical equivalence.
4. Keep all eleven source-before values. Require max absolute current-minus-
   source-before reading≤.025 in the first experimental gate; report every lane's
   distance. This .025 is a conservative experimental bound, not a calibrated
   confidence. The known .15→1 counterexample fails; .15→.1425 can pass only this
   part of admission. Source-boundary measurements must supply these readings.
5. Require an independently specified external-context matcher and pressure-
   excluded diagnostics. The existing weighted cosine .72 is not sufficient.
   Do not claim the above pressure gate solves context aliasing or effect
   extrapolation. Until the external matcher is qualified, exact-context
   fallback means ordinary next-world exposure can remain Unknown.
6. At most one newest admitted observation per exact candidate association for
   this prototype, at the store's bounded scan limit. Do not divide by candidate
   count or learn from repeated pooled copies. Preserve the other retrieved
   records and any contradictory observations in diagnostic receipts rather
   than presenting one sample as a reliable mean.

The reader must reject unavailable source-before data instead of reconstructing
it by subtracting a delta from the current frame. V1's public observation alone
cannot implement steps 1–5. A separately reviewed adapter must derive these facts
from validated sealed measurements and bind the observation digest/sequence to
them. No new production approximate query is smuggled into this design.

The host transports evidence and current physiology only. In any later runtime
attachment, the biological comparison and readout must execute as versioned
neural arithmetic, with its weights, derivatives and costs accounted for. A host
helper adding this scalar to action logits would violate the intended boundary.

## Proposed interface, genotype and ABI boundary

The following are **new type proposals**, not existing exported Rust types:

```
NeedReaderBindingV1 {
  world_epoch, life_epoch, organism_id, genome_id,
  expressed_chemistry_digest, checkpoint_lineage_id,
  profile_identity, source_receipt_digest, candidate_decision_key
}
NeedReaderObservationV1 {
  binding, source_sequence, before_tick, after_tick, source_frame_digest,
  source_bundle_digest, joint_command_count, attribution,
  source_before_drives: [f32;11], raw_deltas: [f32;11], confidence
}
NeedReaderCandidateRowV1 {
  binding, current_tick, current_drives: [f32;11],
  evidence: Unknown | Measured(NeedReaderObservationV1),
  support: Exact | DetachedLocal(receipt) | Unsupported(reason),
  age_ticks, context_similarity: Option<f32>, pressure_distance: [f32;11]
}
NeedReaderFeaturesV1 {
  known, supported, raw_deltas: [f32;11], current_drives: [f32;11],
  local_successor_drives: [f32;11], signed_associations: [f32;11],
  confidence, age_ticks, context_similarity, pressure_distance,
  raw_readout, confidence_gated_readout, clipping_flags
}
```

Binding constructors must be private and derive from validated source and current
receipts; caller-provided IDs or a boolean “validated” do not establish authority.
World/life/checkpoint identifiers need real owner-defined namespaces, not hashes
of OrganismId. Store restore needs explicit binding verification. None exists in
the attached arithmetic fixture, which accepts synthetic inputs only by design.

Proposed `NeedReaderGeneV1`: schema 1, source-foundation identity, biochemical role
contract digest, role per lane (`Burden`, `Availability{target}`, `Informational`,
`TypedExtension{semantic_id,role}`), four ordered knots, four shared nonnegative
basis weights, eleven nonnegative magnitudes, gain, confidence-gate mode and
readout ceiling. All finite, bounded and included in a new recipe digest.
Changing an extension meaning or source chemistry invalidates the role contract.
Do not derive semantics from the sign of an arbitrary evolved value coefficient.
Initial receptor choice, knots and magnitudes are inherited recipe information;
observations, learned weights and basis adaptations are acquired, never inherited
from an adult. Mutation or learning must preserve paired weight sharing and
monotonicity if the zero/benefit bounds are to remain contractual.

There is **no change** to `DriveSnapshot`, `MemoryBank`, store V1, existing family
value lanes, existing `[f32;54]` replay, head 5, old sensor offsets, old genetic
assets or checkpoint coordinates in this step. Use a separate detached input
type, not spare values53–63 or a larger array with implicit legacy decoding.

If detached results justify integration, add a new optional typed plan and a new
foundation candidate identity. Keep head 5 reserved for the already-prototyped V4
meaning; a new distinct decoder could use head 6 only after a collision audit.
Use separate input storage and explicit versioned schemas/strides. Do not merely
append a head enum and reinterpret the old `input_lane` field: the present decoder
and learning kernels assume their fixed flattened input space.

Stable new neural identities must name role/lane/current-or-successor/basis,
not packed GPU offsets. A compiler would place canonical pressure processing in
InteroceptiveMotivational, consequence evidence in MemoryInterface, comparison in
TemporalPredictive, and signed contextual readout through the existing neural
ActionPlanning competition. It must either allocate newly budgeted cells/routes
or prove an explicit new recipe's allocation; it may not overwrite old neurons
or replace the 64 V4 birth branches to make room. The detached network is not
already such a 512-neuron allocation. Learned readout need not be family-specific
initially; any later family-specific weights get explicit new identities and no
hardcoded semantic labels.

Required integration surfaces include `phenotype/{compiled,record,abi_validation,
persistent_address,learning,io_compile}.rs`, foundation/genetic identity and budget
receipts; CPU encoder/forward/eligibility/PPO/replay/continuation; GPU upload and
memory-context/activity/eligibility/plasticity kernels; optimizer moments and
integer ages; save/load and world/life binding. Capture exactly the evidence and
current readings used at action time in a new replay version. Never re-query a
mutable store while evaluating old-policy probability. Pairwise basis gradients,
confidence gating and clipping derivatives need CPU/GPU parity. Until those paths
are complete, fail closed on attach instead of falling back to an old decoder.
No implicit migration loads this candidate into an existing acquired brain.

## Genuine competition and lawful rest

An association can reduce loneliness while increasing fatigue or depleting ATP.
The separate signed lanes can oppose each other before a small learned readout;
the ordinary network retains its other sensory, effort, memory, innate, cognitive
and recurrent contributions. There is no winner-take-all drive selector, fixed
need hierarchy, prescribed activity quota or pressure bonus on every action.
No evidence means no consequence adjustment, not forced exploration or rest.

Current V1 cannot give Rest its own recalled benefit because Rest is targetless.
It can leave a legal Rest/Idle candidate alone while making a costly association
less attractive, but that is not evidence of learned recovery. An honest test of
learned recovery needs a **separate future** self-action association schema, e.g.
`AssociationAnchorV2::SelfBody` bound to world/life/body and the sealed executed
bundle, with actually measured recovery. It must not fabricate a tracked target,
relabel old memories or copy contact relief onto Rest. Simultaneous locomotion
must use realized displacement and the world's single physiology update. Legal
blocked/no-op commands must stay distinct from locomotion with actual travel.

The source-only synthetic conflict fixture uses a *hypothetical* fatigue/ATP
recovery delta to verify arithmetic can favor either work or rest under different
genuine pressures. It is not an admitted V1 Rest record or proof that the world
produces that recovery. Real rest controls wait for the appropriate source schema
and authorization to run them.

## Smallest finite native follow-through, then a grounded action fixture

**First native step, only after coordination:** add one test-only bridge fixture
on the qualified store component. Reuse its validated sealed `Fixture` /
`ExperiencePatchBuilder` construction and exact recall, but exercise the new
evidence-to-neural-feature path rather than repeat the completed store tests.
Use a current contract-valid frame at a later tick with exactly matching context;
its source is a synthetic contract fixture, not a simulated life continuation.
Read current drives from that frame and deltas only from the corresponding
validated retained observation. Bind source sequence/frame digest and assert one
representative observed-target action, no additional selected motor channels.

Freeze seven rows and two gain factors, exactly fourteen evaluations:

1. Loneliness .15, measured delta -.007499993.
2. Loneliness 0, measured delta 0.
3. Loneliness .001, measured delta -.001, respecting source bounds.
4. Loneliness 0, measured increase .007499993.
5. Loneliness .15, measured delta 0.
6. Same .15 context but no measured physiology: Unknown.
7. Query loneliness 0 against the .15 source: exact miss, not reused relief.

For each, use gain 0 and .35 with otherwise identical scratch and work. Construct
fresh scratch only. Evaluate affine/clip/availability stages with actual core
`cpu_spmv_projection` and `finalize_cpu_activations`, and the four-branch burden
targets with actual `DendriticBranchSet` / `apply_dendritic_conjunctions`. Keep
stage boundaries explicit; do not write the scalar model's computed pressure or
association into the native output and count that as a test. Run the complete
eleven-lane arithmetic shape even though this first bridge varies one lane.
Compare signed per-drive neural outputs against the reference, report native
work receipts, intermediate values, maximum error and unknown masks.

Stop at these per-drive features. Report source confidence separately; a host
diagnostic multiplication may check the reference formula but is not native
gate qualification. Do not invoke a selector, credit update, PPO, GPU, persistent
brain or private checkpoint. Existing core primitive tests are sufficient tools
for this stage; a compiled recurrent phenotype and its neural effects remain
separate gates. Fourteen scalar-core evaluations are not fourteen creature ticks
and are not an actual512-brain result.

**Next grounded step, separately frozen and not executed here:** all existing
measured social sources were prescribed Inspect intervals. Passive clones got
the same relief. With exact primitive/family matching they cannot teach Approach,
navigation or Rest. Never relabel an Inspect source as an Approach consequence.

Before claiming action-dependent social choice, obtain a new actual singleton
Approach interval that crosses from outside into peer contact in the same owned
world. The relevant primitive is `HeadlessActionIds::APPROACH = ActionId(101)`,
with `CandidateActionFamily::Approach`, not Inspect. `headless.rs:5040-5074`
computes initial/final peer touch and one biology update in the registered
transaction; `headless.rs:4747-4830` does the corresponding joint-interval work.
`execute_move` uses observed target displacement and actual steering, so proximity
or selecting Approach alone cannot establish contact or movement.

Freeze four matched arms at each of loneliness 0 and .15, at most eight source
intervals: Approach to reachable peer crossing contact; Approach to an equally
reachable toy; passive no-move from the same outside-contact start; and a failed
or blocked Approach that has zero realized travel and does not enter contact.
Keep genome, biological age/expression, current chemistry other than the named
pressure, starting pose, target distance, timebase, interval length, and available
candidate geometry fixed where compatible with each control. Verify reachability
using the actual trajectory, not just equal nominal distance. The blocked arm's
explicit obstacle is its experimental difference. Do not vary fatigue, train the
network, or extend duration to search for a favorable result during this fixture.

Capture pre/post pose, per-segment travel, initial/final contact, body-event doses,
all eleven measured deltas, costs, exact command/family/target, source receipt,
developmental state and world/life binding. Derive sealed neural evidence from
the actual owned decision/execution path, not a fabricated decision receipt.
Passive control deltas remain control observations and do not become fictitious
action memory. Independent cloned arms keep their own fork/world/life provenance;
do not pool their private observations into one creature's memory. No channel
receives a duplicate copy of the joint interval delta.

This frozen fixture could establish an observed Approach-associated contact
transition and its contrast with no-contact controls. It still would not prove
a learned policy, long-range navigation or isolated neural causality. A subsequent
current decision needs separately qualified retrieval of its own compatible
Approach evidence and actual candidate-relative probability/behavior change.
Exact next-state misses remain Unknown until that retrieval work succeeds.

## Bounded next experiment and stop gates

Do not start another life sweep or a device session. Coordinate one serial,
standalone arithmetic compile/test only after the store owner frees the lane.
No live store or 512-neuron private state is needed. First check the following fixtures:

1. **Signs and physical bounds:** all eleven lanes, both delta signs, zero,
   pressure 0/near-zero/.15/.5/1; ATP below/at/above target; curiosity/reproduction and
   untyped extensions informational. Typed boredom requires its exact role
   binding. Nonfinite/out-of-range values fail, never clamp silently at admission.
2. **Evidence states:** Unknown vs measured 0 vs confidence 0 vs unsupported; raw
   deltas unchanged; source-before and current not interchangeable. Confidence
   applied once; altered similarity changes neither delta nor confidence.
3. **Competition:** same fixed evidence, low-vs-high fatigue/ATP and loneliness;
   two opposite signed components can favor work, neither, or a synthetic
   stationary recovery row. Neutral rows retain zero correction. No universal
   action requirement and no new action-selection implementation.
4. **Matched neural controls:** identical work for gain 0, genuine conditioned
   comparison, pressure held at a fixed reference, and permuted need labels.
   Repeat with a single target and two candidate targets in the same family.
   If distinct target evidence collapses to the same family correction, stop.
5. **Source-support controls, separately owned:** exact vs proposed local matcher;
   current/source pressure differences just within/over .025, adversarial
   loneliness 1, wrong organism/life/world/genome/profile/entity/action, different
   interval duration, stale/future source, juvenile/adult mismatch, multi-command
   bundle mismatch, pruned/missing source and non-measured evidence. Unknown must
   remain Unknown without invented labels. Do not repeat closed peer/toy studies.
6. **Only after 1–5:** inspect candidate-relative correction, conditional per-motor
   margins, softmax probabilities and actual displacement in a small matched
   fixture, with fixed seed/counter/temperature/current brain state. First 64
   saturation and initialization-age confounds must be reported. A nonzero
   correction or `Some(locomotion)` is not behavioral success. Equal sampled
   choices need not refute a tiny effect, but do not justify a success claim.

Before persistent integration, require a meaningful candidate contrast that is
not family-common, correct zero-pressure behavior, no Unsupported exposure and
no targetless-Rest overclaim. Before production exposure, additionally require
source binding, independent retrieval qualification, budget receipts, complete
CPU/GPU and gradient parity, default-off identity, rejection of mismatched
checkpoints, and byte-for-byte preservation of old formats and acquired state.

Failure to meet those gates means keep this candidate detached. The smallest
next answer is whether this circuit expresses the desired biology without
inventing consequences. It is not yet whether a whole persistent creature is
better at living.

## Verification status and smallest follow-through

Only source inspection and `git diff --check` have been performed for this
candidate. The ten synthetic tests are authored, not compiled or executed.
No Cargo manifest or module export includes `reader_algebra.rs`. After explicit
coordination of the shared lane, its bounded arithmetic check can use an isolated
output path and `rustc --edition=2021 --test reader_algebra.rs -o <temporary path>`
followed by that test executable. This would be a CPU scalar arithmetic result,
not a Cargo suite pass, source-binding test,512-neuron result or GPU result.
It is intentionally pending; do not interpret the command as a scheduled job.
