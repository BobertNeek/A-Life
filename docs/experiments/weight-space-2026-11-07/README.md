# Native weight-space brain surgery, 7 November 2026

Status: isolated research candidates. No live organism, acquired private weights,
optimizer, replay history, production foundation, or default New Game policy was
replaced. This is a CPU experiment, not GPU gameplay qualification or a learned
capability claim.

## Why this experiment

Cassidy asked for both direct surgery and an ambitious whole-brain attempt, and
for notes that make the later Astra 6.2 attempt easier to execute and audit. The
inspiration is [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/),
MIT CSAIL OASYS, 2 November 2026. The paper's small feedforward game controllers
are not equivalent to A-Life's recurrent, embodied, plastic nine-lobe brain.
Therefore a paper result is motivation, not evidence that this candidate works.

The existing N2048 brain has 2,048 neurons, 24,576 recurrent synapses, 3,072 action
candidate readouts, 1,024 speech readouts, and 4,096 memory readouts. Physical
candidate features enter a native action decoder; there is no LLM at action time.
Ordinary somatic learning remains enabled during the actual trajectory. Founder
Adam is not updated by this experiment.

## Preservation boundary

All trials create disposable fresh founders from the public shipped foundation.
No retained or user-trained life is opened. The code exports a separate,
unpromoted immutable candidate through the supported asset constructor. A
serialized whole-host restore is checked before and after one additional step,
including private state and world, using cloned hosts so that the reported
endpoint remains exactly the requested number of steps. This is an in-process
serialization/restore check, not a new-OS-process or GPU recovery qualification.
The founder optimizer checkpoint must remain byte-for-byte unchanged.

A topology candidate must have its own ABI/foundation identity. The old graph
must remain accepted exactly as it was, and old asset/graph cross-admission must
fail. Reinterpreting old synapse indices as new endpoints would destroy the
meaning of acquired weights and is specifically out of scope.

## Log before results

### Exploratory positive taste edit, superseded

The first hypothesis was to add +0.5 to the 16 Ingest/chemical-0 readouts.
Inspection and actual replay showed that an action logit contains

`motor activation × candidate feature × effective weight`.

The baseline Ingest motors are negative on the initial contact-food frame.
Consequently a positive edit *lowered* the Ingest logit. This was a sign mistake,
not a failure of the native decoder. It was corrected after feedback, and is not
counted as a successful one-shot design.

The exploratory “poison” arm was also invalid as a negative-taste control: changing
Food.hazard_pain does not invert the grounded chemical feature, and unheld
hazardous Food has different acquisition legality. Those trajectories cannot
establish discrimination or learned avoidance. The replacement control uses the
same harmless, edible Food, reach and nutrition, with only chemical-0 set to
−0.85 rather than +0.85. The observed first-frame feature sign is asserted.

An earlier restore comparison advanced the reporting host one unlogged tick;
that was repaired before the frozen signed-surgery suite. Final endpoints now
precede the extra restore-comparison step.

### Whole-weight design v1: failed to construct

The true first whole-weight proposal set every weight to zero, then tried to
construct short positive paths from hunger to Ingest/Approach, fatigue to Rest,
pain to Avoid, and curiosity to Inspect/Contact, with grounded feature readouts.
No rollout, reward or gradient was used to choose that proposal. The two-hop
need→reserve→motor construction failed before a whole-brain behavior trial.
A revised breadth-first search up to six hops also failed at hunger→Ingest.
Neither is a successful brain or a behavioral observation.

### Structural diagnosis, before graph repair

An unlimited directed breadth-first traversal of the actual compiled graph
confirmed that each of the 16 encoded Homeostasis inputs reaches only 33 neurons
and no action-family motor. Hunger neuron 256 has 16 outgoing recurrent edges;
its downstream component is the dead end. Across 183 encoder assignments, 124
have no action-family path and 37 have zero recurrent out-degree. This is a
structural trace, independent of weight signs or activation, not an assertion
that all biological influence is absent: inherited innate priorities and global
biochemical modulation are additional pathways.

The source explains why: deterministic_tile_pairs selects whole 16×16 tiles
under a fixed sparse budget, without checking encoder-to-motor coverage. N2048
coordinates are genome-independent and frozen for foundation compatibility.
Interoception→reserve has four selected tiles for eight source tiles; subsequent
reserve→temporal/action selections can miss their targets. The exact terminal component is hunger256 → Reserve1968–1983 → Temporal1216–1231. That reserve tile has no direct motor route, and that temporal tile is absent from every outgoing temporal projection. The recipe is genome-independent, so changing founder seed cannot repair it. Stable acquired
addresses are intentional. Excluding neural hunger from action is not documented
as an intended design. The safe response is an explicit experimental recipe,
not a silent change to the frozen recipe.

### Revised surgical candidate: predeclared matched suite

Change only the 16 Ingest/chemical-0 readouts by −8. Keep all other weights and
all graph coordinates unchanged. Test the opposite edit (+8) as a wrong-sign
control, alongside untouched baseline. The initial negative motor activation
predicts a first-frame logit increase for positive taste and decrease for
negative taste. Later selection, possession, actual ingestion and energy are
separate outcomes and must not be inferred from logits.

The frozen suite uses eight founder seeds beginning 2026110701, 12 ordinary
steps, four scenes (reachable positive food, equally reachable harmless bitter
food, toy, distant positive food), three arms, body energy 0.4 via the supported
body setter, and otherwise unchanged newborn biology. Hunger is zero in this
newborn fixture; do not call it a hunger challenge. Every target lies ahead,
removing the prior seed-linked left/right confound. No teacher, forced food,
reward shaping, host-score change, gradients or optimizer steps are supplied.

The −8 candidate was revised after the positive-sign pilot, so it is feedback-
informed. Its first successful food rollout is a pilot, not a held-out sample.
Both signs of the weight edit can change later consumption, including bitter
food, so no selective food preference should be claimed without the full
matched result. Broad survival, reversal learning and language are separate gates.

## Whole-brain experimental design, declared before its first rollout

The revised full candidate will use the separate experimental coordinate recipe
and a native, from-zero weight array. It will retain all nine lobes and the exact
N2048 parameter budget. The design intent is a physiological arousal/need circuit,
grounded approach/ingestion/contact/avoidance/rest readouts, damped recurrent
context/memory traces and lateral motor competition. Speech payloads abstain
unless a grounded design can be specified without treating token IDs as meaning.
This is an ambitious care scaffold, not a claim to have hand-written language,
planning or an entire learned mind. All zeroed and nonzero sections will be
recorded. Any post-rollout tuning must get a new version and preserve this first
executable/source receipt.

Controls: untouched frozen graph, same experimental graph with excitatory/readout weights zero and the same
required negative lateral-inhibition weights,
whole candidate, and whole candidate with need/arousal bridge weights zeroed.
Connectivity is necessary, not sufficient: signs, saturation, cadence, biological
normalization and behavior still need measurement. The zero-excitatory control also
reveals behavior supplied by inherited native priors rather than this design.

## Reproduction interface

The source example is `probe_weight_space_care` in `alife_training`, behind the
`cpu-backend` feature. It never reads a saved life. Output locations must be new;
per-trial receipts are durable before the next trial starts. Existing output is
never overwritten. Raw trial receipts include actual candidate features, logits,
selected native joint action, measured outcome and body state, plus exact restore
and unchanged-optimizer assertions. Summary results below will distinguish
completed runs from failures and proposed work.

## Astra handoff requirements

1. Read the exact compiled graph, signed sensor/decoder equations and cadence,
   not only a diagram of named lobes. A nominal route can be a functional dead end.
2. Submit a true one-shot source and immutable executable/weight receipt before
   any behavioral feedback. Keep later revisions separate.
3. Make targeted and from-zero candidates separate factors. Protect the trained
   reference, optimizer, replay and recovery artifacts. Never reset a life to
   make an edit convenient.
4. Match reach, legality, physiology, sensory signs and sampling counters across
   controls. Log attempted and executed actions separately from reward.
5. Report survival/food/navigation, need sensitivity, delayed memory, reversal,
   sleep, language transfer and exact restore separately. A narrow food success
   does not pass the rest. GPU authority and retained-state preservation require
   their own tests before any promotion.
6. Preserve negative results, including constructor failures and sign mistakes.
   Do not convert reasoning about a circuit into a measured learning claim.

A constructor review caught that native lateral-inhibition synapses must remain
strictly negative. The zero-excitatory control therefore uses −0.03 on these
synapses, exactly as the whole candidate does; BFS bridge paths exclude inhibitory
projections. We did not weaken the sign contract to make an all-zero array pass.

Frozen source snapshots under `frozen/` retain the failed first whole-weight
construction, its six-hop revision, and the subsequent signed-surgery suite.
These are historical executable inputs, not current recommended code. Source
and executable SHA-256 for the signed-surgery suite are respectively
`a5da8f53160cc1646921bc137115fa28e85aebbf10e6f894a5efc3156f085839` and
`deaef8ecd10dec5f18852f59fa67f76e89cb6468184ec326d4cddfaa76c4a6ee`.

The experimental coverage recipe preserves static synapse counts but uses 137
active recurrent tiles rather than the frozen recipe's 96. It is not a compute-
matched comparison. Actual execution cost and cadence must be reported alongside
behavior; matching a parameter count does not match biological work.

## Completed signed-surgery results

The frozen suite completed all 96 trials (8 seeds × 4 scenes × 3 arms).
[Machine-readable summary](surgery-v3-summary.json) includes each seed, attempted
Ingest selection, completed physical ingestion, energy and first-frame logits.
The first-frame weight equation predicts every observed logit change with maximum
absolute residual 0.000006662. Thus the edit had its intended initial numerical
effect; that did not predict the complete closed-loop trajectory.

| Scene | Untouched consumed | −8 consumed | +8 consumed |
|---|---:|---:|---:|
| Reachable positive food | 0/8 | 4/8 | 8/8 |
| Matched harmless bitter food | 0/8 | 8/8 | 8/8 |
| Distant positive food | 0/8 | 0/8 | 0/8 |
| Toy | 0/8 | 0/8 | 0/8 |

For reachable positive food, mean final energy was 0.398006, 0.466891 and
0.535668 respectively. Both edited arms consumed bitter food, with mean energy
0.535677 (−8) and 0.535793 (+8), because it was equally nutritious and safe by
construction. No injury occurred. Distant-food attempts did not become completed
ingestion; mean selected-Ingest ticks were 3.0 and 5.5 for the two edits, versus
zero for untouched. Toy trajectories were unchanged at this horizon.

Decision: **do not promote either edit as selective food competence**. A narrow
physical consumption gain is measured, but negative-taste discrimination and
navigation are not established. The opposite-sign arm performing better is a
useful warning about interpreting the first frame as an entire recurrent,
plastic, stochastic life. No gradient or optimizer update explains these runs.
Ordinary somatic updates and changing perception remain part of the trajectory.

## First runnable whole-weight result, retained unchanged

The 137-tile candidate completed 32 trials, two seeds × four scenes × four arms.
[Summary](whole-v3-summary.json) and [frozen source](frozen/whole-v3-coverage.rs.txt)
preserve the original runnable whole-weight design. It consumed no positive
food, both bitter foods, no toys and no distant food. The need-bridge ablation
had the same consumption counts. This fails the intended signed-taste behavior.
The zero-excitatory control consumed neither food sign in these trials.

Two errors were diagnosed after this run. First, the proposed physiological
“arousal” lane7 was temperature stress, not ATP; DriveSnapshot::to_array places
ATP at6. Second, newborn biochemical effects introduce a local threshold of0.25
and a negative regional offset, so nominally silent native motors are negative.
Positive direct bridge weights did not establish a positive motor operating
point. The wrong sign propagated through the grounded readouts. These are design
errors, not evidence of learning or evidence against the coverage graph itself.

The next version will explicitly synthesize a tonic native circuit from a
non-sensed resting interoceptive neuron, using a negative weight only on the
signed Modulatory reserve→temporal projection. All homeostatic/motor-proposal
edges remain nonnegative and lateral-inhibition edges remain negative. Separate
positive need paths will converge on the same temporal/motor units. This changes
weights, not neuron biases, biochemical thresholds, host priors, action legality
or reward. It is a feedback-informed revision and will have a separate frozen
source, executable and pre-rollout candidate assets. The original remains intact.

## Revision4 pre-rollout record

The frozen revised source is [whole-v4-tonic.rs.txt](frozen/whole-v4-tonic.rs.txt).
Its SHA-256 is `24b134b7abf5ef9816c06c4e9ddc42c94f2b314fc93234f851574f79d30a112f`;
executable SHA-256 is
`9fa3c12151f79c039b81118a820a21798f0ae92579880c7ef7e02bfe0402b3f7`.
Fresh candidate asset bytes are written and synced before each arm's first
behavioral step. No prior receipt is overwritten.

This revision additionally uses the final motor in each family as an un-driven
reference. Its grounded readout has −5 times the signal coefficient, per signal
motor. This makes the negative resting motor operating point explicit rather
than incorrectly assuming zero activation. The reference is an ordinary native
neuron/readout, not a host-computed action score. Its own ablation tests whether
it supplies the gain. Other controls independently zero tonic bridges or need
bridges while preserving their common temporal→motor output and all other
weights. They must not be described interchangeably.

A detached first-frame replay adds0.1 only to encoded hunger inputs while leaving
innate candidate biases fixed. Its logit difference measures recurrent input
sensitivity, not a biological hunger episode, physical behavior or learning.
This lets the graph/need ablation be tested even though the actual newborn fixture
has no hunger drive. The complete physical trajectory is still run normally.

Profile limitation: GroundedObjectSlotsV1 exposes raw object chemistry even at
2.5m. Only GroundedTerrainVisionV1 applies contact/chemistry calibration. The
far-food scene here is a reach/navigation condition, not a hidden-taste or
zero-contact-chemical control. Terrain-only HeardLanguage/SemanticPrior coverage
is unqualified in the137-tile recipe. A future separately identified147-tile
recipe could add Multimodal→Temporal source coverage; it first needs a real
Hearing-enabled fixture and separate validation.

Circuit review also found that the revision4 first-compatible-path selection can
share a temporal intermediate between hunger and curiosity. These are mixed
native need signals, not independent, need-specific channels. The global need
ablation removes those bridges; it does not prove that any individual need is
solely responsible for a family. A future revision should reserve independent
intermediates and add per-need ablations before claiming selective motivational
wiring. The reference motor is not a constant: inherited inhibition and future
biochemical changes can change its operating point.
