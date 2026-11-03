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
