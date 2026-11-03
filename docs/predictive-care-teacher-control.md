# Controlled native teacher repair of the disposable care assay

This separate fixed experiment preserves the frozen-decoder negatives at
`385815d2` and `4a6045b8`. It starts fresh copies of the same original untrained
source and one-tick full predictive founder, with no accepted lineage changes.

The explicit downstream trainable mask contains native recurrent synapses and
ActionCandidate decoder coordinates. Memory and speech decoder coordinates
remain frozen. Existing weights, Adam moments and integer ages are retained;
newly unfrozen decoder ages start at their retained value of zero. Both arms use
the same mask and Adam settings, with an explicitly chosen learning rate 0.003.

Teaching uses the **existing grounded held/reachable care demonstrator's Eat
candidate selector**, shared with normal demonstration training. Its label must
refer to the actual current food-target candidate in ordinary perception. It is
offline supervision, never injected into the frame, native sampling, rewards or
physical outcomes. Labels stop when consumed food no longer has a candidate.
The native CPU imitation objective supplies logit derivatives to native recurrent
BPTT and Adam; there is no substitute gradient. The object-slot profile rejects
the semantic prior and lacks direct word ports, so no prior/word assistance is
claimed and no profile guard is relaxed. Ordinary visual/contact/body inputs
remain the selected profile's real observation signal.

The one predeclared comparison has two native care lessons: EatHeldFood at seed
41001 and EatReachableFood at 41002. Each provides 64 actual learner decisions
plus one bootstrap, temperature 1. Each actual recorded lesson receives 16 fixed
offline teacher epochs, one Adam commit per epoch, using native 16-tick replay
windows. Supervised loss is averaged over actual label-bearing rows; no label
or outcome is invented for unavailable/consumed food. This is a deliberately
teacher-supervised control, not a matched-dose rerun of the earlier PPO negative.
Both arms receive the same exposure/update budgets; effective label counts and
actual native outcomes are reported, including any differences after learning.
Critics are preserved unchanged because this phase uses imitation, not PPO.

Before and after both lessons, fresh unassisted native lives evaluate held food
at 51001 and reachable food at 51002, 64 decisions each. Neither teacher nor
semantic prior operates during evaluation. No selection is forced in learner
collection or evaluation. All full motor bundles, food/physiology events,
failures and bootstrap observations are retained. The nominal run has 780 actual
learner decisions and 32 actor updates per arm; there is no sweep or promotion.

The comparison is gated on direct native-world positive controls using restored
**copies** of the earlier held/reachable worlds: legal manual Eat commands must
really consume food and increase energy. These controls are never called learned
behavior. Separate disposable actor-copy checks must show nonzero Ingest decoder
gradients and actual targeted coordinate updates, with preserved ages and frozen
state. No real comparison learner is updated by those checks. Failure stops the
run before normal comparison collection; recovery remains local.

```sh
cargo build --locked -p alife_game_app --features cpu-training \
  --bin run_predictive_care_teacher_control
target/cloud-cpu/debug/run_predictive_care_teacher_control \
  --state LOCAL_MOTION_RUN/state-h1-full.json \
  --upstream-result LOCAL_MOTION_RUN/result.json \
  --reference-care-run LOCAL_FROZEN_LONG_CARE_RUN \
  --output NEW_CONTROL_DIRECTORY
```

`--check-only` runs just the native positive/copy-update gates into a fresh
output. All outputs are disposable, source-bound local recovery files; they are
not normal care-lineage receipts, inference saves or accepted checkpoints. No
PC/GPU, `replay.rs`, inferred meals or denied-PR retry is part of this work.
Report this run's entire control/capture/training/evaluation/export wall time,
the previous cumulative 1,292.091 seconds of experiments, and build/check overhead
separately. Prediction or teacher-loss changes do not prove behavioral gain.

Actor-only imitation recovery is deliberately separate from a joint PPO handoff:
the critic retains its actual source-policy binding and is never marked updated
under the learned actor. The checksummed experimental envelope binds the original
source, unchanged genome/development/profile, current compiler inputs and actual
exported native weights, full actor Adam state and unchanged source critic.
`restore_disposable_teacher_actor` cold-restores these through native compiler,
asset and checkpoint validation. Each epoch also retains its native actor
checkpoint and update receipt; completed lesson envelopes are self-contained.

The first attempt at source `19fec9f1` passed direct-world and decoder-update
checks, captured both baseline assessments and its first real lesson, then
stopped at the ordinary joint-handoff guard: the actor-only update had not updated
the critic's policy binding. That guard remains intact. Its 74.018 seconds and
all captured data remain retained. A deterministic replay of the retained first
lesson confirmed all 16 native teacher updates and native rebind worked; only
the attempted joint PPO packaging was wrong. The corrected envelope does not
forge a new critic binding. No hyperparameter or world was selected or changed
in response to that failure, and both copy-check gates now include cold restore.

## Fixed CPU result

The corrected run used source `eeb19ebd3529dc510dee20fa671c6f3d72198127`.
All 780 actual learner decisions completed: 768 scored/training rows and 12
bootstrap observations. Both arms started at zero meals in both teacher-free
assessments, and both recorded **two real nourished meals per case** after
teaching. These are actual neural selections, not direct-control commands.

| Final unassisted case / arm | Acquisitions | Consumptions / meals | Failed actions | Bundled food-target Ingest commands | First meal, decision (1-based) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Held food / source | 1 | 2 / 2 | 0 | 3 | 3 |
| Held food / pretrained | 1 | 2 / 2 | 0 | 3 | 1 |
| Reachable food / source | 2 | 2 / 2 | 0 | 4 | 4 |
| Reachable food / pretrained | 2 | 2 / 2 | 0 | 4 | 3 |

The ordinary world resource-regrowth cycle permits a second meal during the
64-decision life. Distinct native consumption/ownership transitions and measured
energy increases accompany both events; setup possession is not counted as
acquisition. For example, the source held-food evaluation consumed at scored
decisions 3 and 52, with energy rising approximately 0.792 to 0.911 and 0.903 to
0.955. Biology and resource rules were not changed.

Mean food-target Ingest probabilities rose from 7.87e-6/2.41e-5 in the initial
held/reachable assessments to 0.585/0.651 for the source arm and 0.703/0.834 for
the pretrained arm. These means are conditional on the candidate being present:
final held-food availability was five/three rows for source/pretrained and
reachable availability six/five. They are not an identical-input probability
comparison. Exact native replay logits matched collected logits with zero error.
The original one independent Locomotion head remains; representative Ingest
selection now legitimately forces the native Manipulation channel into full
motor bundles. Evaluation contained no teacher cues or prior assistance.

Training failures remain visible: each reachable-food collection made **58
failed Ingest attempts at other targets** despite acquiring/nourishing from the
lesson food. The source training life recorded one meal; pretrained recorded
two. Across all scored/training rows there were 116 failures, plus two failed
bootstraps, for 118 failures among all 780 decisions. Final evaluation had none.
This is a bounded teaching control, not proof of robust care or generalization.

Each actor received 32 native Adam updates: source 0 to 32 and pretrained 2,976
to 3,008. The shared mask has 24,576 recurrent and 3,072 action-decoder
coordinates, with the remaining 5,120 coordinates frozen. All moments, integer
ages and frozen bits were verified for every update. Each preflight copy check
changed 384 Ingest decoder coordinates with nonzero native recurrent/BPTT
learning; manual native controls separately consumed and raised energy. All six
initial/per-lesson actor-only envelopes cold-restored through native validation.
Both critics remain unchanged at optimizer step 0 and their honest source-policy
binding 1, rather than claiming a joint PPO update.

On the actual first teacher update, 10,496 previously zero recurrent first
moments became nonzero in each arm. This distinguishes genuine recurrent
gradient flow from weights changing only through AdamW decay.

Both arms received 128 actual training decisions and 16 fixed passes per lesson.
Food availability made actual label-bearing rows differ: 64+17 for source and
64+6 for pretrained, giving 1,296/1,120 effective label exposures. Missing-food
rows were never assigned made-up labels. The endpoint food outcomes match.
Slightly earlier first meals, lower teacher loss or different label availability
in these two worlds do not establish a pretraining advantage; retained Adam
histories also differ. This repairs assay sensitivity and demonstrates native
teacher learning. It does not justify promotion, a training speedup claim or a
major-CSR/GPU conclusion.

The complete [12 episode rows](performance/predictive-care-teacher-control-behaviors.csv)
and [64 update rows](performance/predictive-care-teacher-control-updates.csv)
retain all conditions, failures, actual full-bundle Ingest counts, food events,
probability denominators, physiology, teacher losses and targeted updates.

### Cost and retained failure

The corrected executable took **246.026 seconds**, including source loading,
native positive controls, copy-update/cold-restore checks, all collection,
teaching, evaluation and local recovery output. The internal timer reported
246.011 seconds. The retained packaging-failure prefix took 74.018 seconds.
Earlier prediction and frozen-decoder comparisons took 1,292.091 seconds.
These timed experimental executables sum to **1,612.135 seconds (26m52.1s)**.

A single deterministic replay of the retained failed lesson was used to isolate
the packaging error; it was not separately timed. Its saved final native weights,
moments and ages match the corrected first lesson exactly. To disclose that work
and avoid presenting the timed-program sum as an all-inclusive cost, the full
filesystem-clock span from the first attempt's log creation through publication
of the corrected result was **980.518 seconds**, including the diagnostic replay,
intermediate rebuilds, engineering and waiting. Combined with earlier timed
experiments this conservative span is **2,272.609 seconds (37m52.6s)**, before
later validation and before builds that preceded the first attempt. Reported
teacher-control builds totaled 103.99 seconds; earlier reported builds totaled
102 seconds. Some rebuilds are already inside the span and must not be added
twice. This is experimental cost accounting, not a throughput benchmark.

### Reproduction and validation

```sh
python scripts/summarize_predictive_care_teacher_control.py \
  --input LOCAL_CONTROL_RUN --output-prefix NEW_PREFIX
target/cloud-cpu/debug/run_predictive_care_teacher_control \
  --validate-recovery LOCAL_CONTROL_RUN/pretrained/after-lesson-1-actor-only-recovery.json
cargo clippy --locked -p alife_game_app --features cpu-training --lib \
  --bin run_predictive_care_teacher_control -- -D warnings
cargo fmt --all -- --check
cargo check --locked -p alife_game_app --no-default-features
```

The real native run, direct controls, targeted decoder gates, all per-coordinate
update/frozen-state checks, strict Clippy, formatting and feature-disabled build
passed. Both final recovery files validated through the CLI, preserving actor
steps 32/3,008 and unchanged critic state. A corrupted critic-policy binding was
rejected with the original file untouched. The earlier source/predictive states
and negative results remained byte-identical. Only source and aggregate numeric
evidence are published; trained states remain local and recoverable.
