# Read-only preparation scaling and fresh recall validation — 2026-10-01

Implementation: `e8d5633edfbc7dba404007607e26c7d89f49948c`, based on `b94c9f120aff7b53db5accf929f368486bc4df78`. The eventual coherent integration base is `dc3e916eb3d47db936048b93224a85bccec8145b`; this branch retained its fixed measurement baseline. This is dated source/CPU evidence, with no live threading implementation, GPU campaign, desktop test, training, merge or PR.

The decision is to ship the small fresh-recall self-check optimization. Parallel preparation has useful measured potential, especially with two workers, but live integration requires proving ownership across sleep mutations and preserving hint/backend failure ordering. The prototype does not establish a safe whole-loop replacement. Production preparation and world commit order remain unchanged.

## Source and dependency audit

The pure seam starts after each owned grounded draft has consumed its existing serial semantic hint. It reads immutable organism-owned memory, topology and predictor state plus copied homeostasis, receptors, attention policy and prior hysteresis. Recall, attention/routing, routed recall, cognitive projection and finalization can return owned frame/recall results and a hysteresis delta. Rust scoped-thread compilation confirms `Send`/`Sync` for the prototype's actual input/output types, without forced unsafe implementations.

The surrounding live preparation loop is stateful:

| Boundary | Current dependency and consequence |
| --- | --- |
| Sleep/admission prefix | Learning retries, resident synchronization, sleep scheduling, predictor/memory/topology updates, world biology and checkpoint operations mutate authoritative state. |
| Grounded draft | `HeadlessWorld::perception_frame_draft_indexed` requires mutable world access; sensor extraction updates tracked-object state. Keep draft creation serial. |
| Semantic hint | `RuntimeSemanticPrior::prepare` polls replies, updates shared lives/cache/request order, consumes developmental exposure and may write the cache to disk. Keep its existing serial consumption points. |
| Attention selection | Resident hysteresis is assigned before routing; rows failing later still retain that assignment. A worker result must preserve this delta on later failure. |
| Backend upload | `prepare_memory_context_upload` requires mutable backend access, checks live ownership/readiness and may mark device loss. CPU encoding alone does not replace these checks. |
| Result/commit | Errors, receipts and GPU rows must follow scheduled organism order. Neural dispatch, selected-world effects, patch sealing and learning/persistence remain serial authority operations. |

These boundaries are visible in `gpu_live_runtime/staged_tick.rs`, `gpu_live_runtime/semantic_prior.rs`, `alife_world/src/headless.rs` and `alife_gpu_backend/src/closed_loop_runtime.rs`. The outer staged tick can roll back host/world/sidecar state; semantic cache/disk and GPU operations are not covered by that rollback. Moving every serial prefix ahead of every pure suffix can alter irreversible side-effect ordering, backend failure timing and which asynchronous hint is available. Borrowing earlier residents' banks/topologies across later sleep mutations also conflicts with the current map access pattern. Cloning full banks as snapshots would add work and memory; changing storage/ownership or the admission phases exceeds this bounded slice.

Future integration should begin with a narrowly guarded path and an existing serial fallback, prove mixed sleep/prior/error cases, and retain backend admission on the main thread. Existing worker-stage wall sums would overlap and must not be treated as disjoint frame time. Rendering responsiveness needs its own evidence: a scoped join still blocks the tick caller. This experiment establishes CPU preparation throughput only.

Acceptance scope is causal identity and sequence (`AOA-INV-011`, `AOA-AUTH-004`), evidence-only owned memory (`AOA-MEM-001`–`AOA-MEM-003`), grounded concept/gap influence (`AOA-CON-004`, `AOA-CON-005`), and strict saved cognitive state (`AOA-PERSIST-002`, `AOA-PERSIST-004`). It does not certify complete architecture capability.

## Lower-risk production change

`MemoryBank::recall_frame` builds an owned, private-field, non-deserializable `PreparedMemoryRecall` from an immutable validated draft. Its final self-check re-encoded the same candidate queries and recomputed derived receipt/context identities. This fresh-result self-check now runs in debug builds. The change is two comments and one `#[cfg(debug_assertions)]`.

Unconditional config/mode/draft validation, per-candidate encoder/profile/slot checks, bounded search and exact within-frame reuse, context/finite checks, complete bank/capacity/record digest validation and receipt validation remain. Public `validate_for_draft`, prepared attention and finalization stay strict; final frame/upload and save/restore validators are unchanged. Every baseline and routed recall still runs. No frequency, population, physiology, learning, neural execution, memory work or authority changed. There is no schema/ABI migration.

The separately timed single public validator pass costs 4.053 ms for the first baseline 50-call/26-candidate/64-record cohort. This is an unbound pre-routing recall measurement, not both real recall passes; it is not doubled into a savings prediction. The actual complete helper comparison below measures the source change directly.

## CPU prototype and measurements

[Raw times, source/build identities and result digests](evidence/20261001-parallel-preparation.json), [the bounded probe](evidence/20261001-parallel-preparation-probe.rs) and [checked replay builder](evidence/20261001-parallel-preparation-build.py) retain 24 cases: 2/26 input candidates, 0/64/256 actually retained memory records, and 8/16/32/50 independent owners. Each source variant has two process runs with five samples per mode/case, rotating serial/two/four mode order; the second process reverses the initial mode order. Baseline processes ran before optimized processes. Ten-sample pooled medians average the two middle values, with no confidence-interval or host-isolation claim.

For 26 input candidates and 64 records, complete CPU helper wall time:

| Independent owners | Baseline serial ms | Optimized serial ms | Optimized 2 workers ms | Optimized 4 workers ms |
| ---: | ---: | ---: | ---: | ---: |
| 8 | 10.568 | 8.918 | 4.901 | 7.044 |
| 16 | 20.673 | 18.466 | 12.365 | 7.516 |
| 32 | 42.008 | 37.625 | 21.230 | 22.124 |
| 50 | 66.717 | 57.593 | 30.284 | 23.633 |

The serial production-source comparison improves thread/process CPU 10.4–15.4% in these four cases. Across the full grid, the maximum observed serial CPU reduction is 19.4%; one small 2-candidate/256-record/8-owner case is essentially flat, with 0.02% greater CPU time. The actual new serial 50-owner chain still exceeds 50 ms.

At 50 owners/26 candidates/64 records, total process CPU and worker CPU expose the parallel cost:

| Source/mode | Wall ms | Process CPU ms | Summed worker CPU ms |
| --- | ---: | ---: | ---: |
| Baseline serial | 66.717 | 66.697 | 66.568 |
| Baseline 2 workers | 33.889 | 66.864 | 66.464 |
| Baseline 4 workers | 31.873 | 91.266 | 90.165 |
| Optimized serial | 57.593 | 57.588 | 57.426 |
| Optimized 2 workers | 30.284 | 58.690 | 58.266 |
| Optimized 4 workers | 23.633 | 69.190 | 68.603 |

Two-worker baseline 50-owner process wall medians are 33.686 and 34.388 ms. Four-worker baseline medians vary from 67.310 to 26.246 ms, with greater total CPU; its small cohorts can be slower than two workers. No universal four-worker scaling claim follows from these samples, and the cause of variation was not isolated.

Capture, newly spawned bounded worker threads, joins, deterministic ordered result collection and capture destruction are inside the work clocks. Every row executes the same arithmetic and validators within each source variant. The coordinator performs no row preparation in parallel modes. The optimized 50-owner median owned-draft capture is 0.102–0.112 ms across modes. Separate empty-cohort capture/spawn/join/collection timings are 0.004/0.228/0.323 ms for serial/two/four; these are an overhead comparison, not a subtraction from full preparation. Linux process CPU includes coordinator and workers; summed thread CPU counts the row workers only.

Process RSS snapshots are taken before capture and with returned outputs retained. The serial oracle stays alive for comparisons. Libraries, CPU bucket/fixture storage, allocator history and earlier proof serialization affect RSS and cumulative high-water marks. Across the four processes, recorded peak RSS is 139.7–145.6 MiB. The 50-owner/26-candidate/64-record baseline RSS-with-output maxima are 142.9/142.2/142.9 MiB for serial/two/four; optimized maxima are 142.45 MiB in all three modes. These are process measurements, not fresh per-mode peaks, allocation counts, live-game memory or evidence of a memory saving. Every raw snapshot is retained; transient serialized input buffers are dropped before clocks.

The host is Linux x86_64/KVM on five exposed AMD EPYC 9V74 CPUs with a four-core quota (`400000/100000` microseconds). All modes share CPU affinity 0–3 and report available parallelism four. Rust is `1.98.1`, named release rlibs use opt level three with debug assertions disabled. Other host tenants are uncontrolled. No other local build or measurement was run during timed probes.

## Equivalence and limits

The probe reuses the protected attention probe's preparation function unchanged except visibility, and extracts the same private app helpers verbatim. Fixture changes assign distinct owners 811 onward, vary first-object geometry over five distances, and reseal owner-specific memory patches through validated public constructors. Each owner has a separate bank/topology and distinct CPU slot index. Inputs are borrowed immutably, owned drafts are captured inside clocks, and contiguous chunks are joined in original input order. No shared mutable simulation state is introduced.

The helper includes both recall/context stages, attention/familiarity/routing/novelty, projection/finalization/full guards, and pure CPU perception/memory/cognitive upload encoding. It uses two objects, one learned concept/gap, fixed valid receptors, a default predictor/hysteresis and fixed focal-capacity-one policy. It uses the existing N512/54-lane encoding fixture; no experimental fixture, brain promotion/default or neural execution was changed. The helper accepts valid fixtures with `unwrap`; it does not test live failure ordering or learned-predictor behavior. Backend readiness/ownership admission, world/sensor/receptor derivation, actual resident history, hints, sleep/lifecycle, topology observation, save I/O, GPU inference/readback/learning/synchronization and rendering are excluded.

All 1,440 timed batches compare complete typed outputs against their per-process serial oracle, covering 38,160 owner instances. Ordered proof hashes additionally cover frames, attention, recall contexts/keys/receipts, cognitive context and literal perception/memory/cognitive payload words. All 24 initial-input/final-proof hashes match across the two source variants and four processes; all banks/topologies remain unchanged. Full payload/state dumps are regenerated, not duplicated in the receipt.

The baseline executable and build-time identities were preserved before the source change. Cargo reused and overwrote some baseline rlib filenames during rebuilding; their historical hashes are labelled as build-time identities. The initial builder lacked replacement-count assertions. The published builder adds them, generates byte-identical Rust, and has a separately recorded SHA. The receipt retains both historical and replay identities rather than claiming overwritten files still match their old hashes.

Core `candidate_memory_retrieval` passed in release and debug: 35 tests each, one intentionally ignored microbenchmark. This includes the 72-case attention/memory matrix, stale geometry/routing/novelty/owner checks, independent individual exceptions, replay, saturation/contradictions, compaction, malformed/mixed-mode serialization, portable tampering/restore and exact recall persistence. Seven release `candidate_memory_queries` tests passed, including exact query re-encoding, private-field authentication and nonfinite/noncontiguous context rejection. The existing CPU app concept/gap preparation equivalence test passed before and after in release with `gpu-runtime`, without `gpu-tests` or a device. Strict core Clippy passed in debug/release, along with formatting, core boundaries and 77 documentation assertions. Separate read-only Sol6.1 R2 review covered source, fixture adaptation, thread ownership, clocks, provenance and quantitative evidence.

**20 FPS with 50 creatures, sustainable 20 TPS and bounded debt/drops remain Unknown.** The older 64.216 ms attention chain used a different fixture/affinity and is not relabelled with these results. Independent topology timings must not be added to this prototype to fabricate a live tick estimate. Rendering responsiveness and RTX3050/i7-3770K throughput remain unrun.

## Reproduce

From the final evidence checkout, build baseline and optimized CPU libraries in separate worktrees. The builder requires this repository's protected attention probe, core test fixture, app helper source and foundation asset. It does not invoke a provider or create a device.

```bash
source /workspace/cloud-cpu-validation/env.sh
git worktree add --detach /tmp/preparation-baseline b94c9f120aff7b53db5accf929f368486bc4df78
(cd /tmp/preparation-baseline && cargo test -p alife_game_app --release --lib \
  --features gpu-runtime object_bound_concepts_and_gaps_change_predecision_focus_selectively \
  --message-format=json) > /tmp/preparation-before-artifacts.jsonl
cargo test -p alife_game_app --release --lib --features gpu-runtime \
  object_bound_concepts_and_gaps_change_predecision_focus_selectively \
  --message-format=json > /tmp/preparation-after-artifacts.jsonl
python3 docs/performance/evidence/20261001-parallel-preparation-build.py \
  /tmp/preparation-before-artifacts.jsonl /tmp/preparation-before
python3 docs/performance/evidence/20261001-parallel-preparation-build.py \
  /tmp/preparation-after-artifacts.jsonl /tmp/preparation-after
taskset -c 0-3 /tmp/preparation-before > /tmp/preparation-before-1.jsonl
taskset -c 0-3 /tmp/preparation-before --reverse > /tmp/preparation-before-2.jsonl
taskset -c 0-3 /tmp/preparation-after > /tmp/preparation-after-1.jsonl
taskset -c 0-3 /tmp/preparation-after --reverse > /tmp/preparation-after-2.jsonl
```

Mode 1 Micro-Spec; R2 review. Rollback is reverting `e8d5633e`. Source ownership is only `alife_core/src/memory.rs` plus these four dated evidence files. Operational launch source, other workers' owned areas and live parallel/persistence architecture were not edited.
