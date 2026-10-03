# Topology lifecycle validation and digest work — 2026-10-03

## Scope and source

Base: `d2e4a157ec8b14fc21445d55d560ab36c4551d49`. Candidate branch:
`codex/cloud-topology-lifecycle-20261003`. This report records the reviewed
bounded performance candidate and its cloud verification. Separate-branch
publication for full and fast CI is authorized. Main merge, GPU runs, PC
interaction, and training are outside this publication task.

The production change is confined to `TopologySidecar::advance_lifecycle` in
`crates/alife_core/src/topology.rs` and extraction of its existing diagnostics
validation. That file's validated SHA-256 is
`b3dfb71e9ed51fe7cdd3e0dffa8795d1e3e13d27c71c11fbc9f335fe3a37541f`.

## Measured path and smallest slice

The supplied PC baseline was six Nano512 organisms, GPU closed-loop, canonical
warm load, 1080p / 1×, optional traces and captures off: 18.23 FPS, 18.25 TPS,
50.04 ms runtime tick. These are parent-supplied measurements, not cloud GPU
measurements. The sidecar topology category was 4.711 ms/tick; its commit path
calls `observe_sealed_patch` followed by `advance_lifecycle` for each sealed
selection. The lifecycle change targets part of that category.

The preparation category labelled topology/concept (7.153 ms/tick) also includes
routed episodic recall, frame finalization, and final recall validation. It
cannot be attributed to topology alone. This patch does not change preparation
or claim a preparation improvement.

Previously, a lifecycle tick without split or merge validated the full map three
times and calculated its canonical digest twice. Each successful split or merge
also refreshed intermediate diagnostics. The candidate retains the outer
rollback clone, decay, bounded split then merge, existing validated transactional
map mutators, and ignored split/merge failures. It refreshes the digest once
after the final mutation, validates owner/profile and observation metadata, and
publishes the candidate. An ordinary tick therefore performs one full map
validation and one digest calculation. Public sidecar and portable validators
still fully validate the map and compute a fresh digest.

No cached digest is accepted as map-validation evidence. There is no admission
cache, change-only shortcut, cadence reduction, schema change, altered concept
capacity, or learned-state approximation. Successful map mutations must keep
their validated-before-publish contract; failed split/merge mutations must keep
their rollback contract. The private lifecycle optimization relies on those
existing contracts within the same synchronous transaction.

## Equivalence and verification

Three new regression tests compare the complete sidecar, errors, portable data,
and roundtrip state with a frozen copy of the prior lifecycle transaction:

- Six owner identities, empty/8/64/256-concept maps, repeated decay, and gap dismissal.
- Split, merge and relation/simplex/gap pruning, saturated concept capacity,
  ignored split ID overflow, ignored earlier-tick merge failure, and split then
  merge in one tick, including exact next IDs.
- Nine invalid identity/profile/metadata/map cases preserve the original state
  on rejection; public stale-digest and portable tamper rejection remain intact.
  Existing lifecycle repair of stale diagnostics is preserved.

The three tests passed against the prior production implementation before the
optimization. Initial fixture API and observation-count mistakes were corrected;
those were test-fixture failures, not production defects or relaxed contracts.

Passed on the candidate:

- Debug and release `cargo test -p alife_core --lib topology::`: five passed in
  each profile; one manual timing probe ignored by default.
- Existing `topological_map` and `topology_sidecar_degradation` integration tests:
  20 passed, including the 10,000-observation bounded-state test.
- `cargo clippy -p alife_core --lib -- -D warnings`.
- Workspace rustfmt check, static core boundary check, documentation check
  (77 assertions), and diff whitespace check.
- One read-only R2 review by a separate SOL6.1 agent: approved; no blocking or
  nonblocking finding. The reviewer did not run builds or tests.

Full workspace builds/tests, GPU and PC runs, and end-to-end gameplay validation
were not run. Relevant requirements preserved by the scoped equivalence checks
are AOA-CON-001/003/005 and AOA-PERSIST-001/002/004; the optimization follows
AOA-PERF-003/005/006 without asserting broader architecture compliance.

## Matched cloud lifecycle probe

Rust stable 1.98.1, optimized release profile, one test thread. Old and new
lifecycle transactions run in the same executable against identical valid
fixtures. Eight rounds alternate execution order; each round performs 200
decay-focused ticks. Seed construction/cloning and portable equality checks are
outside the timer; the lifecycle's own transaction clone is included. The table
uses the upper median of the eight elapsed wall-time totals.

| Concepts / edges / simplexes / gaps | Prior, 200 calls | Candidate, 200 calls | Reduction |
| --- | ---: | ---: | ---: |
| 8 / 7 / 8 / 8 | 2.1645 ms | 1.1589 ms | 46.5% |
| 64 / 63 / 64 / 64 | 18.2901 ms | 9.2241 ms | 49.6% |
| 256 / 255 / 256 / 64 | 81.0366 ms | 37.5152 ms | 53.7% |

All matched rounds produced identical portable state. This is lifecycle elapsed
wall time on synthetic CPU fixtures. CPU-active time, combined observation and
lifecycle cost, preparation, population scaling, GPU performance, and FPS
improvement remain unmeasured by this probe. No percentage above is a game FPS
or runtime tick claim.

Reproduce the focused timing probe in the validated cloud shell:

```bash
source /workspace/cloud-cpu-validation/env_graphics.sh
cargo test -p alife_core --release --lib lifecycle_matched_timing_probe -- --ignored --nocapture --test-threads=1
```

Commands used one Cargo job, incremental compilation off, dev/test debug symbols
off, and the shared lean cloud target directory. Resource guards retained at
least 5 GiB free disk and stopped above 14 GiB cgroup memory. Across focused
commands the observed high-water cgroup memory was 12.12 GiB (including cache),
and minimum free disk was 15.07 GiB. No guard stopped a command and no generated
output cleanup was needed.

Raw logs, per-command source hashes/diffs, resource receipts, timing output, and
the review receipt are under
`target/artifacts/cloud-topology-lifecycle-20261003/` in the selected environment.

## Controlled PC comparison and rollback

The PC owner can perform one controlled A/B comparison after branch CI passes:

1. Preserve an immutable copy of the existing `d2e4a15` canonical six-Nano512
   save. Use the existing baseline binary and a candidate binary built with the
   same Rust/build profile and runtime configuration. Record both exact SHAs;
   keep other repair branches separate from this comparison.
2. Start each run from the same save copy rather than carrying state forward
   from A to B. Retain six founders, GPU closed-loop, 1080p / 1×, identical
   camera/presentation settings, and optional tracing/captures off. Use the same
   warmup, measurement window, and existing timing counters as the baseline.
3. Record sidecar topology, preparation topology/concept, runtime tick, TPS/FPS,
   and available CPU/GPU timing. Compare the same aggregation in A and B, with
   no gameplay/training changes between them. Preserve both result receipts.

The expected changed category is sidecar topology. It still includes observation
work, so its improvement must be measured rather than projected from the
isolated lifecycle probe. Preparation improvement and FPS improvement remain
Unknown until the controlled comparison; this report contains no such claim.

Rollback is reverting the candidate commit and its tests/report. No persisted
state or asset migration is required. Main merge remains outside this task's
authorization. Cloud shell settings were only sourced for verification; this
candidate adds no persistent environment configuration.
