# Fixed motion-coverage follow-up

This bounded follow-up preserves the completed residual branch at
`c26628387014eca974505b00ec47eb0d6b309ec1`. It changes collection coverage,
not the neural architecture, objective, scale floor or learned-state semantics.

The prespecified collection has six flat-terrain scenes and 18 independent
worlds: six train, six validation and six test worlds, each for 32 ticks. Public
terrain admission makes locomotion operate in the actual X/Z ground plane.
Food bearings span -150, -90, -30, 30, 90 and 150 degrees in training; validation
and test offsets are fixed at +0.17 and -0.23 radians. Distances are 3/4/5 units,
with fixed heldout distance offsets. Balls, obstacles and hazards also use fixed
initial placements. All preparation finishes before native perception/action.
There are no trajectory-dependent scene changes or fabricated motion targets.

Every listed seed, action outcome and failure is retained. Target windows need
real adjacent future observations; gaps and missing terminal observations are
counted by the existing conversion route and never filled. Training-only delta
RMS scaling, floor 0.001, zero-output persistence initialization, 16 epochs,
head/native learning rates 0.03/0.0001 and the full/shuffled-action/recurrent
ablation controls remain fixed. Forecasts are at one and four ticks, under the
same native collection temperature 4. Per-channel raw errors and the fitted
scales are retained; channel scaled MSE is raw MSE divided by squared scale.

```sh
cargo run --locked -p alife_training --features cpu-backend --bin run_residual_predictive -- \
  --coverage-followup --output NEW_DIRECTORY --seed 71 --ticks 32 --temperature 4 \
  --horizons 1,4 --epochs 16 --head-lr 0.03 --native-lr 0.0001 --scale-floor 0.001
```

World seeds are `71 + 10000 + 10 * scene + {1,2,3}`, where `scene` is 0..5
and offsets select train/validation/test. Sampling seeds are independently
fixed per split. The source remains the same procedural untrained founder.
No seed or completed trajectory is selected based on its prediction score.

The runner records total elapsed time for capture, conversion, training,
evaluation and local state publication, excluding Cargo build. A separate
monotonic subprocess timer checks the full executable duration. This CPU
development-build experiment does not establish behavioral learning, useful
capability per total hour, or GPU/training throughput gains. Trained states and
raw trajectories stay local and recoverable. No PC, GPU or `replay.rs` work is
part of this follow-up.

## Recorded CPU result

One fixed run used source commit
`7c21d956afe0e1e07a591691db6dcccaeff0810b`, with the command above, on
2026-11-03. It retained all 576 native decisions: 407 successful and 169 failed
world receipts, with 12 command primitive types. Each split has six independent
worlds. One-tick targets yield 186 pairs per split; four-tick targets yield 168.
Terminal observations unavailable after the final recorded decision are excluded
uniformly. No seed, completed trajectory or channel was discarded based on loss.

External monotonic wall time was **837.398 seconds (13m 57.4s)**, with exit code
0; the runner's internal timer reported 837.362 seconds. This includes capture,
conversion, training all six arms, evaluation and local recovery-state output.
The preceding incremental CLI build took 11.51 seconds and is excluded. This is
an unoptimized CPU development build, not a throughput benchmark.

Final held-out scores average all 77 observation channels. Raw means errors in
the existing observation representation; position/velocity already use the
recorder's fixed signed-unit encoding. Scaled means squared errors divided by
each channel's training-only squared delta-RMS scale, floored at 0.001.

| Horizon | Arm | Test raw MSE | Test scaled MSE |
| --- | --- | ---: | ---: |
| 1 tick | Persistence | 0.0002318543880 | 0.0824980773 |
| 1 tick | Full | 0.0002129651238 | 0.0713292729 |
| 1 tick | Shuffled action | 0.0002235796234 | 0.0789900304 |
| 1 tick | No recurrent features | 0.0002132184505 | 0.0716049080 |
| 4 ticks | Persistence | 0.0010170242355 | 0.0982970623 |
| 4 ticks | Full | 0.0008653496919 | 0.0800452031 |
| 4 ticks | Shuffled action | 0.0008872380015 | 0.0831891140 |
| 4 ticks | No recurrent features | 0.0008678482442 | 0.0805335643 |

Full raw error is 8.15% and 14.91% below matched persistence; scaled error is
13.54% and 18.57% below persistence. Action shuffling worsens both metrics.
Removing recurrent features gives nearly the same result as full prediction;
these measurements provide little evidence of a useful recurrent contribution.
The full and shuffled arms perform 2,976 native optimizer steps at one tick and
2,688 at four ticks; the no-recurrent arm trains only the auxiliary head.

Training delta RMS for Z position/velocity is 0.031600/0.050174 at one tick and
0.093002/0.044914 at four ticks. The previous collection's almost constant
training Z position was approximately 2e-9. Below-floor channels now contribute
only 0.63%/8.37% of held-out persistence scaled error, versus 93.0%/96.2% in the
earlier collection. This diagnoses improved coverage, not a direct comparison
of model quality: the world distribution and held-out trajectories changed.

Full scaled train/validation curves retain all fixed evaluation points:

| Horizon / split | Epoch 0 | Epoch 1 | Epoch 4 | Epoch 16 |
| --- | ---: | ---: | ---: | ---: |
| 1 tick / train | 0.091417 | 0.089667 | 0.086948 | 0.079078 |
| 1 tick / validation | 0.121729 | 0.119670 | 0.116335 | 0.107842 |
| 4 ticks / train | 0.098874 | 0.091915 | 0.082304 | 0.078870 |
| 4 ticks / validation | 0.170643 | 0.164028 | 0.155287 | 0.152187 |

The one-tick curve still descends; the four-tick curve flattens while validation
remains harder than training. This single bounded run does not identify an
optimal epoch budget. Test scores were recorded after the fixed 16 epochs;
there was no test-selected checkpoint or additional hyperparameter search.

The complete [channel table](performance/predictive-motion-coverage-channels.csv)
contains all 462 horizon/split/channel rows, fitted training RMS/scales, and raw
and scaled errors for persistence and every arm. Its scaled values are derived
from each raw channel error and the same fitted scale, not refitted per split.
The runner also saves all per-channel vectors and learning curves in its local
result JSON. Six trained-state artifacts were saved and cold-restored by the
runner; only source and aggregate errors are published. Native disposable PPO
continuation is covered independently by the fine-tuning test documented in
the [residual guide](predictive-residual-trial.md).

No behavioral trial, relearning-time measurement, accepted-brain mutation or
major-CSR-scale run occurred. The useful finding is a measurable prediction task
with actual action consequences and substantially better Z coverage. A next
behavioral experiment would need disposable native continuation and ordinary
world outcomes; prediction loss alone cannot supply that evidence.

## Validation

At the measured source, `cargo test --locked -p alife_training --features
cpu-backend --lib predictive -- --nocapture` passed 14 tests, with two explicit
fixture-producing/benchmark tests ignored. This includes actual public-world
motion capture, training-only scaling, split isolation, recurrent gradients,
recoverable optimizer state and one real disposable native PPO update.

```sh
cargo test --locked -p alife_training --features cpu-backend \
  --test cpu_backend --test backend_handoff --test checkpoint_validation
cargo clippy --locked -p alife_training --features cpu-backend --lib --tests \
  --bin run_residual_predictive --bin train_predictive_pilot \
  --bin capture_predictive_world -- -D warnings
cargo fmt --all -- --check
cargo check --locked -p alife_training --no-default-features
```

The regression command passed all 23 tests. Clippy and formatting passed.
All 462 CSV rows were checked for unique complete coverage, and averaging their
raw/scaled values reproduces all 48 aggregate horizon/split/arm metrics to
relative tolerance 1e-12. Hosted CI is separate from these local CPU checks.
