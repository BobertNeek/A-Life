# CPU preparation measurements — 2026-10-01

These historical measurements support isolated CPU preparation changes now
included in candidate `a11dae6a5708bddf7d4255990f3bd4abaf12759a`.
They do not measure that candidate's game frame time. Windows/RTX before/after
performance and the 20 FPS/50-creature goal remain **Unknown**.

The older receipts retain every timing sample, parameter, source hash and outcome.
Copied console mirrors were removed after exact field comparison. Reproducible
payload dumps retain their lengths/digests; probes regenerate the words, and the
original vectors remain in Git at `a11dae6a`.

The subsequent [prepared-attention comparison](20261001-attention-preparation.md)
has separate fixtures and source `770ef12a`; its report, samples and probe remain
unchanged in the successor. The later [topology comparison](20261001-topology-preparation.md)
times observations separately and excludes that attention chain. These independent
measurements cannot be combined into a frame-speedup estimate.

The [fresh-recall comparison and bounded preparation prototype](20261001-parallel-preparation.md)
uses source `e8d5633e` and separate fixtures. Only the fresh-result diagnostic
guard entered that measured production revision; its threaded timings remain an
evidence-only prototype. The successor [bounded production split](20261001-two-worker-production.md)
uses one scoped child plus the caller for eligible CPU preparation. Serial
housekeeping, draft capture and ordered backend/commit authority remain intact;
the later [Terrain hint extension](20261001-terrain-hint-workers.md) admits
TerrainVision after each owner's serial hint capture. Its documented asynchronous
timing tradeoff does not move prior/cache or neural/commit authority to workers.
These CPU checks and measurements cannot certify Windows/RTX performance.
The [combined validation](../reviews/2026-10-01-cloud-validation.md#terrain-hint-successor--2026-10-02)
preserves the reviewed source and all original receipts after the serial checkpoint.

The later [routing/attention copy removal](20261002-preparation-copy.md) starts
from combined `5abafe19`. It measures fewer allocations; its complete CPU suffix
sweep does not establish a throughput improvement.

## Integrated serial guard comparison

The [combined receipt](evidence/20261001-integrated-serial-preparation.json) and
[checked builder](evidence/20261001-integrated-serial-build.py) compare coherent
`dc3e916e` before both guards with `d9724dfa` containing reviewed `e8d5633e`
fresh-recall and `4eed7acc` finalization guards. They measure the two changes
together on identical inputs; the earlier isolated percentages are not added.

For 26 candidates and 64 actually retained records:

| Independent owners | Helper wall before → after ms | Process CPU before → after ms |
| ---: | ---: | ---: |
| 8 | 10.743 → 7.973 | 10.742 → 7.970 |
| 16 | 20.830 → 16.579 | 20.830 → 16.575 |
| 32 | 39.964 → 32.957 | 39.962 → 32.956 |
| 50 | 63.954 → 49.095 | 63.939 → 49.091 |

Two sequential before→after/after→before process pairs retain nine samples each:
18 per source/case across 24 cases (2/26 candidates, 0/64/256 records,
8/16/32/50 owners). All 864 timed batches and 22,896 full typed instance checks
pass; serialized input and core-result/payload hashes match all four processes.
Host-only upload binding/digest/offset metadata is compared in typed same-process
equality and derived from matching frame/slot fixtures, rather than serialized
into the cross-process proof. Banks/topologies stay unchanged. Proofs and output
destruction are outside timers; result allocation/ordered collection is inside.

The timer includes both recalls/contexts, attention/routing/novelty, projection,
finalization, retained public guards and CPU upload encoding. It excludes world/
sensor/receptor derivation, sleep/hints/resident mutation, backend admission,
topology observation, GPU work, sealing/save/lifecycle and rendering. The separate
public-validator attribution is outside the helper timer. The successor's two
50-owner process medians are 50.008 and 48.735 ms; **live tick/frame throughput
and 20 FPS with 50 creatures remain Unknown**. No production threading ran.

Linux EPYC/KVM, four-core quota, affinity 0–3 and Rust 1.98.1 release/opt-level 3
were used for every process. No local build ran during sampling; other host
tenants are uncontrolled. RSS/high-water samples include fixtures, retained
reference/results and allocator history; they do not prove memory savings.
An initial shared-target successor reused baseline rlibs and was rejected before
timing. The accepted successor uses an isolated target and distinct Core/executable
hashes. Library hashes are build-time identities; executables were preserved.

Reproduce in the two source worktrees using separate `CARGO_TARGET_DIR` values.
Emit artifacts with `cargo +stable build --locked --release -p alife_core -p alife_world -p alife_gpu_backend --message-format=json`. From each worktree,
run the absolute path to the checked builder with the artifact JSONL and a unique
executable path. Preserve executables/manifests, then run `taskset -c 0-3` in
before, after, after, before order. The builder pins existing fixture/probe inputs,
asserts unchanged preparation/proof functions and removes the threading screen.

## Results and measured sources

Each receipt retains exact commits, source/probe/executable hashes, individual
samples, ranges, output proofs and checks. Baseline → implementation below names
the measured originals; the candidate contains equivalent reviewed picks.

| Operation | Measured sources | Representative result | Receipt / reproduction input |
| --- | --- | --- | --- |
| Perception construction, rebase and exact validation | `ad99ac1c` → `352ff546` | 2/8/32 candidates: 74.328/123.221/297.459 → 73.029/117.673/276.814 µs per eight-row cohort; ranges overlap | [Receipt](evidence/20261001-live-perception-preparation.json), [probe](evidence/20261001-live-perception-probe.rs) |
| Indexed terrain preparation | `3e8f3578` → `163ec10b` | TerrainVision, 32/128 distant targets: 3.477288/10.277293 → 1.545382/1.603530 ms per eight rows; GroundedObjectSlots controls approximately unchanged | [Receipt](evidence/20261001-terrain-sight-range-gate.json), [probe](evidence/20261001-terrain-sight-probe.rs) |
| Memory recall / finalize / finalized validation | `a9fefbc4` → `911cd99f` | 26 candidates, 64 records: 6.473649/8.330038/4.703639 → 1.683021/1.921215/0.849823 ms per eight calls | [Receipt](evidence/20261001-memory-frame-validation.json), [probe](evidence/20261001-memory-validation-probe.rs) |
| Fresh memory-upload constructor | `2bad5e02` → `ffeea252` | 26 candidates, 64 records, projection present: 1.980206 → 1.018319 ms per eight calls; all 12 constructor cases improve 47–53%; unchanged validator controls vary within about ±6.1% | [Receipt](evidence/20261001-memory-upload-validation.json), [probe](evidence/20261001-memory-upload-probe.rs) |
| Shared episodic recall | `2bad5e02` → `cba0740b` (combined `d0da0b58`) | 32 repeated queries: 1.73372 → 0.50073 ms/frame; evaluations 4,096 → 128. 16 distinct queries: 0.54455 → 0.55095 ms/frame; 512 evaluations in both | [Receipt](evidence/20261001-memory-quality-and-recall.json); ignored `memory_cpu_recall_microbenchmark` test |
| Receptor compilation, no implementation comparison | `e993fcfc` | 0.922470 ms/eight calls; range 0.894752–0.993219 ms | [Receipt](evidence/20261001-neural-receptor-preparation.json), [probe](evidence/20261001-neural-receptor-probe.rs) |

The terrain gate cannot improve the supplied GroundedObjectSlots run. Allocation
counts fell, but requested bytes measure traffic, not retained memory. Memory
operations overlap semantically; their gains cannot be added into a frame-speedup
estimate. The receptor copy-only control omits cache access/invalidation and
chemistry projection; it does not measure an implemented cache.

## Conditions and equivalence

All runs used AMD EPYC 9V74/KVM Linux 6.18.44, five exposed CPUs, CPU-0 affinity,
and Rust 1.98.1. Fixtures were synthetic and warmed; no GPU device, authoritative
tick, renderer or training campaign ran.

| Probe | Profile and fixture | Sampling |
| --- | --- | --- |
| Perception | Core/backend test rlibs at their opt-level 2 overrides; probe `rustc -O`; eight rows, 2/8/32 candidates, one N512 slot | Three alternating process pairs; 200 warmup cohorts; nine samples of 1,000 cohorts/process. Allocation counts separately over 100 cohorts with counting disabled during timing |
| Terrain | Core/World opt-level 2 test rlibs; probe `rustc -O`; eight observers, 1,716 synthetic boxes, 0/32/128 additional distant objects, both profiles; exact-radius and just-outside targets included | Three alternating pairs, 20 warmup cohorts, five samples of 20 cohorts/process |
| Memory frame | Core opt-level 2 test rlib; probe `rustc -O`; 2/8/26 candidates, 0/64 records, two targets/four families; eight repeated calls on one immutable draft/bank | Same pair/warmup/sample schedule as terrain |
| Upload | True release libraries/probe, opt-level 3, debug assertions off; same candidate/record matrix, projection absent/present, real 54-lane extension fixture; eight calls on one immutable frame/recall/slot | Same pair/warmup/sample schedule as terrain |
| Shared recall | Cargo test profile, debug assertions on, debug/incremental output off; 256-record bank | Three alternating pairs; five batches of 100 calls/workload/process; median of 15 batches divided by 100; destruction included |
| Receptors | Core opt-level 2 test rlib; probe `rustc -O`; one GroundedObjectSlotsV1 N512 phenotype, 1,799 synapses | Three processes; 100 warmup calls; nine samples of 1,000 eight-call cohorts/process |

Perception payload hashes, terrain serialized drafts and memory serialized
frame/recall/key/receipt/context/bank proofs match before/after. The original upload
comparison matched complete u32 vectors across six processes. Full host Debug text was not retained;
its recorded lengths/hashes cannot be independently recomputed from JSON alone.
Proof construction is outside timing. Eight calls do not mean eight live creatures.
Observed medians are not statistical guarantees or population-scaling evidence.

The shared-recall baseline used an isolated output directory and the identical
benchmark appended to its historical test file. An earlier shared-output retry
reused the patched executable and was rejected; none of its samples are retained
as baseline measurements. Required work counts distinguish the actual binaries.

## Reproduction

Use separate worktrees and target directories for each measured revision, and
copy the retained probe to the same relative path in each worktree so its fixture
imports resolve against that revision. Preserve each executable before advancing
to the next source. Set `CARGO_INCREMENTAL=0`, dev/test debug levels zero, bounded
build jobs and one test thread. Use Rust 1.98.1 to match these measurements.

For memory/terrain, build `alife_world --profile test --no-default-features`;
for perception use the `closed_loop_buffer_contracts` test-profile build; for
upload use `alife_gpu_backend --release --no-default-features`. Capture Cargo's
`--message-format=json` output and select the corresponding `.rlib` files from
`compiler-artifact` records. Link the probe with `rustc --edition=2021 -O`, the
matching `-L dependency=...` directory and `--extern` paths. Upload additionally
requires `-C opt-level=3 -C debug-assertions=off`. Probe imports identify the
required libraries. Avoid choosing arbitrary artifacts from a shared cache.
Run the executable with `taskset -c 0`; its source defines the measured schedule.

For shared recall, capture the test executable from each isolated build, append
the identical ignored benchmark to the baseline test file, then run:

```sh
taskset -c 0 <captured-test-executable> memory_cpu_recall_microbenchmark \
  --exact --ignored --nocapture --test-threads=1
```

Verify 4,096/128 repeated-case evaluations before/after and 512 distinct-case
evaluations in both before interpreting timing. Receipt hashes bind the exact
benchmark fixture. Re-run benchmarks only when doing a new comparison.

## Existing Windows observation and limits

The coordinator supplied extracted fields from the release RTX 3050 receipt at
source `c050013d5cefe4760b019e46cb0ebb37d774f300`: eight Nano512 creatures,
GroundedObjectSlotsV1, 331 ticks, 330 paired GPU samples, 60.18 seconds.
Runtime averaged 173.76 ms/tick and preparation 111.85 ms: topology/concept
46.56 ms, episodic 40.42 ms, upload preparation 19.35 ms, grounded perception
2.40 ms. Rollback copies were 0.54 ms; ordinary snapshots/checkpoint/sleep
captures were zero. Nested counters must not be summed as disjoint stages.
The [memory receipt](evidence/20261001-memory-frame-validation.json) retains all
supplied fields. The raw Windows file, exact argv/environment, starting save,
candidate roster and memory/topology occupancy were unavailable in cloud.
This is historical supplied evidence, not a new measurement or current result.

Compare gameplay with the same save, build, graphics, run mode and diagnostics;
retain source/executable identity, preparation timings, candidate/record counts,
GPU timestamp coverage, zero-progress and capture counts. The existing recorder
forces Immediate presentation and detailed timers; optional screenshots/JSONL
writes may affect wall time. Leaf attribution, current hardware behavior and
50-creature performance remain unmeasured. See the [validation record](../reviews/2026-10-01-cloud-validation.md)
for correctness results and preserved failures.
