# Native language connectivity experiment

This follow-up preserves the initial and corrected negative measurements in
[the original pilot](retained-language-pilot.md). It investigates why changing
**eat food / get toy** changed encoded inputs but neither native candidate logits
nor optimizer gradients. It is an explicit disposable CPU experiment, not an
accepted brain migration, a CFF capability qualification or a delivered SLM.

## Measured cause

The public founder uses seed 847501. Two copies of the same native world snapshot
receive different ordinary spatial requests. On the first decision, candidate
features, route flags, effective offsets, structural context, microstep count and
receptor gains remain equal. All 17 compiled routes are enabled. Only encoded
neurons **643, 872 and 875** differ under the original profile.

Breadth-first traversal of the actual compiled recurrent edges proves that none
can reach an ActionCandidate or SpeechPayload source, even before considering
activation thresholds. Neurons 872 and 875 have zero outgoing recurrent edges.
Neuron 643 reaches 33 neurons in association and working context. Across eight
native decisions, the word change affects 264 activation values but zero native
candidate logits and zero gradient coordinates. The native action objective has
14,352 nonzero gradient coordinates, so the update itself is not inactive.

Native candidate decoding adds `activation[source] × candidate_feature × weight`
at ActionCandidate coordinates. Native backward seeds adjoints at those same
source neurons and traverses recurrent edges in reverse. Memory/cognitive
candidate terms use finalized context rather than these neural activations; they
do not supply an unobserved word bypass. These public-founder captures contain
zero additional structural edges. Thus the changed association state cannot
affect this native action loss under the original sparse support.

A detached diagnostic input pulse at the three word neurons also changes zero
logits or gradients. The same pulse at the 16 ingest readout neurons changes 21
logits and 13,228 gradient coordinates under the original profile. These are
interventions on replay copies of real world frames, not manufactured simulation
transitions or evidence of learned behavior. Together with the structural graph,
they distinguish a disconnected word path from a dead decoder, closed route,
sampling-only effect or absence of a native loss.

The fixed 16×16 sparse tiles connect disjoint neuron subsets. Only association
neurons 672–703 can reach any action/speech sources, and that subset does not
reach Approach or Contact. Simply reassigning the three word inputs into that
pool would still leave relevant action paths disconnected. This is a broader
sparse-support issue; this bounded experiment does not repair the whole graph.

## Explicit experimental input profile

`--experimental-word-bridge` selects `TwoTokenReadoutBroadcastV1`. It retains the
original encoder and adds 160 assignments: the same first two heard-token bit
vectors, at gain 0.5, repeat across the existing 128 action-source and 32 speech
source neurons. It contains no dictionary of meanings, desired motor indices,
teacher targets, object identities or reward. The heard inputs still originate
from native spatial perception; the ordinary native decoder and recurrent BPTT
must learn how those raw bits interact with current candidate features.

On the same first native frame, 31 candidate logits now differ between requests
(maximum difference 0.12012434). Across eight frames, 230 logits and all 14,352
previously active gradient coordinates differ. This establishes a usable causal
input to the existing learning objective; it does not establish retention,
meaning or physical competence.

The profile is part of immutable compiler inputs and phenotype identity. Default
`None` emits no additional serialized field or digest bytes. Ordinary phenotype
JSON remains byte-identical to the pre-change capture. Removing the explicit
profile from experimental inputs/phenotypes fails validation; restoring its
private life with an ordinary actor fails. The prior corrected pilot's native
private state, next eight steps and next Adam update also remain exact under
this implementation. Live-policy rebase and ordinary N2048 growth do not accept
this profile. No synapse coordinates, native replay equations or weight-asset
schema change.

A foundation asset alone does **not** record this encoder profile. The full
experimental recovery must retain flagged compiler inputs, actor and private
state. No asset promotion or claim of GPU/runtime compatibility is made.

## What supervision means

The motor loss is existing native marginal imitation on actually available
candidates, including native representative-forced motor channels. It teaches
the assembler's requested command selection, not a successful consequence.
Actual world execution separately determines movement, collision, ownership,
ingestion, biological reward and energy. Those outcomes are recorded, never
filled in from a teacher label.

A positive speech target is permitted only after a genuine preceding successful
physical event and on an eligible silent frame. It supervises native speech
presence, act, token, continuation and stop outputs through the native speech
objective. A separate action label teaches selection of the native Vocalize
candidate. Neither label emits speech in the world. Correct utterances require
the actor to select vocalization and emit the matching words through actual world
execution. Zero positive labels means this protocol supplied no positive
narration teaching, even if it performed speech-head updates for silence.

## Reproduction

```sh
cargo --config profile.test.package.alife_training.opt-level=2 \
  test -p alife_game_app --features cpu-training --lib \
  foundation_training::retained_language::tests
ALIFE_LANGUAGE_AUDIT_DIR=/absolute/new/legacy-capture \
  cargo --config profile.test.package.alife_training.opt-level=2 \
  test -p alife_game_app --features cpu-training --lib \
  capture_native_language_causal_source -- --ignored
ALIFE_LANGUAGE_AUDIT_DIR=/absolute/new/experimental-capture \
  ALIFE_LANGUAGE_AUDIT_BRIDGE=1 \
  cargo --config profile.test.package.alife_training.opt-level=2 \
  test -p alife_game_app --features cpu-training --lib \
  capture_native_language_causal_source -- --ignored
python3 scripts/audit_retained_language_connectivity.py \
  /absolute/new/legacy-capture /absolute/new/experimental-capture
cargo --config profile.test.package.alife_training.opt-level=2 \
  build -p alife_game_app --features cpu-training --profile test \
  --bin experiment_retained_language
target/debug/experiment_retained_language --disposable-public-founder \
  --experimental-word-bridge --output /absolute/new/trial \
  --seed 847501 --rounds 2 --epochs 32 --decisions 24
```

The prospective comparison fixes 64 updates per arm (twice the previous dose):
grounded, counterbalanced shuffled and **equally trained no-speech**. It uses
the scripted bounded translator; no real learned-model delivery is verified.
Teacher and prior are absent in assessment. Player, silent and swapped cues,
relocated near/far positions and visible distractors remain separate conditions.
All trained assessments execute after fresh-process restore. Their complete native
trace hashes and metrics must match in-process restored assessments. Exact
private continuation and next Adam state are checked separately.

The supported `--llamacpp-translation` delivery check failed before any lesson:
`local llama.cpp unavailable at 127.0.0.1:18081: Connection refused (os error 111)`.
No translated receipt, model output or model identity was obtained and no fallback
ran. The scripted comparison is therefore not evidence of learned SLM delivery.

The simulation decisions use actual native recurrent dynamics. The offline loss
still conditions on recorded fast/lifetime offsets and context; changing weights
does not regenerate past biology. Learned founder weights are assessed in fresh
cohorts with exact optimizer continuity across export. This is not the same
individual learning online, and no critic continuity is claimed. Natural sleep
retention and compositional verb/noun generalization remain untested.

The shuffled arm is the matched speech-content control: it retains the ordinary
teacher actor and delivery schedule. The equally trained no-speech arm omits that
actor as well as its speech, so it also changes the visible social context. It is
a secondary control and cannot isolate speech content by itself.

## Completed comparison

[Aggregate measurements and exact provenance](retained-language-bridge-results.json)
record source `cb341b05681eb52f799d9128d2015012956ce942`, seed 847501,
743.586 seconds internal / 743.599 seconds external, 64 updates per arm and
2,016 training plus scored assessment decisions. CPU compilation/regression work
ran concurrently; the elapsed time is not a performance benchmark. Assessment
was repeated for exact fresh-process comparison; those repeated decisions and
diagnostic probes are additional to the 2,016 scored decisions.

Each arm has 288 scored assessment decisions across 12 lives. Every initial
assessment frame contains visible food, toy and distractor. All scores below
come from the **fresh child process**, with no teacher actor/hearing, no semantic
prior, no forced actions and no post-reload optimizer update before assessment.

| Arm | Nourishing meals | Toy acquisitions | Goal-matching physical events | Utterances | Positive training speech labels | Joint failures |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Grounded | 5 | 2 | 5 | 0 | 0 | 52 |
| Shuffled | 5 | 0 | 3 | 0 | 0 | 61 |
| Equally trained no-speech | 3 | 6 | 3 | 0 | 1 | 41 |

Goal-matching events mean nourishing consumption in an EatFood life or toy
acquisition in a GetToy life. Other physical events remain reported rather than
discarded. In particular, the cue-swapped rows retain the original life goal;
their goal-matching count is **not** a request-following score.

All physical successes occur in the near layouts; all far lives have zero meals
and acquisitions. The grounded near EatFood life eats under player, silent and
swapped cues. Its near GetToy life acquires the toy under the requested phrase
and silence, but not the opposite phrase; the opposite phrase instead accompanies
food consumption. The shuffled arm acquires no toys. The no-speech arm acquires
toys across all three cue conditions, including EatFood lives. There is limited
near-scene cue sensitivity, but this single-seed comparison does **not** establish
reliable retained object/action language or successful navigation generalization.
The increased update dose also prevents attributing physical improvement over
the older 32-update negative directly to the new encoder.

Grounded/shuffled training hears 96 teacher frames each, with 223/200 actionable
label rows across the two rounds, but supplies zero positive narration labels.
No-speech has 206 actionable rows and one genuine positive narration label.
Assessment has 5/3/3 eligible narration opportunities respectively, and zero
utterances/correct utterances in every arm. The new path therefore does not
demonstrate speech production. A next teaching experiment needs enough genuine
completed-action experience during collection to supervise positive narration;
manufacturing an outcome or counting silence updates as language learning would
not address that failure.

The three final actor hashes differ, unlike the original disconnected protocol.
The final native word probe changes 248/236/242 logits and 12,208 trainable gradient
coordinates in grounded/shuffled/no-speech respectively. Again, these probes are
causal diagnostics, not the behavioral success criterion.

All three fresh-process checks match exact private state, optimizer state,
eight-step continuation, next Adam update and complete scored assessment traces.
Original corrected-pilot recovery also remains exact under this implementation.
No natural sleep or sleep consolidation occurs in assessment, and no natural
sleep retention claim is made.

A final recovery I/O fix at source
`56f651c9982e41a3c85be53ceab43d454a5b2378` permits repeat cold reload without
overwriting the existing assessment trace. It regenerates the assessment and
verifies the saved trace's native-content hash. A real repeat of the grounded
recovery completed in 35.631 seconds: all 288 assessment decisions/metrics,
private continuation and next Adam state matched the original result exactly;
the recovery JSON, checksum and compressed trace stayed byte-identical. This
additional verification does not add training or change the scored outcomes.

Validation: 270 normal tests passed (5 experiment, 68 compiler/identity/ABI,
158 core/training library, 39 native CPU backend/collector), plus both explicitly
invoked diagnostic captures. Strict Clippy and formatting checks passed. Hosted
CI is reported separately; local tests do not establish its status.
