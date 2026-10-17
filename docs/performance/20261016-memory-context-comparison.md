# Allocation-free memory-context comparisons

Date: 2026-10-16. Baseline: main commit
`73f8ecf03a4bbd90018c0fae7c732c6ec488e1d9`.

## Scope

`GpuMemoryContextUpload::build_local` and
`GpuMemoryContextUpload::validate_for_frame_and_slot` formerly assembled a
16-float-per-candidate `Vec` solely to compare it with the immutable frame
context. They now compare the same ordered lanes directly with an iterator.
The owned upload records, cognitive records, public validation boundaries,
shader layouts, and GPU authority are unchanged.

All header, binding, record-index, payload-finiteness, and cognitive checks
remain in their original order. Context comparison still happens after every
record check. This matters when a context mismatch coexists with a nonfinite
payload in a later row: the original `NonFinitePayload` result takes precedence.
The comparison checks exact length and ordinary `f32` equality, including the
original equality of positive and negative zero. It is not a bitwise-float or
tolerance comparison.

## Allocation evidence

The isolated comparison probe injects the exact current production helper from
`closed_loop_memory.rs`, compiles it with `rustc -O`, and compares it against the
previous `Vec` implementation. It excludes fixture setup and counts allocation
and reallocation requests rather than timing. Each case performs 100 successful
comparisons.

- Empty input: both implementations allocate zero times
- 1, 2, 32, 256, and 1,024 candidates: the original comparison requests one
  allocation of `64 * candidate_count` bytes per comparison; the new helper
  requests zero allocations
- At 32 candidates, each removed comparison avoids 1 allocation and 2,048
  requested bytes

These are isolated host allocation results, not an end-to-end dispatch, GPU,
FPS, or elapsed-time improvement. The two production sites may be reached a
different number of times depending on the flow and debug self-diagnostics.

Raw results and reproducible sources:

- [Results](evidence/20261016-memory-context-comparison.jsonl)
- [Probe](evidence/20261016-memory-context-comparison-bench.rs)
- [Runner](evidence/20261016-memory-context-comparison-run.py)

Run from the repository root:

```sh
python docs/performance/evidence/20261016-memory-context-comparison-run.py
```

## Verification

CPU-only checks cover:

- Every comparison lane, empty input, truncated and extended context, multiple
  rows, source-count packing, signed zero, and legacy nonfinite comparison
  semantics
- Context mismatch combined with later nonfinite or misindexed memory and
  cognitive rows; header errors retain their earlier priority
- All existing memory upload, ABI, binding, projection-width, finite-payload,
  and tamper-rejection tests
- The existing complete batch word/binding golden digests for N512, N1024, and
  N2048, with and without memory
- Backend assertions enabled and disabled, so public rejection is checked
  without relying on debug upload self-comparisons

Focused commands:

```sh
cargo test -p alife_gpu_backend --lib memory_context_comparison
cargo test -p alife_gpu_backend --lib batch_projection
cargo test -p alife_gpu_backend --test closed_loop_memory_context
cargo --config profile.test.package.alife_gpu_backend.debug-assertions=false test -p alife_gpu_backend --lib
cargo --config profile.test.package.alife_gpu_backend.debug-assertions=false test -p alife_gpu_backend --test closed_loop_memory_context
```

No GPU dispatch, training run, frontend build, or full-workspace validation is
claimed by this report.
