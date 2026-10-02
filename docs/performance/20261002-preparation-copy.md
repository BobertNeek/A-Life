# Live preparation copy removal — 2026-10-02

Baseline: combined candidate `5abafe1961efaf099cd07287bd7ca09e8ea754cd`.
Implementation: `4b48c4c9d438b1dd43894aca68e64267a9c633d1`, on
`codex/cloud-preparation-scaling-20261002`. This branch is separate from main,
graphics, gameplay/training integration, and CI-comment remediation.

The change removes a discarded draft clone during successful tracked-object
focal routing and moves an already-owned attention frame into its context.
In the measured C26/two-slot fixture, the two helpers make **five fewer
allocation/reallocation calls and request 4,568 fewer bytes per row**.
Timing differences are small and noisy; a whole-preparation speedup is **not
established**. Windows/RTX FPS/TPS and the 20 FPS/50-creature target remain
**Unknown**. No hardware run, GPU campaign or training execution occurred.

## Contract and ownership

`gpu_live_runtime.rs` owns routing and context attachment;
`gpu_live_runtime/cpu_preparation.rs` borrows its retained captured draft.
Focused routing still validates attention, sorts by the same focal-slot/index
key, reindexes every candidate, and calls the same strict draft constructor.
Empty, non-object and absent-object focus still clone the complete original
input. Attention's three mirrored vectors and budget/hysteresis fields are
copied before its owned frame moves; context validation remains unchanged.

There is no change to attention selection, cognition, physiology, prediction,
learning, tick frequency, population, save state, prior capture, world queries,
GPU admission/dispatch/readback, ordered commit or rollback. This preserves
AOA-INV-001, AOA-ATT-001/002/004/005, AOA-CTX-001 and AOA-MEM-001 without
creating a new contract or compatibility path. WWCD/gameplay and the existing
single brain remain intact. Reverting the implementation commit restores the
previous copy behavior.

## CPU evidence

[The receipt](evidence/20261002-preparation-copy.json) retains every accepted
sample, full source/executable hashes, build metadata and allocation result.
Preserved executables and raw outputs are also available in the cloud workspace
at `/workspace/cloud-cpu-validation/preparation-copy-20261002/`.

Linux 6.18.44, EPYC 9V74/KVM, five exposed CPUs, four-core cgroup quota, Rust
1.98.1 release/opt-level 3, debug assertions off. Full suffix uses affinity 0–3;
two-helper and allocation checks use CPU 0. Builds did not overlap accepted
timing. Other host tenants are uncontrolled. Initial screening samples were
superseded by the source-bound final comparison below.

The isolated serial comparison uses independent owners, 26 candidates, two
object slots, empty episodic banks/topology, and the established attention
fixture. Each median has nine alternating old/new samples of 200 cohorts.
Caller-owned context/attention inputs are cloned before each clock; routing's
old input clone, helper work and result collection are inside. Complete typed
draft/context equality and result destruction are outside the clock.

| Owners | Old two helpers ms | New two helpers ms |
| ---: | ---: | ---: |
| 8 | 0.108231 | 0.103646 |
| 16 | 0.213311 | 0.209194 |
| 32 | 0.460177 | 0.451272 |
| 50 | 0.743788 | 0.729750 |

The allocation probe separately uses repeated calls on one immutable C26/two-slot
owner with empty context. Caller clones, setup, comparison and output destruction
are outside counting; result-vector allocation and consumed-input deallocation
are inside. Both literal old/new helper results compare equal. Counts include
allocation and reallocation calls; bytes are requested traffic, **not retained
RAM/RSS**. Nonfocal/empty-vector cases do not necessarily save five calls.

| Calls | Old → new calls | Old → new requested bytes |
| ---: | ---: | ---: |
| 8 | 177 → 137 | 307,256 → 270,712 |
| 16 | 353 → 273 | 614,512 → 541,424 |
| 32 | 705 → 545 | 1,229,024 → 1,082,848 |
| 50 | 1,101 → 851 | 1,920,350 → 1,691,950 |

The actual full CPU suffix uses independent N512 owners, 54 decoder lanes,
26 candidates, 64 retained sealed memory records, learned predictors and warm
hysteresis. Four processes ran before→after→after→before, each with nine
alternating serial/two-worker samples per cohort. The table combines 18 samples
per implementation/mode/cohort. All 288 timed batches and their 7,632 complete
row comparisons passed. Serial and two-worker results include memory recall,
attention, projection, finalization and CPU upload encoding. They exclude world
perception, receptors, prior capture, admission, GPU work, sealing/lifecycle/save
and rendering; these measurements cannot be added to earlier leaf improvements.

| Owners | Serial old → new ms | Two workers old → new ms |
| ---: | ---: | ---: |
| 8 | 8.774 → 8.487 | 4.838 → 5.075 |
| 16 | 17.450 → 17.606 | 10.142 → 9.659 |
| 32 | 34.879 → 33.925 | 18.205 → 18.537 |
| 50 | 54.478 → 54.801 | 28.688 → 28.832 |

This variability does not establish a full-suffix speedup or regression. The
justification for the small change is its measured allocation reduction and
preserved strict typed outcomes.

## Verification and replay

Micro-Spec mode, independent R2 review by two Sol6.1 workers: clear. Frozen
helper bodies in `copy_contract_tests.rs` exactly match `5abafe19`; the full
serial oracle now calls them rather than sharing the changed implementation.
Fourteen CPU preparation tests pass, including object-slot and Terrain hint
fixtures, retained banks 0/64/256, spawn fallback, panic handling, read-only
state and learning guard checks. Two strengthened copy-contract tests also
pass; they compare both focal routes, all three fallback branches, malformed
attention and complete context output, with explicit owner/schema rejection.
Sequence/tick rejection coverage is not claimed exhaustive. The existing
object-bound concept/gap test passes. The GPU-gated causal attention test was
not enabled or run. Strict release Clippy (`--lib --tests --features gpu-runtime`),
formatting, core boundaries and documentation checks pass.

Source `/workspace/cloud-cpu-validation/env.sh` for lean cloud Cargo/rustc.
Build each chosen revision with `cargo +stable test --locked --release -p
alife_game_app --features gpu-runtime --lib --no-run --message-format=json` and
capture its emitted test executable before changing source. Run its ignored
`gpu_live_runtime::cpu_preparation::tests::production_runner_cpu_timing_evidence`
with `--exact --ignored --nocapture --test-threads=1`. The successor also exposes
`gpu_live_runtime::cpu_preparation::tests::copy_contract_tests::routing_attention_copy_cpu_timing_evidence`.

For allocation replay, run the [builder](evidence/20261002-preparation-copy-allocation-build.py)
from the successor worktree with the emitted Cargo JSONL path and a unique
executable path as its two arguments, then execute that path with `taskset -c 0`.
The generalized replay builder and initial exploratory builder have separate
recorded identities; their generated Rust and allocation outputs match exactly.
The replay manifest records linked-library hashes absent from the original
exploratory manifest.

Next ranked source hypotheses, all unimplemented/unmeasured in this slice:
cache the immutable receptor expression at exact phenotype admission/restore;
avoid discarded perception payload encoding when only its binding is needed;
combine duplicate object-bound topology scans while preserving floating-point
order. Each needs its own cost screen and strict validation/ownership proof.
