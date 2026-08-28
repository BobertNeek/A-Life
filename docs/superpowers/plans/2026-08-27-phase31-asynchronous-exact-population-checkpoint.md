# Phase 3.1 asynchronous exact-population checkpoint implementation plan

> Source design: `docs/superpowers/specs/2026-08-27-phase31-asynchronous-exact-population-checkpoint-design.md`
>
> Baseline: `94f93678652d0a4a58b0d5c55cf12c1273b9adad`

## Goal

Move exact six-founder GPU checkpoint capture and persistence off the Bevy update
path while preserving one GPU-authoritative runtime, exact reload, sleep
durability, bounded coalescing, and fail-closed authority.

## Task 1: atomic save-and-journal authority generations

**Files:**

- Modify: `crates/alife_runtime/src/checkpoint_assets/durable_manifest.rs`
- Modify focused tests in the same module or its existing checkpoint test target.

1. Add RED tests for explicit legacy direct loading, generation loading, malformed
   or unknown pointers, save-prepared failure, journal-prepared failure, pointer
   commit failure, reopen failure, and journal-only generation publication.
2. Run the smallest `alife_runtime` test filter and record the causal failures.
3. Add a field-explicit, versioned authority pointer naming immutable save and
   journal artifacts plus their digests and prior authority identity.
4. Make pointer replacement the only visibility commit for generation-backed
   state. Keep legacy direct state explicit and migrate only on a successful
   publication.
5. Make loaded manifests carry an opaque authority identity used for save CAS
   and journal CAS. Never fall back to legacy files when a pointer exists.
6. Re-run the focused tests until GREEN and inspect the diff.

## Task 2: nonblocking backend population capture

**Files:**

- Modify: `crates/alife_gpu_backend/src/closed_loop_checkpoint.rs`
- Modify the narrow backend module exports required by the public runtime boundary.
- Add or modify one focused physical-GPU test.

1. Add RED tests for complete ordered identity binding, stale or foreign handle
   rejection, tick-T byte immutability after later live mutation, bounded map
   operations, and staging release after completion/failure.
2. Add a versioned opaque capture ticket and immutable captured-population result.
3. Record one encoder/submission that copies every resident's exact topology,
   mutable neural state, replay, homeostasis, throttle, and Completed staging
   ranges to bounded staging buffers.
4. Expose only nonblocking `Pending`, `Ready`, and `Failed` polling. Do not call
   `PollType::Wait` or receive on a blocking channel from the caller path.
5. Validate captured row identities and lengths before returning immutable bytes.
6. Run the focused physical-GPU gate and record adapter/backend evidence.

## Task 3: pure exact-save assembly from immutable capture

**Files:**

- Modify: `crates/alife_runtime/src/checkpoint_assets/state_codec.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Add focused codec/runtime tests.

1. Add RED tests showing immutable captured GPU bytes plus a tick-T host snapshot
   produce the same exact v6 save as the existing synchronous boundary.
2. Split readback from save encoding. The worker-facing codec accepts validated
   captured rows and never reads the live backend.
3. Preserve all existing asset, topology, replay, sleep staging, cognitive
   sidecar, chemistry, embodiment, identity, and checkpoint-tick validation.
4. Reopen the candidate and prove exact equality and restore continuity.

## Task 4: single-flight coordinator and sole worker

**Files:**

- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Add a narrow private module if needed for the transaction state machine.
- Modify focused Phase 3.1 integration tests.

1. Add RED tests for the complete transaction state order, one active request,
   one follow-up flag, one bounded manual request, nonblocking update polling,
   worker failure, CAS failure, and post-pointer reopen fail-stop.
2. Capture the complete owned host snapshot at canonical tick `T` and submit the
   backend batch capture before later neural mutation.
3. Poll the capture and worker without blocking. The worker alone encodes,
   writes, prepares, commits, reopens, validates, compares, and derives durable
   evidence.
4. Install the backend durable permit only after bounded in-memory result checks.
5. Hold dependent Completed-to-Committed promotions and bounded journal entries
   until permit installation. Hand the immutable promotion-journal candidate to
   the same worker and retain its writer lease through journal-only generation
   commit and reopen validation. Fail-stop if a later same-tick failure could
   split GPU and host authority.
6. Route manual save through the same coordinator. Remove normal sleep-boundary
   access to the synchronous whole-population capture path.

## Task 5: focused production evidence

**Files:**

- Modify only the already-reviewed Phase 3.1 focused test target and metrics
  structures required to report the new transaction.

1. Prove two simultaneous exact boundaries yield one transaction and one
   follow-up, with bounded manual-save behavior.
2. Prove save-prepared, journal-prepared, pointer-commit, worker, mapping, CAS,
   and reopen failures preserve or fail-stop at the documented authority point.
3. Prove a captured six-founder save restores exact world, identities,
   embodiment, organs, chemistry, GPU state, memory, topology, sleep, and
   receipts.
4. Prove Completed sleep promotes only after durable permit installation.
5. Prove ordinary sleep play records no synchronous population snapshot or
   blocking map wait, and completion leaves no CPU neural mirror.
6. Run formatting only on intended files, `git diff --check`, focused CPU tests,
   the one focused physical-GPU gate, and the exact save/load integration gate.

## Task 6: checkpoint and one authorized profile

1. Inspect `git diff`, stage only reviewed files, commit, and push.
2. Verify clean status, local equals origin, and ancestry from accepted Phase 3
   `c922883a803bd05b449e92dd06347e76487ffbdd`.
3. Build the optimized production executable once.
4. Run one fresh source-bound six-founder, 1920x1080, RTX 3050/Vulkan, 60-second
   profile with no smoke override.
5. Report frame percentiles and hitches, TPS and tick accounting, GPU authority,
   transaction/coalescing counts, update-thread checkpoint time, worker latency,
   GPU submissions/maps/bytes, exact captures, promotion latency, and remaining
   CPU/GPU stages. Stop at the largest measured cost if any acceptance threshold
   remains RED.
