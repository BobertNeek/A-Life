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

This revision additionally uses the final motor in each family as an not positively driven by the constructed signal paths
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
is unqualified in the 137-tile recipe. A future separately identified 147-tile
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

## Completed revision4 pilot and revision5 rationale

[Revision4 summary](whole-v4-summary.json) covers all 48 trials × 12 steps. The full
candidate consumed positive food2/2 and bitter food0/2. The initial-genetic no-need
lesion had the same counts. Removing tonic bridges or reference readouts prevented
positive-food consumption0/2; removing the reference also led to bitter-food
consumption2/2. These are narrow fresh-founder CPU results, not learned taste,
broad survival or independent hunger control. Distant food was not consumed.

Navigation diagnosis: at the first far-food frame, Approach's signal and reference
activations were approximately+0.563918/+0.563083, giving a logit of−6.247900 under
the universal signal−5×reference rule. Ingest started negative, where that rule
had the intended sign. Native birth dendrites cover only the first 64 motor neurons
(Idle/Rest/Inspect/Approach), with positive inputs, negative threshold and gain6.
They explain why a single reference sign cannot work for every family. Therefore
“from-zero” refers to the complete **static compiled synapse array**. Native birth
dendrite weights, thresholds, gains, neuron dynamics and physiology were retained;
this was never a reset of every parameter in every neural subsystem.

Revision5 uses +5×reference for the positive-baseline Approach/Inspect families,
−1×reference for the fatigue-sensitive Rest family, and retains−5×reference for
the negative-baseline families. It reserves distinct reserve and temporal neurons
for each need axis (reuse is allowed for the same axis across relevant families).
This is a new, feedback-informed weight proposal, not a reclassification of the
original candidate. Candidate output remains entirely native.

All somatic plasticity remains active. A genetic-zero bridge lesion is not a
permanent disconnection: lifetime/fast offsets can reopen it. Revision5 logs the
actual effective value at every constructed bridge on every step, including the
lesions. The one-frame hunger sensitivity was zero because cadence delayed the
signal; a second detached probe repeats the same frozen context for three frames
and measures the last-frame response. Neither probe performs a learning update.

The predeclared next evaluations use new seeds: a two-seed six-arm ablation suite,
eight-seed positive/bitter transfer at forward/left positions, and four-seed
48-step navigation at forward/left/back positions plus an obstacle. Failures and
retained controls will remain in the results. No biological-learning or production
promotion claim follows from completing these tests.

Revision5 frozen source SHA-256 is
`2fc87f1cc19da79dc02dde1c12130842f6721dca18bc936a32ab32010e08476e`,
executable SHA-256 is
`06713b67d5772b82daba72a20e0da8da8ed933931acf0c6c79d606544861fb39`.
The current example also contains builder invariants and style-only cleanups;
the [frozen runtime source](frozen/whole-v5-family.rs.txt) identifies the exact run.

Configured microsteps and executed microsteps differ: the first revision4 work
receipt contains6,144 neural updates (2,048 × 3 executed steps), although the
phenotype allows4. Its576 dendritic operations also confirm participation of the
64 inherited branches. ATP/runtime policy and per-route cadence must be accounted
for when predicting how many frames a signal needs.

## Completed revision5 results

All132 predeclared trials completed: [36 ablations](whole-v5-ablation-summary.json),
[64 held-out transfer trials](whole-v5-transfer-summary.json), and
[32 held-out navigation trials](whole-v5-navigation-summary.json). These use the
same frozen candidate without tuning between suites. An independent read-only
parse confirmed the counts, actual outcomes and preservation flags.

| Condition | Public shipped founder | Engineered candidate |
|---|---:|---:|
| Reachable positive food, forward/left | 0/16 | 16/16 |
| Matched harmless bitter food, forward/left | 0/16 | 0/16 |
| Distant food, forward/left/back,48 steps | 0/12 | 12/12 |
| Food behind obstacle,48 steps | 0/4 | 0/4 |

Actual consumption is a physical world outcome, not a reward proxy or an inferred
logit preference. The raw receipts also distinguish selected ingestion, possession,
completed ingestion and final energy. No injury occurred in these trials. They
are fresh, initially low-energy CPU lives with unchanged newborn homeostasis,
not a comparison with Cassidy's best retained PC/cloud checkpoint and not real
GPU gameplay qualification.

The three-frame detached hunger perturbation increased the full candidate's
positive-food Ingest logit by0.10956764; the old graph and initial no-need lesion
showed0. Need and tonic genetic lesions reopened through ordinary plasticity by
at most about0.000001802 and0.000002488 during the12-step ablations. These results
support a delayed, effective encoded-input pathway. They do not demonstrate
biological hunger conditioning or learned motivational control.

In the revised two-seed ablation, all three engineered lesions still consumed
positive food2/2. The reference lesion consumed bitter food1/2; full and other
lesions consumed0/2. Therefore the positive-food result no longer establishes
that the tonic, need or reference component is individually necessary. The earlier
revision4 necessity result must not be carried over to revision5. The multi-frame
probe isolates the engineered need path more directly than endpoint consumption.

Decision: retain this as a promising **experimental native care founder**, with
measured signed-taste and unobstructed reach behavior, while leaving accepted
brains unchanged. Obstacle competence, reversal learning, delayed memory,
biological need generalization, speech/language, aged/retained organisms, terrain
profile and GPU execution remain unqualified. Static synapse counts are matched,
but137 versus96 active tiles and native work receipts mean compute is not matched.
The summaries expose measured work per trial rather than assuming equivalence.


### What the new-seed tests do and do not replicate

All whole-care trials in these suites use one identical hand-built candidate
digest. New seeds change sampling, body/world identities and positions; they are
not independent trained brains or unseen-genome replications. The near left-food
condition is already in reach at0.25m. The forward/left/back2.5m scenes provide the
actual unobstructed reaching test. ObjectSlots supplies raw distant chemistry,
and Approach invokes the existing bounded visible-goal clearance steering in the
actuator. Neither distant success nor lateral movement establishes learned
route planning or a neural tangent choice.

### Longer unchanged obstacle control

The same frozen revision5 executable/candidate was evaluated for120 steps on
the same four obstacle seeds with matched untouched controls: [8 trials](whole-v5-obstacle-extended-summary.json).
Neither arm consumed any food. The full candidate initially moved around the
obstacle, first reached the food-contact feature at tick60, then stopped choosing
Approach to food and repeatedly attempted ingestion without completing it. Its
final food distance was approximately0.480m. The48-step failure alone was horizon
censoring, not proof of a stall; the120-step extension exposes a later
contact-stage problem. Investigation must distinguish neural target choice and
grip geometry before changing any action interface.

### Prepared-biology pilot, separate from aged neural learning

An optional bounded preparation mode advances the newly admitted world600
ordinary ticks before its first neural dispatch, with no replacement of neural
state, forced actions or invented hormones. One new seed,2026111001, was then
tested for24 steps in matched positive/bitter scenes, untouched/full: [4 trials](whole-v5-prepared-biology-summary.json).
Initial biology was exactly identical in all four: age600, maturation0.328947,
hunger0.812937, energy0.249395. The candidate consumed positive food1/1 and bitter
food0/1; untouched consumed neither. Final positive-food energy was0.383607
versus0.247320 untouched. This is a useful first juvenile, naturally hungry
biochemistry check, not an independent weight-set replication, a mature trained
organism, biological need-causality proof or a survival benchmark. The compiled
construction phenotype stays canonical while current development gates runtime.
The fresh neural state has not lived or learned through those600 preparation
ticks.

Preparation harness source SHA-256 is
`82f3203c5f5df175c15c2c962b101d8d8e63a4574ca35719e995da577879badc`;
executable SHA-256 is
`9f5316a4b7907b5372334e955c93b8963a9ba281a0ea53837eecaec70caf627f`.
This harness change leaves revision5's candidate-weight recipe unchanged.

## Revision6: repair a measured target/grip interaction

The extended obstacle failure provided a concrete weight-level diagnosis. Food
contact flips its Approach logit from+18.43 to−9.72, while obstacle Approach stays
positive. Native locomotion runs before manipulation and turns body yaw toward
that wrong target. At the stalled position the carry grip intersects the
obstacle's effective blocking region, so selected Ingest cannot acquire food.
This is not evidence that a detour command is missing: the earlier part of the
same trajectory already detoured using ordinary Approach and actuator clearance.

Revision6 changes Approach's contact-lane18 coefficient from−5 to−2, with the
matching reference coefficient. Exactly two static weights differ from revision5;
the builder test asserts their family/lane and preserves all sign constraints.
The hypothesis is that contact must not suppress Approach to nutritious food
below Approach to the obstacle before the grip is clear. It changes no physics,
hand legality, action enumeration, reward, phenotype coordinates, plasticity,
biology or optimizer. This is an explicit feedback-informed repair.

The [frozen source](frozen/whole-v6-contact.rs.txt) SHA-256 is
`207df08e994179ed5bc5d6c25fd14f7a27a865fc02ea6e5e83b24adfe4c33c1c`;
executable SHA-256 is
`56469b0903a328f83d60c2da4af34c65951dd140f6ebadd1bdfd4625072e643d`.
The contact mode includes the exact revision5 weight builder as its matched
control. A [one-seed pilot](whole-v6-contact-pilot-summary.json), same old obstacle
seed2026110901 and120 steps, completed ingestion for revision6 and not revision5.
Final energy was0.517650 versus0.380073. This pilot is feedback evidence, not an
unseen-seed result. Before inspecting subsequent outcomes, the unchanged
candidate was assigned four new seeds2026111101–2026111104, both versions, four
scenes (near positive, near bitter, distant left positive, obstacle positive),
120 steps each. The result must include every arm and scene, including failures.

## Running the current interface

From a checkout of this PR, with the repository's Rust toolchain:

```sh
cargo test -p alife_core --profile dev --test n2048_experimental_graph
cargo test -p alife_training --profile dev --features cpu-backend --test cpu_experimental_graph
cargo test -p alife_training --profile dev --features cpu-backend --example probe_weight_space_care
cargo run -p alife_training --profile dev --features cpu-backend --example probe_weight_space_care -- \
  NEW_OUTPUT_DIRECTORY 120 -8 contact 4 2026111101 \
  food,bitter-food,far-food-left,obstacle-food .4 0
python docs/experiments/weight-space-2026-11-07/summarize_receipts.py \
  NEW_OUTPUT_DIRECTORY > NEW_SUMMARY.json
```

Arguments after the output directory are step count, surgery gain, mode, seed
count, first seed, comma-separated scenes, initial energy, optional biology-
only preparation ticks and optional target hazard pain0..1 (default0). Modes: surgery (untouched/signed/opposite), whole (all
component lesions), validate (untouched/current), contact (revision5/current).
Output directories and candidate asset filenames are created exclusively; use
new locations. The current example is revision6. Reproducing an earlier version
requires that version's frozen example source and compatible compiler from this
PR, in a separate disposable worktree; never replace a retained life or current
project asset to reproduce it. Summaries bind the raw completed receipt SHA-256.
All candidates are unpromoted and the experimental graph is opt-in CPU only.

## What Astra should attempt next

A stronger native mind needs tests that this care scaffold has not passed. Keep
these as separate development gates rather than fold them into the food score:

- Sensory generalization: a separately versioned Terrain/Hearing graph with
  essential heard/semantic ports admitted and latency measured. Hide chemistry
  until genuine contact, then test remembered taste rather than distant raw data.
- Motivation: matched low/high biological hunger, fatigue and pain contexts,
  per-need genetic lesions plus logged effective offsets, and action/energy
  consequences. Encoded-input sensitivity alone is not motivated behavior.
- Memory: cue disappears before choice; randomize cue/reward mapping between
  episodes and measure delay curves and memory-state lesions. Current mnemonic
  recurrent weights are a design proposal; speech/memory readout zeros do not
  establish useful recall.
- Learning: preserve one organism's full state through acquisition and reversal,
  compare ordinary plasticity with an explicitly admitted no-update control, and
  distinguish existing reflexes from improvement over experience. Founder Adam
  staying fixed does not mean somatic learning is absent.
- Rest and sleep: fatigue recovery, interruptibility, wake after danger and
  retained skills across sleep/save/load, with actual biological outcomes.
- Language: grounded referent/action transfer through ordinary spatial speech,
  novel speakers/positions, ambiguity and delayed reference. Never equate token
  numbers, a decoded payload or external teacher text with understanding.
- Navigation: visible-goal actuator steering versus neural exploration when the
  target is occluded. Admit new turn/waypoint affordances only as a separately
  controlled profile extension; do not blame an interface for a wrong-target
  readout bug.
- Release qualification: full GPU-authoritative gameplay, exact retained-life
  restore and evidence-preserving acquired-state migration are independent gates.
  This new graph cannot reinterpret the old graph's acquired coordinates.

Preserve the original proposal, every meaningful revision and the causal reason
for each edit. Iterate boldly, but freeze each candidate before evaluating its
next unseen contexts and retain the failures. The present result is native
engineering practice and a reproducible starting point, not a substitute for
those tests or for Cassidy's retained learned creatures.


### Completed revision6 regression and collector recovery

All32 predeclared120-step trials have durable individual receipts. The unchanged
revision6 candidate consumed positive food4/4, harmless bitter food0/4,
distant-left positive food4/4 and obstacle food4/4. Revision5 matched the first
three counts but consumed obstacle food1/4. Thus the old version sometimes
escapes its contact/grip failure through stochastic alternatives; it was not
impossible in every new context. Mean obstacle final energy was0.517552 versus
0.414408. [Per-trial summary and hashes](whole-v6-contact-held-out-summary.json)
include the failures and flags. This remains one fixed engineered weight set
under new sampling/body/world contexts, not a breed-generalization claim.

The original collector was killed (exit137) while cloning/serializing the final
large aggregate after all32 complete trial files had been synced. Its empty
original aggregate and log were preserved. A separately labeled completion
index was reconstructed from exactly those original files after verifying
120 rows and unchanged-optimizer/exact-cold flags in each. No simulation was
rerun and no row was invented or edited. The summary records this aggregation
recovery and the SHA-256 of every source trial, rather than claiming the original
collector exited successfully.

The current collector retains only filenames after each synced trial. It writes
a small journal-v2 index, syncs it, then publishes the completed name with a
non-overwriting hard link. The summarizer supports both historical embedded
aggregates and compact journals, loading one trial at a time. This fixes evidence
collection memory growth without changing neural execution or candidate weights.
The original frozen scientific executables remain unchanged.

## Separate world-rule correction and causal controls

A joint audit found a genuine host-rule inconsistency: Eat's automatic hand
preparation rejected an unheld mobile object based on its hidden hazard_pain
value, while ordinary Grab could acquire the same object and held Eat then
consumed it with real consequences. This was a legacy safe-acquisition policy,
not a geometric impossibility or a learned aversion. The controlling architecture
requires grounded cues and physical outcomes rather than hidden semantic value
preferences. [World fix PR297](https://github.com/BobertNeek/A-Life/pull/297) removes
the two hidden-hazard predicates, retaining liveness, ownership, effector,
reach/occlusion, grip clearance, real taste, nutrition and injury. The shared
preparation path also serves mobile Play/Ball acquisition; that scope is explicit
and covered by world unit tests, although the matrix below tests Food only.

This correction is separate from the brain patch and from every earlier receipt.
The [frozen assay source](frozen/whole-v6-world-factor.rs.txt),
[build bindings](world-factor-build-bindings.json) and
[32-trial factor summary](world-factor-summary.json) record the exact old/new
worlds and unchanged revision5/revision6 candidates. The new-world executable
was built from this PR's f8f96ce base plus the source-only world patch; it did not
borrow an executable from an unverified checkout. Candidate asset bytes match
across all16 old/new condition pairs. All8 harmless complete trial pairs are
byte-identical, not merely similar aggregate counts.

The matrix uses two new seeds2026111201–2026111202, both brain revisions, old/new
world, newborn/600-world-tick-prepared juvenile biology,12 neural steps and
reachable Food with identical positive chemistry+.85, nutrition.6 and geometry.
Only the harmful condition sets hazard_pain=.3. Per four-trial biology context:

| Context | Old-world consumed | Corrected-world consumed |
|---|---:|---:|
| Newborn, harmless | 4/4 | 4/4 |
| Newborn, harmful | 1/4 | 4/4 |
| Prepared juvenile, harmless | 4/4 | 4/4 |
| Prepared juvenile, harmful | 1/4 | 4/4 |

The old exceptions use Contact/Grab before Eat and must not be omitted. Corrected
harmful ingestion causes actual injury approximately0.081 and a measured pain
delta0.24, rather than treating poison as harmless nutrition. Final pain can
return to zero in the prepared condition; trajectory pain is logged separately
from endpoint pain. Founder Adam remains unchanged and exact cold-next-state
checks pass for every trial.

**This is oracle-removal and honest-consequence evidence, not improved poison
avoidance.** Positive-tasting harmful food supplies no negative-taste warning in
this assay. A creature must learn or otherwise develop aversion from grounded
cues and experience; blocking it with a hidden value gate would fake that
ability. None of these trials establishes such learning.

The earlier contact-stage neural failure and its two-weight repair remain real
under the original world. Further review separately found that rendered/authored
obstacle radius and the world minimum solid radius are not obviously normalized;
that geometry question is not resolved by these brain or hazard results. No
geometry relaxation or new detour affordance is included here.

Journal durability claims are limited to completed process-failure recovery.
Individual files are synced, but no power-loss or storage-device failure test was
performed and directory fsync is not claimed. Current journals declare seed count,
scenes and arms; the summarizer rejects duplicate files/identities, missing matrix
members and paths outside the receipt directory. Historical embedded receipts
lack some declarations and receive balanced-observed-matrix checks instead.
