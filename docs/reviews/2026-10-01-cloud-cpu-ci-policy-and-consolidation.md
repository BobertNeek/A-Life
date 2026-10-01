# Cloud CPU CI policy and consolidation — 2026-10-01

The follow-up branch `codex/cloud-ci-followup-20261001` now supports the normal
unfiltered CPU workspace suite and contains both independently reviewed CPU
preparation changes. Main, the frozen integration candidate `f8ccfb13`, founder
assets, training implementation, and training campaigns remain untouched.

## Source and validation boundaries

The published CPU checkpoint is
`450305408dc48dec6de12e6e80be6cf60ced95ce`. The complete CPU execution and test
registration bind to that source. Subsequent code checkpoint
`eac8d00d390d163472f687714fde04e35240258a` fixes strict Clippy only inside
`gpu-tests` helpers/modules. Test names, guards, assertions, and default CPU
paths remain unchanged; explicit-profile strict Clippy passes at that checkpoint.
This report is a documentation-only successor.

| Check | Result |
| --- | --- |
| `cargo test --locked --workspace --all-targets` | **1,569 passed, zero failed, 15 ignored, zero filtered; 196 targets; 280.166 seconds** |
| App library in that run | 209 passed, zero failed |
| Focused FVR library checks | 14 passed; actual current save ownership, foundation projection, file roundtrip, source preservation, and cleanup covered |
| CPU workspace `-- --list` | 1,584 cases; none of the 125 audited required-hardware names registered; 14 selected CPU contracts retained |
| Explicit `alife_tools/gpu-tests` workspace `-- --list` | 1,840 cases; all 125 audited required-hardware names registered; no test body executed |
| Strict workspace Clippy | Passed with `-D warnings` |
| Strict explicit-GPU-profile Clippy | Passed with `-D warnings` at `eac8d00` |
| Formatting, static/dependency boundaries and boundary self-test | Passed |
| Documentation assertions | 77/77 passed |
| Meaningful headless benchmark | `benchmark_tiers_smoke_runs_tier_1_and_10_without_bevy_or_gpu` passed in the unfiltered suite |

The ignored tests retain their existing manual status. Physical GPU neural
execution, GPU behavior/performance gates, all-feature graphical builds, Windows
FPS, and a new training campaign were not run. Actions access remains unavailable
to this worker; the coordinator checks the exact published SHA separately.

## Current file fixtures and CPU/GPU policy

The FVR06 launch helper previously loaded the legacy `tiny_save.json` from disk,
although another in-memory helper already produced current state. FVR01/FVR02
integration preflights had the same historical disk dependency. All now write
isolated current files and clean up through RAII. Their thirty distinct founders
come from typed individual birth records, with builtin foundation identity,
current genomes/phenotypes/biochemical graphs, and fresh zero-acquired summaries.
This loads an existing test population; it does not change the product's separate
one-to-eight New Game limit. No historical cognition or GPU checkpoint is invented.
Strict persistence/migration and existing assertions remain enforced.

Five FVR tests allow honest unavailable adapter diagnostics and exercise preflight
metadata/file boundaries. They remain CPU tests. The former performance receipt
test is named `fvr06_record_performance_serializes_preflight_receipt_metadata_for_30_creatures`
to describe what it verifies. The new owned-save boundary test also checks unique
genome/entity IDs, all thirty foundation projections, exact roundtrip/source bytes,
and cleanup. Persistence ownership is scoped against AOA-AUTH-001,
AOA-PERSIST-001/002/004, and AOA-INV-003.

The initial 130-name filter audit included those five CPU-safe tests. The corrected
required-hardware set is 98 backend plus 27 app names. Backend guards already
existed; Tools' dev-dependency accidentally enabled them throughout CPU CI.
Tools now uses the narrow backend `test-support` feature for synthetic admission
budgets. `gpu-tests` explicitly enables that support and the real hardware tests;
Tools' explicit GPU feature propagates to App, Backend, and Training.
Mixed shader, source-audit, zero-tick, fake-adapter rejection, and receipt contracts
remain available in the default suite. Existing GPU gate command specifications
are unchanged. See [Development](../DEVELOPMENT.md#standard-checks).

GPU-only lint cleanup preserves the exact fast-memory nonzero condition by using
`fast_len()`, the implementation of `len()`. `is_empty()` additionally examines
lifetime records and would change that assertion. Other cleanup uses equivalent
option/copy/divisibility expressions, borrowed one-element slices, and a callback
alias preserving non-static captured lifetimes. No lint suppression was added.

## Consolidated performance source and evidence

| Original | Follow-up equivalent | Scope |
| --- | --- | --- |
| `352ff546` | `6e7d7d1` | Prior perception allocation change; retained once |
| `163ec10b` | `0819f59` | Existing radius predicate precedes the pure terrain sight ray |
| `911cd99f` | `ebaa40c` | Shared memory frame validation reused inside authenticated candidate loops |

The new code picks and their supporting `3e8f357`, `a9fefbc`, and `6c272139`
evidence picks have identical stable patch IDs to their originals. Their source
and measurement provenance remain intact. The full CPU suite covers their
aggregate interactions with current persistence and launch fixtures.
Memory's public input guards, two outer draft checks, and every candidate query,
key, context, and digest authentication remain. Memory evidence/arbitration
boundaries are scoped against AOA-MEM-002/006 and AOA-INV-003.

The [memory receipt](../performance/20261001-memory-frame-validation.md) records
six serialized-identical synthetic cases. Its eight-call, 26-candidate,
64-record CPU medians are recall 6.474 to 1.683 ms, finalization 8.330 to 1.921 ms,
and finalized validation 4.704 to 0.850 ms, on the original measured source.
The [terrain receipt](../performance/20261001-terrain-sight-range-gate.md) is
conditional on TerrainVision and out-of-radius objects. These original synthetic
CPU receipts establish no new Windows FPS or physical GPU improvement.

## Environment and resource receipts

The lean startup recipe is `/workspace/cloud-cpu-validation/setup.sh`; it creates
`env.sh`, verifies/install-checks the official stable toolchain, and runs repository
prerequisites without building the workspace. Source
`/workspace/cloud-cpu-validation/env.sh` in each development shell.

Verified: Rust/Cargo 1.98.1, rustfmt and Clippy, C compiler 14.2.0, pkg-config 1.8.1,
Git, and Python 3.12.14. No extra native dependency was needed for these headless
features. The environment has four vCPUs, 16 GiB memory, no swap, and a 32 GB disk.
CPU checks use two jobs, one test thread, `CARGO_INCREMENTAL=0`, and dev/test debug
levels zero. Generated core outputs were refreshed to avoid stale executables
from another worktree; source and data were retained.

Completed consolidated CPU checks peaked at 13.75 GiB cgroup memory. The explicit
GPU compile hit the 14 GiB protective guard twice while clean file cache alone
occupied 13.2 GiB. Read-only `POSIX_FADV_DONTNEED` evicted clean pages for completed
generated executables, reducing cgroup memory from 14.8 to 6.4 GB without deleting
or changing files. The unfinished GPU compile and lint then passed with one job
under the same guard. Final disk headroom exceeded 16.5 GiB; the 5 GiB disk floor
never stopped a check. These are cgroup totals, including cache, not process RSS.

Durable local receipts are under
`/workspace/A-Life/target/artifacts/cloud-cpu-ci-policy-513e468/`: `results.json`,
per-check logs/resource journals, `test-summary.json`, feature registration,
source/preservation and exact-pick receipts, environment revalidation, cache
reclamation, publication, and historical superseded runs. The initial fixture
constructor failure and memory stops are retained; later completed checks pass.

## Remaining evidence limit

Production preflight currently constructs a gameplay receipt marked authoritative
without dispatching a neural tick; its named batched-runtime-receipt path is not
written or validated by that preflight function. The CPU test proves serialization
only. This pre-existing production evidence gap is recorded here and is not repaired
or counted as GPU execution evidence in this CI/environment task.
