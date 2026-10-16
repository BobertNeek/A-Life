# Reuse immutable per-route work receipts

## Change and trust boundary

`derive_executed_work` now reads the recurrent-synapse count already owned by
`BrainPhenotype::budgets().routes`, instead of walking the same immutable
projection span during every firing microstep. This benefits all three ordinary
awake-tick callers: affordability, batch packing and the completed-work receipt.
No new cache, allocation, resident field or invalidation protocol is introduced.

`BrainPhenotype` has private fields and read-only graph/budget accessors.
Its compiler constructors and custom deserializer validate canonical projection
and receipt route indices, range alignment, per-route synapse-kind counts and
global totals. The independent full recount at that boundary is retained.
The canonical hash also covers the receipts. A new core regression transfers one
claimed synapse between routes while keeping global totals, local budgets and
a recomputed correct hash consistent; full phenotype validation and
deserialization must still reject it. Reordered receipts are also rejected.

The immutable phenotype and its receipts are cloned or rebuilt together on
admission, restore/rebind and growth. Lifetime weights, eligibility, sleep state,
dendritic branches and v1.1 packed topology overlays do not mutate that phenotype.
Accounting for those overlays remains exactly the same phenotype-based accounting
as before this patch; this report does not qualify a new structural-accounting
policy. The existing independent GPU active-tile/active-synapse comparison remains
unchanged and still rejects a mismatch.

All route-schedule validation, masks, firing cadence, microstep loops, checked
arithmetic, candidate/memory work, ATP policy and GPU row receipt checks remain.
This preserves AOA-BRAIN-009, AOA-REG-003 and AOA-PERF-001/002/007. No biological,
learning, sleep, perception, action or neural execution mechanism is removed.

## CPU component evidence

The [harness](evidence/20261016-live-work-receipts-bench.rs) compiles the actual
production accounting source and a frozen copy of the original scanning
function from `69f00c05ec6774c3655ae87c5d4baa5df3861929` with identical optimized
settings. It uses the existing source-matched optimized core library. The only
core source change here is new `cfg(test)` coverage, absent from that library.
The backend library supplies types for unused dispatch-header definitions; no
backend or GPU execution enters the timed loop.

Hardware: Linux x86_64, AMD EPYC 9V74, one pinned CPU, rustc 1.99.0, core and
harness opt-level 3, debug assertions off. Each class uses homogeneous independent
phenotype clones, GroundedObjectSlotsV1, maturation 0.35 and seed 47601. Real policy
decisions change masks and microsteps through four pressure inputs. Both modes
receive the same fresh schedule; setup and comparisons are outside the timer.
There are two warmup pairs and nine alternating-order samples per fixture.

Median milliseconds per population tick, including three accounting calls per
organism, are:

| Brain class | Population | Original scan | Existing receipts |
| --- | ---: | ---: | ---: |
| N512 | 1 | 0.00416 | 0.000307 |
| N512 | 10 | 0.0435 | 0.00290 |
| N512 | 50 | 0.231 | 0.0157 |
| N1024 | 1 | 0.00756 | 0.000299 |
| N1024 | 10 | 0.0794 | 0.00288 |
| N1024 | 50 | 0.808 | 0.0163 |
| N2048 | 1 | 0.0522 | 0.000925 |
| N2048 | 10 | 0.535 | 0.00894 |
| N2048 | 50 | 4.01 | 0.0459 |

For the N2048 schedule, each organism's 20-tick sample replaces 2,603,520
source-level synapse-kind visits with 1,170 existing receipt reads. That is
130,176,000 visits eliminated for the 50-organism sample; it is a deterministic
source work count, not a measured memory-traffic claim. All 98,820 final-sample
counter rows, plus 21,960 warmup rows, match exactly.

[Raw samples and source/artifact hashes](evidence/20261016-live-work-receipts.json)
include the build recipe, schedule examples and all brain/population rows.
These are warm-cache host work-accounting measurements, with other host tenants
uncontrolled. They do not establish GPU speed, FPS, whole-frame performance,
Windows performance, behavioral fidelity, save/load acceptance or population
scaling qualification under AOA-PERF-005/AOA-GRADE-006.

## Verification

- 14 CPU activity tests pass, including 2,010 changing-schedule comparisons
  against the original scan, every singleton/all-but-one route, all promoted
  classes, both sensor profiles, maturation endpoints, microsteps 1/2/3/4/7/255,
  decoder-only routes, malformed schedules and exact policy cost/receipt equality
- Three core record tests pass, including the rehashed forged-count rejection
- 99 additional CPU backend/unit/buffer/memory regressions pass
- Strict backend all-target/all-feature Clippy, affected core test Clippy,
  formatting, core boundaries and 77 documentation assertions pass

No physical GPU, training, game or user-computer task was run.
