# Executed-channel evidence V1

## Why there are two kinds of evidence

A motor bundle can move toward one target and manipulate another. Its measured
hunger, pain, energy, reward and other physiological changes belong to the whole
executed interval. The representative candidate is the memory query for that
joint consequence, not proof that its target caused every change.

For example, movement toward Food may really collide with Hazard. Conversely,
movement toward Hazard can coincide with a secondary Ingest/Food command that
consumes food. Neither observation proves that Food caused pain or Hazard was
nutritious. Do not remove the shared consequence or duplicate it onto each
channel to make an attribution.

## Actual execution facts

`ChannelPhysicalOutcome::execution_evidence` optionally carries
`ExecutedChannelEvidenceV1`: executed primitive, commanded entity, success, and
that channel execution path's `BodyEventDelta` inputs. Its paired physical
receipt identifies actual contact/displacement. Commanded entity and contacted
entity can legitimately differ. Successful consumption requires the commanded
and consumed entity to agree.

World captures these facts before composing the shared ambient, hazard,
reflex and pending-player-care contributions. Nutrition in this evidence is a
body **input**, not isolated measured hunger relief. Praise, scold, pet and slap
remain external interval inputs, not reinforcement causally assigned to the
next Ingest or to an unsealed previous action. No object-kind label is used to
infer causality.

The existing aggregate measured physiology, biological credit and neural joint
plasticity stay shared. In particular, existing channel-terminal measured
hunger/danger fields retain their whole-bundle semantics; do not reinterpret
them as channel-local effects because execution facts are also available.

## Lazy factual lookup APIs

Ordinary `recall_frame` and `recall_frame_runtime` prepare scalar bundle
expectancy without searching factual execution evidence. Their candidate-aligned
`executed_actions()` entries are absent and `execution_evidence_work` is zero.
The CPU collector does not request factual lookup merely to populate reports;
the live game likewise does not pay for an unused reporting projection.
Authenticated execution facts are still stored/indexed under ordinary episode
admission. Missing projection here means not requested, not missing history.

An offboard viewer explicitly calls
`MemoryBank::inspect_execution_evidence(&FinalizedMemoryRecall)` or the
corresponding `MemorySidecarState` method. Its `ExecutionEvidenceInspectionV1`
contains candidate-aligned `rows` and `diagnostic_work_units`. This read-only
inspection does not change bank state, measured outcomes, cognitive receipts or
creature energy. Its work metric belongs to the external inspection, not a
brain debit. It queries the current bank with the supplied historical query;
it does not bind to the original decision's bank snapshot. Later admissions can
change results, so this is not proof of facts available at that old decision.
For that reconstruction, inspect the retained original bank snapshot instead.
Ordinary diagnostic JSON is not an instruction to charge biology.

A future actual cognition consumer can instead request
`recall_frame_with_execution_evidence`, or
`recall_frame_runtime_with_execution_evidence`. These explicit cognition APIs
perform the bounded lookup and include its work once in the returned cognitive
recall receipt. CPU/live-game work aggregation accounts for that receipt. A real
neural consumer must use this charged path, not disguise its work as offboard
viewing. These are workload APIs, not permission gates or new policy rules.

`PreparedMemoryRecall::executed_actions()` and
`FinalizedMemoryRecall::executed_actions()` expose candidate-aligned optional
`ExecutedActionRecallV1` entries when requested. Each names its memory record and
a typed `ExecutedActionMemoryV1` fact containing actual source sequence,
pre-action query, event tick, bundle digest and channel receipt. Recall matches
exact organism, profile/schema/sensory ABI, query version, tracked target,
action family and primitive; it does not borrow category evidence to claim an
unobserved object was consumed. Conflicting same-event facts abstain. Event tick
can exceed record.last_tick, which describes pre-action query time.

The facts remain separate from scalar bundle values and learned physiological
expectancy. They say what action phase/input occurred, not its isolated
physiological effect or what will result next time. No new neural feature,
logit gate or behavioral attribution fix is introduced.

## Costs and bounds

Requested lookup is capped at 64 records per candidate. Serialized facts are
capped at 64 KiB and `MAX_MOTOR_CHANNELS` entries per episode. Internal records
couple unchanged original wire bytes to immutable typed facts validated at
admission/cold load. Internal validation, digest, index and recall reuse that
checked representation rather than repeatedly parsing the same JSON. Requested
work counts an index lookup, record access, fact comparison and final projection;
it does not charge for JSON parsing that no longer occurs during that lookup.
Explicit cognitive lookup charges this work even when scalar recall was cached.
An empty execution index costs zero. Offboard inspection reports the same work
metric separately and never feeds it into the creature's cognitive receipt.

`CpuCollectedDecision::executed_action_recall_work` reports requested cognitive
work already included in `memory_ops`, not another charge. Actual debit depends
on the host policy, phenotype metabolism/efficiency and available energy;
component times tariff alone is not measured physiology. Explicit additional
cognition can indirectly alter future energy/behavior. External viewing should
not. Authenticated storage/validation/serialization still has host overhead;
the lazy split does not claim those costs vanish or establish game FPS parity.

## Legacy, retention and trust boundaries

Absent execution evidence means unknown. Old records are not upgraded from
contact-like floats, representative labels or shared physiology. Optional
absence leaves legacy serialization and canonical digest streams unchanged.
New facts are checked against the actual sealed bundle and included in patch
and portable-memory authentication. The private checked cache exposes no mutable
bytes or facts; exports preserve original bytes exactly, including valid
whitespace. Public portable records remain mutable import data and their
validation/checksum paths revalidate their bytes. Internal export can reuse checked
facts only after matching the exact exported bytes; it retains the other portable
shape, owner and checksum checks. Cold restore checks fact ownership,
query/version scope, sequence/time bounds and proof shape.

Facts use ordinary memory admission and capacity. A low-information episode
that was not admitted does not gain admission merely by having execution
metadata. Merged episodes retain the latest known bundle's facts; later known
bundles may replace earlier secondary facts. A later unknown/legacy observation
does not upgrade an earlier fact's actual sequence. This is bounded current
recall, not an exhaustive per-action event archive.

Native captured reporting rows come from this validated path. A standalone
imported diagnostic JSON row is not independently authenticated against the
original bank just because it deserializes. Do not use edited reporting fields
as trusted learned evidence, training rewards or causal labels. Retained sealed
patches/checkpoints remain necessary for independent provenance verification.
