# Action-conditioned predictive pilot

This is an opt-in, disposable native CPU experiment. A small tanh interaction
head predicts the next observation from causal recurrent activations, the
current sensor/body observation, one-hot categorical motor choices and declared
continuous motor parameters. The objective is mean squared error over scored ticks and observation
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
test split, when supplied, is evaluated once at the end. It also reports the
current-observation persistence baseline on the same scored pairs. By default
it writes no files. `--output NEW_STATE.json` explicitly saves a checksummed,
self-contained **experimental** training state containing the original native
source, its identity, topology binding, weights, both Adam moments, exact integer
ages, mask, auxiliary head, dataset/config identities and parent-state identity.
Split membership is retained across resumed datasets by collection group,
exact example contents and native source-patch identity, so known evaluation
examples cannot silently migrate into training on continuation. This is still
not a general near-duplicate detector or a provenance signature.
The head stays absent from the foundation codec and gameplay saves. Existing
native inference/checkpoint schemas stay unchanged; the separate experimental
envelope is not an accepted lineage or a promotion receipt.

## Public simulation capture and disposable continuation

No suitable historical recording was present in this cloud workspace. The
included golden traces are summaries, the sensory fixture is one frame, and the
sample replay journal has zero events. The new recorder instead steps the actual
public canonical CPU simulator with a procedural untrained founder:

```sh
cargo run --locked -p alife_training --features cpu-backend --bin capture_predictive_world -- \
  --output /tmp/public-predictive-71 --seed 71 --ticks 12
cargo run --locked -p alife_training --features cpu-backend --bin train_predictive_pilot -- \
  --handoff /tmp/public-predictive-71/initial-handoff.json \
  --dataset /tmp/public-predictive-71/dataset.json --seed 71 --epochs 4 \
  --output /tmp/public-predictive-state-1.json
cargo run --locked -p alife_training --features cpu-backend --bin train_predictive_pilot -- \
  --resume /tmp/public-predictive-state-1.json \
  --dataset /tmp/public-predictive-71/dataset.json --seed 71 --epochs 1 \
  --output /tmp/public-predictive-state-2.json
```

The three worlds have separate world/sampling seeds and whole-collection
train/validation/test assignments. Capture never updates the founder optimizer.
`recording.json` retains complete native decisions, commands submitted to world
execution, per-channel physical outcomes and failures. The conditioning is the
submitted motor bundle; success/failure receipts are retained for audit and are
not inputs. No successful physical execution is invented when a command fails.
`dataset.json` uses only adjacent pairs whose source tick plus one equals both
the actual outcome tick and the next pre-action observation tick. Gaps and
missing terminal observations are excluded and counted, never synthesized.

Every converted example has its actual native initial state and one replay
tick. Its full microstep recurrence is differentiable; cross-decision private
state evolution is detached. Conversion replays the native tick and checks its
activations against the recorded result. Observation names/scaling and primitive,
speech-token and coordination vocabularies are fixed before recording; entity
categories come from the initial public world inventory. All five standard
motor channels, continuous parameters, target presence/position/entity,
coordination and native speech payloads are represented. Unknown primitive,
species-specific channel, entity or payload schema is rejected. Raw motor
values remain in the recording; numeric inputs use fixed f32 signed-unit scaling.
The recorder currently supports the public grounded-object-slots profile only.

`capture_predictive_world --recording EXISTING_RECORDING.json --output NEW_DIR`
revalidates and converts a saved capture. It does not silently accept unrelated
archives or single-frame fixtures. All output paths must be new. Experimental
state publication writes/syncs complete pending bytes before linking the final
new file; a complete `.NAME.pending` file can also be passed to `--resume`.
Corrupt/truncated or incompatible state is rejected. No original source,
previous state, accepted brain or recovery artifact is overwritten.

For native fine-tuning without the auxiliary head, the supported library route
is `ExperimentalPredictiveState::read(path)?.into_disposable_founder()?`.
It exports the current genetic weights with the existing foundation codec,
recompiles/rebinds to the actual new source asset, and retains both moments,
integer ages, optimizer step and mask. It returns a disposable
`CpuFoundationTrainer` and matching compiler inputs for a fresh experimental
cohort/native learner. It does not forge a campaign receipt, reset an optimizer,
restore a historical individual or promote the experimental founder.

Run matched controls from the same initial handoff/dataset/seed and epoch count
with `--inputs full`, `--inputs no_action` and `--inputs no_recurrent`. The latter
uses zero recurrent features and freezes native genetic weights, training only
the matched auxiliary head. `no_action` zeros both categorical and continuous
motor inputs. These are prediction controls, not behavioral evaluations.

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
  indices become one-hot vectors, with no ordinal geometry. Optional
  `schema.continuous_motors` names every numeric motor coordinate; when present,
  each episode's `motor_values` contains N finite vectors in that exact order.
- Each episode has a unique `episode_id`, `split_group`, `split` (`train`,
  `validation`, or `test`), an ordinary native `TrainingSequence` in `replay`,
  N action bundles and N+1 sensor/body observations for N replay ticks. Row t
  describes the current observation/action before its successor at t+1.
  The last observation is target only; no target enters the predictor input.
  Replay inputs and private state must be captured from the same causal events.
  The recorder's optional `evidence` binds group/organism/sequence/frame,
  actual outcome/successor ticks and full-patch/motor-bundle hashes.
- Split groups identify complete lives/collection namespaces. Every window
  from one group must remain in one split. Train and validation are required.
  The validator rejects group overlap, duplicate episode IDs, exact copied
  episode contents across splits, invalid dimensions, nonfinite values,
  out-of-range motor choices and incompatible native replay topology.

Collection identity and semantic alignment remain the caller's responsibility.
The validator cannot prove that renamed near-duplicate trajectories are
independent, that source descriptions are truthful, or that a replay sensor
vector was captured before the target. The native recorder/converter additionally
validates actual sealed patch contracts and event alignment, but hashes are not
an attestation of an untrusted producer's history. The authored numerical fixture is correctness evidence
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

## Public simulation smoke result

The public recorder ran three fresh worlds with founder seed 71, world seeds
72/73/74, sampling seeds 101/202/303 and 12 world ticks each. It captured 36
actual decisions and produced 33 adjacent pairs (11 per split); three final
decisions supplied bootstrap observations only. No gap pairs were filled or
needed. All converted recurrent activations matched their recorded native
outputs exactly (maximum absolute difference 0.0). The source has 2,048 neurons
and 32,768 synapses, with 24,576 recurrent weight coordinates trainable. These
are weight counts, not the larger intended recurrent-unit scale. The inputs
have 42 sensor lanes, 35 body/drive/hormone lanes, 23 categorical factors and 101
continuous motor coordinates.

All learned arms started from the same frozen untrained source, head seed 71,
16 hidden units, 32 recurrent features, head learning rate 0.03 and four epochs
over the same 11 training pairs. Native AdamW used its source configuration.

| Predictor | Validation MSE (11 pairs) | Test MSE (11 pairs) |
|---|---:|---:|
| Current-observation persistence | 0.0000142802 | 0.0000104614 |
| Full recurrent/state/action pilot | 0.0283340 | 0.0276801 |
| Same pilot, action inputs zeroed | 0.0273038 | 0.0269314 |
| Same head, recurrent inputs zeroed and native weights frozen | 0.0299542 | 0.0292005 |

**The pilot did not beat persistence.** The full arm's validation loss fell from
0.0347018 to 0.0283340, but that does not establish useful prediction. Removing
action inputs scored slightly better in this tiny smoke; there is no positive
action-conditioning or behavioral result. These worlds are short and mostly
stable, with one world per split. The deterministic head/row configuration is
matched, but the ablations naturally change initial predictions. This is not
a statistical comparison or evidence that action inputs are generally harmful.

The full arm made 44 native Adam updates; `no_recurrent` made zero native updates.
Restoring the full experimental state and running one more epoch (44 -> 55
updates) matched five uninterrupted epochs **exactly** for native weights,
moments, integer ages, mask and auxiliary head. Its original source and previous
state files stayed unchanged. The CLI also verified lossless native-founder
rebinding. This qualifies disposable state continuation, not gameplay learning.

Input identities (SHA-256 of the actual canonical JSON files):

- Recording: `9fa22d6f1fe17f3b4e245276b241ff6cb75f82237d3e5fef387bc3ec36f5843a`
- Dataset: `0ca1d59642d04342158ff8a3e43a9616ce84ec668465d57662bd2dfa95fb9d33`
- Initial untrained handoff: `849d7df1f346c057a9ec1b1aad83b9c2218b06724c34e62fb8506f4a173ec1fc`
- Full experimental-state payload after four epochs: `e655db7a06df6eac708aa4b7bcf4c8c19a36975782e388a80810051221684309`

Generated recordings and trained experimental states remain local and are not
repository artifacts. Timings are smoke observations only: the full arm ran
while validation/build work was also active, and the CLI repeatedly validates
and hashes the small dataset. No timing here establishes training throughput.

## Cadence branch compatibility

The predictive adjoint extension and opt-in cadence branch
`0200f370470e69b134cef80efe0f77d18a5efe28` were composed in an isolated detached
checkout, without changing either owner's branch. The merge was clean. Eight
pilot tests, both cadence unit contracts and both public cadence integration
contracts passed there. An additional temporary cross-feature test compared
predictive outputs, native recurrent gradients, Adam state and head updates
with reference versus cadence traversal; all were bit-exact. Timing assays
remained opt-in and were not rerun for this compatibility check.

The adjoint seeds enter the shared reverse pass, so the cadence branch's
`incoming_at` traversal applies to them as well. Experimental state restores a
new trainer in its default reference mode; the cadence choice remains
process-local and must be explicitly selected again after restore. No CSR
indices or runtime execution choice are added to checkpoint identity.
