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
