# TerrainVision preparation with frozen hints — 2026-10-01

Source `5a8000a8dd093c550145cb6a04e293d40253ed71` extends `e540e820` to enable the existing production CPU runner for GroundedTerrainVisionV1, including N2048 with its semantic prior enabled. It implements the explicitly authorized hint snapshot boundary. It does not change the prior model, disable hints, move neural action/learning authority, or alter tick/population policy. Live performance at 20 FPS and 50 real creatures remains **Unknown**. The [compact verification receipt](evidence/20261001-terrain-hint-workers.json) records source hashes, fixture scope and check-log hashes.

## Contract and implementation

Mode 1 Micro-Spec, independent R2 review. Intent: reuse the existing owned per-owner hint capture to admit TerrainVision into the bounded worker path. Acceptance: serial capture once per eligible owner; complete serial-reference equality for identical frozen TerrainVision/N2048 inputs; owner isolation; no repoll on spawn fallback or panic; unchanged prior/cache identity and neural/world commit boundaries. Scope excludes actor transaction redesign, a worker pool, four-worker execution, launch-limit changes, GPU/training and main integration.

The production change removes the hint-presence worker exclusion and its parameter. `staged_tick.rs` still calls `prior.prepare(draft, sequence)` serially at the same position, immediately after that owner's housekeeping, receptor and grounded draft preparation. The returned owned draft contains the available semantic context and binds it to that organism, tick and sequence-derived preparation. All worker computation uses that immutable capture. No worker receives a prior actor, reply receiver, request queue, cache or controller.

At least eight captured dispatchable owners and two available CPUs select two participants: one scoped child plus the caller. Resource discovery failure, one CPU, smaller cohorts and failed thread creation use serial computation. The actual captured count is checked after sleep/retry/failure filtering. Sleeping or retained-learning-pending owners keep their previous non-dispatch behavior. Hints never select a serial fallback merely because they are enabled. Brain class and graphics settings do not change the worker gate. There is no four-worker production mode.

Ordered publication, hysteresis transitions, complete backend slot re-admission, strict input/batch guards, single neural dispatch, outcome sealing, learning/discard, physiology, world advance, lifecycle and persistence are unchanged from the previously reviewed production implementation. The prior remains nonauthoritative under AOA-SLM-001/002/003/004/005; captures retain AOA-CTX-001 and AOA-TIME-001/003 owner/time binding, with memory/attention under AOA-MEM-001/002 and AOA-ATT-002.

## Explicit asynchronous timing semantics

The boundary is each owner's existing serial `prepare` call, not one simultaneous wall-clock snapshot of the entire population. A reply ready before that owner's poll can affect its captured frame. A reply arriving afterward cannot change that frame, even if it arrives while another owner is captured or workers are computing. It remains for a later normal preparation. The frozen-input serial reference therefore means the same captured hints, rather than reproducing the old interleaved CPU delay before the next owner polls.

For a two-participant batch, all eligible owners are captured in roster order before the pure suffix begins. Removing the earlier per-owner suffix delays can move an asynchronous delivery to a later preparation; earlier request submission can also alter later availability. With identical captures, outputs and learning inputs are unchanged. Identical wall-clock reply schedules across the old and new paths do not imply identical captures. This is the authorized timing tradeoff, not a claim that every historical decision is identical.

Small or one-CPU execution retains its original interleaved capture/computation. If a scheduled eligible batch drops below eight captured rows, it computes those captures serially after the capture loop. Worker completion order never changes capture or publication order.

`semantic_prior.rs`, the queue/provider source and language source remain byte-identical to the base. In particular the prior file SHA-256 remains `0d37ee61dc7e3dffc39c597c92446ba61b28b7f9f09fc85145205367a27d30df`. Its source participates in provider identity; editing it would invalidate cache identity and could reject saved private-prior resume data. This extension requires no such migration.

Successful pending replies still validate, update the shared bounded cache and attempt its existing synchronous best-effort disk write inside the serial capture call. No write is moved to a worker or deferred batch flush; serialization and ignored I/O-failure policy are unchanged. Later owners can still see cache entries inserted by earlier owners. The calls can occur earlier in wall time because the preceding pure CPU suffix no longer intervenes. Network requests, validation/stale handling, active-packet expiry, recent-hearing memory, developmental gain and request bounds retain their implementation. Existing active hints may span later sequences until context/tick expiry; their original issue sequence is not newly restricted to every consuming row.

The existing queue has four queued-request slots, plus a possible in-flight request. Faster serial capture can produce a tighter submission burst and change which requests encounter that bound. A rejected submission still leaves that owner's pending/last-request state unchanged, so ordinary subsequent preparations remain eligible under the existing context/tick rules. No new blocking wait, unbounded retry, request-cap increase or fabricated hint is introduced. Prior delivery/cache/backpressure health must be measured on the real hints-on population; capture-boundary tests do not establish service throughput.

## Failure and recovery limits

Thread-creation failure reuses the captured rows once, without a second prior call, repeated housekeeping or another exposure. Both participant panics remain caught and joined before publication; private partial results are discarded and the existing staged restoration/fail-stop behavior applies. Typed per-owner failures keep their existing ordered summaries and hysteresis rules. No failed CPU batch automatically retries a neural/learning transaction.

Existing host rollback restores world/resident/memory/sleep authority; it does not undo prior actor/controller, request or cache effects. Capturing all eligible owners before a suffix panic can therefore leave prior effects for more owners than the old interleaved path. The controller already counts unaided preparation attempts rather than successful world commits. A later explicit tick attempt prepares hints again under existing policy. This is not exact prior-state rollback or a new committed-tick exposure guarantee. Full live GPU recovery remains **Unknown**; the CPU tests cannot certify it.

## CPU verification scope

The controlled-channel tests use real GroundedTerrainVisionV1 drafts, current procedural N2048 CPU slot metadata, organism-owned banks/topologies, mixed acquired/cold predictors and warm attention hysteresis. The native construction follows the existing `initial_n2048_care_asset` path and validates the resulting asset through the production N2048 candidate/compiler-input route. Builtin N2048 assets reject TerrainVision and are not relabelled. Tests use eight noncontiguous owners. This is a typed reference-class fixture with empty episodic banks, not the missing trained v3 50-owner founder save or a live GPU population.

One serial capture per owner controls replies ready before polling, readiness for a later owner while earlier owners are captured, empty/disconnected inboxes and replacements arriving after capture. Complete outcomes match the frozen serial suffix, the production serial runner and the two-participant runner, including frame/recall/context/projection/upload words/bindings. Reverse ordering retains each owner's context/predictor/slot. Spawn fallback checks exactly one job visit and no repoll. Caller/child panic cases check the joined healthy half, no continuation or input mutation, and retention of late queued replies. The existing real generic staged-world rollback test remains relevant.

These are capture-boundary tests with controlled typed channels. They do not instantiate the LocalSLM provider, its actual pending-reply queue or disk cache. Those mechanisms are source-reviewed and unchanged. They are not neural action parity, durable-state recovery or whole-tick performance proof. No new timing benchmark is claimed; the earlier object-slot CPU medians belong to their original source and inputs.

## Verification and integration

The source was checked with Rust 1.98.1 and `/workspace/cloud-cpu-validation/env.sh`, one test thread and two build jobs. No hardware-test feature was enabled.

- Corrected focused Terrain debug tests: three passed. The initial debug run had nine existing preparation cases pass and the three new fixtures fail before capture; the next fixture attempt exposed the forward-cone geometry mismatch. Both fixture defects were corrected using existing source construction/sensing rules. No failed fixture result is presented as successful evidence.
- Debug backend metadata re-admission test: one passed.
- Full focused release suite: 13 passed, zero failed, one dated timing test ignored. This covers the existing object-slot/reference/failure/rollback/learning-guard cases and all three new terrain cases. No timing benchmark was invoked.
- Strict `cargo clippy -p alife_game_app --all-targets --features gpu-runtime -- -D warnings` passed in debug and release. The `foundation-training` feature compilation passed; it performed no training.
- Formatting, Git whitespace, the full core dependency/source boundary script and 77 documentation assertions passed. Independent R2 Sol6.1 source/test/evidence review cleared the bounded extension; CI and desktop integration are not certified here.

Parent integration should apply source `5a8000a8` after the existing production runner `f03d2dc3` and its finalization prerequisite, then take the separate report/receipt commit. The four touched source/test files match the committed source hashes in the receipt. Actor/provider/language identity inputs exactly match `e540e820`. No main merge or PR is part of this handoff.

## Corrected hardware handoff

The earlier `-NewGame -Population 50` command is withdrawn. Source currently caps New Game at eight in both world and app admission. Loading a save with `--population 50` requires exactly 50 canonical saved agents; it cannot synthesize or remove creatures. These limits were verified on main `e76e9e12`, candidate `f8ccfb13` and the production base `e540e820`.

The current founder metadata refresh preserves the bit patterns of all 32,768 trained v2 weights and their address bindings, but the generated `assets/founders/terrain-care-n2048-v3/` package is absent from the inspected inputs. The old terrain-care-n2048-v1 path is not asserted current. The existing CPU population helper can construct Nano512/GroundedObjectSlotsV1 newborns, but has no CLI exporter and does not supply a trained TerrainVision cohort.

A runnable 50-owner terrain check is **Blocked by missing inputs**:

- The current generated trained N2048/TerrainVision founder package and its preserved-weight/provenance receipt.
- A disposable, current-format `PortableSaveFile` containing exactly 50 distinct canonical N2048/TerrainVision organism records and their valid foundation/birth bindings, with valid current GPU-resume coverage where required.
- Its matching `RuntimeConfig`, `AssetManifest`, asset root and `alife.ca10.environment_manifest.v1` scenario paths.

No admission limit should be silently changed to make the test run. Fixture preparation is separate from this extension and requires coordination with its existing owner. Until those inputs are validated, no executable 50-owner command is provided.

After PC authorization and fixture validation, use the existing optimized production recorder with that explicit disposable manifest/scenario, 1920×1080, 1x cadence, the existing verified prior service and GPU-required policy. Its existing five-second warmup/sixty-second measurement is the smallest relevant first receipt. Record the final integrated source SHA and executable hash, actual admitted/live population, brain/sensor/prior settings and delivery health, frame count/FPS and p50/p95/p99/max/hitches, configured/achieved TPS, completed world ticks, deferred debt, dropped catch-up ticks, checkpoint waits and all internal/CPU/GPU/readback stages. Parallel suffix leaf intervals are intentionally disabled; their enclosing wall time appears in the preparation residual. A short receipt and retained worst-frame debt samples do not establish a sustained bounded-debt trajectory.

The whole admitted tick, provider contention, renderer responsiveness, GPU recovery and 20 FPS/50-creature target remain **Unknown**. This work ran no desktop/device/training campaign and did not merge main or open a PR. Reverting the extension restores the prior-presence serial fallback with no schema or GPU ABI migration.
