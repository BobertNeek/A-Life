# Continuous GPU requested-allocation phase ledger

The disabled raw-gradient cache adds 278,544 requested bytes to the default
256→384 frozen-replay resize overlap. The later fresh PPO epoch repeats that
same arena pair without the cache; residual fitting and policy installation do
not establish a larger training peak that makes the addition free. A complete
session comparison must also include the live arena and checkpoint capture.
This is a source audit and integer layout arithmetic, not a workload or VRAM
measurement. No code, tests, GPU execution, PC work or held capture changes ran.

A conservative **requested-byte policy guard is feasible from runtime CPU
inputs**. Unknown private live bytes are an input to that guard, not an intrinsic
implementation blocker. Legal dendrites add only about 4.36 MB to the two-arena
bound, so they do not force rejection under the existing 2 GiB policy. The current
2 MiB raw-buffer gate alone is insufficient. No guard is implemented or actual
private-process fit established here; physical allocator/external-VRAM limits
remain a separate qualification question.

## Source identity and accounting scope

Main is `b9f78ed077bd98c58998c8ad1fa23036ca30b096`; the relevant continuous,
trainer/PPO, critic and backend files are unchanged from `0fab094c`. Candidate
`b1fd11290c99ec0ca6aa457a466cca046831885e` supplies only the disabled test-entry
cache path. The [arithmetic receipt](20261101-gpu-phase-admission-ledger.json)
records exact source hashes and labels conditional examples. No private current
checkpoint, resident receipt or hardware allocation report was loaded.

Count requested `Buffer.size()` and source-known HAL staging separately from
physical VRAM. Let `L` be current live arena requested bytes plus any other live
non-arena buffers, excluding the timestamp buffers counted separately below.
Live and offline backends share Device/Queue/kernels, not
separate memory heaps. Each owns two timestamp sets: 16-byte resolve plus 16-byte readback per
set, 64 bytes/backend, plus opaque query/driver resources. The fresh offline
backend has empty class buckets; its empty admission receipt is not free 2 GiB.

The production profile declares 2 GiB logical/physical allocation ceilings. Its
arena receipt sums class buckets and excludes raw trainer/PPO/cache buffers.
Neither this policy nor per-buffer device limits measure available physical
VRAM. Host replay/window RAM and disk temporary limits are separate budgets.
Anchors: `closed_loop_runtime.rs:2812,4777`; `closed_loop_admission.rs:15–39`.

## Exact arena formulas

Use `N` neurons, `S` compiled synapses, `m` phenotype microsteps, `T` replay ticks,
`r` routes, `D` branches, `I` dendritic inputs. Offline GPU capacity `C=32` is fixed
at `MAX_ACTION_CANDIDATES`, even when actual candidates per captured row vary.
It also reserves all 64 structural slots per tick. Actual inactive microsteps do
not shrink the allocation.

| Resource | Requested words, before multiplying by 4 |
| --- | --- |
| State | `3U`, where `U=(mT+1)N` |
| Gradients | `2U+mTN+2S+T(C+192)` |
| Outputs | `T(3C+192)+4` |
| Training packet | `T(3N+64C+4+r+S+320+384)+3N` |
| Metadata | `12N+54+10S+5D+4I` |
| Weights/optimizer/mask | `5S` combined |

A complete trainer arena `B(T,D,I)` is four times the table plus
`256mT` header bytes and 16 step-readback bytes. Separate persistent actor
accumulation `A=4S` is not in `B`. `replay_allocation_bytes` returns
`max(initial32tick/nonreplay arena, replay arena)`, excluding old/new overlap,
PPO, `A`, uploads and other resources. Shape equality is exact ticks plus
bitwise dendrite metadata; a change constructs the **entire new arena before
replacing the old**. Copies of weights/optimizer submit without a wait.
PPO's old bindgroup retains prior output/state until rebind; queued commands
retain their referenced buffers until completion.
Anchors: main `trainer.rs:259,388–454,1783,2241–2303,2393–2431`.

PPO uses fixed capacity `Q=2048`, not the current loss count:
`P=128+256Q+4(45Q+1)+20(N+1)`, **934,040 B at N2048**. Rebind replaces bindings,
not these buffers. The raw cache is `K=4w(S+N+2)`: **278,544 B for two N2048/S32768
windows**. It is allocated after the first frozen window's metric wait and
retained through the second frozen resize and ordered accumulation, then dropped
before KL/apply. Current source checks only its 2 MiB/per-buffer bounds.
Anchors: main `ppo.rs:450,585,1083`; candidate `ppo.rs:1292,1434–1488,1607`.

## Actual successful default lifecycle

Below, `B_t` retains its actual initial branch metadata. Add `L`, both backend
timestamp resources, `A`, known upload staging and the row-specific readbacks
where applicable. Segments/gaps change chunk geometry; they do not justify an
assumed 512→256 transition.

| Phase | Shapes and simultaneous requested resources |
| --- | --- |
| Startup/collection verification | Initial nonreplay arena; first captured row **each cohort**, and actual gaps, call `verify(T1)`. Numeric receipt has compact `4T(C+3N)` plus equal readback staging: 49,408 B at T1/N2048. Not per captured row. |
| Initial frozen values | All 513 references are processed per segment in chunks of 512. Uninterrupted case is **512 then 1 bootstrap**, leaving `B1`; each predict reads `4T` bytes and waits. Resize envelope `B_old+B_new+P`, e.g. 512↔1, with binding/queued retention. |
| Frozen global PPO validation | Loss spans are 256/burn0 then 384/burn128. First resize is 1→256. Candidate allocates `K` only after that metric wait; second resize has `B256+B384+P+A+K`. Metrics staging is `32*loss_rows`=8192 B for 256. |
| First effective batch | Both windows fit batch 8. Cache reuses raw vectors in original scaled order on final `B384`; status 8 B per accumulation waits. `K` is dropped before KL/preflight/apply. Value coefficient is **0** here: no 40,980 B value backup or value update. |
| Later fresh PPO epoch | Default epochs 2; next epoch performs 384→256→384 freshly. Same arena-pair envelope, no `K`. Value preflight 4 B and actor status 8 B are blocking sequential reads. Early KL/error can prevent later phases, so do not borrow an unexecuted future peak. |
| Residual value fitting | Captured CPU features include 513 states. A separate GPU head trains 512 Bellman rows while **old PPO `P` and `B384` remain alive**. Private set 4,251,732 B plus max sequential checkpoint staging 24,588 B =4,276,320 B, before initialization/upload stages. Two accepted actor steps reuse this same batch, no growth. |
| Install/rebind | Private critic drops; replacing value state drops old `P` and leaves a CPU pending checkpoint. Export/rebind read weight buffers (4 bytes per actor synapse) sequentially. Install changes the existing live arena in place: 32 identity bytes + 4 times actual uploaded live weight words. Its initialized COPY_SRC buffers also have bounded init staging. No full live bucket replacement. `B384` stays allocated; replay shape becomes None. |
| Snapshot/final handoff | Trainer checkpoint staging is 16S=524,288 B; pending value checkpoint is CPU-only. Live reads described below are sequential. No cache or old PPO/critic buffers. |
| Next cohort | The first-row verifier forces retained 384→1 and blocks **before** later 1→512 values. A hypothetical 384→512 overlap is not executed by this default call graph. |

Anchors: `continuous_world.rs:656–686,1078,1198–1222,1286–1383,1425–1465`;
`foundation_training.rs:4110`; trainer checkpoint `trainer.rs:1528–1602`.
Critic: `critic_residual.rs:11,125–177`; `critic_calibration.rs:38–86`.
For feature width N, F states and R rows, its set is
`4NF+32R+16(N+1)+68`; max batch 2048/2049 requests 16,883,796 B, not the entire
36,001-state host corpus. Generic growth could overlap old/new data, but actual
continuous rotations start with the maximal batch and later fit its capacity.

For live snapshots define `U=4*mutable_state_words.len` and
`T_live=4*(immutable_plan_words.len+immutable_weight_words.len)`. Exact population
capture requests `E=sum(brain_record_bytes+identity_bytes+T_live+U)`; one brain
is **`176+T_live+U`**. These are class-slot strides, not occupied synapse counts.
The sequential snapshot/handoff extra is `max(E,maxU,maxT_live,16S_actor)`.
Existing CPU resident ranges give exact lengths. A public class-plan slot receipt
also bounds E via logical commit+padding (subtract dispatch/staging/compact
readback terms for exact E). No GPU or future replay callback is required.
Anchors: `closed_loop_exact_capture.rs:423–476`;
`closed_loop_checkpoint.rs:1632`; `closed_loop_sleep.rs:936,1730–1755`;
`closed_loop_buffers/bucket.rs:207–221,2423–2434,4281–4297`.
Actual private live receipt/strides remain runtime inputs, unobserved here.

## Conditional 2 GiB decision and uploads

For N2048/S32768/m4, replay C32:
`B_T=(366352+4r)T+2392312+20D+16I` bytes. Legal D≤4096/I≤131072 adds
**2,179,072 B per arena**, independent of T. These limits do not force rejection.
For the explicitly conditional **r17,D0,I0** example, B256=96,195,832 and
B384=143,097,592 B. Older measured scratch examples are 52 B larger because their
fixture has D1/I2: `4*(5+8)=52`, not a formula mismatch or private-world estimate.

Counting `P+A+K`, an 8192 B critical readback allowance and both backend timestamps,
the pair subtotal is 240,645,400 B; maximizing legal metadata in both arenas gives
**245,003,544 B**. This excludes live arena and uploads. Under 2 GiB it leaves
**1,902,480,104 B** for those other terms. Actual L is available to the caller,
but the fact that profile L≤2 GiB alone proves no remaining headroom.

Pending uploads are source-bounded requested HAL allocations, not wholly
unknowable. wgpu-core 29 `Queue::write_buffer` creates exact data-size staging;
resources survive until their submission retires. Sum all writes since an
existing completed boundary, even when some may retire sooner. At the critical
384-window phase:

- `queue.write_buffer` requests at most training packet bytes +`256mT` headers
  +`1536T` speech-target bytes +128 PPO header +256 bytes per loss row.
- Mapped-at-creation trainer init can add shadow staging for headers, metadata,
  weights and mask: `256mT+4M(D,I)+8S`. DeviceExt pads to 4 B; all these sizes are
  already aligned. Zero buffers do not use this path.

Forward and raw-cache backward methods encode commands only; they add no header
uploads. Later accumulation/apply methods rewrite headers after existing blocking
metric/status/preflight receipts, in separate pending-write intervals. Those
writes belong to their respective phase bounds rather than the critical sum.

Conditional r17 numbers are 65,101,440 B writes and 4,243,672 B worst-metadata init
shadows. Adding both yields **314,348,656 B**, leaving **1,833,134,992 B** under2 GiB
for actual live/other baseline. This remains a conservative critical-phase
subtotal: compare every listed value/critic/init/install/snapshot phase too.
Critic initial buffers are all initialized; installer initialized COPY_SRC
sources may similarly need shadow staging. Their lengths are known from the
same CPU formulas. Allocator padding/reserved blocks are a separate quantity.
Cached dependency anchors: wgpu-core 29.0.3 `queue.rs:518–568`,
`device/resource.rs:1077–1167`, `resource.rs:1120–1136`;
wgpu 29.0.3 `util/device.rs:40–75`. No feature or dependency changed.

## Minimal admission seam and allocator report

At the cache's existing post-metric-wait allocation point, all preceding queue
writes have completed. Known inputs are current exact buffers, phenotype
N/S/m/routes, fixed `P/A`, window count and device limits. Actual main already
has spans/T/burn geometry and the live arena receipt. Replay refs contain no
futureD/I, but legal metadata caps suffice without invoking loaders early.
Future exact metadata could tighten only about 4.36 MB for this two-arena case.

The minimum seam is an **optional-cache admission decision supplied from that
caller context**, using checked phase maxima plus the existing declared policy.
Compute max over value chunks, frozen/later PPO transitions, residual fit and
actual slot snapshot/install ranges; count upload/init/requested readback
lifetimes. Read L at runtime or use a proven CPU class-plan upper bound with
actual growth/alignment parameters. If no trustworthy bound fits, decline the
optional cache and keep original callbacks/errors ordered. Do not treat the
staging backend's empty arena receipt, host 8 GiB limit or disk quota as GPU space.
No generic future exact peak can be inferred from the first loaded window alone.

`Device::generate_allocator_report()` in pinned wgpu 29.0.3 is an optional
diagnostic of the allocator's tracked resources. **Some does not prove complete
application-buffer coverage or a conservative shared-device total.** In the
pinned Dx12 backend, `generate_allocator_report` always wraps the allocator's
report in Some, but the committed-buffer workaround bypasses that allocator:
when suballocation is unsupported, buffers use `CreateCommittedResource` and
`Allocation::none`. Those buffers are absent from the returned suballocation
report. Source: wgpu-hal 29.0.3 `dx12/device.rs:2588` and
`dx12/suballocation.rs:21–32,100–126,174–189,405–453`.
This is not only a workaround-specific limitation: Dx12 also creates its
unconditional 256 KiB zero buffer directly with `CreateCommittedResource`, outside
the allocator report (`dx12/device.rs:72–113`; `dx12/mod.rs:512`).

Backend and allocation-path coverage must be established before using such a
report as the complete baseline; otherwise retain explicit source-accounted
live/offline buffers for the requested-byte guard. A report may still help
explain tracked allocations omitted by the empty staging arena receipt, without
double-counting the same manually accounted resources. Allocated bytes describe
tracked live suballocations; reserved bytes include unused block capacity. This
is distinct from disabled internal counters. None means unknown, not zero;
Some alone is insufficient. Pending frees, allocator fragmentation, heap
placement, untracked resources and other processes also prevent equating the
report with free VRAM. It neither reserves memory nor replaces the future phase
bound or physical qualification.

## Collection bottleneck and held fusion scope

The numeric verifier is **not per row**: it runs once at cohort start and actual
waking gaps. At T1 it has 6 compute dispatches (initialization + 4 forward microsteps + candidate output), a compact
24,704 B receipt and a blocking readback; shape change can add a copy submission.
No source-based or measured evidence makes one such call per 513-row cohort a
material default collection bottleneck. Many real segments could increase its
frequency; no timing claim follows from source alone.

The true every-row seam is the already-batched live pre-capture→neural tick→post-
capture waits. Held `ed393a6c2885827773f303d3fde2a104048adfe9` changes exactly
that seam through `prepare_training_tick_capture` and
`tick_memory_batch_training_captured`. It fuses eligible stable ranges/generations
when no structural capture is needed and paired staging fits its ceiling, with
structural/oversize legacy fallback. It does not modify the numeric verifier.
The full cc7 handoff object was not available locally; session history identifies
it as carrying this held fusion. `time-ledger.md:138` records the existing
unverified/unpublished publication hold. No fusion/capture implementation is
proposed or changed here, and its comparison must include the complete runtime
boundary, not differently scoped timer values.
