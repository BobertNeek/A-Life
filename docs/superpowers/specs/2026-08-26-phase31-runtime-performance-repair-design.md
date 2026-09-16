# A-Life Phase 3.1 Runtime Performance Repair Design

## Status and scope

This design implements the measured Phase 3.1 repair tranche on branch `phase3-performance-stabilization-20260826`, descended from accepted Phase 3 commit `c922883a803bd05b449e92dd06347e76487ffbdd`.

The production workload remains the canonical six-founder Nano512 New Game at 1920x1080 with the GPU-required Vulkan path. The repair does not change neural weights, training, brain capacity, simulation cadence, sleep semantics, persistence authority, or subsystem participation.

## Measured cause

The accepted six-founder release baseline at source `99b4c2bbf763b9000fcf7da1e96534c2e7729adf` achieved 1.50 simulation ticks per second against a configured 20 TPS. The render median was 8.27 ms, so frame p95 did not explain the simulation failure.

The localization receipt at source `7031cf27ee23613e7a8da8bb85fdb86a1a31a3f3` assigned the largest CPU costs to synchronous sleep durability and preparation. The durable audit at source `79d97171ca8684bac704335845150a68b7f22584` measured:

- 22 sleep boundaries and 22 whole-population captures;
- 354 blocking mutable-slot readbacks moving 78,752,256 bytes;
- 6.838 seconds in exact checkpoint capture;
- 22 checkpoint manifest publications costing 3.520 seconds;
- 18 per-organism promotion publications costing 2.822 seconds;
- 220 architecturally invalid ordinary-seal snapshots moving 48,942,080 bytes;
- 1.771 seconds serializing and hashing full resident and topology JSON;
- only 0.249 seconds in whole-world authority advance and 0.119 seconds in resident synchronization, although repeated registry/world cloning remains unnecessary production work.

The tracked evidence is under `docs/performance/evidence/`.

## Global invariants

1. GPU neural tensors remain resident on the production GPU. There is no continuously synchronized CPU neural mirror.
2. Exact full neural state is captured only for save, sleep durability, migration, device recovery, or explicit debug operations.
3. A sleep transition requiring durability does not become authoritative `Committed` until its exact transaction publishes successfully.
4. An in-flight durability transaction blocks only sleep progression that could invalidate its staged state. It does not block renderer presentation or input.
5. Every successful durable transaction represents one exact causal tick and the complete authoritative population.
6. Failure retains the previously published durable save and leaves the staged transition uncommitted. No stale checkpoint is relabelled as current.
7. Same-tick sleep promotions publish once for the whole validated batch.
8. Canonical v2 validation and the explicit Nano512 compatibility ABI remain unchanged.
9. Save/load restoration reproduces exact world, organism, physiology, memory, topology, sleep, receipt, and GPU checkpoint state.

## 1. Compact ordinary-seal GPU authority receipt

Add a versioned `GpuAuthorityReceiptV1` at the GPU admission/transaction boundary. It contains no tensor data. It binds:

- schema and version;
- organism ID, class, slot, and slot generation;
- phenotype hash and hardware receipt generation;
- originating and outcome ticks;
- dispatch and transaction generations;
- active activation side;
- output fast-weight, eligibility, and replay generations;
- sealed sequence and patch digest;
- the existing compact learning-commit evidence;
- a deterministic receipt digest.

The existing 16-word GPU fast-plasticity commit record already returns the required post-commit generations. `GpuLearningReceipt` will carry the transaction generation instead of discarding it. The host constructs and validates `GpuAuthorityReceiptV1` only from the sealed patch, validated handle, and the compact GPU record returned by that transaction.

`commit_sealed_batch` will use the receipt digest as the neural authority input to `seal_cognitive_subsystems`. It will not call `snapshot_brain` and will not serialize full `ResidentCognition` or `TopologySidecar` JSON. Compact memory and topology update receipts or their existing canonical checkpoint digests remain separately bound. Missing, stale, mismatched, or unsupported receipt versions fail closed.

Exact `snapshot_brain` remains available at explicit durable and diagnostic boundaries.

## 2. Asynchronous exact sleep checkpoint transaction

Introduce a production runtime state machine with these states:

- `Idle` — no sleep durability operation is pending.
- `GpuCapturePending` — one exact population snapshot batch has been submitted and is associated with a causal tick, sleep transition candidates, handles, generations, and a GPU fence/readback ticket.
- `PublishPending` — all GPU readbacks completed and validated; an immutable replacement save and asset writes are executing off the Bevy update path.
- `ReadyToCommit` — publication succeeded and returned a validated durable reference.
- `Failed` — capture, validation, asset write, or publication failed; the previous durable reference remains authoritative.

The backend exposes a batch begin/poll interface. Begin records every handle and expected generation, encodes all copies into one submission where the current layout permits, starts nonblocking map operations, and returns a ticket. Poll uses nonblocking device progress and returns `Pending`, an exact ordered snapshot batch, or a fail-closed error. It never uses `PollType::Wait` on the render-critical Bevy update path.

After capture completes, immutable snapshot data, the staged world/save authority, and the checkpoint store are passed to one background publication job. The job writes content-addressed assets, constructs the complete replacement `PortableSaveFile`, validates it, and uses the existing atomic durable manifest publication. Bevy polls the job result without waiting.

Organisms whose sleep state participates in the transaction remain at the last authoritative pre-transition state, with a runtime-owned pending transition marker. Their incompatible sleep progression is suppressed until `ReadyToCommit`. Other presentation and input work continues. On success, the runtime installs the staged sleep states and durable reference together at a tick boundary. On failure, it clears the candidate, retains prior durable authority, and reports a fail-closed runtime error without fabricating progress.

Only one sleep durability transaction may be active. Same-tick requests join its ordered population transaction. A later incompatible request waits for completion.

## 3. Batched Completed-to-Committed promotion

Replace the per-organism `promote_durable_completed_sleep` loop with one `promote_durable_completed_sleep_batch` operation.

The batch:

1. sorts and deduplicates organism IDs;
2. validates every current saved checkpoint and proposed promoted sleep state before mutation;
3. clones the durable save once;
4. applies every validated promotion to the candidate;
5. validates the complete candidate save;
6. publishes the candidate once atomically;
7. installs the durable reference only after publication succeeds.

Any invalid member rejects the whole batch. The previous manifest and all in-memory committed states remain unchanged.

## 4. Failure-atomic single-record world replacement

Add `HeadlessWorld::replace_organism_record` and a registry-level exact replacement primitive.

The operation validates the current registry and world binding before mutation. The replacement must preserve the target organism ID and world entity binding and must satisfy the complete record contract. The registry stores the original record, replaces only the target entry, validates both indices and world bindings, and restores the original record on any error.

The operation does not clone the full registry or `HeadlessWorld`. It updates `next_organism_id` only after successful validation. Existing full-registry replacement remains for restore/migration boundaries.

Production runtime call sites that update one canonical record, including cognitive work accounting and cognitive subsystem sealing, use the single-record API.

## Failure handling

- A malformed compact authority receipt rejects the cognitive seal before state-graph publication.
- A GPU capture failure marks the pending durability transaction failed and preserves the previous manifest.
- A background publication failure returns no durable reference and does not install staged sleep transitions.
- Device loss during capture uses the existing fail-stop path and the last valid durable checkpoint.
- Cancellation or shutdown joins or safely abandons the publication worker without publishing a partial manifest.
- Save and explicit manual checkpoint requests either wait on or coalesce with the compatible exact transaction. They never read a half-staged sleep state.

## Focused causal tests

1. A successful ordinary seal emits and validates `GpuAuthorityReceiptV1`, performs zero full mutable-slot snapshots, and preserves strict state-graph tick validation.
2. Tampered schema, handle, phenotype, tick, sequence, or generation fields fail closed.
3. Same-tick promotions for multiple organisms produce one publish and install every committed state only after success.
4. One invalid promotion rejects the whole batch with unchanged durable and in-memory state.
5. A pending GPU capture returns control to the Bevy update loop without a blocking wait and prevents incompatible sleep progression.
6. Capture or publication failure retains the prior durable reference and pre-transition sleep authority.
7. A completed transaction restores exactly through the existing production load path, including GPU state, replay, memory, topology, physiology, lineage, and sleep.
8. Valid single-record replacement changes only the target record. Invalid replacement leaves the entire world digest unchanged.
9. The canonical six-founder persistence/save-load gate remains green before any 60-second performance rerun.

## Verification order

Implementation is RED-first and split into four reviewed checkpoints: single-record replacement, compact GPU authority receipt, coalesced promotion, then asynchronous durability transaction. After all focused CPU and physical-GPU gates pass, build one optimized executable and rerun the canonical six-founder 60-second profile. A short real-input and exact save/load smoke follows only after the performance target is evaluated.

No training, weight transformation, feature expansion, population scaling, or ecology work belongs to this tranche.
