# Phase 3.1 asynchronous exact-population checkpoint design

## Status and authority

Status: conditionally approved on 2026-08-27. The corrections below are part of
that approval and authorize the bounded implementation tranche.

This is a non-normative implementation ADR for the measured Phase 3.1 persistence bottleneck. The controlling A-Life v2.0 architecture remains authoritative. The design preserves AOA-AUTH-002, AOA-INV-010/011, AOA-SLEEP-003/007, AOA-STR-007, AOA-PERSIST-001/002/004/006, and AOA-FAIL-001/002.

Source baseline: `94f93678652d0a4a58b0d5c55cf12c1273b9adad` on `phase3-performance-stabilization-20260826`.

## Decision

Add one versioned, single-flight `ExactPopulationCheckpointTransactionV1`. It captures one exact population state at canonical tick `T`, moves GPU mapping and CPU persistence work off the Bevy update path, and installs no durable sleep authority until the exact manifest has committed and reloaded successfully.

The live brain stays GPU-resident. The transaction may own temporary immutable staging buffers and encoded checkpoint bytes. It destroys them at `Complete` or `Failed`. No continuously synchronized CPU neural state is added.

## Transaction state machine

The only valid state sequence is:

```text
Idle
  -> CaptureSubmitted
  -> MappingPending
  -> CpuBytesReady
  -> Encoding
  -> ManifestPrepared
  -> CasCommitted
  -> ReloadValidated
  -> DurablePermitInstalled
  -> Complete
  -> Idle
```

Any fallible state may move to `Failed`. `Failed` records the transaction ID, tick, expected base digest, failed stage, and stable error code. It cannot return to an earlier state or retry against newer live state. The runtime either restores the prior authoritative state where no GPU mutation committed, or fail-stops where continuation could split authority.

Every state transition validates the transaction schema version, monotonically increasing transaction ID, tick `T`, expected manifest digest, organism set digest, capture identity digest, and predecessor state.

## Ownership and one-writer rule

`GpuLiveBrainRuntime` owns one checkpoint coordinator. The coordinator owns exactly one active transaction, one `checkpoint_needed_after_current` flag, one optional manual-save request, and one bounded map of deferred sleep-journal candidates keyed by organism ID.

The GPU backend owns the capture ticket, transaction staging buffers, map callbacks, and captured identity metadata until `CpuBytesReady`. It exposes no mutable staging reference.

One checkpoint worker owns the immutable host snapshot and immutable captured GPU bytes during `Encoding` and `ManifestPrepared`. It cannot access the live world, resident maps, GPU backend, handles, Bevy resources, or newer journal state.

`GpuLiveCheckpointDurability` grants the active transaction an exclusive writer lease. While held, no manual publication, second exact checkpoint, journal rebase, or other manifest writer may run. `SAVE_CAS_GUARD` remains the filesystem CAS guard, not a substitute for the runtime writer lease.

## Tick-T host capture

After the canonical tick has completed GPU work, world/body/biochemistry sealing, authority advance, resident synchronization, memory/topology observation, and population reconciliation, the coordinator clones one immutable host snapshot for tick `T`.

The snapshot contains the complete portable world and ecology, organism records and stable identities, embodiment pose and velocity, body and organs, biochemical graph and homeostasis, genome and development, lifecycle, sleep state, memory and topology sidecars, predictor and cognitive context, language and life statistics, replay patch authority, asset manifest, save configuration, and the expected durable-manifest digest.

The snapshot contains owned values only. It has no references to live maps or GPU buffers. Its bounded size is checked against the admitted population and production brain-class limits before GPU submission.

## GPU batch capture and command ordering

The backend validates the complete ordered handle set before recording a command. Each row binds:

- organism ID, brain class, phenotype hash, backend ownership token, slot, and slot generation;
- graph epoch and exact live topology identity;
- logical dispatch and transaction generations;
- active activation side;
- active weight bank and generation;
- active and inactive eligibility generations and bank;
- replay generation, cursor, count, and digest;
- ATP, throttle, and activity sequence identity;
- pending eligibility or retained-learning identity;
- Completed sleep request, staged output, cycle, and replay identity when present.

One encoder copies every resident's required immutable topology ranges, mutable neural ranges, and required Completed staging ranges into a transaction-owned bounded staging arena. The backend submits that encoder before accepting any later neural mutation command. Queue ordering therefore fixes the copied bytes at tick `T`, even if unrelated live work advances after submission.

The backend registers bounded map callbacks and returns an opaque `GpuExactPopulationCaptureTicketV1`. The ticket contains the transaction identity and expected row identities, but no caller-supplied authority fields. Capture uses one queue submission and the smallest practical bounded buffer count. It must not issue one blocking mapping per field or organism.

Bevy updates call a nonblocking poll method. It may request nonblocking device progress, inspect callback completion, and return `Pending`, `Ready`, or `Failed`. It never calls `PollType::Wait`, waits on a channel, joins a worker, or performs filesystem I/O.

When every map completes, the backend validates mapped lengths and all captured identity records against the ticket. It then returns one immutable ordered population capture and releases GPU staging ownership. Later live advancement cannot mutate these bytes.

## Worker boundary

The single worker receives only:

- the immutable tick-`T` host snapshot;
- the immutable ordered GPU capture;
- the expected manifest digest and writer lease identity;
- cloned content-store and destination path configuration.

The worker reconstructs the existing exact v6 brain save records without recompiling, regenerating, truncating, or relabelling neural state. It encodes and hashes all assets, writes content-addressed files, merges the complete asset manifest, constructs the replacement `PortableSaveFile`, runs full save and asset-root validation, and prepares exact manifest and journal-reset bytes.

The same worker owns the complete post-commit reopen. After the authority commit,
it resolves the committed generation, reopens the save and reset journal, validates
the complete asset graph, compares the reopened save exactly with the prepared
replacement, and derives immutable validated durable-reference evidence. Its
success result contains only typed identities, digests, counters, and the durable
reference. Its failure result names the failed stage and last known-good authority
generation. The worker never delegates filesystem reads, deserialization, or
asset validation to the update thread.

Content-addressed assets written before CAS are non-authoritative. A failed transaction may leave unreachable immutable assets, but it never makes them current. The worker reports timing and byte counters for GPU-copy completion, encoding, asset writes, validation, and manifest preparation.

## Authority generation, CAS commit, and reload

The current two-step save rename followed by journal rename is not a process-crash
atomic pair commit. This tranche replaces that visibility contract with a
versioned `GpuCheckpointAuthorityPointerV1`. The pointer is the sole authority
commit point. It names one immutable authority generation containing:

- one exact save artifact and its typed content digest;
- one sleep-journal artifact and its typed content digest;
- the authority generation number and canonical authority digest;
- the prior authority generation and digest;
- a stable content identity for the versioned validated asset manifest, plus the
  save and journal schema versions needed to reopen the pair.

The asset-manifest identity is derived from the versioned manifest record and
its content references, never from an absolute filesystem path. Reopen validates
the manifest against the supplied asset root. Moving a complete valid save and
asset tree therefore preserves authority identity.

Exact-checkpoint publication prepares a new immutable save artifact and an empty
journal artifact. Journal-only publication prepares a new immutable journal
artifact and reuses the currently named immutable save artifact. In both cases,
all prepared artifacts are flushed, reopened, and validated before the pointer is
changed. One atomic pointer replacement under `SAVE_CAS_GUARD` makes the pair
visible. Unreachable prepared artifacts are not authority.

The prepared replacement remains bound to tick `T`, the expected authority
generation and digest, the exact base-save digest, and the capture identity
digest. CAS compares the on-disk pointer identity, not only an in-memory save
digest. Any failure while preparing the save, preparing the journal, or before
the pointer replacement leaves the prior pointer authoritative and reopenable.
There is no attempt to roll back two independently visible file renames.

After the atomic pointer replacement, the worker performs the full reopen and
validation described above. A post-commit reopen failure cannot make the prior
generation current again. It returns a fail-stop result that names both the newly
committed authority generation and the last known-good generation. Runtime
continuation and dependent sleep promotion remain blocked until explicit
recovery. The prior immutable generation remains available for diagnosis or an
explicit recovery operation, but is never silently substituted.

Compatibility is explicit. When no authority pointer exists, the loader may
open only the already-supported direct save plus journal layout as
`LegacyDirectV1`. It validates that layout using its existing schemas and exposes
an authority token identifying the legacy source. The first successful new
publication prepares a complete generation and atomically installs a V1 pointer.
An unknown pointer schema, malformed pointer, missing named artifact, or digest
mismatch fails closed. Pointer-bearing state never falls back to the direct
legacy files.

The Bevy/runtime poll receives the worker's immutable success or failure result.
It performs only bounded in-memory checks of transaction ID, tick, expected base,
capture identity, authority generation, and durable-reference identity. It then
asks the backend for an opaque permit bound to that exact reference. Only
`install_prevalidated_durable_checkpoint(permit)` may advance backend durable
authority. The update path performs no filesystem I/O, save deserialization,
asset-graph traversal, worker join, or blocking wait.

No code path may catch a checkpoint error and fall back to the synchronous population capture.

## Journal interaction

The committed journal stays anchored to the prior exact base until the new exact CAS succeeds. No journal is rebased against an uncommitted candidate.

While a transaction is active, the runtime may hold at most one canonical deferred journal chain per admitted organism. The bound is the admitted population limit multiplied by the journal schema's maximum compound entries for one organism transition. Entries retain source, target, tick, ordinal, cycle, replay, and compact neural authority. If another transition would exceed that bound, the organism remains at the last journaled sleep state until the transaction completes.

After `DurablePermitInstalled`, the runtime revalidates each deferred chain against the newly loaded exact base and publishes one bounded journal update. Unknown, skipped, duplicate, stale, or mismatched entries fail closed. A later exact boundary sets `checkpoint_needed_after_current = true`; it does not allocate another transaction record.

The update path may construct and revalidate the bounded in-memory journal
candidate, but it hands that immutable candidate back to the same worker. The
worker alone serializes, hashes, writes, flushes, commits the journal-only
authority generation, reopens it, and validates it under the existing writer
lease. The transaction does not enter `Complete` or release that lease until the
deferred journal generation is worker-validated. A failure after the exact base
commit but before journal-generation validation fail-stops; it never reopens a
second update-thread writer path.

## Sleep-promotion interaction

`Submitted -> Completed` may create the transaction's exact candidate state. No corresponding `Completed -> Committed` promotion becomes authoritative while the transaction is before `DurablePermitInstalled`.

After permit installation, all corresponding promotions are revalidated against the exact published Completed state and applied through the existing batched promotion path. Promotion journal publication occurs once for the validated same-tick set. Memory compaction and replay cleanup retain their existing preflight and post-publication atomicity rules.

If a new exact boundary arises while one transaction is active, its organism stays before the durability-dependent transition and sets the single follow-up flag. The next transaction starts from then-current canonical state only after the first transaction and deferred journal publication finish.

## Manual-save handling

A manual save requested in `Idle` starts the same exact-population transaction with a manual destination intent.

A request during an active transaction may coalesce only when it names the same destination and exact tick. Otherwise the coordinator stores one bounded manual request for a fresh current-tick transaction after completion. An identical later request coalesces with that slot. A different second request returns an explicit busy error. No request is dropped or silently redirected to the older tick.

The UI remains responsive and reports queued, writing, committed, or failed status from read-only transaction telemetry.

## Failure handling

- Pre-submit identity or capacity failure records no GPU command and leaves all authority unchanged.
- Fence, map, length, or captured-identity failure enters `Failed`, discards staging, and leaves the prior manifest and journal authoritative.
- Encoding, hashing, validation, content-store, or worker failure publishes no manifest and permits no sleep promotion.
- Authority CAS conflict publishes nothing and does not retry against a newer live snapshot.
- Save-preparation or journal-preparation failure leaves the prior authority pointer and generation unchanged.
- Authority-pointer replacement failure leaves the prior authority generation current; prepared artifacts remain unreachable.
- Post-pointer reopen or validation failure fail-stops, reports the committed and last known-good generations, and permits no dependent promotion. It does not claim rollback of the committed pointer.
- A failure after a GPU sleep operation that cannot be rolled back fail-stops the backend before staged host rollback.
- Shutdown stops new transactions, lets a submitted GPU mapping reach a safe release point, joins the sole worker outside the update path, and never commits an unvalidated candidate.

There is no silent retry, stale relabelling, CPU neural fallback, regenerated state, or default substitution.

## Bounded coalescing

The coordinator has four fixed bounds:

1. one active exact transaction;
2. one Boolean follow-up exact-checkpoint request;
3. one optional manual-save request;
4. one schema-bounded deferred journal chain per admitted organism.

Same-tick exact requests merge into the active population capture. Later requests set the Boolean. The system applies backpressure by holding durability-dependent sleep transitions, not by growing a queue.

## Focused evidence required before profiling

1. A GPU batch-capture test binds every organism, handle, slot generation, graph epoch, and neural generation.
2. A post-submit live mutation does not change captured tick-`T` bytes.
3. Completion and failure release all staging bytes and leave no CPU neural mirror.
4. Repeated Bevy polling remains nonblocking and performs no `PollType::Wait`.
5. Two concurrent exact boundaries create one active transaction and one follow-up flag.
6. Manual save coalesces by exact identity or occupies the single bounded request slot.
7. Failure injection after save preparation, after journal preparation, and before authority-pointer replacement leaves the prior authority generation current and blocks promotion.
8. Failure injection during worker encoding or publication preserves the prior authority; post-pointer reopen failure fail-stops and reports the last known-good generation without exposing a mixed save/journal pair.
9. The captured six-founder save restores exact world, identities, embodiment, organs, chemistry, GPU state, memory, topology, sleep, and receipts.
10. Completed sleep promotes only after the exact durable permit is installed.
11. Stale, foreign, unsupported, or tampered capture evidence fails closed.
12. Ordinary sleep-boundary play cannot call the old synchronous population-capture path.

The focused metrics must count transactions, coalescing depth, update-thread checkpoint wall time, worker stage latency, GPU copy submissions, map operations, bytes copied, exact organism captures, promotion latency, failures, and released staging bytes.

## Rejected approaches

- Relabelling stale organism checkpoints with tick `T`.
- A continuously synchronized CPU copy of neural tensors.
- Per-organism or delta-log persistence redesign.
- One blocking map per organism or field.
- Concurrent manifest or journal writers.
- Promotion before durable permit installation.
- Fallback to the old synchronous whole-population capture.
- Training, weight transformation, regeneration, truncation, or reduced workload.

## Review gate

The supervisor conditionally approved this ADR. The two required corrections are
now explicit: post-CAS reopen belongs wholly to the worker, and an atomic
versioned authority pointer replaces the unsupported two-rename crash claim.
Implementation proceeds RED-first through authority generation, backend batch
capture, runtime coordination, worker publication, and exact continuity gates.
Exactly one new optimized six-founder profile is allowed after the focused gates
pass and a clean implementation commit is pushed.
