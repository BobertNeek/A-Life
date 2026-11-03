# Action-conditioned predictive pilot

This is an opt-in, disposable native CPU experiment. A small tanh interaction
head predicts the next observation from causal recurrent activations, the
current sensor/body observation, and one-hot choices for every declared motor
channel. The objective is mean squared error over scored ticks and observation
coordinates. It does not predict rewards or claim general intelligence.

The head's activation adjoints enter the existing native sparse recurrent BPTT
implementation. The existing clipped AdamW updates genetic recurrent weights,
including its moments, integer ages, frozen coordinates and sign guards. The
auxiliary head uses SGD. Replay context, including lifetime/fast weights,
structural edges, chemistry and activity decisions, is detached. Burn-in is a
detached boundary. This does not train the evolution of lifetime plasticity.

Run an authored numerical contract trial, with no GPU or recorded world data:

```sh
cargo test --locked -p alife_training --features cpu-backend --lib cpu::predictive:: -- --nocapture
cargo test --locked -p alife_training --features cpu-backend --lib numerical_fixture_trial -- --ignored --nocapture
```

To exercise the CLI with the same authored fixture, generate its initial
untrained handoff and dataset into a new temporary directory, then use those
paths in the command below. These are numerical fixtures, not world recordings:

```sh
ALIFE_PREDICTIVE_FIXTURE_OUTPUT=/tmp/alife-predictive-fixture \
  cargo test --locked -p alife_training --features cpu-backend --lib \
  write_numerical_fixture_inputs -- --ignored --nocapture
```

Run a dataset against an existing native training handoff:

```sh
cargo run --locked -p alife_training --features cpu-backend --bin train_predictive_pilot -- \
  --handoff /path/to/training-handoff.json --dataset /path/to/dataset.json \
  --seed 71 --epochs 4 --head-lr 0.03
```

The command reads both files, validates the handoff and entire dataset, intersects
the source stage mask with recurrent coordinates, trains an in-memory copy, and
prints before/after train and validation MSE plus elapsed CPU training time. A
test split, when supplied, is evaluated once at the end. No files, checkpoints,
accepted baseline, policy lineage or gameplay saves are written. The auxiliary
head has no serialization implementation and is absent from the foundation codec.
Existing inference/checkpoint schemas stay unchanged. This CLI intentionally
does not publish a trained founder or integrate a training objective with PPO.

## Dataset contract

`cpu::predictive::PredictiveDataset` is the JSON schema (unknown fields are
rejected); the numerical test fixture shows a complete construction and tests
JSON round trips. Top-level fields are `schema_version: 1`, a candid
`source_description`, `schema`, and `episodes`.

- `schema.version` identifies the exact observation/action semantics. `sensors`
  and `body` list distinct feature names in fixed order. Each observation is
  their concatenation. Use predeclared physical units/scaling; do not fit
  normalization on validation/test data. Both groups must be present.
- `schema.motors` lists all categorical channels and named `choices`, including
  explicit no-op choices. Each action supplies one choice index per channel;
  indices become one-hot vectors, with no ordinal geometry. Continuous motor
  parameters are not supported by this first pilot; do not quietly omit them
  from evidence for an organism that executes them.
- Each episode has a unique `episode_id`, `split_group`, `split` (`train`,
  `validation`, or `test`), an ordinary native `TrainingSequence` in `replay`,
  N action bundles and N+1 sensor/body observations for N replay ticks. Row t
  describes the current observation/action before its successor at t+1.
  The last observation is target only; no target enters the predictor input.
  Replay inputs and private state must be captured from the same causal events.
- Split groups identify complete lives/collection namespaces. Every window
  from one group must remain in one split. Train and validation are required.
  The validator rejects group overlap, duplicate episode IDs, exact copied
  episode contents across splits, invalid dimensions, nonfinite values,
  out-of-range motor choices and incompatible native replay topology.

Collection identity and semantic alignment remain the caller's responsibility.
The validator cannot prove that renamed near-duplicate trajectories are
independent, that source descriptions are truthful, or that a replay sensor
vector was captured before the target. No world-data adapter or dataset is
invented by this pilot. The authored numerical fixture is correctness evidence
only. There is no measured gameplay gain, throughput improvement, GPU result,
or evidence at the intended 24,576 recurrent / 8,192 decoder-unit scale or its
fan-in sweep. Those require real collection and separate empirical trials.

## Numerical smoke trial

On the cloud x86_64 native CPU (four reported logical workers), the authored
fixture used 2,048 neurons, 32,768 compiled synapses, three ticks with one
detached burn-in tick, 16 selected recurrent features and an eight-unit hidden
head. Twelve episode updates took 0.889 seconds in the test profile. Training
MSE changed from 0.0868884 to 0.00906704; the separately labeled authored
validation episode changed from 0.0877837 to 0.00862835. Each episode has only
two scored ticks and deliberately related authored values. This is an arithmetic
and optimizer smoke trial, not a generalization, creature-learning or speedup
measurement. Reproduce it with `numerical_fixture_trial` above; timings depend
on the build profile and host load.
