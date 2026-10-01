# Fifty-creature optimization plan — 2026-10-01

The highest-value first change is to stop building a complete preliminary
neural frame solely to obtain memory evidence for attention. Retain both
attention-sensitive recall and final routed evidence, but finalize the frame
once. The larger architectural opportunity is a bounded CPU preparation stage
over individual state, together with removing repeated topology reconstruction
and packing. GPU inference is already batched. These are source-grounded
candidates; **20 FPS with 50 creatures remains Unknown**.

## Objective and evidence boundary

Deliver playable, stable care and learning for 50 distinct individuals at
1920×1080 on i7-3770K / 32 GB / RTX 3050 8 GB. Preserve GPU-authoritative neural
decisions, personal memories, meaningful individual exceptions, useful learning,
crash/corruption checks, and fresh-process save/load. Use Creatures 3 / Docking
Station's care, readable response, and continuity as gameplay guidance.

The current normal simulation cadence is 20 Hz (`alife_world/src/headless.rs:66`,
`alife_game_app/src/schema.rs:71`, `double_buffered_scheduler.rs:39`). The target
therefore requires both frames around/below 50 ms and sustained 20 progressed
ticks/s with all due awake cognitive rows completed. Reducing that cadence,
silently dropping catch-up, or only displaying old frames faster is not success.

Reviewed source: memory branch head
`33982d3988f9dd0ca319d7bf2d84996bd5ee2f7e`; coherent prior integration candidate
`c8894f9a43c80f054c312c9d61c24f5ae6e2b1e1`. Memory implementation
`cba0740bb85c7c9cbc9c3ea923aa02ee4d8be3d9` and original upload optimization
`ffeea2528d0475541ff1078a3510762538d71c5e` are already represented in the reviewed
memory source. Parent integration work is combining these with graphics,
persistence and lint repairs. Rebase the measurements and implementation onto
that actual integrated tree; this plan does not certify its unseen changes.

| Evidence | Result | Limits |
| --- | --- | --- |
| Coordinator-supplied older hardware A/B, eight fresh Nano512 creatures at 1080p, first validation optimization `911cd99f` | FPS 5.72 → 10.90; TPS 5.74 → 10.92; runtime 165 → 82.53 ms/tick | Predates the memory-quality/reuse branch; raw desktop receipt is absent here; do not relabel as current-source proof |
| Coordinator-supplied later leaf attribution, 479 ticks on that earlier source | Baseline/routed recall 6.953/6.704 ms; baseline/routed finalization 6.021/5.982 ms; topology observation 7.481 ms per tick | Other GPU activity invalidates comparative FPS inference; full leaf run SHA/tree and raw receipt locator remain Unknown in this checkout |
| Same leaf attribution | Input validation 0.370 ms; batch validation 0.000173 ms/tick | Small checks are not the leading optimization target |
| Earlier GPU timing supplied by coordinator | Inference 2.41 ms; plasticity 0.42 ms | Different timing context; cannot add to CPU leaf data as a measured complete tick |
| [Within-frame recall reuse](20261001-memory-quality-and-recall.md), cloud CPU fixture | Repeated case 1.73372 → 0.50073 ms/call; distinct case 0.54455 → 0.55095 | Test-profile synthetic workload; target game speedup and cross-phase reuse remain Unknown |
| [Upload constructor](20261001-memory-upload-validation.md), cloud release CPU fixture | 47–53% lower constructor time | Does not establish whole-game FPS or additive benefit with recall gains |

These hardware figures are reported measurements supplied to this task, not new
measurements performed here. Older [live-path analysis](20261001-live-tick-path-examination.md)
also identifies preparation dominance, but its earlier source/run differs.

## Cost and scaling model

Measure mutually exclusive wall stages of the actual tick:

`T_tick = preamble + preparation + inference transaction + outcome/sealing +
learning transaction + sidecar/lifecycle work + world/final publication/persistence`.

Split CPU preparation into world/receptors, baseline recall, attention evidence,
routing, routed recall, context/projection, finalization and packing. Split each
GPU transaction into host work, submission/completion delay and measured device
execution. A transaction's wall time already includes its wait and GPU work;
GPU timestamps and nested leaf timers must not be added again.

Let `P` be awake population, `C` candidates per creature, `U` distinct exact
recall groups, `K` searched records (bounded), `D` feature width, `W` world objects,
`N` nearby objects, and `M` topology size. Current preparation is roughly
`O(P*(U*K*D + C*encoding/validation + topology work + local sensing))`, with two
recall/finalization passes. Sorting/sweeping topology costs scale with `M` even
when only a few entries change. The world index build is approximately
`O(W log cells)`; dense local sensing can approach `O(P*N)`. Existing candidate
and search bounds cap downstream work, but do not necessarily bound enumeration
before truncation. Instrument actual `C/U/K/N/M`, allocations, and bytes.

The five reported leaf stages sum to **33.141 ms at eight creatures**. A purely
linear extrapolation gives:

| Population | Selected leaf cost, ms/tick | Baseline finalization portion, ms/tick |
| ---: | ---: | ---: |
| 8 | 33.141 | 6.021 |
| 16 | 66.282 | 12.042 |
| 32 | 132.564 | 24.084 |
| 50 | 207.131 | 37.631 |

This is a capacity warning, not a predicted benchmark: populations, learned
histories, candidate counts, branch changes and hardware contention alter it.
Removing baseline finalization alone leaves about **169.5 ms** of that
extrapolated 50-creature subset. Even ideal division across four cores gives
42.4 ms before remaining serial work, rendering and synchronization; real
parallel speedup is lower. The user hardware has four physical cores, so use
measured effective capacity, not eight logical threads as eight full workers.
Serial preparation at 50 ms gives less than 1 ms per creature after overhead.
This makes both work removal and controlled parallelism worth testing.

## Baseline, alternatives, and decision

**Baseline:** integrate known fixes, profile 8/16/32/50, optimize the dominant
leaves in place, reuse allocations, and retain ordered execution and existing
checks. It has low integration risk but leaves the serial ceiling intact.

**Broader alternative:** move bounded recall, context preparation, and topology
onto the GPU with resident per-organism storage. This could remove host scans
and packing; it also adds index maintenance, transfers, synchronization, layout
and restore work. Current device timings are small compared with CPU
preparation. No measurement justifies this wholesale relocation yet.

**Selected hybrid:** profile the integrated source, split preliminary attention
evidence from finalization, remove redundant work at production boundaries,
then parallelize independent individual preparation and optimize topology's
mutation path. Consider GPU relocation only for a measured remaining regular
workload. Separate render scheduling is an additional responsiveness change.

The following scores are design judgments, not measured speed predictions.
Five means favorable, including lower effort/overhead:

| Dimension | Baseline | Broad GPU relocation | Selected hybrid |
| --- | ---: | ---: | ---: |
| Correctness likelihood | 5 | 2 | 4 |
| Simplicity | 5 | 2 | 4 |
| Implementation effort | 5 | 1 | 3 |
| Execution steps | 5 | 1 | 3 |
| Moving parts | 5 | 2 | 3 |
| Failure points/isolation | 5 | 2 | 4 |
| Bottleneck resolution potential | 2 | 4 | 4 |
| Observability | 4 | 2 | 4 |
| Testability | 5 | 2 | 4 |
| Maintainability | 5 | 2 | 4 |
| Resource overhead | 3 | 2 | 4 |
| Reversibility | 5 | 2 | 4 |

## Source-grounded work packages

### A. Attention evidence first; final neural frame once

At `gpu_live_runtime/staged_tick.rs:580–681`, a serial awake-creature loop calls
baseline recall, constructs context, clones recall/context/draft, finalizes a
complete frame, validates it, derives attention evidence, routes the draft,
then recalls and finalizes again. `memory_query.rs:64–109` derives attention
evidence from recall context and query identity; it does not inspect the
baseline neural frame. `memory.rs:268–282` binds cognitive context without
altering retrieved candidate values. This supports a bounded extraction seam.

First expose attention evidence from validated `PreparedMemoryRecall` plus
draft/query identity. Share the existing salience formula and required checks;
retain baseline context, topology, receptors and hysteresis. Eliminate only the
complete preliminary frame and its discarded final keys. Keep the final routed
recall, finalized frame, selected episodic key and GPU upload unchanged.

Then test **cross-phase recall reuse**: use a frame-local cache of channel
results keyed by organism, bank generation, profile/schema, exact state/target
features, target identity and (for family results) exact action features/family.
The current reuse is inside one `MemoryBank::recall_frame` call
(`memory.rs:894–1049`); it does not cover the two calls. Routing changes candidate
indexes, novelty and frame identity. Reuse only unchanged channel values;
re-encode candidate-local queries/keys against the routed draft and new digests.
Never reuse a finalized key or authorize reuse by quantized bucket equality.

Benefit: one fewer full construction/validation and potentially fewer repeated
searches. Failure: attention actually requires a field that the extracted view
omits, or routing invalidates most cache entries. Observe comparison counts and
finalization calls as well as time. No baseline recall or memory influence is
removed merely because its output appears similar in a fixture.

### B. Bounded parallel preparation, deterministic collection

Source currently prepares each awake creature serially before one GPU batch.
World draft extraction mutates per-organism tracking
(`alife_world/src/headless.rs:2671–2742`), semantic-prior preparation may mutate,
and sleep/checkpoint work shares backend/state. Do not put that whole loop into
a parallel iterator.

Keep lifecycle, residency, canonical biology, tracking updates and genuinely
mutable prior work ordered. Freeze a tick-bound read-only view; prepare recall,
context, routing and host packing on disjoint per-organism inputs. Workers
return prepared values plus proposed hysteresis changes. Collect in stable ID
order and commit resident changes only after the applicable checks succeed.
Keep world outcome resolution/sealing deterministic and the GPU action batch
authoritative. Profile one, two, then three workers; reserve capacity for render
and shared work. Avoid cloning entire memory banks to cross the thread boundary.

Benefit: reduce the population-linear serial span. Risks: pool overhead at eight,
cache/bandwidth pressure, shared cache locks, source types that cannot be safely
sent, or nondeterministic proposal/commit order. Reject if total wall time,
behavior, transaction order or worst-frame latency worsens.

### C. Topology mutation and versioned preparation artifacts

`alife_core/src/topology.rs:1542–1642` clones the map, mutates the clone, validates
and hashes it, derives replacements, then calls `TopologyMutationPlan::validate_against`.
That function (`2058–2105`) hashes the original and clones/reconstructs another
map for exact equality. `advance_lifecycle` (`2198–2211`) adds another staged
clone. `gpu_live_runtime.rs:3999–4071` sorts topology concepts/gaps twice per
awake preparation through the two cognitive-context calls.

Start with a smaller change: reuse one validated, version-bound top-concept/gap
summary for both context constructions; use bounded top-k selection instead
of sorting all entries if measured. Cache compiled immutable receptor expression
by phenotype hash/graph epoch at admission or restore; current
`staged_tick.rs:545–553` recompiles it per awake row. Canonical receptor values
remain fresh per tick. Keep buffers/scratch per worker and re-use capacity.

Next evaluate a **single staged topology mutation**: retain the already-built
candidate map until commit and eliminate the second reconstruct-and-compare
oracle in production, while retaining checked IDs, finite values, capacity,
references, replay rejection, version binding and atomic rejection. Use the
old exact reconstruction in focused tests/debug qualification. If whole-map
cloning remains dominant, consider bounded touched-entry preflight and undo;
that is a separate, higher-risk step and must prove rollback.

Topology is labelled diagnostic in local guardrails, but its concepts/gaps feed
actual attention and context. Do not disable, defer, or silently stale that
evidence to obtain a timing win. Keep canonical digest semantics; avoid naive
incremental BLAKE3 patching. Compute a digest once per committed version where
consumers permit it, rather than recomputing a proof with unchanged inputs.

Benefit: remove map-wide proof and sorting repetition without less learning.
Risks: memory overhead of retaining a candidate, incorrect invalidation,
allocation failures after partial mutation, or misunderstood diagnostic checks.
Measure clone/validate/hash/sort separately; the 7.481 ms observation total
does not establish how much any one of these contributes.

### D. Spatial, layout, and transfer scaling

A spatial batch index already exists (`headless.rs:1914–1940`); nearby lookup
visits 27 cells (`4773–4797`). Optimize the remaining work rather than proposing
a new index as if none exists. Compare spread and crowded populations;
count enumerated neighbors before truncation. For GroundedTerrainVision, profile
terrain/obstacle visibility separately from the object-slot baseline. Extend
an obstacle broadphase or incrementally update cells only if those paths bind.
Keep candidates grounded, unscored and deterministically ordered.

Current bounded recall still uses tree indexes and temporary shortlists
(`memory/candidate_index.rs`, `memory/candidate_recall.rs`). If cache misses and
layout dominate after A/B, test contiguous per-organism records and shortlists,
worker-local scratch and shared immutable feature representations. Preserve
stable persistent IDs, category/individual exceptions, feature exactness,
retention, cap bounds, and derived-index rebuild on restore. No global shared
memory or learned-policy cache is proposed.

Inference already submits a cohort and waits once
(`alife_gpu_backend/src/closed_loop_runtime/tick.rs:510–544`). Learning and
readbacks are separate dependent transactions. Measure class/chunk submissions,
bytes and host wait before adding synchronization machinery. Reuse stable upload
arenas; pack once where possible and submit grouped independent learning chunks
only while preserving their atomicity/fail-stop contracts. Do not blindly fuse
inference and plasticity across the CPU-selected physical outcome.

A GPU recall prototype is justified only if remaining bounded feature distance
or gather cost dominates. Include persistent per-organism segments, bank
generation invalidation, CPU update/restore transfer cost, shader bounds and
result readback. A raw feature bank of `50*256*96*4` is about 4.7 MiB, but that
illustration excludes all indexes/neural arenas and proves neither VRAM
admission nor speed. Reject offload if transfer/wait increases the critical path.

### E. Render from committed presentation independently

`bevy_shell.rs:737–740` runs synchronous `tick_outcome` inside the graphical
system. Its normal pacing permits one tick per frame (`714–732`). A
double-buffered scheduler name alone does not mean the heavy tick runs on a
worker. Evaluate one runtime owner receiving bounded commands and publishing
immutable committed presentation snapshots. Audit thread/device ownership
first; keep Bevy objects on their appropriate thread.

Use latest-state presentation with tick/version and snapshot age; never an
independently advancing canonical world. Preserve command order, current-world
legality, pause/step sealed boundaries, atomic load, save completion and lifecycle
archive ordering. Bound outstanding work instead of accumulating ticks or input
without limit. Shared GPU compute/render contention still needs measurement.
This can improve frame pacing while TPS stays low; report that as a
responsiveness result until the 20 Hz cognitive budget also passes.

## First bounded implementation and milestones

This task changes documentation/skill only. Runtime packages below are future
work; avoid overlapping the current integration workers.

1. **Bind the integrated baseline.** Reuse the current performance receipt and
   counters (`production_voxel_renderer/performance_receipt.rs`); record source,
   tree, executable, features, founder/save, profile, backend, adapter, mode and
   diagnostic settings. Recover the older raw receipts if available. Profile
   8/16/32/50 and distinguish exclusive leaves. Done: no unexplained source or
   cadence mismatch; missing target evidence remains Unknown.
2. **Implement A's single-finalization seam.** Affected future files:
   `alife_core/src/memory_query.rs`/`memory.rs`,
   `alife_game_app/src/gpu_live_runtime/staged_tick.rs`, and relevant existing
   memory/attention tests. Add a prepared-evidence helper with immutable inputs,
   no new save schema, no GPU layout change, and no second complete frame.
   Compare old/new attention evidence for empty/populated/reversal/individual
   exception fixtures; reject stale, foreign and malformed inputs. Preserve the
   existing causal attention-upload regression. Done: one finalization per
   successful awake row, same selected evidence boundaries, target timing receipt.
3. **Select the next measured constraint.** If recall still dominates, prototype
   cross-phase exact channel reuse. If topology dominates, apply C's summary/
   single-candidate change. If serial span dominates after duplicate work removal,
   apply B with 1/2/3 workers. Change one mechanism per A/B. Record the actual
   decision in this plan rather than executing every candidate automatically.
4. **Scale and prove the gameplay path.** Run the matrix below and one evolving
   care/learning/sleep/restore/lifecycle scenario. Evaluate D/E only where their
   measured cost or responsiveness justifies them. Done: source-bound target
   receipts and the important existing checks pass; otherwise report the binding
   stage and smallest next action without promoting the target.

## Benchmark and validation matrix

Do not run the user's desktop or load its GPU from this cloud task. Target tests
require an explicitly authorized future window before 09:00 Alberta; do not
infer permission from that condition alone. Cloud CPU tests validate mechanisms,
not the i7/RTX target. No training campaign is required or authorized here.

| Axis | Bounded first matrix |
| --- | --- |
| Population | 8, 16, 32, 50 awake independent creatures; record birth/death count and actual resident rows |
| Source | Integrated baseline vs one candidate; separate build outputs/executable hashes; three alternating A/B pairs |
| Common configuration | Same 1080p profile, Vulkan adapter, founder, seed, world, diagnostic flags and 20 Hz schedule |
| State | Fresh records and a copied learned-memory save; spread/crowded 50-creature case; avoid changing the learned-bank workload between paired runs |
| Timing | Warm startup separately, retain a 60 s steady interval, p50/p95/p99/max frame/tick and deadline misses; preserve all samples and timestamp coverage |
| Useful work | FPS, progressed TPS, due/completed awake rows per organism, zero-progress reasons, catch-up debt, search groups/evaluations, topology size, work receipts |
| Resources | CPU occupancy, exclusive preparation, host waits, GPU execution, upload/readback bytes, allocation/copy counts, RSS and peak VRAM |
| Scheduling | 1x rendered care first; max/headless-max only as separate capacity diagnostics; 1/2/3 CPU workers only for the parallel prototype |

The existing CLI exposes `production-voxel --profile Balanced1080p --population P
--resolution 1920x1080 --graphics-backend vulkan --brain-policy gpu-required
--require-gpu --run-mode 1x --record-performance --smoke-seconds 90`. Its New Game
flags include `--new-game --seed N --founder builtin-nano512`; learned runs need
an isolated copied save/settings path instead. Confirm the integrated CLI and
asset manifest, preserve each output immediately, and exclude startup from the
retained interval. This is a future invocation template, not a command run here.

| Success or counterexample | Smallest relevant validation | Failure response |
| --- | --- | --- |
| 20 FPS and 20 progressed ticks/s at 50 | Named target adapter, p95 frame and tick near/below 50 ms, awake rows account for every due interval, no growing debt; report tails/bursts separately | Keep target Unknown/failed; identify the binding exclusive stage |
| Attention stays memory-sensitive | Existing `v11_attention_causally_changes_finalized_upload_and_holds_top_k_primary`; prepared vs finalized evidence on empty, learned, poisoned-instance and changed-context cases | Restore old preliminary path and repair the helper |
| Individual memory/learning preserved | Existing `candidate_memory_retrieval`, `candidate_memory_queries`, isolation and reversal/retention tests; later ordinary GPU lesson and independent response | Reject shared/stale reuse; no behavioral promotion from CPU tests alone |
| Topology change remains atomic | Existing `topology_sidecar_degradation` invalid/replay/capacity cases, exact reference comparison in qualification and targeted rejection after a late invalid replacement | Keep one validated candidate or revert; do not drop rollback |
| Parallel execution is causal | Stable organism/result ordering, identical seeded inputs, stale-view/foreign-owner rejection; no lost hysteresis updates; serial vs parallel behavior within existing tolerances | Revert scheduling change; fix ownership/commit before retiming |
| Sleep, birth/death and exact persistence work | Relevant existing sleep atomicity, memory crash-phase and fresh-process restore checks; care scenario with pause/save/load and archive before insertion/retirement | Preserve original save, abort partial commit, revert candidate |
| Source/doc boundaries | `scripts/check_core_boundaries.sh`, `scripts/docs_check.sh`, focused Rust checks for actual implementation; `git diff --check` | Repair the affected surface only |

For A, start future CPU verification with
`cargo test -p alife_core --test candidate_memory_retrieval --test candidate_memory_queries -- --test-threads=1`.
The attention regression is GPU-gated; run it only in the authorized hardware
environment with the source's required features. Do not substitute a dummy CPU
neural path. Broaden existing checks only for failures or the implementation's
actual additional scope.

## Recovery, progress, and receipt

Classify timing failures separately from semantic failures. Repair the smallest
local defect, rerun the relevant check, and retain the prior path if benefit is
unproven. Invalid IDs, epochs, stale/foreign views, nonfinite state, failed
learning, device loss and failed multi-system transactions remain explicit;
there is no CPU neural fallback. Save capture stays at sealed boundaries and
birth/death archiving precedes GPU admission/retirement.

Execution progress is recorded by checking the existing roadmap tasks and
appending concise milestone results here. Material decisions: selected hybrid;
single-finalization first; no unmeasured GPU rewrite; no cadence reduction;
runtime implementation deferred to the integrated source. Record deviations
with source, reason and evidence rather than adding a separate ledger.

Final receipt must state source/tree/config/hardware, files changed, existing
checks and results, paired raw measurements, awake work/cadence, observed
behavior/persistence, limitations, and the measured next constraint. Current
receipt: planning/skill only; target performance and live behavioral proof
remain Unknown.

Planning validation: official skill frontmatter validator passes; existing
documentation assertions pass 77/77; local links, cost arithmetic and whitespace
check pass. Independent Sol 6.1 source review reports no P0/P1 findings.

Rejected approaches: lower cognition to inflate FPS; share personal experience
across creatures; skip memory-driven attention; remove crash/corruption/learning
guards; count faster old-frame presentation as cognitive throughput; add another
GPU batching layer without noticing the existing batch; or implement every
speculative optimization before measuring the integrated tree.
