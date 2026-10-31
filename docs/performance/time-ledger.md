# Performance time ledger

Updated 2026-10-31. Source audit: main
[`eaf9e12067d649e9e27c16e6600e6e8dabfc7e99`](https://github.com/BobertNeek/A-Life/commit/eaf9e12067d649e9e27c16e6600e6e8dabfc7e99).
This ledger records what consumes time, what that work buys, and experiments
that could show it should be cheaper or less frequent. Append new measurement
cohorts with their provenance; do not replace historical timings with estimates
for newer code. **Current device and whole-training costs are unknown, not zero.**
The project desktop GPU is reported unavailable because driver startup is broken; this
documentation change runs no desktop workload and transfers no real checkpoint.

> **Applicability correction: PR219 is a CPU-only candidate. It does not reduce
> the historical overnight GPU PPO phase (2,284.019 s in the final b817
> session).** The targeted
> continuous same-individual command uses the GPU `FoundationTrainer`, and GPU
> PPO already reuses its forward state for backward. The CPU percentages below
> are not GPU PPO, whole-run or FPS improvements. Backend/source evidence is
> recorded in the CPU appendix.

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
| H1: final b817 session | 2026-10-28, Windows RTX 3050, continuing assisted care of one N2048 individual; 139,264 new retained rows, 272 updates and 139,536 progressed world ticks (6,976.8 simulated seconds). All H1 phase totals belong to this final b817 session. CPU/RAM, exact driver/API and build profile are not established here. | Read-only inspection on 2026-10-31 at 04:48 UTC of the project receipt, adjacent native owner logs and training handoff; identity and path below. Host elapsed is 8,836.258 s; producer elapsed 8,766.645 s. These are historical run measurements; no benchmark was rerun for this ledger. |
| H2: early H1 slice | First eight 512-row windows (4,096 rows) of the b817 session, recorded 2026-10-28 12:39 UTC, Windows RTX 3050, combined PR202/203 build [`b817c1513a6023a5395e89b9e5fc7374b0c028c2`](https://github.com/BobertNeek/A-Life/commit/b817c1513a6023a5395e89b9e5fc7374b0c028c2); PR204 was test-only. | Recorded early-session progress: mean collection 20.94 s/window, mean PPO 8.63 s/update. Earlier PR203-only sample: 24.43 and 16.13 s respectively. Different sequential trajectories and assistance; not paired or controlled. No whole-host or snapshot denominator for this slice. |
| H3: isolated cache copy | [PR214](https://github.com/BobertNeek/A-Life/pull/214), head `a63010a1`, 2026-10-30; Linux, rustc 1.98.1, opt3 source-extracted types/cache, synthetic finite N2048 payload, 25 candidates/tick, seven alternating paired trials. CPU model/RAM not established in the cited report. | Old cache-hit deep-copy median 5.20 ms [4.47–6.10] for 256 ticks/no burn-in; 7.98 ms [7.80–9.15] for 384 ticks/128 burn-in. Shared hits remove payload copies. First retain still compacts/copies. Windows memory guard stubbed in this assay. No admitted trajectory, GPU, whole-PPO or learning/hour measurement. |
| S1: current source | 2026-10-31 audit of main `eaf9e120`; no timing workload run. | Code boundaries below establish what timers include, not how expensive current work is. [PR210](https://github.com/BobertNeek/A-Life/pull/210) changed host authority/replay costs; [PR211](https://github.com/BobertNeek/A-Life/pull/211) removed disabled PPO value-head work; [PR213](https://github.com/BobertNeek/A-Life/pull/213) changed checkpoint serialization; PR214 changed cache-hit ownership. Historical proportions cannot certify today's proportions or device speedup. |

H1 provenance: read-only inspection on **2026-10-31 at 04:48 UTC** of
`C:\Users\PC\Documents\Codex\2026-10-07\task-2\care-continuous-night-20261028\lane\last-continuous-session.json`,
together with the adjacent native owner logs and training handoff. Recorded
source commit is `b817c1513a6023a5395e89b9e5fc7374b0c028c2`, tree
`1d4e6846882094286118a77a0688994cce65b11b`, and binary SHA-256
`b625b5c9b70420131fa64794ba34569ce828a6bd14086645b4c95de3d6511512`.

Earlier-session records identify **8,704 unsaved rows lost** in a separate
attempt. Keep that loss separate from H1's final b817 session;
do not add it to retained rows or silently mix other attempts into this session's
timers. A whole-night accounting would need those other attempt intervals too.
H1 also records substantial teaching/food repositioning; nourishing outcomes do
not prove independent food finding or survival.

## Historical amounts and conditional reconciliation

Use the recovered inclusive host interval **T = 8,836.258 s**, not an inversion
of rounded rates. It yields 56,737.863 retained rows/hour and 0.789565 simulated
seconds/host second, consistent with the previously rounded 56,738 and 0.790.
The host interval includes invocation/admission through native recovery,
publication/cleanup, final admission and event reading; the final compact receipt
write and sidecar housekeeping are outside it. The producer is a narrower native
interval. The table reports counter/host-wall ratios, not independent CPU/GPU
service-time shares or a proven exclusive partition. Times are reported to
0.001 s; current-main costs remain unmeasured.

| Stage | H1 reported seconds | Ratio to 8,836.258 s host wall | Current main amount/share |
| --- | ---: | ---: | --- |
| Collection, inclusive of its tick/capture/replay work | 5,839.860 | 66.09% | Unknown s and % |
| PPO using the GPU trainer, inclusive of preparation and residual critic | 2,284.019 | 25.85% | Unknown s and % |
| Full periodic/final snapshots | 321.254 | 3.64% | Unknown s and % |
| Policy install | 116.120 | 1.31% | Unknown s and % |
| Replay sealing | 101.036 | 1.14% | Unknown s and % |
| Restore | 3.601 | 0.04% | Unknown s and % |
| Six-counter arithmetic subtotal | 8,665.890 | 98.07% | Not an extra stage |
| Host minus six-counter subtotal, derived only | 170.368 | 1.93% | Not proven exclusive overhead |
| Enclosing producer interval; do not add to its children | 8,766.645 | 99.21% | Unknown s |
| Inclusive host interval | 8,836.258 | 100% by definition | Unknown s |

The six reported phase counters sum arithmetically to **8,665.890 s**.
`8,836.258 - 8,665.890 = 170.368 s` is a difference, **not a measured or proven
exclusive overhead stage**. Likewise, host minus producer is 69.613 s and
producer minus that subtotal is 100.755 s; these differences do not identify
which work consumed them. The three collection/PPO/snapshot counters alone sum
to 8,445.133 s and leave 391.125 s, which already includes the install/seal/
restore amounts above. Never count those counters again inside an added
remainder or add enclosing producer time to them.

Inspection of the
[b817 continuous producer](https://github.com/BobertNeek/A-Life/blob/b817c1513a6023a5395e89b9e5fc7374b0c028c2/crates/alife_game_app/src/foundation_training_cycle/continuous_world.rs#L834)
confirms that these three host intervals are sequential and do not nest in one
invocation: collection closes before replay seal/boundary wait; PPO closes before
policy installation; snapshots run after installation/event handling or at the
final boundary. That supports their top-level host interval separation in this
identified invocation; it does not by itself establish an exhaustive exclusive
breakdown for every counter, wrapper activity or asynchronous worker. Persistence
workers and GPU work can overlap host work; wall spans include waits and are not
independent CPU and GPU service times.

H1's foundation-training capture counters record **279,072 readback submissions**
and **247,262,257,152 copied bytes** (230.28 GiB). For 139,536 progressed ticks,
the exact aggregate ratios are two capture readback submissions/tick and
886,016 bytes/submission (1,772,032 bytes/tick), matching the pre/post capture
structure in the audited code. These are logical capture-traffic counts, not all
GPU submissions/traffic, elapsed transfer time, PCIe bandwidth, or proof of a
bottleneck. Current capture/transfer duration remains unknown; measure the full
outer tick and attribute capture/dispatch/readback separately before prioritizing
a change from these counts.

### What the capture bytes contain

The [mutable slot layout](https://github.com/BobertNeek/A-Life/blob/4d772c327b85d7d1757f4b9c22658d0ced42c03e/crates/alife_gpu_backend/src/closed_loop_buffers/bucket.rs#L2092)
accounts for an N2048 slot as follows. These are source-derived byte amounts,
not timed transfer costs or measured removable work.

| Mutable slot component | Bytes |
| --- | ---: |
| Lifetime/fast weights in both banks | 525,312 |
| Recurrent/decoder eligibility in both banks | 262,656 |
| Activations, accumulator, homeostasis and encoded input | 49,152 |
| Replay events | 14,336 |
| Replay eligibility samples | 32,768 |
| Replay spans | 1,024 |
| Candidate, diagnostic, selection, speech, extension, learning and pending-eligibility records | 544 |
| Alignment padding | 224 |
| Full mutable slot | 886,016 |

The [capture implementation](https://github.com/BobertNeek/A-Life/blob/4d772c327b85d7d1757f4b9c22658d0ced42c03e/crates/alife_gpu_backend/src/closed_loop_runtime/training_capture.rs#L82)
is byte-identical between b817 and main `4d772c32`. It copies the mutable slot;
extra structural plan/weight ranges are conditional and cached when structural
metadata remains unchanged. The immutable phenotype is shared; this is not an
unconditional transfer of the inherited genotype on every capture. H1's exact
886,016-byte aggregate average is consistent with one such slot per capture,
but cannot rule out offsetting variations among individual captures.

Pre/post captures have distinct consumers. Replay uses the
[pre-state initial activations/homeostasis](https://github.com/BobertNeek/A-Life/blob/4d772c327b85d7d1757f4b9c22658d0ced42c03e/crates/alife_game_app/src/foundation_training.rs#L358),
checks [pre/post weights and cross-row continuity](https://github.com/BobertNeek/A-Life/blob/4d772c327b85d7d1757f4b9c22658d0ced42c03e/crates/alife_game_app/src/foundation_training.rs#L380),
and reads [post-inference encoded inputs](https://github.com/BobertNeek/A-Life/blob/4d772c327b85d7d1757f4b9c22658d0ced42c03e/crates/alife_game_app/src/foundation_training.rs#L530).
The continuous trainer also retains
[post-inference activation features](https://github.com/BobertNeek/A-Life/blob/4d772c327b85d7d1757f4b9c22658d0ced42c03e/crates/alife_game_app/src/foundation_training_cycle/continuous_world.rs#L1116)
for the residual critic. Selective ranges need a complete consumer audit and
replay/continuity/critic validation before claiming savings; neither their net
byte reduction nor GPU latency benefit is established here.

H2's two reported means total 29.57 s versus 40.56 s for the earlier sample.
That is 27.1% less elapsed time across those two stages, or a 37.2% higher
reciprocal stage rate. It is an uncontrolled observation, not an attributed
code speedup or retained rows/hour improvement. Do not extrapolate it across H1.

### Conditional Amdahl scenarios

If H1's counter ratios represent exclusive work and remain representative,
changing only a fraction `s` by a factor `k` gives the
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

### Cadence and useful-learning break-even

Illustratively, if a genuinely exclusive training phase occupies 26% of wall
time and its entire cost can be halved, the new wall budget is
`0.74 + 0.26 / 2 = 0.87`. Collected exposures/hour would rise by
`1 / 0.87 ≈ 1.15×`; useful learning per newly collected exposure must retain at
least **87%** of baseline to break even in useful learning/hour. This assumes
unchanged collection cost, retained exposure count, assistance, failure/loss
rates and other stages, with useful learning measured on comparable outcomes.

This is a qualified learning-efficiency scenario, not a promised raw speedup,
a PR219 GPU benefit, or proof that halving PPO epochs halves the training phase.
The historical PPO counter is 25.85% of host wall; using that as an exclusive
removable fraction is still conditional. Preparation, frozen validation and
critic work need not scale with epochs. If less of the phase shrinks, throughput
gains are smaller and required learning retention is closer to 100%. Keep the
cadence/epoch alternatives below as deliberate behavior/learning tradeoffs to
measure, separate from removing redundant implementation work.

Decompose actual GPU PPO preparation, forward/backward, host encoding/submission,
blocking metrics/status readbacks, critic work and Adam validation/application
before prioritizing per-weight Adam arithmetic. The already-batched production
inference tick is a different boundary and does not supply that decomposition.

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
PR215/PR216. Fusion concerns queue/wait boundaries and expects equal capture
bytes; it does not establish that either pre/post state is unnecessary or that
less data can be copied.

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
| CPU versus GPU execution of the offline PPO learner trades CPU graph traversal against GPU parallelism, encoding, dispatch and readback costs. | Conditional experiment: is GPU PPO actually faster than CPU PPO for this small recurrent workload on the user's i7/RTX 3050 after all dispatch/readback overhead? Match compiled model/topology, trainable mask, recurrent/private state, weights, actor/critic moments and exact ages, replay/masks/targets, objective, burn-in, microsteps, epochs, KL rules and effective-batch/optimizer budgets before comparison. In particular, compare the same actor objective and full residual-critic work rather than different default learner variants. | The hardware crossover and whole-run impact are unknown. Cloud EPYC/Xeon CPU timings cannot answer the i7/RTX 3050 question. A faster matched learner could still lose its benefit to transfer, setup, collection or persistence costs; numerical/state equivalence and useful learning must remain acceptable. This is a proposed experiment, not an implementation, automatic switch or gameplay neural fallback. | On that actual PC, use the same locally generated public/untrained fixture and paired alternating/ABBA trials; identify exact CPU, GPU/driver/API, build, affinity, power and concurrency. Time equivalent outer learner boundaries through completed actor/critic updates and required status checks. Include GPU host encode/submit/waits/uploads/readbacks and equivalent CPU preparation; report cold/setup and warm/amortized costs separately. Reject an apparent crossover caused by unequal workloads, accepted updates/KL stops, missing synchronization or out-of-tolerance outputs/continuation. |
| Residual critic fits the full sampled Bellman residual on actual next-state features, helping later policy credit. | Must critic update count track every accepted actor update? Test bounded/convergence-based fitting and reuse of unchanged data/evaluation; do not replace the objective with GAE return regression. Current code already retains a residual object and reuses same-batch evaluation. | Unknown subset of PPO, not an extra 26%. Medium confidence for measured duplicate removal; low for less fitting without worse credit. Underfit values can corrupt advantage quality; numerical/order changes may alter learning. | Separate residual preparation/evaluation/step timing, compare same feature/transition batches, residual error and finite gradients, exact optimizer-age continuation, held-out value quality and later policy behavior. Reject if faster fitting yields worse learning/hour or conceals failed updates. |
| Full snapshots preserve the same individual's actor/value Adam state, private life and recovery boundary. | Must all state be materialized every 32 updates? Compare bounded adaptive cadence, cheaper immutable/delta persistence and asynchronous preparation with atomic admitted publication. Preserve a complete prior snapshot and final reserve. The observed loss of 8,704 rows makes failure cost material. | Historical snapshot parent ≈3.6%; even elimination would cap gain near 3.8% in that conditional model. Medium confidence in amortization, low in net gain until failure cost measured. Less frequent snapshots increase lost work/recovery lag; extra background buffers can increase memory and contention. | Interrupted-write/crash/recovery trials on disposable fixtures; verify world/body identity, weights, moments, integer ages, sampling stream and published ownership. Measure snapshot stall plus expected discarded work/hour. Reject if a cadence gains provisional rows but reduces retained rows/hour. |
| Restore/install/seal/admission/recovery/guards protect continuity and bound resources. | Which validations traverse unchanged authority repeatedly, and which prove genuinely new bytes/ownership? Cache only content-bound immutable results, coalesce writes and amortize cold work; preserve revalidation at external/mutable boundaries. | H1 records install 116.120 s, sealing 101.036 s and restore 3.601 s. Its remaining six-counter gap is 170.368 s arithmetically, not proven exclusive overhead; there is no guaranteed removable fraction. Medium confidence in source-directed duplicate removal. Weaker validation can accept corrupt state, replay another life, or lose acquired learning. | Attribute remaining wall time first; corruption/foreign-owner/mid-publication tests, cold/warm trials and a complete recovery cycle. Reject caches that survive relevant mutation, skip fresh external-byte checks or move time/memory elsewhere without net benefit. |

The backend comparison is measurement guidance only. Establish a declared
numerical tolerance and exact identity/mask/integer-age checks before timing;
do not silently accept a different objective or optimizer history as equivalent
work. Measure a small range around the actual target cohort/window sizes, since
the crossover may change with workload size. Use host wall time for the decision;
GPU timestamps are nested attribution and are not added to it. Real checkpoint
transfer remains separately unauthorized. A real-state continuation assay or
subsequent backend change is outside this recorded comparison and needs separate
qualification; this documentation update performs neither a benchmark nor a
switch.

## Small next measurements and promotion rules

Retain H1's recovered receipt path and binary/source/tree identity. Treat it as
one final b817 session; archive other attempts separately before making a
whole-night comparison. Preserve raw `host_wall_seconds`, `producer_seconds` and
counter boundaries, and investigate the arithmetic differences without naming
them exclusive overhead prematurely. Include startup, sleeps, bootstrap, unsaved
work, crashes, recovery and cleanup in their actual intervals; do not subtract
inconvenient time.

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
  inclusive wall and admitted retained progress. Per-stage current values remain
  unknown until actual receipts exist.
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

## CPU replay appendix: 2026-10-31

This is an opt-in CPU learner measurement, separate from H1's Windows/GPU
collection and retained-learning totals. [PR219](https://github.com/BobertNeek/A-Life/pull/219)
compares base `eaf9e120` with implementation `0f57ff6f`; the complete benchmark
report and raw trials are versioned at
[`17c7a33a`](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/docs/performance/20261031-cpu-ppo-costs.md).
Follow-up [`594955d8`](https://github.com/BobertNeek/A-Life/commit/594955d81678932fcc743e8d4d36aff08d3b87f9)
adds an unequal-row residual regression and test-helper visibility; production
replay behavior is unchanged. The documentation-only follow-up
[`dc71292f`](https://github.com/BobertNeek/A-Life/blob/dc71292f0b1077ea3ccb46afded60408fe15e4c5/docs/performance/20261031-cpu-ppo-costs.md#saved-call-reconciliation-original-epyc-cohort-only)
explains the unchanged-input boundary and saved-call reconciliation. These are
versioned candidate results, not new
measurements of current gameplay, GPU execution or whole-training throughput.

**Backend applicability to the overnight target.** At audited main `3f70fedd`,
the [host command selector](https://github.com/BobertNeek/A-Life/blob/3f70fedd100bb2b1b4fb395d5efcf5e5f6aa707a/scripts/run_continuous_care.py#L46)
maps `same_individual` to `train_n2048_care --resident-continuous STORE REQUEST_JSON`;
the [CLI handler](https://github.com/BobertNeek/A-Life/blob/3f70fedd100bb2b1b4fb395d5efcf5e5f6aa707a/crates/alife_game_app/src/bin/train_n2048_care.rs#L78)
calls the continuous resident path. Its
[handoff is `NativeGpu`](https://github.com/BobertNeek/A-Life/blob/3f70fedd100bb2b1b4fb395d5efcf5e5f6aa707a/crates/alife_game_app/src/foundation_training_cycle/continuous_world.rs#L592),
and [trainer construction uses `into_gpu_training_session`](https://github.com/BobertNeek/A-Life/blob/3f70fedd100bb2b1b4fb395d5efcf5e5f6aa707a/crates/alife_game_app/src/foundation_training_cycle/continuous_world.rs#L681).
[`FoundationTrainer` owns a `GpuAuthoritativeSession` and wgpu state](https://github.com/BobertNeek/A-Life/blob/3f70fedd100bb2b1b4fb395d5efcf5e5f6aa707a/crates/alife_training/src/trainer.rs#L151).
The audited historical [b817 producer uses the same GPU session path](https://github.com/BobertNeek/A-Life/blob/b817c1513a6023a5395e89b9e5fc7374b0c028c2/crates/alife_game_app/src/foundation_training_cycle/continuous_world.rs#L655).
This invocation does not call the CPU cohort learners changed by PR219, so its
reported overnight PPO interval receives no saving from that CPU-only change.
H1's recovered receipt now identifies the final b817 session and its binary;
the GPU source trace and that historical receipt still do not measure current
GPU PPO subcosts or establish a CPU/GPU hardware crossover.

GPU [window evaluation encodes forward once](https://github.com/BobertNeek/A-Life/blob/3f70fedd100bb2b1b4fb395d5efcf5e5f6aa707a/crates/alife_training/src/ppo.rs#L1505),
then [gradient accumulation uses the retained replay state](https://github.com/BobertNeek/A-Life/blob/3f70fedd100bb2b1b4fb395d5efcf5e5f6aa707a/crates/alife_training/src/ppo.rs#L1338).
Its backward path does not repeat the full recurrent forward as the old CPU
probe did. GPU PPO replay/objective/gradient/Adam encoders and
[blocking metric/status readbacks](https://github.com/BobertNeek/A-Life/blob/3f70fedd100bb2b1b4fb395d5efcf5e5f6aa707a/crates/alife_training/src/ppo.rs#L1785)
are training work, separate from the already-batched production inference tick
described above. Neither batching that tick nor this CPU tape change establishes
a reduction in those GPU training costs; they require their own measurements.

The removed work was a **second full forward replay** immediately before
backward. Loss evaluation had already computed the exact tape for that owned
window, and weights, window data and private replay state were unchanged before
backward. Reusing that tape removes redundant traversal and tape construction;
it does not skip the necessary first forward. The tape is local to one window
iteration and is dropped before effective-batch optimizer commit. Every later
window/epoch performs a fresh replay of current weights, including after an
optimizer update. Public standalone gradient probes still perform their own
forward. No cross-update tape cache, lower PPO cadence or reduced learning
objective is introduced.

### C1: EPYC trials and boundaries

Cloud Linux x86_64 VM, AMD EPYC 9V74, five visible virtual CPUs/four-CPU quota,
16 GiB; Rust 1.98.1, test profile with training opt-level 3 and core/world/runtime
opt-level 2, debug assertions enabled, one benchmark thread. These are not release
builds. Synthetic trials run baseline then candidate; native replay trials run
candidate then baseline. Separate pinned trials use virtual CPU 2. VM contention
and frequency are uncontrolled; no formal confidence interval is claimed.

| Timed learner cohort | Baseline median ms | Candidate median ms | Less learner wall time |
| --- | ---: | ---: | ---: |
| Synthetic generic PPO, unpinned | 667.699 | 476.640 | 28.6% |
| Synthetic residual PPO, unpinned | 678.542 | 476.151 | 29.8% |
| Synthetic residual PPO, pinned | 689.248 | 527.895 | 23.4% |
| Native-collected object-slot fixture replay | 483.414 | 357.698 | 26.0% |
| Native-collected terrain-vision fixture replay | 641.001 | 454.189 | 29.1% |

Each row has seven timed trials after one discarded warm-up, with actor and
critic restored outside each timed interval. The interval encloses the cohort
learner, including frozen preflight, owned-window loading/cloning, forward,
backward and paired actor/critic updates. It excludes native collection, window
preparation, initial optimizer restore and checkpoint serialization/publication.
Whole-training shares and retained rows/hour remain unknown; do not apply these
CPU percentages to H1's GPU PPO share or FPS.
Even where the CPU learner gain transfers, unchanged collection, preparation,
I/O and publication costs dilute the whole-run gain; their fraction is not
measured by these learner timers.

Synthetic N2048 uses 2,048 neurons/32,768 synapses, four microsteps, two 40-tick
windows each with eight detached burn-in ticks, 16 candidates/six motor factors,
64 loss rows, two epochs and permissive KL=100. Ten full-window forwards become
six, including unchanged frozen preflight. Native CPU fixtures collect 65 real
decisions/profile for 64 loss rows, with ordinary KL=0.02: object-slot windows
32/32 and effective batch 1; terrain windows 24/24/16 and effective batch 2.
Both versions stop at matching KL boundaries after two/three accepted paired
updates respectively. These fresh untrained fixtures are not imported acquired
checkpoints or evidence of competence.

Representative individual residual-cohort trials, milliseconds in process order:

| EPYC process | Seven enclosing trials, ms |
| --- | --- |
| Baseline unpinned | 672.687, 682.771, 670.485, 678.542, 701.588, 676.648, 680.300 |
| Candidate unpinned | 487.043, 487.053, 476.151, 467.584, 466.630, 464.100, 482.782 |
| Baseline pinned | 687.705, 675.265, 689.669, 709.767, 667.107, 700.990, 689.248 |
| Candidate pinned | 491.897, 542.803, 527.895, 548.321, 505.220, 560.138, 512.054 |

The [complete CSV](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/docs/performance/20261031-cpu-ppo-trials.csv)
also retains every saved component and native learner trial; the
[native JSON](https://github.com/BobertNeek/A-Life/blob/17c7a33a69b63b9dbac100655df1530c54696c34/docs/performance/20261031-native-cpu-replay-trials.json)
records configuration, source hashes, decisions/private-host hashes and equality
scope. Sequential samples share cache/allocator state and are not independent.

The baseline isolated medians identify why this work matters: one forward with
tape creation/destruction costs 50.223 ms, backward on an existing tape 45.290 ms,
window evaluation 51.080 ms and the public fresh-forward gradient probe 92.055 ms
(11 timed samples/scope after warm-up). These scopes overlap and must not be
summed or presented as an exclusive cohort breakdown. Tape-only allocation time
is unknown. Its calculated float payload is 4,939,264 bytes for this fixture,
excluding metadata/allocator/other buffers; this is not RSS. Destruction alone
was 0.042 ms in a separate candidate-only check, not an allocation comparison.

Four saved forwards at roughly 50.223 ms explain the scale of the original
EPYC residual-cohort reduction: about 201 ms of isolated forward work versus
202.391 ms less enclosing time. This is a call-count sanity check, not an exact
exclusive accounting; cache/allocator context and the pinned spread differ.
Native KL-stopped and Xeon cohorts are different workloads, so this model is not
applied to their savings.

### Equality limits and independent replication

The EPYC comparisons retain byte-identical complete actor/critic checkpoints
and receipts after the **first timed full cohort** of each comparison process.
Native collected-decision and private-host hashes match, with private-host state
asserted unchanged. This covers the tested final states, not every intermediate
batch, every timed trial, arbitrary inputs, long-term learning or all checkpoints.
Separate regressions compare four successive generic updates with fresh-forward
recomputation; the `594955d8` residual test compares two updates with 3-row and
5-row windows in one effective batch, nonzero critic moments and complete final
checkpoints/KL receipts. Fresh replay after optimizer changes remains essential.

The [CPU validation report](https://github.com/BobertNeek/A-Life/blob/dc71292f0b1077ea3ccb46afded60408fe15e4c5/docs/performance/20261031-cpu-ppo-costs.md#validity-and-falsification)
records 15 backend and 21 collector tests passing (36 affected tests).

### C2: independent Xeon ABBA replication

The [archived original-trial receipt](https://chatgpt.com/api/library/files/libfile_10486ee4bb448191adb5fa5edb152ab5/download),
`A-Life-PR219-Xeon-ABBA-original-trials-and-receipts.json`, is 48,057 bytes with
SHA-256 `63797c9cf04e1bd1b0667cfca3f8333aa037287d1986b73641c82d1add6ca0ff`.
It is a later packaging index of original trial files, not a benchmark rerun.
Its ten embedded UTF-8 originals, production-source hashes and benchmark-module
hash were verified, and all 56 timed samples/configurations were checked against
those originals. This receipt replaces the earlier unknown qualifier for C2.

This separate VM reports Intel Xeon Platinum 8573C, five visible CPUs/four-CPU
quota, 16 GiB, CPU2 affinity and Rust 1.99.1. The test profile uses training
opt-level 3, core/world/runtime opt-level 2, debug info 0 and debug assertions.
Frozen baseline/candidate binaries run after builds complete, in process order
**baseline-1 → candidate-1 → candidate-2 → baseline-2** (ABBA), with no concurrent
local build. Source is baseline `eaf9e120` versus candidate `17c7a33a`, using the
same native benchmark module. These values are not pooled with C1's EPYC trials.

Each process collects the same two fresh native fixtures once, with 65 decisions
and 64 loss rows/profile. Object-slot windows are 32/32, effective batch 1;
terrain windows are 24/24/16, effective batch 2; later windows have eight burn-in
ticks. Both use two requested epochs, temperature 0.9 and KL=0.02. Actor/critic
state is restored before one warm-up and each of seven timed cohorts/profile.
Learner time excludes collection, window preparation, restore and checkpoint
serialization/publication. The receipt's one-off collection/preparation values
are not replicated end-to-end measurements or retained-learning rates.

Within C2, the two processes per variant contribute 14 samples/profile:

| Xeon native fixture | Baseline median ms | Candidate median ms | Less learner wall time |
| --- | ---: | ---: | ---: |
| Object slots | 255.304 | 182.207 | 28.6% |
| Terrain vision | 344.277 | 240.515 | 30.1% |

All 56 timed learner values follow, rounded to 0.001 ms; the linked receipt
retains full precision. Rows preserve process/profile execution order.

| Xeon process / profile | Seven learner trials, ms |
| --- | --- |
| baseline-1 / object slots | 255.635, 260.634, 257.516, 245.517, 254.412, 248.531, 326.813 |
| baseline-1 / terrain vision | 318.111, 408.849, 353.043, 431.439, 357.088, 636.750, 337.264 |
| candidate-1 / object slots | 188.899, 224.955, 173.616, 177.640, 181.320, 180.324, 177.570 |
| candidate-1 / terrain vision | 221.585, 240.978, 269.743, 231.614, 240.053, 232.573, 231.320 |
| candidate-2 / object slots | 179.280, 190.151, 183.095, 216.484, 187.442, 188.688, 180.949 |
| candidate-2 / terrain vision | 260.414, 246.662, 270.554, 236.362, 239.549, 242.317, 271.495 |
| baseline-2 / object slots | 328.107, 267.236, 265.546, 254.318, 252.117, 254.973, 242.453 |
| baseline-2 / terrain vision | 328.594, 345.086, 337.334, 371.253, 326.382, 343.469, 336.510 |

Four process runs and their sequential samples share VM/cache/allocator state;
frequency/contention remain uncontrolled. ABBA reduces a simple order bias but
does not make 56 statistically independent repetitions; no formal confidence
interval is claimed. C2 corroborates the CPU learner gain in this fixture, not a
GPU benefit or the i7/RTX 3050 hardware crossover.

The exported **first timed full-cohort receipt objects** match across all four
processes, including actor/value ages, policy, one completed epoch and identical
KL-stop sequences: two accepted paired updates for object slots and three for
terrain. Decision and private-host hashes also match by profile. The index
reports four retained complete checkpoint files of 4,688,876 bytes with the same
SHA-256 `948831e152fe9d679404200661574d714d724d579bb28f85c01860ac29981709`.
Those full checkpoint bytes are not included in this 48 KB export, so their
hashes cannot be recomputed from this package. Exported receipt objects were
checked for equality; complete-checkpoint hash equality is index-level evidence.
Neither establishes every intermediate update,
every timed trial's checkpoint, arbitrary inputs or acquired-state behavior.

Confidence is high that the immediate repeated forward is redundant under the
stated unchanged-state boundary; measured magnitude is workload/VM-specific.
There is no intended learning/behavior tradeoff from removing this duplicate,
but fixture equality does not prove acquired-state behavior. Reject the change
if fresh-process replications lose the enclosing gain or checkpoint/receipt
equality fails across optimizer updates. Next measure an admitted CPU trajectory
from collection through publication with equal sampling, PPO/optimizer budgets,
assistance and retained state; GPU and full outer training-tick costs remain
separate unmeasured questions.

## CPU host-step appendix: 2026-10-31

The [bounded host-step report](https://github.com/BobertNeek/A-Life/blob/5b244a66d183be0330e9585390312ac7124103af/docs/performance/20261031-cpu-host-step-costs.md)
measures the actual `CpuCanonicalHost::step_with_preparation` wrapper with
test-only clocks, based on source `17c7a33a`. It is a CPU collection fixture,
separate from the CPU PPO learner assays above and H1's GPU collection/PPO.
The GPU staged tick does not call this CPU transaction wrapper; **none of these
percentages establishes a GPU collection share or whole-training saving**.

One fresh, untrained Standard2048 organism uses GroundedTerrainVisionV1, world
seed `0x7A1123`, sampling seed 42 and temperature 0.9. Each trial advances 65 real
world ticks and waking decisions, retaining ordinary perception, legality,
biology and private learning. The preparation callback is identity; no SLM,
GPU, offline PPO/Adam, replay disk writer or durable publication runs here.
Hardware is a cloud AMD EPYC 9V74 VM, five visible CPUs/four-CPU quota, 16 GiB,
unpinned CPUs 0–4; Rust 1.98.1, test profile with training opt-level 3 and
core/world/runtime opt-level 2, debug assertions enabled. No concurrent local
build/test ran during the timed rollouts; frequency and VM contention were
uncontrolled. These are not desktop or long-lived acquired-state measurements.

Seven retained timing-enabled trials (455 ticks), after warm-up, give these
arithmetic means per 65-tick trial. The four production intervals are exclusive
and occur once per successful tick here; they reconcile to the enclosing call
with its explicit remainder. They are not independently selected medians.

| Exclusive CPU host interval | Mean ms / 65 ticks | Share of outer step calls |
| --- | ---: | ---: |
| Founder/private authority validation | 744.725 | 28.20% |
| Full host clone | 5.128 | 0.19% |
| `step_inner` | 1,887.418 | 71.47% |
| Commit and old-host drop | 3.554 | 0.13% |
| Timer/record/return remainder | 0.131 | 0.0050% |
| Total outer step calls | 2,640.956 | 100% |

The surrounding collection loop averaged 2,641.089 ms; its additional 0.133 ms
is test-driver iteration and moving decisions. `step_inner` combines perception,
neural execution, world legality, biology, private learning and sealing. It also
contains a separate perception-world clone. Its 71.47% is not a neural-compute
share, and that inner copy must not be counted again as the outer clone.

Validation binds frozen founder weights and published authority to actual
private state before mutation. It hashes all 32,768 founder weights each tick
and recomputes a resident binding from neural/topology checkpoints, memory,
sampling and other private exports; those internal costs are not yet split.
Audit potentially repeated **immutable** founder work first. Reusing a sealed
runtime binding needs an explicit mutation/generation invalidation contract;
current mutable world/private/sampling consistency and fresh external-byte
checks on restore/admission still need protection. The full 28.2% boundary is
not established removable work, and no validation cache is implemented here.

The outer clone and commit/drop together consume about 0.33% in this fixture
and preserve atomic biology, cognition and sampling changes. **Rank outer clone
removal low**; larger/older worlds may differ. Decompose the inner aggregate
before choosing an inner optimization. Reject a proposed validation reuse if
mutation, corruption, restore or policy-change rejection changes, or if an
enclosing collection trial loses its gain. This documents measurement leads;
it implements no optimization and uses no held PR215 work.

Enabled/disabled timer order alternated; early/late uninstrumented controls had
overlapping ranges and drift. Observer overhead remains unresolved, so no
correction or speedup is inferred from the tiny remainder. All 32 rollouts
(2,080 ticks including warm-ups/controls) matched serialized decision and full
private-checkpoint hashes. That covers tested final states and decision
sequences, not arbitrary inputs, failure-path latency or learned competence.
The [summary and seven trial-level amounts](https://github.com/BobertNeek/A-Life/blob/5b244a66d183be0330e9585390312ac7124103af/docs/performance/20261031-host-step-summary.json)
and [raw per-tick CSV](https://github.com/BobertNeek/A-Life/blob/5b244a66d183be0330e9585390312ac7124103af/docs/performance/20261031-host-step-ticks.csv)
retain conditions, source hashes and controls. No formal confidence interval
is claimed for these dependent short VM samples.
