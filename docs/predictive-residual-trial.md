# Residual prediction trial

This experiment lives on `codex/predictive-residual-trial`. The original pilot
remains recoverable, unchanged, on `codex/action-predictive-pilot` at
`3fef0dc957e54f6b54cf8b3761786dbd98c73dbf`. No accepted creature or trained
artifact is published or migrated.

The prediction is `current_observation + scale * predicted_delta`. Output
weights and biases start at zero, so every arm begins at exactly the
current-observation persistence baseline. The same small 16-unit tanh head and
32 selected recurrent features are retained; the architecture is not expanded.
Native recurrent gradients still enter the existing bounded reverse pass and
full AdamW. The head uses SGD.

Each channel's scale is its RMS observed delta on the **training split only**,
floored at 0.001. Scales are fitted independently for each horizon and retained
in the experimental state. No target mean is subtracted: zero model output
means zero change, not an estimated drift. Both raw observation MSE and scaled
delta MSE are reported, with per-channel errors and sensor/body aggregates.
Raw and scaled MSE have different units and must not be compared numerically.

The collection uses three public scene preparations, applied through world
editor APIs before any observation/action: nearby food/ball, food with an
obstacle, and nearby hazard/toy. Food distance, bearing and other geometry vary
between whole-world train/validation/test episodes. Native actions are sampled
at temperature 4; physics, biology, contact, failure and successor observations
are actual simulator results. Commands and failures are retained, with no
outcome-based filtering or artificial rewards. This is a fresh procedural
untrained founder, not an accepted or learned creature.

There are nine worlds, with three whole worlds per split and 32 ticks per world
by default. Targets at one and four ticks are actual future pre-action
observations. Windows containing sleep/time gaps are excluded; missing future
states are never padded. Only the initial action is given to the model. Later
actions follow the frozen collection policy, so the four-tick task forecasts
that policy's observed consequences; it is not a controlled open-loop rollout.

The matched arms are full conditioning, a deterministic within-split
derangement of the full action bundle, and zero recurrent inputs with native
genetic weights frozen. Shuffling does not move observations, replay state or
targets, and never brings validation/test actions into training. Every arm
uses the same initial head seed and epoch row order. Training/validation curves
are recorded at epochs 0, 1, 4 and 16, with test scoring after the fixed run.
The configuration is one empirical trial, not an architecture search or proof
of behavioral improvement.

```sh
source /workspace/cloud-cpu-validation/env.sh
cargo run --locked -p alife_training --features cpu-backend --bin run_residual_predictive -- \
  --output /tmp/alife-residual-public-71 --seed 71 --ticks 32 --temperature 4 \
  --horizons 1,4 --epochs 16 --head-lr 0.03 --native-lr 0.0001 --scale-floor 0.001
```

The output directory must be new. It contains compressed raw recordings and
horizon datasets, the initial native source, final checksummed local
experimental states, and numerical results. Reuse collection with
`--recording RECORDINGS.json.zst`. Each state restores weights, moments, integer
ages, mask, head, horizon and fitted scales exactly. The generic pilot CLI reads
compressed datasets for disposable continuation; its `head_objective` field
identifies whether reported head loss is raw or scaled.

The native fine-tuning handoff is independently exercised by:

```sh
cargo test --locked -p alife_training --features cpu-backend --lib \
  exported_predictive_founder_completes_actual_native_disposable_ppo_update -- --nocapture
```

That test exports/rebinds a predictively updated founder, collects five fresh
native decisions, and performs one actual PPO/residual-critic update over four
transitions. Actor steps go 0 -> 1 -> 1 -> 2 (initial, predictive, rebound,
native); the critic reaches step 1. Frozen coordinates, both moments, integer
ages, source/topology binding and original-state immutability are checked. The
native optimizer state contains no auxiliary head. No behavior gain is inferred.

## Measured trial

The fixed configuration above captured 288 actual decisions across nine worlds:
232 successful and 56 failed world receipts, with ten command primitives
represented. Approach remained dominant (285 channel commands); the record also
contains 97 Inspect, 12 Eat, 6 Play, 1 Grab and 1 Vocalize commands, plus Idle,
Rest, Flee and no-locomotion. Counts are per motor channel and can exceed the
number of joint decisions. All failures were retained.

Each one-tick split has 93 pairs; each four-tick split has 84. Every learned arm
began at exactly the corresponding persistence score. Final test results:

| Forecast | Predictor | Raw observation MSE | Train-scaled delta MSE |
|---|---|---:|---:|
| 1 tick | Persistence | 0.000271738 | 1.520123 |
| 1 tick | Full residual | **0.000268528** | **1.513998** |
| 1 tick | Actions shuffled within split | 0.000271299 | 1.516510 |
| 1 tick | Recurrent features removed, native weights frozen | 0.000268738 | 1.514172 |
| 4 ticks | Persistence | **0.000930582** | 3.124525 |
| 4 ticks | Full residual | 0.000937727 | **3.115940** |
| 4 ticks | Actions shuffled within split | 0.000940482 | 3.116249 |
| 4 ticks | Recurrent features removed, native weights frozen | 0.000938538 | 3.116015 |

The full one-tick model reduces raw test MSE by **1.18%** versus persistence.
The recurrent ablation reduces it by 1.10%, so this trial establishes little
incremental benefit from recurrent training. The full four-tick model's raw MSE
is **0.77% worse** than persistence even though scaled loss improves slightly;
that is not an across-the-board prediction improvement. No significance or
behavioral benefit is established by these correlated, small episode sets.

For diagnosis, a separate baseline fits only the mean training delta per
channel, then predicts `current + mean_delta`. Its test raw/scaled errors are
0.000271345 / 1.516572 at one tick and 0.000942010 / 3.116585 at four ticks.
The four-tick trained models are consequently close to learning average drift.

Full-model train/validation scaled loss at epochs 0, 1, 4 and 16:

| Forecast | Epoch 0 | Epoch 1 | Epoch 4 | Epoch 16 |
|---|---:|---:|---:|---:|
| 1 tick, train | 0.091423 | 0.089608 | 0.085615 | 0.080278 |
| 1 tick, validation | 0.559821 | 0.557908 | 0.553700 | 0.549086 |
| 4 ticks, train | 0.099134 | 0.093678 | 0.079067 | 0.073951 |
| 4 ticks, validation | 0.865198 | 0.860155 | 0.847116 | 0.843542 |

Training is still reducing loss, but the much larger heldout gap is also a
coverage problem. Only seven of 77 channels have training delta RMS above the
predeclared 0.001 floor. Training z-position/z-velocity changes are around
2e-9, while test RMS changes are approximately 0.0060/0.0085 at one tick and
0.0123/0.0090 at four ticks. Channels below the training scale floor account
for **93.0% / 96.2%** of scaled test persistence error. Thus the aggregate scaled
score is dominated by motion consequences scarcely observed in training.
The scale floor was not changed after seeing this result. Residual zero-output
initialization removes the earlier absolute-head error on constant channels;
the remaining issue is not simply quiet channels contributing static offsets.
This evidence points to consequence coverage and optimization diagnostics
before adding architecture. It does not prove that more epochs alone will
close the heldout gap.

Full/action-shuffled arms made 1,488 native updates at one tick and 1,344 at
four ticks. Recurrent ablations made zero native updates. Every final local
experimental state passed exact weight/moment/age/head restoration. The first
run recorded training-loop timings but not total end-to-end wall time; those
numbers do not establish useful capability per total hour. The runner now
records end-to-end elapsed time including capture/load, conversion, training,
evaluation and local state publication, excluding Cargo build time. No native
behavior/relearning test was performed for these residual models.

Source identity: `849d7df1f346c057a9ec1b1aad83b9c2218b06724c34e62fb8506f4a173ec1fc`.
Raw-recordings identity: `52f1151c3e9f3befa1a37a62ef6085d7336158d2dd3e87e9f23b069f1557c415`.
One-/four-tick dataset identities:
`775dad8c6dcdd76e42fceb55b5ec6e5b1e4a9cf1988deb77ec54d9b2ec6df4c3` /
`675afe0bcf4e8e09e282bcd1d3a9a8f59c10957c92caac52cd81a2ba5e7fce2b`.
Recordings, datasets and trained states remain local; only source and these
aggregate observations are published.

The saved one-tick full model was also continued through the supported CLI with
its compressed dataset. Native steps advanced **1,488 -> 1,581** with the source,
parent identity, fitted scales and horizon preserved. This post-run continuation
smoke is separate from the fixed 16-epoch comparison above; its extra evaluation
is not used to strengthen the primary prediction claim. Epoch training validates
the immutable dataset once while retaining native/finite-update guards per row.
The original state is never overwritten:

```sh
cargo run --locked -p alife_training --features cpu-backend --bin train_predictive_pilot -- \
  --resume OUTPUT_DIR/state-h1-full.json --dataset OUTPUT_DIR/dataset-h1.json.zst \
  --seed 71 --epochs 1 --head-lr 0.03 --output NEW_STATE.json
```
