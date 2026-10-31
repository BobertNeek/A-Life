# Frozen PPO raw-gradient cache correctness prototype

Five actual-controller tests passed on explicit software Vulkan. This establishes
correctness for the tested synthetic cases, with no throughput or physical GPU
qualification. The ordinary entrypoint remains fresh and unchanged in behavior;
only a `gpu-tests` experimental entrypoint requests reuse. No default, CLI,
`PpoConfig`, serialization or shader change was made.

The same PPO controller admits reuse only when the entire cohort fits one
effective batch of at most eight windows. Speech or auxiliary objectives fall
back to original evaluation. The global frozen pass computes raw actor gradients
and their norm, then saves only those bits and value-head gradient plane3. After
all old-probability checks pass, original loader callbacks and CPU validation
still run. Cached vectors feed the existing ordered scaled accumulation,
finite checks, clipping, KL gate, value backup/rollback and Adam application.
The final frozen window's tape/objective stays resident. The cache is discarded
before the first KL decision/application; every later epoch evaluates afresh.
Repeated loads must return immutable, bit-identical data; no content digest was
added. Fatal device/resource errors remain fail-stop rather than deferred.

At N2048/S32768 the copy-only allocation requests 139,272 bytes per window:
131,072 actor-gradient bytes, 4 raw-norm bytes and 8,196 value-gradient bytes.
The active pair requests **278,544 bytes**; eight windows request **1,114,176
bytes**. Arithmetic and device maximum-buffer bounds are checked, with a 2 MiB
prototype cap and fallback on unsupported bounds. These are requested buffer
bytes, not measured residency or VRAM. No full tapes, window handles or CPU
replay data are retained by the cache.

The tests use the real public trainer/controller with a procedural N2048
phenotype, 32,768 synapses, one trainable decoder coordinate and alternating
active/inactive replay ticks. Windows T3/burn0 and T5/burn1 have unequal 3/4 loss
rows. They compare normal and experimental paths after exact checkpoint rewinds:

- Epochs1/2 and value coefficient0/.5, owned and shared windows, identical loader
  schedules and receipt bits. A warm accepted update always seeds nonzero actor
  and value-head moments/history first, so coefficient0 preserves nonzero value-head
  parameters, moments and optimizer history. Reuse stays at two windows across
  two accepted epochs; the second epoch evaluates afresh.
- Every actor weight/moment bit and integer age, value parameter/moment bits,
  optimizer/policy counters, logical recurrent state/output words, all five value
  planes, and objective adjoint/metric/value/preflight words match.
- KL cancellation preserves persistent bits and supports an accepted restart.
- Finite1e30 policy targets produce raw-norm overflow only at accumulation. A
  later stale old probability still wins before any reuse; the nonstale control
  fails at the original first accumulation. A repeated-loader failure keeps its
  exact callback order. Failure assertions compare persistent float bits and
  counters; error scratch may differ and is not claimed equivalent.
- Nine windows fall back with zero cache bytes/reuse and exact normal-controller
  results. The pre-existing nine-window controller fixture also passed.

All five tests passed in 35.57 seconds under the **unoptimized test profile**;
that duration is not a performance measurement. Whole-process peak child RSS was
217,736 KiB, including every fixture, software GPU buffers and verification;
it is neither incremental cache memory nor VRAM. Strict GPU-test all-target
Clippy and a normal non-`gpu-tests` Cargo check passed. Independent implementation
and fixture source reviews found no material blocker.

Executed source is clean commit `c545cb209f39ac5da5388567024083c46f4f6638`, tree
`cdb71ee6c0ff9b4663f1d23ad5f49bb476c6c5da`, based on legacy controller/support
`f1a3cad491bb2dfb80c7d68f6b6ffe77396e9b6c`. The dedicated target forced current
training compilation rather than trusting shared-worktree modification times.
Binary SHA256 is `ae1912c73f45883759269b3ec936f3b8ff39d412c910e374f2e22dcea130fcba`.
The [receipt](20261031-frozen-gradient-cache.json) retains raw logs, source/binary
hashes and commands. No external nine-window semantic JSON was emitted in this
run; new pair equality was checked within the same binary/session.

```sh
source /workspace/cloud-cpu-validation/env.sh
source /workspace/vulkan-assay/env.sh
export CARGO_TARGET_DIR=/workspace/A-Life-frozen-gradient-cache/target/frozen-tests
cargo test -p alife_training --features gpu-tests --test software_ppo_controller -- --ignored --test-threads=1 --nocapture
```

Remaining qualification includes representative full-controller release timing,
physical GPU correctness/performance, speech/auxiliary and memory-cap fallback
execution, and a forced actor-apply/value-rollback case. The current cases use
terminal-only synthetic transitions and do not establish nonterminal GAE or
creature learning. No real checkpoint or PC work was used. This remains an
experimental branch without a PR or default promotion.
