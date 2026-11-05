# Load-only integration qualification

Independent source review of `26963404226f5905c1a9db81836a2fe663ded834`
identified a real coverage gap: the ordinary and neural-emission public motor
bundle wrappers called `apply_registered_motor_bundle_inner` directly, while
only the staged-novelty wrapper checked `load_only`. Non-vocal channels could
execute physical actions and advance biology without recording an attempt.

A new public fixture reproduced this: a valid posture Idle bundle returned
success under `Scope`, advanced biology tick0→1, changed the complete world
debug state and left all seven attempt counters0. The reproduction is retained
in `/workspace/language-load-only-qualification-v35/motor-gap-repro.txt`.
An earlier fixture compilation typo (`HeadlessActionIds::IDLE`) was corrected
before the reproduction; its failed build receipt is also retained. This is
not evidence that the original standalone CLI used the bypass. Its previous
14-actor/6-private native admission and hash preservation result remains intact.
The saved recovery closure is not reexecuted by this qualification.

The fix moves the existing action check to the shared inner transaction before
validation or mutation. All three public motor-bundle wrappers now reach it.
There is no checkpoint-schema, ordinary admission, policy binding or acquired
state change. Normal inactive execution follows the same native transaction.

## Coverage and contract

The two isolated world tests exercise actual APIs: all three bundle wrappers
with posture-only and mixed non-vocal bundles, four command wrappers and four
tick wrappers. Every blocked call must retain the entire derived world debug
representation. After dropping the scope, native receipts and complete world
state must match an otherwise identical inactive control. The infallible
`advance_tick` wrapper's expected panic is caught and state remains unchanged.
These tests perform28 successful public fixture transactions/tick calls plus14
blocked calls, without reading any trained actor or scored language case.

Six existing ordinary world fixtures were run on the pre-fix transaction and
then on the patched transaction: factorized joint rollback, posture semantics,
neural-emission chemistry, registered late-failure rollback, and two ordinary
world-tick/biology rollback cases. This compares the same existing assertions,
not bitwise serialized output across two independently built revisions.

Additional isolated source fixtures cover scope nesting and concurrent-owner
rejection, joined worker counts, clean reentry/reset, early-error cleanup and
`catch_unwind` cleanup; the filesystem pipeline; genuine native credit; and a
disposable critic's real prediction, residual-step and proposal paths. Existing
actor, critic, replay and native host restore/rollback integrations are selected
separately. The credit fixture witnesses one public native decision, reconstructs
its exact pending decision from the captured native tape, and compares allowed
credit with the real collected final checkpoint. Refused credit must leave the
same exact valid continuation available. The critic fixture is unrelated to
the absent matching critic/version of the saved pilot; no owner critic is
substituted or certified.

The filesystem fixture uses newly constructed public actor/private inputs.
It exercises the full command implementation for valid admission, owner digest
mismatch, initial Unix symlink escape, malformed actor dependency binding, and
an intentional fixture-only file change between the actual two read passes.
It checks unchanged original fixture files on failures, except the explicitly
changed fixture checksum, and owner release afterward. The production entry
point supplies a no-op to a private callback seam used for this last case.

`Scope` remains an admission flag. Its safe contract requires an otherwise
quiescent process at entry and all workers joined before snapshot/drop. It does
not drain operations already admitted while inactive, and a paused counter
increment can race a premature snapshot/drop/reset. Embedding alongside ongoing
simulation is unsupported; ordinary work intentionally errors, with a panic in
existing infallible wrappers. No broader synchronization redesign is claimed.
This barrier covers the named execution paths, not every possible arbitrary
world editor or unrelated GPU/runtime API. The standalone utility exposes only
the audited native restore call graph.

Pinned trusted inputs require no concurrent writer. The initial root/canonical
check and repeated byte hashes do not promise immutable inode/path identity.
The64MiB zstd decoded-size bound does not apply to plain JSON, and native JSON
recursion limits are disabled. Arbitrary hostile-input hardening and cross-
platform symlink testing remain outside this qualification. There is no measured
performance comparison; several atomic reads can occur per ordinary tick.

## Receipts

Build and test receipts are retained in
`/workspace/language-load-only-qualification-v35`. The independent source review
is complete; its findings drove this bounded correction and explicit contract.

| Corrected-source selection | Passed | Failed |
|---|---:|---:|
| Actual world bundle/command/tick refusal and inactive controls | 2 | 0 |
| Existing ordinary world transaction/advance regressions | 6 | 0 |
| Utility pure checks | 3 | 0 |
| Isolated utility admission, lifecycle, filesystem and genuine credit | 4 | 0 |
| Native critic refusal plus existing CPU staging, replay, Adam and restore | 14 | 0 |
| Total distinct selected tests | 29 | 0 |

The six existing world cases also passed before the fix. The pre-fix guard
regression failed as expected and is preserved, along with its earlier compile
error. The two caught fixture panics (scope unwind and infallible tick wrapper)
are expected assertions, not failed final tests. The14 CPU cases each actually
ran one test; no zero-match filter is counted as a pass.

An initial workspace build of the CPU selection was stopped by the storage
guard before any of those14 cases ran, at521,338,880 free bytes versus the
482,148,352-byte floor. `/tmp` is a separate permitted filesystem, so an offline
fresh source build there completed and the14 cases ran once. No cache, retained
state or failure evidence was deleted or moved. This resource abort is retained
as `cpu-integration-build.json`; it is not a behavioral test failure. Source-only
build output in `/tmp` is not a payload archive or independent backup.

The source-safe test summary is
[`native-load-only-integration-qualification-results.json`](native-load-only-integration-qualification-results.json).
The standalone production binary is rebuilt under
`/tmp/alife-load-only-qualification-target-v35/debug/verify_actor_private_recovery`;
its new-build receipt and explicit early joint/PPO refusals are recorded locally.
The rebuilt11,191,752-byte executable has SHA256
`f6e9132c89af4b3324f662af56b67d259bb7dce093114b93924deea67c2282e7`.
No saved owner closure is admitted again. The owner14-actor/6-private result
remains the original V34 observation at2696340.

To avoid new workspace artifacts below the available headroom, use the separate
target directory for source builds, preserving the workspace floor:

```sh
env PATH=/workspace/.cargo/bin:$PATH CARGO_HOME=/workspace/.cargo RUSTUP_HOME=/workspace/.rustup CARGO_TARGET_DIR=/tmp/alife-load-only-qualification-target-v35 CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 cargo build --offline --profile test -p alife_game_app --features cpu-training --config profile.test.package.alife_training.opt-level=2 --bin verify_actor_private_recovery
```

The isolated utility selection is the existing documented `cargo test --bin
verify_actor_private_recovery` command followed by
`-- --ignored --test-threads=1`. The world selection is
`cargo test -p alife_world --lib headless::task_6_factorized_motor_tests::load_only_ -- --ignored --test-threads=1`
under the same toolchain/test-debug environment. The exact14 native CPU case
names and commands are retained in `cpu-integrations.json`; all use disposable
public fixtures. The public numerical summary contains no brain/Adam/private
payload. Final publication and available exact-head hosted status are separate
local receipts, not an assertion of mainline readiness or a new model review.

The482,148,352-byte free-space floor remains mandatory. No accepted actor,
optimizer history, private state, scored episode or recovery input is an output
of these tests. No trained-state transfer, archive, deletion, PR/model-triggering
route, external model call, language experiment or promotion is authorized here.
