# Performance time ledger

Updated 2026-10-31. Source audit: main
[`eaf9e12067d649e9e27c16e6600e6e8dabfc7e99`](https://github.com/BobertNeek/A-Life/commit/eaf9e12067d649e9e27c16e6600e6e8dabfc7e99).
This ledger records what consumes time, what that work buys, and experiments
that could show it should be cheaper or less frequent. Append new measurement
cohorts with their provenance; do not replace historical timings with estimates
for newer code. **Current device and whole-training costs are unknown, not zero.**
The project desktop GPU is reported unavailable because driver startup is broken; this
documentation change runs no desktop workload and transfers no real checkpoint.

## What to optimize

Keep these outcomes separate:

| Outcome | Denominator and required context |
| --- | --- |
| Gameplay responsiveness | Rendered frames/s, frame-time median/p95, and neural/world tick latency on the actual device, graphics settings, population, brain/body classes and scene. A renderer improvement does not establish faster learning. |
| Simulation throughput | Unique progressed world ticks / declared ticks per simulated second / elapsed wall seconds. Count sleep and uncaptured ticks explicitly. Do not substitute captured rows or repeated replay exposures for world progress. |
| Retained learning throughput | Newly trained decision rows actually retained by admitted final actor/value/private state, divided by inclusive host hours. Exclude bootstrap-only, failed, untrained and rolled-back rows from the numerator; their elapsed cost still belongs in the run denominator. Report actor/value optimizer ages separately from unique rows. |
| Useful capability per hour | Retained learning/hour together with later unprompted food finding, nourishment, survival, retention/transfer and assistance required. More rows or optimizer steps alone do not establish better learning. |

Genetics, learned instincts and runtime plasticity remain goals. Optimizations
must identify whether they preserve execution semantics or change the organism's
learning/cadence. The owner's acceptance of fractional-second staleness and
occasional loss of recent work makes bounded cadence experiments relevant; it
does not define a new numerical staleness or loss budget. See the
[compute and persistence design](../architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md#37-compute-architecture-and-work-accounting).

## Evidence cohorts and limits

| ID | Date, code and hardware/workload | Source and limits |
| --- | --- | --- |
| H1: combined night | 2026-10-28, Windows RTX 3050, continuing assisted care of one N2048 individual; 139,264 retained rows and 6,976.8 retained simulated seconds reported. Multiple code segments; b817 was the later active segment, not the identity of the entire aggregate. CPU/RAM, exact driver/API and build profile are not established here. | Project operational report at 15:01 UTC and the 2026-10-31 task handoff: 56,738 rows/hour, 0.790 sim/wall, cumulative collection 5,840 s, PPO 2,284 s, snapshots 321 s. Raw inclusive `host_wall_seconds`, complete per-segment receipts/binary hashes, exact aggregate coverage and cohort membership were not supplied in this checkout. These are transcribed historical observations, not a newly reproduced benchmark. |
| H2: later segment | First eight 512-row windows (4,096 rows), reported 2026-10-28 12:39 UTC, Windows RTX 3050, combined PR202/203 build [`b817c1513a6023a5395e89b9e5fc7374b0c028c2`](https://github.com/BobertNeek/A-Life/commit/b817c1513a6023a5395e89b9e5fc7374b0c028c2); PR204 was test-only. | Operational report and task handoff: mean collection 20.94 s/window, mean PPO 8.63 s/update. Earlier PR203-only sample: 24.43 and 16.13 s respectively. Different sequential trajectories and assistance; not paired or controlled. No whole-host or snapshot denominator for this slice. |
| H3: isolated cache copy | [PR214](https://github.com/BobertNeek/A-Life/pull/214), head `a63010a1`, 2026-10-30; Linux, rustc 1.98.1, opt3 source-extracted types/cache, synthetic finite N2048 payload, 25 candidates/tick, seven alternating paired trials. CPU model/RAM not established in the cited report. | Old cache-hit deep-copy median 5.20 ms [4.47–6.10] for 256 ticks/no burn-in; 7.98 ms [7.80–9.15] for 384 ticks/128 burn-in. Shared hits remove payload copies. First retain still compacts/copies. Windows memory guard stubbed in this assay. No admitted trajectory, GPU, whole-PPO or learning/hour measurement. |
| S1: current source | 2026-10-31 audit of main `eaf9e120`; no timing workload run. | Code boundaries below establish what timers include, not how expensive current work is. [PR210](https://github.com/BobertNeek/A-Life/pull/210) changed host authority/replay costs; [PR211](https://github.com/BobertNeek/A-Life/pull/211) removed disabled PPO value-head work; [PR213](https://github.com/BobertNeek/A-Life/pull/213) changed checkpoint serialization; PR214 changed cache-hit ownership. Historical proportions cannot certify today's proportions or device speedup. |
| C1: boundary-export observer | 2026-10-31, Linux x86_64 AMD EPYC 9V74, rustc 1.98.1 release; actual candidate observer with the source-extracted 116-u64 runtime metrics type, synthetic compact zero-counter and 512-row window receipts. Seven alternating paired trials, 5,000 events/arm/trial after 500 warmups/arm. | [Export-cost record](continuous-runtime-metrics.md#isolated-export-cost-linux-cpu-2026-10-31): added paired median 73.09 us/event [71.78–85.03] for zero counters, 69.48 us [63.92–73.00] populated. Includes JSON construction/encoding and allocation-request instrumentation; excludes initial snapshot, output delivery and all runtime/GPU/training work. Added 561 allocation/reallocation requests and 94,095 requested bytes/event, not peak RSS. This measures diagnostic cost, not any current training stage. |

Earlier that night, **8,704 unsaved rows were lost** (reported in the handoff).
They are not an extra retained gain. Raw receipts must establish whether their
elapsed work and reconciliation downtime are already in H1's denominator and
whether their stage counters occur in H1's totals. Do not silently add or remove
them. H1's operational report also records substantial teaching/food repositioning;
its nourishing outcomes do not prove independent food finding or survival.

## Historical amounts and conditional reconciliation

H1's exact inclusive denominator **T is unknown** until the raw receipt is
available. The reported sim/wall rate suggests roughly 8,832 s (2.45 h).
For transparency, `6976.8 / 0.790 = 8831.39 s`, while
`139264 * 3600 / 56738 = 8836.24 s`. These reported rates do not recover an
identical exact denominator. Use raw inclusive elapsed, not either inversion,
for a future definitive reconciliation.

| Stage | H1 amount | Conditional share if T ≈ 8,832 s | Current main amount/share |
| --- | ---: | ---: | --- |
| Collection, inclusive of its tick/capture/replay work | 5,840 s cumulative | ≈66.1% | Unknown s and % |
| Training timer named PPO, inclusive of preparation and residual critic | 2,284 s cumulative | ≈25.9% | Unknown s and % |
| Full periodic/final snapshots | 321 s cumulative | ≈3.6% | Unknown s and % |
| Other host time, derived only under the coverage assumptions below | ≈387 s if T = 8,832 s | ≈4.4% | Unknown s and % |
| Inclusive host interval | ≈8.83 ks inferred; exact unknown | 100% by definition | Unknown s |

The three reported stage amounts sum to **8,445 s**. Conditional accounting is
`R = T - collection - PPO - snapshots`; with the alternative rate-derived T,
the arithmetic remainder is about 391 s. Neither result is a measured residual
stage, a statistical interval, nor proof that all timers cover the same cohort.

Inspection of the
[b817 continuous producer](https://github.com/BobertNeek/A-Life/blob/b817c1513a6023a5395e89b9e5fc7374b0c028c2/crates/alife_game_app/src/foundation_training_cycle/continuous_world.rs#L834)
confirms that these three host intervals are sequential and do not nest in one
invocation: collection closes before replay seal/boundary wait; PPO closes before
policy installation; snapshots run after installation/event handling or at the
final boundary. This permits addition **within matching invocation coverage**.
It does not verify H1's aggregation, earlier segments, failed work, or host wrapper
coverage. Persistence workers and GPU work can overlap host work; wall spans
include waits and are not independent CPU and GPU service times.

H2's two reported means total 29.57 s versus 40.56 s for the earlier sample.
That is 27.1% less elapsed time across those two stages, or a 37.2% higher
reciprocal stage rate. It is an uncontrolled observation, not an attributed
code speedup or retained rows/hour improvement. Do not extrapolate it across H1.

### Conditional Amdahl scenarios

If H1's provisional shares are exclusive, cover the same elapsed cohort, and
remain representative, changing only a fraction `s` by a factor `k` gives the
overall scenario `1 / ((1 - s) + s / k)`. Elimination uses `1 / (1 - s)`.
These are arithmetic ceilings/scenarios, **not predictions or current
measurements**; added synchronization, memory pressure, lost work or worse
learning can erase the apparent gain.

| Historical parent share, rounded | Halve that parent's elapsed cost | Eliminate that parent entirely |
| --- | ---: | ---: |
| Collection ≈66% | ≈1.49× overall | ≈2.94× overall ceiling |
| Broad PPO ≈26% | ≈1.15× overall | ≈1.35× overall ceiling |
| Snapshots ≈3.6% | ≈1.018× overall | ≈1.037× overall ceiling |

Eliminating collection or PPO is not a useful learning implementation: these
rows show why measuring collection's removable work matters more than assuming
an arbitrarily large PPO speedup will transform the whole run. For a substage,
use its measured share of the whole interval, not its parent's entire share.
Do not multiply these separate scenarios or count residual-critic savings again
on top of the PPO parent.

## Timer boundaries on current main

Paths here are inspected at S1; Git history preserves this audit if they move.
Substages are attribution inside their parent, never additional total time.

| Stage and units to collect | Boundaries, work included, and overlap uncertainty |
| --- | --- |
| Collection: s/window, s/world tick, s/retained row | [Continuous producer](../../crates/alife_game_app/src/foundation_training_cycle/continuous_world.rs): `collection` starts before the control loop and ends at `FoundationCausalCollectionMetrics::measure`. Includes runtime ticks, no-progress calls, teacher/care handling, pre/post training capture, feature extraction and replay append. Excludes replay seal, subsequent boundary drain, PPO, install and full snapshots. Captured rows include bootstrap and possibly untrained suffixes; divide by retained rows only after final admission. Setup/restore and any pre-loop progress are outside this interval. |
| Full tick, perception and host preparation: ms/tick and % of collection | [Staged tick](../../crates/alife_game_app/src/gpu_live_runtime/staged_tick.rs), [runtime](../../crates/alife_game_app/src/gpu_live_runtime.rs), [CPU preparation](../../crates/alife_game_app/src/gpu_live_runtime/cpu_preparation.rs). World perception, legal candidates, memory/context, neural input validation, world outcome validation/sealing, biology, learning, memory/topology and lifecycle work. Existing wall counters have nested boundaries; inspect each before adding them. Current subcosts unknown; none may be assigned all 5,840 historical seconds. |
| GPU inference dispatch and compact readback: ms/batch, ms/row, bytes and submissions | [Backend tick](../../crates/alife_gpu_backend/src/closed_loop_runtime/tick.rs) builds one encoder for admitted class chunks, submits it once, registers mappings, waits and validates/commits results. Multiple neural kernels and other tick phases still exist. Current main already batches class chunks; a proposal to do that again removes no demonstrated work. Host time includes preparation, mapping, queue/device wait and decode. GPU work overlaps the host wait. No device speedup measured. |
| Training pre/post state capture: ms/boundary, bytes/row, submissions/window | [Training capture](../../crates/alife_gpu_backend/src/closed_loop_runtime/training_capture.rs) preflights handles, constructs metadata/ranges, packetizes bounded staging, copies/maps mutable state and required structural data, and reconstructs snapshots. [Staged tick](../../crates/alife_game_app/src/gpu_live_runtime/staged_tick.rs) calls it before and after inference. Separate queue boundaries remain around inference; large requests may need multiple packets. Current time/share unknown; byte/submission counters are traffic, not elapsed time. |
| Replay append/seal/load/cache/packing: s/window and ms/load | Append and feature extraction are inside collection; seal is outside it; old-value replay loads and training-window loads/packing are inside PPO. Cache retention has an initial compact copy, logical cap and per-access process guard. A hit avoids some decode/copy work, not replay validation/packing/compute in general. H3 is only the old hit-copy operation, not the whole loader or training stage. |
| PPO and residual critic: s/update, s/accepted optimizer step | `ppo_started` in the continuous producer encloses replay loading/old-value evaluation, biological rewards/targets, GAE/batch construction, frozen on-policy validation, recurrent actor PPO, value checkpoint access, residual-critic training and reconstruction of value state. It ends before policy export/install/rebind. Actor loss windows are up to 256 ticks with up to 128 burn-in; default epochs = 2 and KL stopping can reduce actual work. [Residual fitting](../../crates/alife_game_app/src/foundation_training_cycle/critic_residual.rs) uses real current/next features and batches up to 2,048 loss rows; accepted actor updates determine critic update count. Critic state/evaluations are reused within the window. Its time/share is unknown and already inside PPO; it must not be added again. |
| Full snapshots: s/snapshot and s/retained hour | Periodic and final `snapshot` timers enclose quiesce, exact portable GPU/world capture, durable snapshot publication, boundary rebind, handoff and `on_snapshot` callback. Callback cost is inside the timer. [Host default](../../scripts/run_continuous_care.py) requests a snapshot every 32 updates, plus finalization; actual H1 request/cadence unverified. Ordinary journal work and per-step training captures are separate mechanisms. |
| Remaining host work: s/invocation and s/host session | Restore/setup, replay seal, boundary drain, policy export/install/rebind, event writes, guards, output retirement, final admission, recovery/publication and cleanup have distinct or unassigned costs. Some are already timed (`restore_seconds`, `replay_seal_seconds`, `boundary_wait_seconds`, `install_seconds`); do not add them to a remainder that already contains them. The host receipt's `host_wall_seconds` includes admission through recovery/publication/cleanup, final admission and event read, including crash/reconciliation downtime; excludes final compact receipt and sidecar housekeeping. `producer_seconds` is a narrower native interval. |

The legacy `inference_transaction_wall_ns` starts **after pre-capture** and ends
after inference, post-capture and training-row assembly. It is not a full training
transaction timer. GPU timestamps omit the **separate pre/post training-capture
copies**, host work and map/wait latency. More precisely, S1's tick timestamps
bracket [`record_staged_closed_loop`](../../crates/alife_gpu_backend/src/closed_loop_pipeline.rs), which itself records compact selection/speech
copies (and optional selector diagnostic copies); they do not exclude every copy
command. A future comparison must name the exact query boundaries rather than
call every timestamp a pure kernel timer.

The training tick comparison must measure the **full outer runtime tick**,
including both pre/post capture boundaries, production inference, world outcome
and learning work. For the narrower capture/inference transaction, place a wall
boundary before pre-capture and close it after post-capture/row assembly. Treat
production submit/wait, GPU timestamps and individual captures as nested
attribution; do not substitute the single batched submit or the legacy inference
timer for this full cost. Neither outer duration is currently measured here.

The held fused pre/tick/post candidate `ed393a6c` is **unverified and unpublished**;
its publication was denied and remains pending. Its broader host interval includes
capture copies, unlike the legacy interval's omitted pre-capture. A smaller/larger
timer value with changed scope cannot establish speedup. No candidate code or
artifact is reproduced here, and this ledger changes neither it nor held
PR215/PR216.

## Necessity and alternatives to test

The impact estimates below are hypotheses. A historical parent share is a
conditional ceiling, not a measured removable cost. Confidence is in the proposed
mechanism; current whole-run speedup confidence remains low until measured.

| Work and why it exists | Necessity/cadence question and alternative | Expected impact/confidence and learning/behavior tradeoff | Test that could reject the proposal |
| --- | --- | --- | --- |
| Full organism/world tick maintains causal biology, movement, ecology, sleep and learning. | Must every subsystem do full work at every 20 Hz world tick? Use existing multirate design: event-driven dirty work and reduced cadence for quiet subsystems/organisms; preserve elapsed-time integration and real outcomes. Sleeping/offscreen is not automatically safe to skip. | Potentially substantial within collection's historical ≈66% parent; removable fraction unknown. Medium confidence in avoiding unchanged work; low in safe broad tick decimation. Coarser integration can change hunger, contact, sleep, development, plasticity and selection pressure. | Compare equal simulated horizons on matched saved states/seeds: physiology trajectories, contacts/actions, sleep transitions, newborn/death ordering, learned response and exact resume. Reject if reduced work fabricates progress or loses relevant events/behavior. |
| Perception/candidates/memory provide current grounded senses and legal choices. | Reuse spatial/terrain/immutable computations and refresh expensive channels on movement/events or a declared bounded cadence. Rebind fresh decision identity and revalidate current legality; prompt hazards/social signals may need immediate refresh. Existing gameplay focused recall is already every four ticks/on changes and concept/quiet topology work every ten; training uses the audited per-frame path. | Potentially large if host preparation dominates; current amount unknown. Medium confidence for immutable/dirty reuse, low for a selected staleness schedule. Stale perception can miss food/hazards, change exploration, and remove learning opportunities. | Paired traces with moved/consumed food, occlusion, novel speech, pain and topology changes. Measure cache invalidations, preparation time, unassisted nourishment/survival and retained learning/hour; reject wrong target binding or degraded behavior at the proposed cadence. |
| Cognition and action selection close the perception/action feedback loop; local learning assigns real consequences. | Does every organism need fresh inference and a fresh action decision on every world tick? Compare a declared bounded cognition/action cadence, persistent motor intent between decisions, event-triggered interrupts and quieter-organism policies. Advance time-scaled biology, chemistry, eligibility/plasticity and real action consequences correctly; do not invent decision rows on intervening ticks. | Potentially substantial inside collection; current neural fraction and cadence response unknown. Low confidence until behavior measured. This deliberately changes behavioral timing, unlike removing redundant copies/validation. Slower reactions, different recurrent dynamics, missed credit and altered instincts/exploration are plausible costs; elapsed-time discounting and learning rates need explicit treatment. | Equal simulated-horizon trials with moving targets, sudden pain/hazards, social input, food contact, sleep and developmental transitions. Record reaction latency, actual decisions, consequences, learning retention, assistance and capability per wall hour. Reject a faster simulation that weakens useful behavior beyond the explicitly chosen tradeoff. |
| Neural inference and compact readback choose actions and bind exact GPU decisions to world execution. | Keep authoritative selection/outcome dependency; reduce unnecessary waits, uploads and duplicate validation or increase useful population batching. Class chunks already share one submit/wait on S1. Consider asynchronous pipelining only across genuinely independent work. | Unknown ms and collection share. Medium confidence that fewer boundaries can help a latency-bound device; no device evidence. Delayed control can change action/physics feedback and hide device errors. | On available hardware, count submit/map/wait boundaries and compare identical admitted inputs; inspect selected actions, recurrent state, eligibility, device-loss fail-stop and mixed-class order. Reject if elapsed collection/retained learning does not improve or causal binding changes. |
| Pre/post capture and replay make recurrent on-policy behavior reproducible with actual weights, private state and local plasticity. | Is every full mutable range needed twice each captured decision? Explore bounded field-minimal capture, immutable sharing, generation/delta schemes, safe queue-boundary combination, or fewer selected training windows while the organism still advances normally. Reconstruct and validate required replay; dropping ordinary ticks is a separate policy. | Possibly meaningful bytes/waits; current time unknown. Medium confidence in removing redundant data, low in a full sparse reconstruction design. Sparse capture can bias examples, lose plasticity/structural transitions or invalidate old log probabilities. Held fused work remains outside this task. | Exact replay/continuity checks across fast/lifetime weights, recurrent banks, growth, sleep and policy install; compare old joint probabilities/gradients, bytes, memory and full collection time. Reject gaps that cannot be reconstructed or selection bias that reduces capability/hour. |
| PPO replay, burn-in, frozen validation, gradients and epochs turn retained trajectories into actor updates. | Do two epochs, the same burn-in and the same update cadence buy enough improvement? Compare one vs two epochs, adaptive KL/learning-progress stopping, bounded recurrent reuse/packing, and longer cohorts/amortized fixed work. Retain on-policy and real private-state boundaries. PR214 already shares cache-hit payloads. | Historical PPO parent ≈26%, shared with preparation and critic. At most part is epoch-dependent; halving epochs cannot be assumed to halve PPO. Medium confidence for eliminating duplicate preparation; low for equal learning with fewer updates. Longer cohorts can delay learning or increase policy drift; fewer exposures can lower sample efficiency. | Same initial admitted state and matched evaluation budget; record actual epochs, KL stops, accepted actor steps, unique rows/exposures, loss/entropy and preparation vs execution time. Reject a rows/hour gain that worsens unprompted behavior, retention or capability per wall hour. |
| Residual critic fits the full sampled Bellman residual on actual next-state features, helping later policy credit. | Must critic update count track every accepted actor update? Test bounded/convergence-based fitting and reuse of unchanged data/evaluation; do not replace the objective with GAE return regression. Current code already retains a residual object and reuses same-batch evaluation. | Unknown subset of PPO, not an extra 26%. Medium confidence for measured duplicate removal; low for less fitting without worse credit. Underfit values can corrupt advantage quality; numerical/order changes may alter learning. | Separate residual preparation/evaluation/step timing, compare same feature/transition batches, residual error and finite gradients, exact optimizer-age continuation, held-out value quality and later policy behavior. Reject if faster fitting yields worse learning/hour or conceals failed updates. |
| Full snapshots preserve the same individual's actor/value Adam state, private life and recovery boundary. | Must all state be materialized every 32 updates? Compare bounded adaptive cadence, cheaper immutable/delta persistence and asynchronous preparation with atomic admitted publication. Preserve a complete prior snapshot and final reserve. The observed loss of 8,704 rows makes failure cost material. | Historical snapshot parent ≈3.6%; even elimination would cap gain near 3.8% in that conditional model. Medium confidence in amortization, low in net gain until failure cost measured. Less frequent snapshots increase lost work/recovery lag; extra background buffers can increase memory and contention. | Interrupted-write/crash/recovery trials on disposable fixtures; verify world/body identity, weights, moments, integer ages, sampling stream and published ownership. Measure snapshot stall plus expected discarded work/hour. Reject if a cadence gains provisional rows but reduces retained rows/hour. |
| Restore/install/seal/admission/recovery/guards protect continuity and bound resources. | Which validations traverse unchanged authority repeatedly, and which prove genuinely new bytes/ownership? Cache only content-bound immutable results, coalesce writes and amortize cold work; preserve revalidation at external/mutable boundaries. | Conditional H1 remainder ≈4.4%, with unknown membership; no guaranteed removable fraction. Medium confidence in source-directed duplicate removal. Weaker validation can accept corrupt state, replay another life, or lose acquired learning. | Attribute residual wall time first; corruption/foreign-owner/mid-publication tests, cold/warm trials and a complete recovery cycle. Reject caches that survive relevant mutation, skip fresh external-byte checks or move time/memory elsewhere without net benefit. |

## Small next measurements and promotion rules

First recover the original H1 receipts if available. Record immutable source paths
and hashes, binary/source/tree identity and the request; map each segment and
failed attempt once. Establish whether native stage sums and retained rows refer
to the same sessions. Reconcile raw `host_wall_seconds` against `producer_seconds`
plus wrapper work. Include startup, sleeps, bootstrap, unsaved work, crashes,
recovery and cleanup in their actual intervals; do not subtract inconvenient time.

For current main, the first useful device assay is a small matched run covering
collection, at least two PPO windows, an installed-policy boundary, one snapshot
and exact restore. This is future measurement guidance, not authorization for a
heavy desktop run. Record GPU/driver/API, CPU/OS, memory, build mode/features,
source SHA and dirty tree, binary hash, prior/provider and actual assistance,
brain/body/population, cohort length, cadence, seed and starting-state identity.
Alternate baseline/candidate order, distinguish cold/warm trials, report trial
count and median/spread (p95 only with its sample count/method), and measure the
profiling observer effect. Keep provisional row rate and retained rate separate.

Use existing accounting before adding new instrumentation:

- Continuous same-individual training exposes collection/PPO/install/restore,
  replay-seal/boundary-wait, snapshot and producer counters; the host receipt adds
  inclusive wall and admitted retained progress. Boundary-only
  [`runtime_performance`](continuous-runtime-metrics.md) additionally exports the
  existing runtime-instance cumulative snapshot and checked deltas since the
  previous observation, with tick/active-row counts, unknown reset/saturated
  deltas and explicit nonadditive peak treatment. The compact host receipt keeps
  the initial/untrained/error evidence. No new per-tick timer or output is added.
  Per-stage current values remain unknown until actual runtime receipts exist;
  C1 measures only the observer's synthetic CPU cost.
- Serial cycle/resident-episode profiling through `ALIFE_FOUNDATION_PROFILE`
  writes `stage-timing.json`. It is **not** a full continuous-session profiler.
  [The recorder](../../crates/alife_game_app/src/foundation_training_cycle/stage_timing.rs)
  produces parent/child elapsed and exclusive host seconds, covered time and an
  explicit unassigned remainder. Sum exclusive seconds plus remainder, never
  parent elapsed plus child elapsed. Nested legacy collection counters are not
  additional stages.
- [The read-only summary tool](../../scripts/summarize_stage_timing.py) accepts
  receipt paths/directories. Missing stages are unknown; measured zero requires
  an observed call. Keep error cohorts separate and inspect mixed contexts.
  Content deduplication is a copy heuristic, not proof of unique run identity.
- Existing [host ownership/retention tests](../../scripts/test_run_continuous_care.py)
  cover inclusive admission cost, lost work and refusal to fabricate retained
  rows. Recorder tests cover nested accounting, absent vs zero and partial
  errors. Replay/cache/critic tests protect continuity and learning boundaries;
  these CPU fixtures do not certify GPU speed or autonomous behavior.

Rank measured candidates by useful capability and retained learning per inclusive
hour. The historical evidence makes collection and the broad PPO stage the first
places to measure, but does not identify which current substage dominates. Promote
a performance claim only with unchanged timing scope, reconciled cohort totals,
preserved continuation and reported behavior/assistance. Any remaining unattributed
time stays explicit rather than being called negligible.
