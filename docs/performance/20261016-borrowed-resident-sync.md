# Borrowed resident synchronization — 2026-10-16

Baseline: `69f00c05ec6774c3655ae87c5d4baa5df3861929`.
This is a CPU ownership/copy change in `gpu_live_runtime.rs`, independent of
the receptor-expression cache and batch-upload validation changes.

The two synchronization calls per ordinary resident tick no longer construct
owned admission snapshots. The end-of-tick population pass also borrows the
world record and iterates the resident map directly, removing its record clone
and temporary ID vector. The pre-sleep mutable record and whole staged-authority
transaction clones remain unchanged.

## Preserved boundary

The direct helper keeps the exact original order: full record validation,
alive/current-tick check, age derivation, resident-genome equality, homeostasis
assignment, then development derivation and assignment. Population iteration
keeps ascending `BTreeMap` order, missing-record rejection, dead-record skipping,
and any earlier resident updates before a later failure. The world is borrowed
immutably and the independent resident map mutably.

Immutable genetics validation and every dynamic record guard remain in
`WorldOrganismRecord::validate_contract`. A cold borrowed record may warm that
record's existing process-local proof; the proof is neither serialized nor
canonical state. Current development is still derived afresh, including its
small output vectors. This preserves the touched AOA-AUTH-001/002/005 and
AOA-TIME-003 boundaries without introducing another authority or cache.

## CPU allocation evidence

The [source-bound receipt](evidence/20261016-borrowed-resident-sync.json) records
the exact helper bodies, linked core/world artifacts, complete samples, source
hashes, commands, and verification results. The
[probe builder](evidence/20261016-resident-sync-probe-build.py) checks the frozen
old helper bodies against the baseline before extracting the changed helpers.

The probe runs both synchronization passes on independently owned N2048
organism records. It covers builtin genetic records and records carrying an
explicit inherited 32,768-weight N2048 asset. Both modes use warm immutable
validation proofs. Caller setup and initial resident clones are outside
counting; validation, admission/record clones, ID collection, and development
construction are inside. Resulting genome/homeostasis/development fields match
and the canonical world signature is unchanged.

| Residents | Explicit inherited asset | Old → new allocation/reallocation calls | Old → new requested bytes |
| ---: | :---: | ---: | ---: |
| 8 | No | 1,201 → 464 | 413,044 → 276,180 |
| 50 | No | 7,501 → 2,900 | 2,581,550 → 1,726,150 |
| 8 | Yes | 1,225 → 464 | 3,558,772 → 276,180 |
| 50 | Yes | 7,651 → 2,900 | 22,242,350 → 1,726,150 |

Counts include allocation and reallocation requests, not deallocations. Bytes
are requested allocation traffic, not retained RAM or RSS. The probe's minimal
resident contains exactly the three fields used by these helpers; actual
complete app-resident behavior is checked separately by the regression tests.

The optimized Linux probe was pinned to CPU 0, after local builds/tests ended.
Each fixture used two warmup pairs and nine alternating old/new samples of 20
repetitions. Median times for the two synchronization passes are:

| Residents | Explicit inherited asset | Old ms | New ms |
| ---: | :---: | ---: | ---: |
| 8 | No | 0.2305 | 0.1996 |
| 50 | No | 1.4420 | 1.2606 |
| 8 | Yes | 0.3052 | 0.1977 |
| 50 | Yes | 1.9794 | 1.2779 |

All 36 sample pairs compared their final resident maps exactly, covering 1,044
final resident records. Intermediate invocations are not separately observed.
Core/world artifacts and the probe use opt-level 3 with debug assertions off.
Timing excludes fixture setup and initial owner clones. The allocation counter
is disabled during timing, although its allocator-wrapper branch remains.
Other host tenants are uncontrolled. These component timings cannot be added
to the receptor or batch-upload measurements or claimed as whole-frame gains.

## Verification and limits

The actual app regression suite uses frozen original helper bodies and compares
complete serialized resident state and errors. Coverage includes advancing
biology, stale ticks, genome mismatches, dead direct rejection/population skip,
missing records, ordered partial progress, independent cold restored records,
and immutable/dynamic malformed-record validation before later errors.
Seven focused sync tests, 19 existing CPU preparation tests, and two newborn
admission tests passed. Three explicit timing tests were intentionally ignored.
Strict affected Clippy, formatting, core boundaries, and all 77 documentation
assertions passed. App test code uses the one-job, symbol-free opt-level-0
profile with debug assertions enabled; optimized timing is separate.

This is not end-to-end gameplay, GPU, training, Windows, rendering, fresh-process
GPU save/load, or FPS/TPS evidence. Broader performance and AOA-PERSIST-004
acceptance remain **Unknown**. No GPU or policy-collection run was performed.
