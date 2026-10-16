# Retained per-preparation episodic encoding — 2026-10-16

## Scope and lifecycle

`PreparedMemoryRecall` now retains the exact `PerceptionContextBlock` that each
constructor already builds to obtain its context digest. Both audited
`MemoryBank::recall_frame` and ordinary `recall_frame_runtime` establish it once.
The encoded block is moved into the finalized perception frame rather than
reconstructed during cognitive attachment and finalization.

The episodic context and retained block are private owned fields. Their getters
are immutable. `with_cognitive_context` replaces a separate cognitive field and
does not change episodic lanes. `PreparedMemoryRecall` has no serde path; no
persistent format, checkpoint, bank contents, or `RuntimeMemoryRecallCache`
changes. Each new recall constructs a new block. No retained encoded block is shared across ticks.

Audited `validate_for_draft` and `FinalizedMemoryRecall::validate_for_frame` retain
their existing full recomputation. Cognitive attachment preserves the ordering
of cognitive validation, episodic validation, and receipt validation. Finalization
retains source validation, the block/receipt digest comparison, perception-frame
validation, and exact candidate decision-key construction. Failure consumes only
temporary owned values and does not mutate the bank or world.

This preserves the current implementation of AOA-CTX-001, AOA-CTX-003,
AOA-MEM-001 through AOA-MEM-003, AOA-TIME-003, AOA-PERSIST-004, and
AOA-FAIL-001/002. It changes neither cognitive authority nor the architecture.

## Allocation evidence

Per prepared row, the measured call and requested-byte counts were:

| Mode | Candidates | Alloc/realloc calls, old → retained | Requested bytes, old → retained |
| --- | ---: | ---: | ---: |
| Audited | 1 | 116 → 104 | 20,561 → 19,881 |
| Audited | 3 | 216 → 200 | 59,515 → 56,851 |
| Audited | 32 | 1,461 → 1,439 | 437,296 → 414,792 |
| Runtime cold | 1 | 75 → 63 | 13,006 → 12,326 |
| Runtime cold | 3 | 149 → 133 | 32,528 → 29,864 |
| Runtime cold | 32 | 1,100 → 1,078 | 315,357 → 292,853 |
| Runtime cached | 1 | 69 → 57 | 12,238 → 11,558 |
| Runtime cached | 3 | 135 → 119 | 30,480 → 27,816 |
| Runtime cached | 32 | 967 → 945 | 292,957 → 270,453 |

At 32 candidates, all three measured flows avoided 22 allocation/reallocation
calls and 22,504 requested bytes per row. The reduction includes temporary typed
float storage and the canonical digest builder's existing allocation traffic;
the canonical digest algorithm is unchanged. Baseline and retained frame digests
and every candidate-key digest matched for all nine mode/size combinations.

Prepared-clone measurements show the cost explicitly: 2 → 3 allocation calls,
with 608 → 672 bytes at one candidate, 1,824 → 2,016 at three candidates, and
19,456 → 21,504 at 32 candidates. Thus the additional clone payload is exactly
64 bytes per candidate in these fixtures.

The component measures one recall, one cognitive-context attachment, and consuming
finalization. It separately measures cloning a prepared recall. Fixtures are an
empty bank and 1, 3, or 32 targetless candidates; audited, runtime-cold, and
runtime-cached paths are measured. Each recall-flow count is repeated sixteen times and must
be identical; prepared cloning is counted once per size and variant. Input/cache setup, input cloning, result validation, proof collection,
and result destruction are outside counting. Allocation/reallocation calls and
requested bytes are measured; deallocations, retained RAM, RSS, GPU cost, and
live-game frame time are not measured.

Both core libraries are compiled separately from their stated source paths using
Rust optimization level 2, debug assertions off, and 256 codegen units. They share
only the same third-party dependency rlibs from the isolated test build. This is
component allocation evidence, not a full release-profile performance result.

There is a deliberate lifetime tradeoff: the already-created 16-lane-per-candidate
Vec remains owned until finalization, up to 2 KiB at the existing 32-candidate
limit, plus block metadata. Cloning a prepared recall clones this additional Vec.
The ordinary consuming preparation flow avoids two temporary block encodings;
not every cloning workflow is cheaper. The frame ultimately owns the same payload
it previously reconstructed.

Reproduction uses the [probe](evidence/20261016-retained-recall-context-bench.rs)
and [runner](evidence/20261016-retained-recall-context-run.py). The runner accepts
baseline source root, candidate source root, dependency directory, and output
directory. It performs no network or GPU operations. Recorded commands, source
hashes, library hashes, and raw per-row counts are in the
[receipt](evidence/20261016-retained-recall-context.json),
[baseline rows](evidence/20261016-retained-recall-context-baseline.jsonl), and
[retained rows](evidence/20261016-retained-recall-context-retained.jsonl).
The baseline's 150 core/workspace file blobs were checked against main commit
`d460ac63552efad2abfb02518242d0247d224a91`; see the
[baseline verification](evidence/20261016-retained-recall-context-baseline.json).

## Verification

- Four new private tests compare the complete frame/finalized recall against
  frozen reconstruction helpers, exercise both constructors, repeated cognitive
  replacement, independent clone ownership, pointer-preserving finalization,
  changed drafts, malformed source/receipt errors, and a replaced retained block
- A new public populated-memory test covers audited, cold runtime, and cached
  runtime construction, frame/context/receipt equality, exact key bindings,
  serialized frame equality, and an unchanged bank
- Candidate-memory retrieval: 48 passed, 1 existing ignored benchmark
- Candidate-memory queries: 7 passed
- Canonical digest tests: 5 passed; perception digest goldens: 3 passed
- Core unit suite: 36 passed, 1 existing ignored; perception candidates: 10 passed;
  v1.1 cognitive contracts: 4 passed
- Core library/tests Clippy with warnings denied, formatting, static core
  boundaries, and documentation assertions: passed (77/77 docs assertions)

No frontend, neural executor, GPU, save schema, or renderer change is included.
