# Bounded CPU host-step cost measurement

Base source is `17c7a33a69b63b9dbac100655df1530c54696c34` (PR #219).
This experiment is on a separate branch and does not change PR #219's head. It measures the actual `CpuCanonicalHost::step_with_preparation` method
with test-only clocks; no learning/simulation operation is removed or bypassed.

## Applicability to continuous same-individual training

PR #219 accelerates the opt-in CPU PPO cohort learners only. It does not speed
the GPU PPO phase of the continuous same-individual trainer. At source
`17c7a33a`, the Python runner records its native invocation as
`train_n2048_care --resident-continuous STORE REQUEST_JSON` for its default
`same_individual` mode ([command selection](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/scripts/run_continuous_care.py#L46),
[native argv](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/scripts/run_continuous_care.py#L630)).
The [CLI dispatch](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_game_app/src/bin/train_n2048_care.rs#L78)
enters `run_resident_continuous_care`. That path requests a
[required GPU backend and GPU training session](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_game_app/src/foundation_training_cycle/continuous_world.rs#L656),
then invokes
[`train_recurrent_ppo_cohort_with_windows`](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_game_app/src/foundation_training_cycle/continuous_world.rs#L1304).
There is no CPU PPO selection in this path.

The GPU learner already calls
[`evaluate_ppo_window`](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_training/src/ppo.rs#L1338)
to encode forward/objective work and then `accumulate_replay_gradients` against
the retained device state. Its
[`encode_replay_gradients`](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_training/src/trainer.rs#L669)
does not repeat the neural forward. GPU PPO still has replay preparation,
command encoding/submissions, metric and validation readbacks, BPTT, gradient
accumulation and paired actor/critic Adam work. Those training operations are
distinct from the batched live inference tick. Neither their exclusive times
nor their share of the historical continuous run was measured by these CPU
fixtures. No CPU gain or host-stage percentage here should be assigned to that
historical GPU PPO phase or collection phase.

This is source-level backend identification. The deployed invocation manifest,
binary hash and hardware receipt were not available in this cloud fixture; no
desktop process or acquired checkpoint was inspected to reverify the historical
run. Reconcile its `native_argv`, binary revision and native hardware receipt
before attaching any new measurements to that run.

### Shared and divergent host work

The GPU runtime's [staged tick](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_game_app/src/gpu_live_runtime/staged_tick.rs#L181)
builds one world perception index per tick, then captures receptors and an indexed
grounded perception draft per waking resident. It calls
[`prepare_cpu_rows`](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_game_app/src/gpu_live_runtime/cpu_preparation.rs#L76),
whose audited path uses shared `prepare_canonical_frame` for memory recall,
attention routing, cognitive context/projection, finalization and frame
validation. It then packs/validates `GpuPerceptionUpload` and
`GpuMemoryContextUpload`. One resident uses one preparation worker; the two-worker
threshold is eight rows. Its optional approximate path is a separate branch.
Continuous training takes the audited path: `uses_audited_cognition()` is true
when training sampling is enabled
([source](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_game_app/src/gpu_live_runtime.rs#L4708)).

The CPU host also calls shared `prepare_canonical_frame`, but wraps its tick in
full founder/private binding validation, `self.clone()` and commit/drop. The GPU
staged tick does not call this CPU wrapper. Thus the four exclusive CPU intervals
below do not identify the GPU runtime's collection bottleneck. Shared routines
are useful device-free assay targets; separate call counts, world/memory sizes,
packing and retained learning still prevent transferring CPU tick percentages.

### GPU PPO host/submission audit (source counts, no timings)

For the unchanged GPU cohort learner, let `W` be all cohort windows and `B` the
windows in one attempted effective batch. The following are direct source-call
counts on a successful stable-shape path; queue writes, implicit driver work,
initial allocation and shape-change copies are additional. An accepted batch
assumes nonzero critic loss. These are not measured device executions or times.

| Boundary | Direct queue submissions | Blocking readback calls | Why it exists |
| --- | ---: | ---: | --- |
| Frozen all-window preflight | `2W` | `W` | Forward/objective plus metric copy verifies collection-policy probabilities before any update. |
| Begin effective batch | 1 | 0 | Clears accumulated actor gradients. |
| Each attempted window | 4 | 2 | Forward/objective and metric read; backward/accumulate and two-float status read. |
| Accepted batch tail | 5 | 2 | Critic gradient preflight/read, value rollback backup, paired actor/critic Adam apply, actor status read. |
| KL-stopped batch tail | 0 | 0 | Cancels accumulation without optimizer mutation. |

Including begin, an accepted critic-enabled batch has `4B + 6` direct submits and
`2B + 2` blocking readbacks; a KL-stopped attempt has `4B + 1` and `2B`. Each replay
shape change adds one submit copying retained weights and full Adam arrays into
new scratch buffers. Checkpoint/value prediction work outside this cohort adds
other reads and is excluded. Error rollback can add a submission and wait.

Host encoding also scales with recurrent microsteps, separately from queue
submissions. [`record_forward` / `record_backward`](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_training/src/trainer.rs#L1697)
encode `microsteps × ticks` neural forward dispatches and
`2 × microsteps × (ticks - burn_in_ticks)` backward dispatches per attempted
window, plus the other head/reduction dispatches. A four-microstep 256-loss-row
window therefore has 1,024 forward and 2,048 backward neural dispatches without
burn-in; 128 burn-in ticks add 512 forward dispatches. These are static source
counts, not a measured encoding cost or evidence that required recurrence can
be skipped.

The continuous source slices loss at 256 rows, adds up to 128 burn-in rows and
uses an effective batch of eight windows. For an uninterrupted 512-row segment,
that yields 256- and 384-tick replay windows. `prepare_replay` retains only the
last exact `(tick count, dendritic branch shape)` allocation; alternating those
two windows changes its shape each time, causing scratch allocation and the
weight/full-Adam device copy described above
([shape predicate](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_training/src/trainer.rs#L95),
[preparation](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_training/src/trainer.rs#L382)).
Actual segment lengths and initial branch changes must be counted in the run;
no allocation latency was measured here. A capacity/pool proposal would still
need current weights, both moments and exact optimizer ages on every switch,
fresh activations after updates and bounded memory. Measure allocation/copy
frequency and enclosing cost before selecting it over a readback change.

Source boundaries are [`ppo.rs:1299–1451`](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_training/src/ppo.rs#L1299),
[`trainer.rs:930–1090`](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_training/src/trainer.rs#L930)
and the readback helpers at
[`ppo.rs:1785`](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_training/src/ppo.rs#L1785)
and [`trainer.rs:1646`](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/crates/alife_training/src/trainer.rs#L1646).
Each helper creates a staging buffer, encodes/submits its copy, maps it and waits
for that submission. Even an eight-byte status read can synchronize pending GPU
work; its elapsed time cannot be inferred from transfer size. GPU PPO encode,
submit, wait/readback, BPTT and Adam times all remain **unknown** here.

The frozen preflight and current-weight replay remain necessary correctness
boundaries. A possible future change is batching accumulation-status reads before
the optimizer, but it must retain sticky failure evidence across windows, reject
the same invalid data before any Adam mutation, preserve paired rollback, and
demonstrate an enclosing GPU PPO gain. The highest-value next GPU measurement is
exclusive host encoding versus the per-window metric/status waits and accepted
batch preflight/Adam waits on an actual production invocation. A device-free host
assay cannot answer that latency question; live inference-tick batching does not
remove these offline PPO fences.

### Device-free assay of the GPU runtime's host routines

The separate ignored test `single_resident_gpu_host_preparation_timing` calls
the actual `prepare_cpu_row` audited path, using the existing
`OwnerFixture::terrain(811, true)` and a real host-side N2048 slot layout. It
creates no device or runtime and performs no neural dispatch. The scenario has
one organism, two visible foods, GroundedTerrainVisionV1 and 25 native candidates.
Its predictor has the fixture's test learning example; episodic memory is
**empty**, and its fixed receptor gains/one-focus attention policy are controlled
test inputs. This is not an acquired or representative long-lived memory bank.

One process alternated enabled/disabled timer order for 32 repetitions of the
same frozen preparation input: one explicit discarded warm-up and 31 retained
trials per mode. A reference preparation also runs before the loop. Full typed
output equality (including candidate order, context, bindings and upload words)
is checked on every call; fixture/private snapshots and the world signature are
unchanged afterward. This is typed equality, not a checkpoint byte comparison or
a simulation/learning outcome test. The world does not advance.

| Exclusive preparation interval | Enabled mean µs / call | Share of outer preparation |
| --- | ---: | ---: |
| Baseline recall/context/attention evidence | 230.359 | 19.20% |
| Attention selection and focal routing | 45.654 | 3.81% |
| Routed recall/context/projection/finalization/validation | 783.940 | 65.35% |
| GPU upload packing and validation on host | 135.835 | 11.32% |
| Untimed outer work and observer remainder | 3.780 | 0.32% |
| Total preparation call | 1199.568 | 100% |

These are arithmetic means from the same 31 enabled calls, so the exclusive
rows are additive up to rounding. Within those groups, baseline recall averaged
157.303 µs, routed recall 163.231 µs, finalization 348.315 µs and final-frame
validation 117.780 µs. These are **nested** scopes, not extra additive costs.
The `packing` interval includes both perception and memory upload construction,
their binding/contract validation and receptor-effect binding. No GPU upload or
copy occurs in this test.

With timers disabled, preparation median was 1084.008 µs
(966.741–1511.164); enabled median was 1078.480 µs
(973.301–3846.984). The enabled mean is higher and its final trial contains a
large finalization spike. Opposite median/mean directions and overlapping ranges
leave observer overhead unresolved; no correction or speedup is inferred.
No formal confidence interval is reported for these dependent, short VM samples.

In a **separate** span at the fixture world's tick 0, one perception index build
averaged 1.216 µs and indexed native perception/candidate construction averaged
29.718 µs over 31 retained trials. The frozen memory/context input was retimed by
the established fixture to tick 1000. Do not add these into a claimed end-to-end
tick. Each perception result equals the unindexed reference. Raw index/draft
values are repeated in the two mode records for each trial; there were 32 index
and draft calls, not 64. The runtime normally builds one index per world tick,
then one draft/preparation per waking resident. This assay measures calls rather
than frequencies observed in a deployed run.

Both recalls have a reason: baseline recall supplies evidence for attention;
focal routing changes candidates, so routed recall/finalization must bind the
chosen candidate frame and current cognitive context. They cannot simply be
deleted or reused across changed frames. Empty-memory recall still costs about
0.16 ms here; this points to fixed frame/binding work as a hypothesis, not a
measurement of record-search cost. Finalization/validation and host packing are
the next bounded shared-routine targets. A future change should identify a
specific repeated immutable computation, preserve the complete prepared frame
and rejection behavior, and improve an enclosing `prepare_cpu_row` measurement
across empty and retained banks. Reject it if any binding, candidate order,
private state, corruption rejection or enclosing benefit changes adversely.

The assay used the same EPYC VM and unpinned 0–4 affinity described below,
Rust 1.98.1 and the ordinary test profile (`alife_game_app`, core, world, GPU
backend and runtime opt-level 2; debug assertions enabled). No concurrent local
build ran during timed calls. The first command omitted `gpu-runtime` and ran
zero tests; a later attempt rejected an object-slot-only history seeder on the
terrain fixture. Neither yielded timing evidence. The passing assay uses native
terrain input with an explicitly empty bank and selected one test.

All individual samples, disabled-scope flags, source hashes and conditions are
in [the raw host-preparation data](20261031-gpu-host-preparation-trials.json).
Run the named test with `cargo test --locked -p alife_game_app --features
gpu-runtime --lib single_resident_gpu_host_preparation_timing -- --ignored
--nocapture`. This result does not establish GPU collection throughput, PPO cost,
long-lived memory scaling or a comparison against the CPU transaction wrapper.

### Training capture: bytes and necessity, independently of wait fusion

The parent recovered a final historical b817 receipt reporting 139,536 world
ticks, 279,072 capture readback submissions and 247,262,257,152 copied bytes.
It reports collection 5,839.860 s within 8,836.258 host seconds. This worker did
not import the receipt/checkpoint or inspect its binary; these receipt inputs
are transcribed from that handoff. Counts/bytes do not supply transfer time.
Collection includes other neural/world/host work and cannot be called capture
latency or device bandwidth.

The source's two captures surround the actual neural inference, before world
selection processing and outcome-bound plasticity. They are not duplicate
observations of an unchanged state. The
[staged consumer](https://github.com/BobertNeek/A-Life/blob/4d772c327b85d7d1757f4b9c22658d0ced42c03e/crates/alife_game_app/src/gpu_live_runtime/staged_tick.rs#L838)
keeps both boundaries, then admits them with dispatch, weight-generation,
selection, work and sealed-outcome evidence.

| Snapshot portion | Consumer and purpose |
| --- | --- |
| Pre active activations and neural homeostasis | First-row recurrent/EMA/metabolic replay initial state; continuity against the preceding post state. |
| Pre active lifetime/fast weight prefixes | Detached private plastic context added to genetic weights on each replay row; structural additions retain their exact live-slot mapping. |
| Post encoded neural input | The actual GPU encoder result becomes the replay tick input. |
| Post active activations | Detached critic features for the collected transition, and recurrent continuity evidence. |
| Post neural homeostasis and active lifetime/fast prefixes | Continuity and checks that weights did not change inside inference. |
| Both identities, generations, activation/weight bank labels and dendrites | Bind actual frame/dispatch/state; reject topology, reset, sleep, ownership or unrecorded weight changes. |

These uses are in
[`foundation_replay_sequence`](https://github.com/BobertNeek/A-Life/blob/4d772c327b85d7d1757f4b9c22658d0ced42c03e/crates/alife_game_app/src/foundation_training.rs#L355),
[`ReplayContinuity`](https://github.com/BobertNeek/A-Life/blob/4d772c327b85d7d1757f4b9c22658d0ced42c03e/crates/alife_game_app/src/foundation_training.rs#L4348)
and [continuous critic feature capture](https://github.com/BobertNeek/A-Life/blob/4d772c327b85d7d1757f4b9c22658d0ced42c03e/crates/alife_game_app/src/foundation_training_cycle/continuous_world.rs#L1116).
Ordinary post-action learning may change lifetime/fast weights and their active
generation before the next row. Sleep, reset, structural changes and policy
installation add other discontinuities. Replacing the next pre state with the
previous post state would therefore require a proven boundary/invalidation
contract; simple cross-tick snapshot reuse is not justified.

The large routine copy is **the mutable slot arena**, not immutable graph data.
The compiled phenotype is shared in host metadata. Additional production-only
structural synapses are versioned/cached by resident handle/slot, exact slot
metadata and the structural checkpoint. Only an uncached/changed structure adds
plan and genetic/alpha word-range reads. See
[`training_capture.rs`](https://github.com/BobertNeek/A-Life/blob/4d772c327b85d7d1757f4b9c22658d0ced42c03e/crates/alife_gpu_backend/src/closed_loop_runtime/training_capture.rs#L76)
and its [structural cache predicate](https://github.com/BobertNeek/A-Life/blob/4d772c327b85d7d1757f4b9c22658d0ced42c03e/crates/alife_gpu_backend/src/closed_loop_runtime/training_capture.rs#L258).
Cached structure is not immutable forever: generation/topology changes still
require exact rebinding. This investigation changes none of those rules.

A deterministic, device-free assay invokes the actual
`GpuFixedClassArenaPlan::new(n2048, 1, ...)` and `slot_ranges(0)`. It asserts
contiguous disjoint mutable ranges and accounts for final alignment padding.
The layout source file has the same Git blob
`8a577aeb29ffb8cf2dcb62e9a399186249ecb971` in b817 and current merge 4d772c32.

| Fixed mutable slot portion | Bytes per full capture |
| --- | ---: |
| Lifetime and fast plastic weights, both banks | 525,312 |
| Recurrent and decoder eligibility, both banks | 262,656 |
| Activation banks, accumulator, neural homeostasis and encoded input | 49,152 |
| Replay event/sample/span scratch | 48,128 |
| Selection/logit/diagnostic/speech/extension/learning/pending headers | 544 |
| Alignment padding | 224 |
| Total mutable arena | **886,016** |

`279072 × 886016 = 247262257152` exactly. Under the receipt's same one-resident
N2048 fixed-slot conditions, the full mutable arenas alone account for every
reported byte; the totals leave no extra structural readback bytes in that
counter. The arena also reserves 4,077,824 immutable-plan and 262,656
immutable-weight bytes, but those capacities are **not** routine copied bytes.
The raw [per-field byte ledger](20261031-capture-layout-ledger.json) retains all
23 mutable field ranges, provenance, padding, receipt inputs and arithmetic.
One deterministic assay passed; it is not an elapsed-time benchmark or GPU test.
Twelve existing fixed-arena tests, strict library/tests Clippy and formatting
passed after adding this ignored assay. No held fusion code is used here.

Two separate questions follow. Both logical boundaries serve current replay and
validation. Whether they require two independent synchronization waits is a
submission/ordering question; fusing waits need not reduce copied bytes. Whether
both require the **entire** mutable arena is a payload-contract question. The
training consumers above select active activation/weight prefixes, homeostasis
and post encoded inputs rather than all eligibility, inactive banks and replay
scratch. Those other fields remain necessary private state for normal neural
learning, sleep and full capture/restore; this is not permission to discard or
reset them on the GPU or in canonical checkpoints. Actor/critic Adam checkpoints
are a separate retained learner contract and must remain complete.

A compact training-only read view is an **unmeasured proposal**, distinct from
wait fusion. For a 2,048-neuron, 32,768-live-synapse fixture, retaining active
activation + homeostasis + both active plastic-weight prefixes at pre, plus
encoded inputs at post, would copy 286,720 + 294,912 = 581,632 mutable bytes per
pair versus 1,772,032 full bytes. This is a source-derived candidate read set,
not an implemented format, admitted historical row, proved minimum or speedup.
Actual live counts, added structural reads and every consumer must be checked.
The existing public full-snapshot API must not silently return zero-filled holes
or misrepresent a partial view as full acquired state.

The next falsifiable no-device experiment is a poison/differential read-set test
through the production replay conversion, continuity, writer admission and
critic-feature extraction. Starting from existing synthetic replay fixtures,
vary each retained field and poison each proposed omitted range; compare the
complete outputs and rejection behavior. Cover both active banks, partial
microsteps, nonzero private plastic history, structural additions, generation
changes, sleep/reset boundaries and malformed evidence. Any newly required
omitted word or lost rejection falsifies the proposed compact contract. Only
then time host packing/conversion in an enclosing fixture. This cannot establish
device copy or wait savings: those still require a real-device comparison that
preserves exact pre/post evidence, outcomes, private continuation and full Adam.

## Workload and boundaries

One fresh, untrained Standard2048 CPU host uses GroundedTerrainVisionV1, world
seed `0x7A1123`, sampling seed 42 and temperature 0.9. Each rollout advances 65
real world ticks, producing 65 real waking decisions from one living organism.
The fixture is the existing canonical CPU collector fixture. No acquired user
checkpoint is imported. Ordinary perception, legality, physiology, private
plasticity/memory, sampling and world advancement remain active. The preparation
callback is identity: there is no SLM provider, GPU, offline PPO/Adam update,
replay disk writer or durable publication in this measurement.

The four timestamps surround these exclusive production intervals:

1. `host.rs:615–623`: founder weight binding plus the resident authority loop.
2. `host.rs:624`: `let mut next = self.clone()`.
3. `host.rs:625`: `next.step_inner(...)`.
4. `host.rs:626`: `*self = next`, including destruction of the old host.

Line numbers refer to the uninstrumented base. Each interval executes once per
successful world tick. Founder validation runs once per tick; resident authority
runs once per resident, so both run 65 times per rollout here. One whole-host
clone, one inner call and one commit/drop also occur 65 times. `step_inner`
additionally clones a perception world once per tick; that copy is inside the
inner interval, not the outer clone interval. These counts are not a claim about
multi-organism or sleeping cohorts.

## Results

The table uses pooled arithmetic means per 65-tick trial from seven timing-enabled
trials (455 ticks). Its percentages use the sum of outer `host.step` call times.
These exclusive means are additive, unlike independently selected medians.

| Exclusive interval | Mean ms / 65 ticks | Share of outer step calls |
| --- | ---: | ---: |
| Founder/private authority validation | 744.725 | 28.20% |
| Full host clone | 5.128 | 0.19% |
| step_inner | 1887.418 | 71.47% |
| Commit and old-host drop | 3.554 | 0.13% |
| Timer/record/return remainder | 0.131 | 0.0050% |
| Total outer step calls | 2640.956 | 100% |

The surrounding collection loop averaged 2641.089 ms; its additional
0.133 ms includes test-driver iteration and moving returned decisions. The outer
call remainder includes observer bookkeeping, return and surrounding clocks;
clock costs embedded inside the four intervals are not isolated by that row.
`step_inner` is a composite of perception, neural execution, world legality,
biology, private learning and sealing. Its 71.5% must not be relabelled neural
compute or GPU work.

## Instrumentation control and uncertainty

The original host body was compiled as an uninstrumented control, then five
optional clocks and a pre-reserved per-thread row recorder were added under
`cfg(test)`. The instrumented binary alternates enabled/disabled order by trial
parity, starting each mode from the same initial host snapshot. A late original
control checks process-order drift. Every mode/process has one discarded warm-up
and seven timed rollouts; warm-up samples are retained in the raw data.

| Process / mode | Collection-loop median ms | Observed range ms |
| --- | ---: | ---: |
| uninstrumented / uninstrumented | 2743.169 | 2581.640–2915.422 |
| instrumented / disabled | 2651.303 | 2568.193–2732.035 |
| instrumented / enabled | 2635.463 | 2561.848–2718.419 |
| uninstrumented-late / uninstrumented | 2686.588 | 2573.813–2736.202 |

The enabled median is about 0.6% below the disabled median. The ranges overlap
widely and the early/late controls drift. Total instrumentation overhead is
therefore **not resolved** by this experiment; no negative overhead or speedup
is inferred, and no timing correction is subtracted. The small outer remainder
is only part of observer cost, because the timestamp/access costs fall inside
measured intervals too. Five extra `Instant` reads occur per enabled step (325
per rollout), plus recorder work. All modes share the driver's two outer-clock
reads per step. Short sequential/paired trials are not independent samples;
there is no formal confidence interval or controlled CPU frequency/contention.

All 32 rollouts (2,080 ticks including warm-ups) produced identical hashes of
their serialized decisions and full private checkpoints. Decisions:
`a526952ddde93e1fe857a584be01d427624a275afb4d1d8ac661624cf3b804b6`.
Private host:
`a1fe832be57e41fe0ecd97039a51b5247be6d37cd491ee095f8f412d3d350695`.
These also match the earlier native terrain replay fixture. This is final-state
and decision-sequence equivalence for these rollouts, not all-input proof or
learned competence. The experiment does not test failure-path latency.

## Why the work exists and what the result changes

Validation binds the frozen founder and published organism authority to actual
private state before any tick mutation. `validate_founder` digests all 32,768
weights per tick. `validate_resident_authority` recomputes `resident_digest`,
which materializes/serializes neural and topology checkpoints, predictor,
activity, sleep, hysteresis, replay, language, topology and memory exports plus
sampling state. Successful validation also exports the active bank separately
for its memory digest. The fixture thus performs 65 full pre-tick resident
bindings per rollout. Their individual internal costs were not split further.

The rollback clone and final commit preserve atomic changes to biology,
cognition and sampling. Their combined measured share is about 0.33%, so this
fixture does **not** justify optimizing the outer transaction copy/drop. Larger
or older worlds could differ. Keep this rollback behavior.

The useful lead is the 28.2% pre-tick authority boundary. A possible future
experiment is reusing an already sealed private-state binding only while an
explicit internal generation proves the resident/sampling state unchanged,
while still comparing against the current world record. Full validation remains
necessary for untrusted restore/admission; every mutation, restore and policy
change must invalidate any runtime cache. No such cache is implemented here.
Reject that proposal if unchanged-state identity cannot be guaranteed, corruption
rejection changes, or an enclosing native collection measurement does not improve.
The remaining 71.5% inner aggregate needs its own attribution before picking an
inner optimization; this experiment deliberately stops at the four boundaries.

## Provenance and reproduction

Hardware: cloud x86_64 VM reporting AMD EPYC 9V74, five visible virtual CPUs,
four-CPU cgroup quota, 16 GiB limit. CPU affinity is unpinned (0–4). Rust 1.98.1;
test profile with `alife_training` opt-level 3, core/world/runtime opt-level 2,
debug assertions enabled. The control and instrumented methods use the same
fixture/harness logic; formatting only changed the test module. There were no
concurrent local builds/tests during timed rollouts. These test-profile, fresh
CPU-world costs must not be assigned to deployed GPU collection or long retained
lifetimes, or converted into a whole-training speedup.

- [Raw per-tick data, including warm-ups](20261031-host-step-ticks.csv)
- [Summary, source hashes and conditions](20261031-host-step-summary.json)
- [Early uninstrumented trials](20261031-host-step-uninstrumented.json)
- [Enabled/disabled trials](20261031-host-step-instrumented.json)
- [Late uninstrumented trials](20261031-host-step-uninstrumented-late.json)

On the experiment branch, run `bounded_native_host_spans` explicitly using
`cargo test --locked -p alife_training --features cpu-backend --lib --config
profile.test.package.alife_training.opt-level=3 bounded_native_host_spans --
--ignored --nocapture`, with `ALIFE_HOST_BENCH_TAG` and
`ALIFE_BENCH_OUTPUT_DIR` set. For the original control, remove only the test-only
timing statements (keep benchmark registration), set
`ALIFE_HOST_CONTROL_ONLY=1`, and use a distinct output tag. Controls run before
and after the enabled/disabled process. The committed source diff is the probe;
it is not a runtime profiling framework or a simulation optimization.
