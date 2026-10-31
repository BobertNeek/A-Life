# Continuous runtime timing receipts

Continuous training already enables `GpuLivePerformanceMetrics` after restore
and any legacy sleep/adoption. `runtime_performance` now exports those existing
counters on `continuous-started-v1`, completed and untrained window events, and
`continuous-world-finished-v1`. No timer, neural/learning cadence or per-tick
output was added. Existing receipt kinds, schema numbers and fields remain.
An older receipt without this field means **unmeasured**, not zero.
The getter copies cached host counters; it does not query or synchronize a GPU.

`cumulative` is the exact runtime-instance snapshot, including any counters
already populated before measurement was enabled. The started event retains
that initial baseline. The final snapshot includes final checkpoint/quiescence
work. It is not a sum of trained rows or only waking collection time.

`delta` subtracts the previous **observation**, starting with the initial
snapshot. Thus a window delta includes intervening periodic checkpoint work
after the preceding window event, as well as the new window. The final delta
includes work since the last window observation. A failed delivery is still an
observation; consumers must retain receipt/stream errors rather than assume
every interval was delivered. World-tick endpoints and `committed_world_ticks`
distinguish world progress from `tick_calls`, `inference_rows` (active rows),
`learning_rows` and checkpoint polling.

`delta_status` is `initial_snapshot` with a null first delta; `complete` means
counter subtraction succeeded, not that a training run or GPU assay passed.
Decreased/reset/wrapped counters and saturated `u64::MAX` counters have null
field deltas and named reasons in `delta_unavailable_fields`; valid fields keep
their measured zeros. A changed counter schema or backward world clock makes
the whole interval delta unavailable. Raw cumulative values remain visible.
`sleep_journal_pending_entries_peak` is a runtime high-water mark, retained only
in cumulative snapshots and explicitly excluded from additive deltas.

An error after measurement starts attempts one
`continuous-runtime-performance-error-v1` event through the existing callback.
It includes counters accumulated since the previous observation, including
no-progress/failed calls where existing instrumentation records them. Some
existing timers finish only on success: an error snapshot is partial evidence,
not a reconstruction of unclosed spans. A returned diagnostic-delivery error
does not replace the original operation error. Setup failures before the
measurement boundary have no invented runtime timing. Fatal process/panic/OOM
failures cannot guarantee a receipt.

The existing finite-care retirement receipt keeps the initial snapshot, last
untrained-window snapshot and error snapshot in `runtime_performance_boundaries`.
Missing legacy evidence remains null. Its existing `observed_updates` retains
the last eight completed update events, and `native_summary` retains the final
native snapshot. These diagnostic fields do not determine retained training
rows, optimizer ages, seed ownership or checkpoint admission.

## Reading the existing timing ledger

All `*_wall_ns` fields are monotonic **host wall nanoseconds**, not GPU kernel
time. Timers are inclusive, nested and sometimes concurrent. Do not sum parent
and child fields, worker durations and update-thread waits, or infer an exclusive
partition from their names. Existing outer collection/PPO/install/snapshot
seconds remain available alongside this more detailed ledger.

| Existing scope | What to question after measuring |
| --- | --- |
| `tick_wall_ns`, `tick_preamble_wall_ns`, rollback clone counters | Separate tick frequency/no-progress polling from committed world progress; inspect actual rollback ownership before removing copies. |
| `perception_sleep_preparation_wall_ns` and named `preparation_*` counters | Find the expensive sensory, memory, attention, validation or sleep component before changing its cadence or authority. Child work can overlap under preparation workers. |
| `inference_transaction_wall_ns`, inference rows and readback counts | Host inference transaction includes submissions/waits, training-row work and the post-capture path. It is not a standalone kernel measurement. |
| `learning_transaction_wall_ns`, learning rows/readbacks | Outcome-bound learning is distinct from the external PPO/critic phase; preserve its causal state and commit rules. |
| selection, body/biochemistry seal, world advance and population reconciliation | Measure real action/physics/biology work; validation is necessary until an equivalent bounded improvement is established. |
| ordinary/checkpoint snapshot wall time, bytes, poll/map waits | Separate copied bytes, host waits and persistence work before claiming a bandwidth or synchronization bottleneck. These fields nest within broader operations. |
| sleep journal worker, enqueue, validation, encoding and persistence counters | Worker lifetime overlaps other work; update-thread enqueue/wait costs answer a different question. Preserve durability and recovery checks. |

In the legacy training path the inference timer starts **after pre-capture**,
and includes tick/post-capture work. The separately held fused-capture candidate
includes joint boundary copies/maps in that inference transaction; its legacy
structural/budget fallback still performs pre-capture outside it. The same
field name must not be treated as an unchanged stage definition across modes.
This export does not include or activate that held candidate. Its capture-mode
counters and isolated whole-operation timing fixture remain separate evidence.

No current GPU/gameplay/training stage durations are established by the CPU
export benchmark. Use newly collected receipts to rank costs, then ask whether
the work is necessary, can share immutable inputs, can batch safely, or can
avoid a synchronization boundary without changing causal learning and biology.

## Isolated export cost (Linux CPU, 2026-10-31)

A release-mode standalone harness extracted the exact current 116-u64 metrics
type and included the actual candidate observer source. It compared cloning and
JSON-encoding the same synthetic compact window event, with/without the export.
These counter values are synthetic inputs, **not measured runtime stage times**.
The populated fixture describes a 512-trained-row/one-bootstrap window; neither
fixture executes an organism, runtime tick, neural kernel or accepted checkpoint.

Each case retained seven alternating paired trials of 5,000 events/arm, after
500 warmup events/arm. The initial observer baseline, setup, logging and disk I/O
were outside timers. Both arms used the same allocation-request instrumentation.
Rust 1.98.1, serde 1.0.228 and serde_json 1.0.150; optimized Cargo release build.
The cloud CPU reported AMD EPYC 9V74 (x86_64 Linux 6.18.57).

| Fixture | Base median us/event | Export median us/event | Paired added median (min–max) us/event | First JSON bytes, base → export |
| --- | ---: | ---: | ---: | ---: |
| Zero counters/untrained window | 3.60 | 76.76 | 73.09 (71.78–85.03) | 1,480 → 10,342 |
| Populated 512-row window | 3.87 | 73.35 | 69.48 (63.92–73.00) | 1,583 → 11,377 |

Both fixtures added 561 allocation/reallocation requests and 94,095 requested
bytes per event. Requested bytes count allocation requests, including full
reallocation sizes; they are **not peak live memory or RSS**. Serialized sizes
grow slightly with counter/tick digit counts. The measurement includes allocator
instrumentation overhead and does not qualify Windows, GPU, stdout/disk delivery,
whole-game or training throughput. No new export runs on each successful tick;
this cost is paid only at the documented boundaries. Initial observation cost
was not measured. All raw pairs, source hashes and the harness are retained with
the diagnostic change's evidence.
