# CPU PPO learner cost ledger

Base source: `eaf9e12067d649e9e27c16e6600e6e8dabfc7e99`. The change retains the
forward tape already produced by CPU loss evaluation for that window's BPTT.
Both `train_cpu_ppo_cohort` and `train_cpu_residual_cohort` use it. This is an
opt-in CPU learner result; GPU training and ordinary simulation were not timed.

## Exact before/after call chain

The removed work is a second full forward evaluation and its tape construction,
not merely allocation/storage reuse. `SparseGraph::backward` never calls forward.

Before, in base `eaf9e120`:

1. [`evaluate_window`](https://github.com/BobertNeek/A-Life/blob/eaf9e12067d649e9e27c16e6600e6e8dabfc7e99/crates/alife_training/src/cpu/objective.rs#L421)
   calls `evaluate_replay`, which calls `forward`. `evaluate_replay` moves out
   its evaluation and drops the derivative tape; `evaluate_window` then uses
   the returned logits/features.
2. The update loop calls
   [`probe_replay_gradients_with_speech`](https://github.com/BobertNeek/A-Life/blob/eaf9e12067d649e9e27c16e6600e6e8dabfc7e99/crates/alife_training/src/cpu/mod.rs#L255).
   That public standalone probe calls `forward` again, then calls
   `SparseGraph::backward` on the newly recomputed tape.

After, in initial candidate `0f57ff6f`:

1. [`evaluate_window`](https://github.com/BobertNeek/A-Life/blob/0f57ff6f7eee4856d75b7188c0eec84801a859b2/crates/alife_training/src/cpu/objective.rs#L426)
   calls `forward`, reads the same logits/features, and returns that exact tape.
2. Both the
   [PPO](https://github.com/BobertNeek/A-Life/blob/0f57ff6f7eee4856d75b7188c0eec84801a859b2/crates/alife_training/src/cpu/objective.rs#L536)
   and [residual](https://github.com/BobertNeek/A-Life/blob/0f57ff6f7eee4856d75b7188c0eec84801a859b2/crates/alife_training/src/cpu/cohort.rs#L259)
   update loops call `replay_gradients_from_tape`, which expands detached burn-in
   adjoints and calls `SparseGraph::backward` without another forward.

`load_window` returns an owned window. Neither it nor the trainer is mutated
between forward and backward. No callback runs during this interval. The tape
is local to the inner window iteration and is dropped at its end; actor/value
optimizer application follows the entire effective-batch loop. The next window
and epoch call `evaluate_window` again. There is no persistent tape field, pool
or cache. Public standalone probes retain their independent fresh forward.
Serial continuation and admitted/cooperative collectors reach the residual
cohort path; the generic PPO cohort path is updated separately. Frozen all-window
preflight, KL-stop placement, gradient reduction order, paired Adam proposal/
commit ordering and all checkpoint codecs are unchanged.

## Conditions

Measured on 2026-10-31 in a cloud Linux x86_64 VM reporting AMD EPYC 9V74,
five visible virtual CPUs, a four-CPU cgroup quota and 16 GiB memory limit.
Rust 1.98.1; test profile with `alife_training` opt-level 3, core/world/runtime
opt-level 2 and debug assertions enabled. This is not a release-build measurement.
One benchmark thread; no desktop workload, imported private checkpoint or GPU
was used. A second comparison pinned the process to virtual CPU 2. VM contention
and frequency were not controlled.

The synthetic fixture uses the compiled production N2048 graph: 2,048 neurons,
32,768 synapses, 17 routes, four microsteps, 24,576 trainable coordinates, 40
replay ticks including eight detached burn-in ticks, 16 candidates, six motor
factors, one dendritic branch, private offsets and reduced microstep counts every
seventh tick. Factor metadata is synthetically valid rather than a native
sampling receipt; the six-factor case exercises the objective's supported shape.
There are no structural additions or speech targets in the timing fixture. Synthetic Adam history includes nonzero moments, frozen coordinates and
unequal integer ages including 16,777,217. PPO uses two epochs, temperature 0.7
and a deliberately permissive target KL of 100 to execute the complete workload.
The enclosing cohort has two windows, 64 loss rows and one paired actor/critic
update per epoch. These are numerical rows, not canonical world decisions or
evidence of learned behavior.

## Component boundaries

Initial baseline medians below use one warm-up and 11 samples for graph/window
scopes, 21 for the smaller scopes. These isolated scopes overlap; do not add
all rows together or interpret them as an exclusive whole-run profile.

| Production boundary | Median ms | Why it exists; what is necessary or improvable |
| --- | ---: | --- |
| `TrainingSequence::validate_for` | 0.856 | Shape, identity and finite private replay admission. Keep validation of each newly loaded window; identical immutable data need not be revalidated before the same window's backward pass. |
| `SparseGraph::forward`, including tape creation and destruction | 50.223 | Replays recurrent state, metabolism, routes, private context and decoder logits under current weights. One current forward is necessary. Repeating it immediately for BPTT was redundant. Allocation versus arithmetic time inside this boundary is **unknown**. |
| Tape float storage and retention | **time unknown** | Calculated payload is 4,939,264 bytes for this shape, including evaluation outputs and decoder context. This excludes Vec metadata, allocator overhead and other learner buffers; it is not RSS. Retain one tape only until that window's backward pass/accumulation ends, then drop it before optimizer commit. No cross-update cache is introduced. |
| `SparseGraph::backward` on an existing tape | 45.290 | Full recurrent/metabolic/source and decoder derivatives, detached at burn-in. Its own adjoint/gradient allocations are included. Shortening burn-in or omitting recurrent credit changes the learning problem. |
| `cpu_ppo_objective`, all 32 rows | 0.028 | Exact joint representative/conditional motor probabilities, forced-slot entropy, clipping and KL. Already a small cost here; changing PPO math is unwarranted. |
| `evaluate_window` | 51.080 | Admission, forward, PPO, loss normalization and feature extraction. Includes the forward row above. |
| Public gradient probe, including another forward | 92.055 | Public standalone probes need a fresh forward. Cohort training already has the exact tape, so it now calls backward on that tape instead. Includes backward above. |
| Frozen residual-window preflight | 52.794 | Checks every old probability and captured feature before the first mutation, plus residual-row/critic validation. This pass remains necessary and unchanged. |
| Residual critic gradient | 0.142 | Full Bellman residual through current and actual-next features, including bias. No detached semi-gradient substitution. |
| Actor `apply_gradients` | 1.103 | Full clipped AdamW, per-coordinate ages/moments, sign constraints and atomic proposal validation. Required state remains intact. Repeated beta powers are a possible optimization, but its enclosing benefit is small in this workload. |
| Critic Adam proposal | 0.012 | Full paired critic Adam. Includes the benchmark's small input-gradient allocation. |
| Owned replay-window clone | 0.165 | In-memory loader cost in this fixture. No replay disk I/O, decompression, hashing or already-merged Arc-cache improvement is measured here. |

A tape is built inside forward; no separate tape-only creation measurement was
made. A separate final candidate check measured tape destruction alone at
**0.042 ms median** (11 samples), with construction outside that timer; this is
not a baseline/candidate allocation comparison. Holding the tape is a lifetime
and memory requirement, not a separately measured compute stage.

## Enclosing measurements

Seven samples per cohort, after one warm-up; milliseconds, medians:

| Condition and learner | Baseline | Tape reuse | Less wall time | Speed ratio |
| --- | ---: | ---: | ---: | ---: |
| Initial single-thread CPU PPO | 667.699 | 476.640 | 28.6% | 1.40x |
| Initial single-thread residual PPO | 678.542 | 476.151 | 29.8% | 1.43x |
| Pinned CPU PPO | 681.659 | 466.089 | 31.6% | 1.46x |
| Pinned residual PPO | 689.248 | 527.895 | 23.4% | 1.31x |

Pinned residual ranges were 667.107–709.767 ms before and 491.897–560.138 ms after.
The timing spread is real; do not treat the best median as a universal gain.
Baseline replayed a window once for frozen preflight and twice per epoch. The
new path replays once for preflight and once per epoch: ten whole-window forward
passes become six for this two-window/two-epoch workload. Four complete redundant
forwards, their tapes and repeat sequence scans are removed; BPTT and both Adam
histories are unchanged. End-to-end collection, biology, prior work, I/O,
publication, and simulation throughput remain **unknown**.

Complete serialized actor/critic checkpoints and update receipts were byte-equal
at the end of the first timed trial in each of the four comparison processes
and the final harness check (4,640,835 bytes; SHA-256
`4a5c44449119153f6b7ef1915d4f7b88f1e37f959187834e21ba98ffb675f766`). Equality
covers weights, both moments, integer ages, frozen coordinates, masks, config,
policy versions, update counts and KL receipts for this fixture.

## Individual enclosing trials and uncertainty

All saved cohort trials are listed below in milliseconds and execution order.
Each row has seven trials after one discarded warm-up, with a fresh actor and
critic restored outside each timed scope. Each process runs PPO before residual
PPO. The baseline process precedes the candidate process in each comparison;
these are not randomized or statistically independent trials. `candidate-final`
is the later harness/destruction-timer check, not another matched comparison.

| Process / cohort | Individual timed trials (ms) |
| --- | --- |
| baseline / PPO | 666.369, 668.833, 663.075, 667.699, 680.845, 665.424, 681.189 |
| baseline / Residual | 672.687, 682.771, 670.485, 678.542, 701.588, 676.648, 680.300 |
| optimized / PPO | 462.999, 463.122, 469.518, 488.150, 476.640, 498.458, 521.879 |
| optimized / Residual | 487.043, 487.053, 476.151, 467.584, 466.630, 464.100, 482.782 |
| baseline-pinned / PPO | 720.031, 681.659, 674.094, 693.076, 690.805, 676.475, 653.381 |
| baseline-pinned / Residual | 687.705, 675.265, 689.669, 709.767, 667.107, 700.990, 689.248 |
| optimized-pinned / PPO | 452.889, 466.089, 454.692, 463.048, 566.605, 517.273, 491.532 |
| optimized-pinned / Residual | 491.897, 542.803, 527.895, 548.321, 505.220, 560.138, 512.054 |
| candidate-final / PPO | 481.546, 476.584, 496.045, 509.799, 552.743, 517.888, 510.392 |
| candidate-final / Residual | 476.057, 468.924, 490.786, 454.109, 457.441, 463.444, 466.077 |

The [recorded individual trials](20261031-cpu-ppo-trials.csv) preserve every saved
component/cohort sample from these five processes, plus the native replay
checks below. The ignored benchmark also
emits raw JSON arrays for reproduction. Frozen preflight, forward, backward, window evaluation and public
probe have 11 timed samples each; validation, joint PPO, residual gradients,
window clone, actor Adam and critic proposal have 21 each. Each scope has one
discarded warm-up. The destruction-only scope has 11 samples, with construction
outside its timer. Component scopes overlap and remain non-additive.

Checkpoint bytes are captured after the first timed trial's entire cohort for
both learner variants: after two epochs and two paired actor/critic updates.
This comparison does not record every intermediate batch or all seven trials.
The separate successive-update regression checks the final result of four
updates against recomputation; it does not assert equivalence for all possible
inputs or establish behavioral learning.

These numerical trial files are the EPYC cohort. Independent Xeon ABBA checks
are a distinct environment/cohort and must not be pooled with these samples.

No formal confidence interval is claimed. Short sequential samples share VM,
allocator and cache state; CPU frequency/contention are uncontrolled. Observed
ranges and median ratios describe these runs only. Actual workload variants,
independent process replications and end-to-end measurements are needed before
predicting a user's runtime reduction.

## Native-collected CPU replay check

A follow-up explicitly invoked benchmark collects 65 real sampled decisions
from each of two fresh production CPU host fixtures, retaining the living last
action as bootstrap. It uses native candidate masks/log probabilities,
physiology rewards and captured private replay context, not hand-authored PPO
rows. These remain fresh untrained fixtures, not imported acquired checkpoints
or evidence of competence. Each case has 64 loss rows, eight burn-in ticks on
later windows, temperature 0.9, two requested epochs and the ordinary target KL
0.02. Full nonzero synthetic Adam history and frozen coordinates are retained.

Both processes use the same compiler/profile above, are unpinned and run without
concurrent local tests/builds. The candidate runs before the baseline in this
comparison. For baseline compilation only, the three production CPU source files
are temporarily replaced by their exact `eaf9e120` bytes in the same working path
and restored in a `finally` block. Source hashes and every trial/configuration
are preserved in [native replay evidence](20261031-native-cpu-replay-trials.json).
The candidate production code is still `0f57ff6f`; follow-up changes are tests,
benchmark lint and documentation only. No alternate checkpoint is substituted.

| Case | Candidates | Loss rows per window | Effective batch | Baseline median ms | Candidate median ms | Less learner wall time |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| object-slots | 30 | 32/32 | 1 | 483.414 | 357.698 | 26.0% |
| terrain-vision | 25 | 24/24/16 | 2 | 641.001 | 454.189 | 29.1% |

There are seven timed trials after one warm-up per case; every trial restores
both complete optimizers before timing. Both implementations stop on exactly
the same KL boundaries. Object slots accepts two paired steps and stops before
the third, at KL 0.023399284; terrain vision accepts three paired steps and stops
before the fourth, at KL 0.036066584. Both complete one full epoch. The terrain
case exercises a partial final effective batch as well as real KL early stopping.

Captured-decision hashes match for each profile. The host's full private
checkpoint is asserted unchanged across the offline update experiment, and its
hash matches between implementations. Combined complete actor/critic checkpoints
and receipts from each first timed full cohort are byte-equal: 4,688,876 bytes,
SHA-256 `948831e152fe9d679404200661574d714d724d579bb28f85c01860ac29981709`.
These comparisons check final state, not every intermediate batch or all trials.

Collection is measured once per process/profile (about 2.6 seconds), and window
preparation once; those are not replicated end-to-end measurements. Setup,
checkpoint publication and actual learned-state continuation are not timed.
The learner percentage must not be presented as a whole-training or GPU gain.
The same VM-dependence and non-independent-sample caveats above apply.

Reproduce on either source variant using the identical native benchmark module:

```sh
ALIFE_NATIVE_BENCH_TAG=candidate ALIFE_BENCH_OUTPUT_DIR=/tmp/alife-native-replay \
  cargo test --locked -p alife_training --features cpu-backend --test cpu_collector \
  --config profile.test.package.alife_training.opt-level=3 \
  native_collected_replay_ledger -- --ignored --nocapture
```

## Validity and falsification

The tape comes from the exact loaded window and current trainer weights. It is a
local value in the per-window loop, never stored on the trainer or reused in
another iteration. The actor cannot step until that loop finishes its effective
batch. Every following window and epoch calls forward again. The regression
`cohort_tapes_follow_each_optimizer_step_with_auxiliary_and_speech_loss` compares
four successive updates against the public path that independently recomputes
forward for every gradient. It covers unequal window lengths, burn-in, inactive
microsteps, auxiliary imitation, six-step speech, frozen state and integer ages
above 2^24. A subsequent normal unit regression,
`residual_tape_reuse_matches_recomputed_forward_for_unequal_windows`, combines
3-row and 5-row windows in the same effective batch. It uses nonzero critic
parameters/moments and compares complete actor/critic checkpoints and KL receipts
over two updates against the public fresh-forward probe path.

All 15 CPU backend tests and all 21 CPU collector tests passed, including
finite-difference, clipping, stale-policy, private restore, sleep and preflight
rejection coverage. The expanded CPU-enabled package run passed 195 tests across 16
targets (one explicitly ignored benchmark). Full package Clippy with warnings
denied, workspace formatting and core boundaries passed. CPU/GPU differential
fixtures also compiled with `cpu-backend,gpu-tests` without requesting an adapter. A synthetic equality result does not qualify a
learned policy or a real checkpoint continuation.

Reproduce the ignored production-code benchmark with:

```sh
ALIFE_BENCH_TAG=candidate ALIFE_BENCH_OUTPUT_DIR=/tmp/alife-hotpath \
  cargo test --locked -p alife_training --features cpu-backend --lib \
  --config profile.test.package.alife_training.opt-level=3 \
  synthetic_cpu_hotpath_ledger -- --ignored --nocapture
```

For an old baseline checkout, copy the benchmark module and its `cfg(test)`
registration, and change its `evaluate_window` destructuring from four tuple
members to the old three. Preserve identical compiler/profile/fixture settings.
Compare alternating fresh processes on the same virtual CPU with no concurrent
builds; reject the performance claim if enclosing medians cease improving beyond
noise. Compare complete checkpoint bytes after each batch if divergence appears,
including uneven effective batches, actual masks/private context, clipping and KL
stops. An admitted real-data experiment must keep the same source, collection
exposure, PPO epochs/effective batches, optimizer budgets and full retained state;
measure collection-to-publication wall time as well as learner time.

GPU hypotheses remain unmeasured: `trainer.rs::record_forward` dispatches once per
microstep; `record_backward` twice per trained microstep. GPU PPO already reuses
forward state. `evaluate_ppo_window`, gradient accumulation and Adam application
also perform blocking metrics/status readbacks. Time GPU work with timestamp
queries and host encode/submit/readback separately before changing those paths.
Moving diagnostics onto fewer readbacks may help, but preserve finite/atomic
failure checks and paired actor/critic rollback. Actor WGSL validates Adam
proposals and recomputes them during application; staging proposals or caching
bias corrections may help only if measured cost justifies the additional state.
No GPU or checkpoint-format change is claimed by this CPU result.
