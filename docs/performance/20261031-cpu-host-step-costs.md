# Bounded CPU host-step cost measurement

Base source is `17c7a33a69b63b9dbac100655df1530c54696c34` (PR #219).
This experiment is on a separate branch and does not change PR #219's head. It measures the actual `CpuCanonicalHost::step_with_preparation` method
with test-only clocks; no learning/simulation operation is removed or bypassed.

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
