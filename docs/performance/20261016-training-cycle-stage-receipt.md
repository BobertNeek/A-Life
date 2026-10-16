# Opt-in serial training-cycle stage receipt

## Scope and use

This measurement-only candidate is based on main
`73f8ecf03a4bbd90018c0fae7c732c6ec488e1d9`. It does not optimize a phase,
change learned state, add a GPU query/readback, or establish a speedup.
The normal gameplay path does not call this recorder.

Set the existing `ALIFE_FOUNDATION_PROFILE` environment variable for an existing
single-cycle training command to emit `stage-timing.json` in its newly owned
output directory. Unset it to disable the recorder. Presence, including an empty
value or `0`, enables it, matching the existing runtime profile switch.

For a usable later measurement, set `ALIFE_PHASE31_SOURCE_HEAD` to the measured
source identity and `ALIFE_FOUNDATION_PROFILE_BINARY_SHA256` to the binary's
SHA-256. These are caller-declared labels, not verified hashes. Missing values
are JSON null and must not be treated as identified builds. The sidecar also
includes executable display path, package version, OS, architecture and debug
assertion configuration. A source commit alone does not identify local edits;
retain a patch/tree manifest alongside the later target-adapter receipt.

No intended-adapter execution, training, GPU correctness test, observer-overhead
measurement, or stage ranking was performed for this change.

## Accounting

The monotonic host wall clock begins before source validation/admission and ends
after the cycle returns, before sidecar serialization/write. Spans measure wall
time around existing calls, including their waits. They are not GPU kernel times.

Each span has an ID, parent ID (null for roots), one call, complete/interrupted
status, inclusive elapsed seconds, and exclusive seconds with direct child time
removed. `exclusive_covered_seconds` sums only exclusive span durations;
`unassigned_seconds` exposes native-total time outside the selected spans. Never
sum inclusive parent and child spans. An absent span was not reached/executed;
a present zero-duration span is an observed zero at clock resolution.

Selected spans cover:

- Source admission; world/lesson setup; backend/runtime setup; initial durable boundary
- Collection parent with first priming/tick/capture, trainer construction/restore,
  real initial actor checkpoint capture, and remaining collection children
- Old-value replay load/predict/restore verification and rollout target preparation
  in one combined span, including reward construction and PPO batch/GAE preparation
- Update parent with PPO, conditional acquisition rehearsal and archived replay children
- Export/write/redecode; next-cohort construction/real decision; trainer rebind
- Final real actor capture/serialization/write; final value capture/serialization/write;
  final cycle receipt write

Source/configuration metadata, scenario diagnostics outside the durable boundary,
output claim, several receipt preparations and object drops remain unassigned.
The old production-tick and replay-append counters retain their original scope
and are labeled nested collection counters; do not add them to collection time.
No new per-tick or per-row logging/timers are introduced. Profile-disabled calls
allocate no recorder, read no clock, evaluate no metadata closure and write no
sidecar; a few no-op span calls occur only at serial command boundaries.

After collection, context records requested/actual trainable decisions, captured
rows, terminal death tick, replay segment count and uncaptured world ticks
between replay rows. The last two are capture-gap evidence, not proof that every
gap was sleep. Rehearsal/archived epochs are requested settings; stage presence
and status identify whether those calls ran/completed. On failure before those
observations, fields are absent rather than invented as zero. This first slice
does not separate wall-clock timings for every sleep/terminal segment.

## Failure and identity boundaries

The output helper retains its original atomic `create_dir` ownership claim.
Profiling never creates or writes to a source/foreign output. Admission failures
and rejected existing output paths have no owned place for a sidecar and therefore
cannot persist one. After ownership, an ordinary returned error unwinds active
span guards as interrupted; the original error and partial spans are flushed once
at completion. Process kill/abort cannot flush. A sidecar-write failure is a
best-effort stderr diagnostic and never replaces training success or its error.

The sidecar is separate from canonical cycle, replay, actor/value checkpoint and
asset identities. Existing real captures, restore comparison, disk reopen,
next-cohort tick and optimizer rebind remain in place, with no extra GPU work.
Existing canonical receipt timing fields still naturally include observer cost
when enabled. No runtime bit-equality or performance claim follows from CPU tests.

## CPU qualification and remaining gates

Fresh production-recorder tests use a deterministic clock, plus the existing
output-ownership tests. They cover nested/exclusive accounting, unassigned time,
zero/absent distinction, partial failure/error preservation, disabled no-op,
JSON roundtrip, source/foreign directory protection, optional write failure and
synthetic canonical-file preservation. The synthetic file test is a recorder
isolation check, not a trained-state equality run.

The external candidate evidence records exact commands/results. Full candidate
app metadata-only checks use existing cached dependency artifacts and must be
reported separately from a clean Cargo rebuild. Disk headroom is insufficient
for a full dependency/test rebuild; full Cargo training-library/replay execution
and actual GPU tests are not established by metadata checks.

PR59's standalone current-main patch applies cleanly to this candidate. Its
trainer pipeline retention and replay acquisition changes touch separate files.

A later separately authorized intended-adapter receipt must pin source/tree,
binary, adapter, prior, seeds and workload; distinguish cold/warm and optional
work; reconcile native/host totals; and compare profile off/on to bound observer
overhead. Only that evidence can rank further speed work such as packed actor
checkpoint staging/readback.
