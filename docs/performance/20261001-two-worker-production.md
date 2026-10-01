# Bounded production CPU preparation — 2026-10-01

Production source `f03d2dc3b53cb4e2139f1f0d1c50f15b92138d72` implements the split proposed by the earlier CPU admission audit. Its base is coherent candidate `ad49a32558233f6cdabf974eb7361ce0364ffbdd`, plus the already reviewed finalization patch (`4eed7acc` cherry-picked here as `1c53224a`). Parent integration is separate. No desktop, GPU device, neural training, provider or hardware campaign is part of this receipt. The requested 20 FPS/50 creatures and sustainable 20 TPS with bounded debt remain **Unknown**.

The later [TerrainVision hint-boundary extension](20261001-terrain-hint-workers.md) supersedes the prior-presence serial fallback described below. These source-bound measurements and their original scope remain unchanged.

## Contract and ownership

Mode 1 Micro-Spec, R2 independent Sol6.1 review. The established admission design serves as the bounded contract; there is no task queue, generic executor, new dependency or persistent worker pool.

- Keep serial retirement/reconciliation, retained-learning retry, ATP/sleep/replay, journal/checkpoint work, receptor derivation, grounded draft construction and semantic hint collection.
- Capture each draft immediately after its owner's housekeeping. Creating all drafts after all body updates would change successful perception.
- Preserve every memory query, attention/routing/novelty step, learned prediction, cognitive projection, strict finalization, upload word and receptor binding.
- Compute against immutable owner-specific sidecars/predictors. Publish results and hysteresis in the original owner order; keep existing neural dispatch, outcome sealing, eligibility, learning/discard, physiology, world advance, lifecycle and persistence authority.
- Bound computation to two CPU participants, with one scoped child and the caller. Never let CPU preparation choose a neural action or own a GPU transaction.

Production ownership expands only in the app's staged preparation and one backend metadata admission method. Rendering, launch/operational source, training policy, sandbox save summaries, shaders, the architecture authority and current brain remain unchanged.

## Implemented ordering

`staged_tick.rs` retains the serial per-owner prefix and captures owned draft/receptor/sequence/homeostatic/attention inputs. Sleeping, retained-learning-pending and otherwise nondispatchable rows produce their existing summaries and no CPU job. Owner IDs remain attached to captures; they are never reconstructed from an index in the dense awake vector.

`GpuClosedLoopBackend::memory_context_slot` checks device readiness, backend/handle ownership, live resident and sensor profile before returning owned `GpuBrainSlot` metadata. It exposes no device object, neural state or learned weights. Snapshot occurs after all serial housekeeping for a parallel batch. The workers borrow bank, topology and predictor by each captured owner; these are not cloned or mutated. One bounded draft clone is used for focal routing so retained captures remain available when thread creation fails.

`cpu_preparation.rs` extracts the existing suffix. It keeps baseline and routed recalls, authenticated queries, attention evidence, body/receptor modulation, familiarity/novelty, focal routing, learned prediction, projection, strict decision keys, public finalized-recall validation and exact upload encoding. Each row returns its complete result, preparation stage and optional hysteresis transition. A typed error after attention selection retains that explicit transition; an error before selection has none.

Both CPU participants finish before any result is published or neural work submitted. The child owns the latter contiguous chunk; the caller owns the first. Collection extends results in owner order regardless of completion order. Successful rows undergo readiness/handle/profile admission again and full slot equality, including identity, counts/ranges, decoder stride, slot/generation and bucket ownership token. Existing strict memory-input and whole-batch duplicate/pending-eligibility/activity-sequence checks still run before dispatch.

Ordered collection applies the owner-bound hysteresis transition, including typed late errors, then appends valid rows or their existing preparation-failure summaries. Durable sleep promotion/compaction publication and the existing single GPU/world/learning commit path remain at their prior boundaries after preparation.

## Serial fallback and asynchronous hints

Default eligible execution uses two participants when at least eight dispatchable captures and two available CPUs exist. Smaller cohorts and one-CPU hosts remain serial. No invocation creates more than one child. Eligibility is checked again using the actual captured awake-row count, rather than the larger scheduled roster.

A concrete hint counterexample dictated a narrower fallback: a terrain prior reply for owner B can become ready while owner A runs its previous serial CPU suffix. Collecting all hints first could move that delivery to a later tick. With an active TerrainVision semantic prior, the code therefore retains the original interleaved serial path: housekeeping/draft/hint capture, complete preparation and ordered collection for one owner before the next owner. Prior code, cache identity, request/delivery rules and consumption point were not modified. GroundedObjectSlots bypasses terrain-prior polling as before.

Thread-creation failure runs the same retained captured jobs serially, exactly once, without repeating housekeeping or consuming hints again. Both calling-thread and child panics are caught, the child is always joined, and a controlled error returns through the existing staged restoration. Private partial results are discarded. The outer wrapper still restores host authority and invokes its established fail-stop when irreversible GPU housekeeping was already committed. It does not promise to undo an accepted persistence job, prior/cache I/O or committed GPU state. CPU failures never retry neural/learning transactions.

## Honest timing attribution

The existing receipt's preparation leaves are additive serial intervals. Summing overlapping worker intervals and subtracting them from enclosing wall time would create a false residual. Parallel suffix leaf clocks are therefore disabled, including a selected parallel batch that falls back after spawn failure. Enclosing preparation wall time is still measured, and the existing other/instrumentation residual includes the parallel suffix. Serial leaf clocks remain available. Metadata snapshot/re-admission overhead also belongs to that residual; the upload leaf now times pure encoding rather than backend admission. No renderer receipt code or profiling framework was added.

## CPU verification scope

The focused tests compare complete `PreparedCpuFrame` values and stages/hysteresis against both a separately spelled-out frozen serial suffix and the two-worker runner. Full equality includes frames, recall contexts/keys/receipts, learned projections, literal upload words, host binding/digest/offset metadata and receptor effects. Memory, topology, predictor, captured draft and receptor snapshots remain unchanged.

Cases include noncontiguous owners, skipped roster positions, reversed order and unequal chunks; actual learned predictors and warm hysteresis; selective object-bound concepts/gaps; 26 candidates with actual 0/64/256 sealed memory records; early/late errors in both chunks; creation failure with exactly one visit per captured row; an immutable captured hint despite a later queued reply; caller/child panic and controlled error propagation through the real generic staged-world rollback helper; full metadata mismatch and device-error propagation without a device; and the actual unchanged learning sequence guard's stale/duplicate rejection.

These tests are CPU evidence. The mixed skip fixture represents captured awake rows and does not execute the live sleep/retry prefix. The generic rollback test restores real world state with an empty resident map; live GPU recovery, nonempty runtime restoration and real eligibility/learning commits remain source-reviewed and require later authorized device proof. The hint snapshot test uses a queued typed reply, not a LocalSLM provider lifecycle. Existing serial polling is retained to avoid introducing a new live timing policy.

Relevant preserved mechanisms trace to `AOA-INV-003/005/010/011`, `AOA-AUTH-002/004/005`, `AOA-TIME-001/003/006`, `AOA-CTX-001`, `AOA-ATT-002`, `AOA-MEM-001/002` and `AOA-SLM-004`. This is not architecture-wide or gameplay certification.


## Source-bound production-runner measurement

The [compact receipt](evidence/20261001-two-worker-production.json) preserves every integer sample, source/test executable hashes, Cargo compiler-artifact profile, fixture scope and whole-process resource record. The retained release unit-test executable invokes the actual implemented `prepare_cpu_rows` function; test-only fault checks are inactive. This is not a separate preparation prototype or a live GPU tick. Source files match commit `f03d2dc3` byte-for-byte.

One process ran nine alternating serial/two-participant samples per cohort, after one reference/equality pass per mode. Timings include the full pure CPU suffix, bounded draft clone, output allocation and scoped-thread creation/join. Every timed result matches the complete serial reference: 72 timed batches and 1,908 complete typed owner comparisons. The separate initial two-worker pass checks exactly one job visit per owner. Proof comparisons and retained reference outputs are outside the clocks.

| Distinct owners | Serial median ms | Two participants median ms | Serial range ms | Two participants range ms |
| ---: | ---: | ---: | ---: | ---: |
| 8 | 8.036 | 4.771 | 7.883–8.238 | 4.467–5.356 |
| 16 | 17.589 | 9.415 | 17.255–18.951 | 8.608–11.943 |
| 32 | 32.567 | 17.642 | 32.216–33.837 | 16.641–22.155 |
| 50 | 55.194 | 27.538 | 53.221–57.248 | 26.254–45.347 |

These are synthetic immutable owner inputs, not a live population: 26 copied/indexed candidates, two food observations, actual 64-record sealed banks, acquired predictors, warm hysteresis, focal capacity one and empty topology. The established N512/54-lane asset provides CPU slot metadata only. Selective concept/gap behavior and 0/256-record occupancy are checked separately; their timings are not represented by this table.

The shared Linux/KVM EPYC 9V74 host exposes five CPUs, with a four-core cgroup quota. The whole process and its child use affinity 0–3, Rust 1.98.1, release opt level 3/debug assertions off and the lean cloud environment. One-process observed wall reductions are 40.6–50.1%; no confidence interval or isolation guarantee is asserted. The 50-owner parallel outlier was 45.347 ms. Across the entire process, user/system CPU was 11.868/0.100 s, elapsed wall 11.492 s and peak RSS 102.387 MiB. Those resource totals include fixtures, class-plan allocation/allocator retention, reference outputs, proofs and console output. They are not per-stage CPU/RSS, a before/after allocation comparison, or a prediction of frame time.

Backend metadata snapshot/re-admission, serial housekeeping/draft/receptor derivation, actual neural inference/readback/learning, sealing, world advance, lifecycle/persistence and rendering are excluded. There is no whole admitted live preparation measurement. These medians cannot be added to earlier attention/topology/finalization results, whose source and inputs differ.

Reproduce on this source with `/workspace/cloud-cpu-validation/env.sh` sourced:

```bash
cargo test -p alife_game_app --release --lib --features gpu-runtime \
  gpu_live_runtime::cpu_preparation::tests::production_runner_cpu_timing_evidence \
  -- --exact --ignored --nocapture --test-threads=1
```

Pin the executable and all its child threads to the recorded affinity for comparable cloud conditions. Capture the exact executable from Cargo compiler-artifact output and preserve its hash before any shared-target rebuild. No GPU-test feature, device, provider or training campaign is required. The original raw run/resource records remain under `/tmp/alife-two-worker-production-*`; the published receipt retains all timing samples and build/source identities.

## Verification and handoff

- Focused CPU suite: 10 passed in debug and release; one dated timing test normally ignored, then explicitly passed in release. The nine-awake-owner unequal-chunk extension passed in debug and final release. Existing core suites: 7 query, 35 retrieval and 9 three-factor tests passed; the existing core microbenchmark remains ignored.
- Existing backend CPU regressions: 29 buffer contracts and 14 memory-context contracts passed. The existing app object-bound concept/gap test passed in release.
- Strict Clippy all-targets with `gpu-runtime` passed debug/release. Normal app and `foundation-training` feature code checks passed; no training executed. Formatting, full core dependency/source boundaries and 77 documentation assertions passed.
- Independent R2 Sol6.1 review cleared source, frozen-oracle/equality/failure tests and documentation. This receipt does not certify CI, desktop integration, renderer responsiveness or hardware performance.

Parent integration should take production commit `f03d2dc3` after its existing finalization integration; do not duplicate the prerequisite cherry-pick `1c53224a` if `4eed7acc` is already present. The associated report/receipt commit is separate. Reverting the production commit restores serial preparation, with no save/schema/GPU ABI migration. Operational launch source and main Jules workflows were untouched. Future device evidence remains **Unknown** and requires separately authorized hardware proof.
