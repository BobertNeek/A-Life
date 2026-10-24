# Mixed-care history hashing: synthetic allocation measurements

The remaining total-history cost after PR142/PR148 is real on main
`84ecdd11`: every fresh ledger read reconstructs all allocations and hashes the
protected prefix; every write verifies that prefix again. PR148 only removed
the duplicate post-write hash when the protected boundary has not expanded.

`mixed_care_ledger.canonical_hash` now hashes the same list punctuation and
sorted JSON for each allocation in order, avoiding a full-prefix slice, JSON
string, and encoded byte buffer. Digest bytes are identical to the former
`sha256(json.dumps(runs[:count], sort_keys=True, allow_nan=False).encode())`.
All protected allocations are still visited. Serialization temporaries scale
with the largest allocation instead of all history; the decoded history itself
still scales with total history.

## Measured synthetic results

Linux x86-64 cloud CPU, AMD EPYC 7763, CPython 3.12.14. These are storage-layer
fixtures, with 16 accepted attempts per allocation, nested receipts, exact large
integer ages, and a 512-byte note payload. Each fixture has one current
allocation and N-1 immutable segments. It contains no native checkpoint bytes
and does not exercise the scheduler's typed plan, lineage or source-file checks.
No training was run. These are neither native RTX results nor gameplay evidence.

[The utility](../../scripts/benchmark_mixed_care_ledger.py) creates and removes
disposable fixtures, excludes construction and warming, and compares the old
hash with production on otherwise identical code. Wall/CPU times are medians of
five untraced calls. Memory comes from a separate `tracemalloc` invocation with
the result retained; existing input graphs are excluded. Traced allocation peak
is not resident RAM. Live block counts in the [raw results](20261024-mixed-ledger-history-hash.json)
are retained allocations, not total allocation churn.

| Allocations | Logical JSON MiB | Prefix hash peak MiB, old → new | Full read peak MiB, old → new | Unchanged write peak MiB, old → new |
| ---: | ---: | ---: | ---: | ---: |
| 10 | 0.183 | 0.331 → 0.038 | 64.369 → 64.368 | 64.045 → 64.045 |
| 100 | 1.832 | 3.629 → 0.038 | 68.022 → 68.020 | 64.045 → 64.045 |
| 1,000 | 18.363 | 36.698 → 0.038 | 104.495 → 104.490 | 64.045 → 64.045 |
| 5,000 | 91.971 | 183.946 → 0.039 | 382.375 → 266.893 | 183.965 → 64.045 |

At 5,000 allocations the measured full-read peak falls 30.2%, and the unchanged
write peak falls 65.2%. Full reads retain approximately 198.4 MiB and 1.49 million
traced live blocks in both versions: no history was dropped to obtain the gain.

| 5,000 allocations | Old wall median ms | New wall median ms | Old CPU median ms | New CPU median ms |
| --- | ---: | ---: | ---: | ---: |
| Prefix hash | 516.1 | 495.8 | 516.0 | 495.7 |
| Full read | 1605.0 | 1461.4 | 1604.5 | 1461.2 |
| Unchanged write | 514.0 | 508.7 | 514.0 | 508.6 |

Timing differences vary in the smaller cases, including small regressions.
This change is justified by bounded serialization allocations; these warm-cache
synthetic medians do not establish a native training throughput improvement.

The separately counted traversal confirms N document reads, N-1 segment reads,
and one nonempty prefix hash of N-1 allocations for each full read. The second
hash call only initializes the empty prefix. An unchanged write still hashes
all N-1 protected allocations once and reuses that verified digest after commit.
Document counts instrument `read_document`; they exclude the write's raw
manifest reads, which retain their existing authority checks.

Reproduce the two runs recorded in the raw results:

```sh
python3 scripts/benchmark_mixed_care_ledger.py --sizes 10 100 1000 --repeats 5
python3 scripts/benchmark_mixed_care_ledger.py --sizes 5000 --repeats 5
```

## Remaining costs and preserved contracts

Full reconstruction and verification remain O(total history). Capped file reads
request `MAX_DOCUMENT_BYTES + 1`, producing an approximately 64 MiB traced
temporary on this Python build even for tiny documents; this masks the smaller
histories' end-to-end peak improvement. Snapshot verification can also retain
decoded segment graphs in its admission-local deduplication cache. Neither is
changed here. Pointer-array and wrapper-cache reductions have a smaller scope
than the populated prefix JSON buffers in receipt-rich histories. A different
bounded-read strategy or smaller verification cache needs separate measurement
and tests for complete, fail-closed validation.

Manifest and segment formats, every recovery dependency, checkpoint authority,
actor/value Adam state, seed reservation/exclusions, and recovery idempotence
are unchanged. No checkpoint or historical object is removed. Exact digest
compatibility and mutation rejection at the beginning, middle and end of the
archive are exercised alongside the existing migration, corruption, pending
resolution, exclusion, failed-publication and recovery tests. The local host
suite ran 118 tests successfully with one platform-specific skip; the mixed-care
subset passed all 67 tests.
