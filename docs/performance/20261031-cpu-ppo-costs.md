# CPU PPO learner cost ledger

Base source: `eaf9e12067d649e9e27c16e6600e6e8dabfc7e99`. The change retains the
forward tape already produced by CPU loss evaluation for that window's BPTT.
Both `train_cpu_ppo_cohort` and `train_cpu_residual_cohort` use it. This is an
opt-in CPU learner result; GPU training and ordinary simulation were not timed.

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

Pinned residual ranges were 667.106–709.773 ms before and 491.899–560.143 ms after.
The timing spread is real; do not treat the best median as a universal gain.
Baseline replayed a window once for frozen preflight and twice per epoch. The
new path replays once for preflight and once per epoch: ten whole-window forward
passes become six for this two-window/two-epoch workload. Four complete redundant
forwards, their tapes and repeat sequence scans are removed; BPTT and both Adam
histories are unchanged. End-to-end collection, biology, prior work, I/O,
publication, and simulation throughput remain **unknown**.

Complete serialized actor/critic checkpoints and update receipts were byte-equal
in all four comparison runs and the final harness check (4,640,835 bytes; SHA-256
`4a5c44449119153f6b7ef1915d4f7b88f1e37f959187834e21ba98ffb675f766`). Equality
covers weights, both moments, integer ages, frozen coordinates, masks, config,
policy versions, update counts and KL receipts for this fixture.

## Validity and falsification

The tape comes from the exact loaded window and current trainer weights. It is a
local value in the per-window loop, never stored on the trainer or reused in
another iteration. The actor cannot step until that loop finishes its effective
batch. Every following window and epoch calls forward again. The regression
`cohort_tapes_follow_each_optimizer_step_with_auxiliary_and_speech_loss` compares
four successive updates against the public path that independently recomputes
forward for every gradient. It covers unequal window lengths, burn-in, inactive
microsteps, auxiliary imitation, six-step speech, frozen state and integer ages
above 2^24. All 15 CPU backend tests and all 21 CPU collector tests passed, including
finite-difference, clipping, stale-policy, private restore, sleep and preflight
rejection coverage. A synthetic equality result does not qualify a
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
