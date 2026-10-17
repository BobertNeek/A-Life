# Read-only serial-cycle stage timing summary

## Status and prerequisite

This host-only diagnostic is based on main
`73f8ecf03a4bbd90018c0fae7c732c6ec488e1d9`. It consumes schema-1
`stage-timing.json` sidecars from the separately proposed
[serial-cycle recorder, PR75](https://github.com/BobertNeek/A-Life/pull/75),
reviewed at head `83f903a8ae03bb501849fe768afd3154e45e3ab6`.
That recorder remains a draft prerequisite for producing actual receipts; this
Python tool and its synthetic tests do not depend on merging or building it.

The earlier standalone reviewed summarizer files were unavailable during this
integration. This is an explicitly rebuilt implementation, independently
requalified against the recorder schema, rather than a claimed byte-for-byte
integration of that earlier artifact.

No actual training receipts, GPU/adapter execution, native Rust build,
observer-overhead measurement or stage performance ranking were qualified.
Measured performance and any optimization conclusion remain **Unknown**.

## Commands

Python 3.10 or newer is sufficient; only the standard library is used.
Run from the repository root:

```text
python scripts/summarize_stage_timing.py path/to/stage-timing.json
python scripts/summarize_stage_timing.py path/to/run-directory --format json
python scripts/summarize_stage_timing.py path/to/first.json path/to/second.json --format json
python -m unittest discover -s scripts -p 'test_summarize_stage_timing.py' -v
```

Directories are recursively searched only for `stage-timing.json`; explicit
file arguments may use other names. Overlapping paths are read once. The tool
reads receipts and emits text or JSON to standard output. It writes no input,
checkpoint, receipt, lock, configuration or report file. Redirect stdout to a
separate derived report if needed; never redirect over original evidence.
Missing inputs, no discovered receipts or any malformed/unsupported receipt
exit with code 2 and no partial summary. Invalid file paths are named on stderr.

## Interpretation

- Stages are summarized by exclusive seconds, with direct child time removed.
  All same-name spans are summed **within each receipt first**. Median and p95
  then describe those per-receipt observations, rather than weighting runs by
  call count. p95 is nearest rank: sorted value at `ceil(0.95*n)-1`.
- Inclusive parent and child durations are validated, never added to rank
  stages. The nested legacy collection counters remain context diagnostics and
  are never inserted into stage totals. Native total, exclusive covered and
  unassigned time are reported separately.
- Schema-1 complete and error receipts form separate cohorts. Error cohorts
  contain partial work; interrupted span counts are explicit. Even completed
  spans in an error receipt do not make it a completed-cycle sample.
- Missing stages are excluded from that stage's timing distribution and counted
  as missing. A present measured zero is counted as observed zero. No observed
  values produces JSON null median/p95, never an invented zero. Known optional
  stages are listed even when absent from every input; unknown future names are
  retained. Missing post-collection observations and intentional null values
  remain distinct in the original contexts.
- JSON retains each unique receipt's original context, status, error, source
  paths and canonical-content SHA-256. Differing context fields warn that pooled
  medians/p95 are descriptive only. Seed, source/binary declaration, workload,
  optional work and observed actual decisions are included in that comparison;
  only the explicitly nested legacy timing counters are excluded. Select a
  comparable input set before interpreting stage distributions.
- Canonical full-JSON content hashes deduplicate copied, reformatted,
  key-reordered and equivalent `1`/`1.0` receipts, retaining all discovered source
  aliases. Distinct span observations or metadata remain distinct. This is a
  **heuristic**: schema 1 has no unique run ID, so independent runs with identical
  whole-receipt content collapse too. Retain original run provenance and inspect
  the aliases before treating a count as independent measurements.

Input validation rejects duplicate JSON keys, nonfinite/exponent-overflow values,
unsupported schema/time basis, invalid statuses/types, duplicate span IDs,
dangling/cyclic parents and inconsistent duration accounting. Parent lookup uses
IDs because the recorder serializes children before parents. Duration equations
use `math.isclose` with relative tolerance `1e-7` and absolute tolerance `1e-9`
seconds to allow separately converted floating-point durations.

The clock measures host wall time around existing calls, including waits, not
GPU kernel time. Source and binary fields are caller-declared labels, not
verified hashes. Equal contexts do not identify adapter, prior, cold/warm state,
dirty source/tree, wrapper overhead or observer cost; the recorder does not
establish those. Capture gaps are not identified sleep segments. A later
source/binary/adapter/workload-bound profile-off/on receipt is still needed
before any causal, whole-cycle speedup or intended-adapter performance claim.

## Regression boundary

The synthetic stdlib suite covers post-order nesting and accounting, repeated
stages, missing/zero/null distinctions, percentiles, partial failures, mixed
contexts, copied/reformatted receipts, malformed input, JSON/text CLI behavior
and read-only input preservation. CI includes the suite in its existing receipt
diagnostics job. These tests qualify the analyzer's host behavior only.
