# Native actor/private load-only admission

`verify_actor_private_recovery` is a separate recovery utility for the existing
V33 owner manifest. It exposes no experiment dispatch, assessment, policy fork,
checkpoint writer, root override or repair/default fallback. It does not restore
a critic. Matching critic/version state is unavailable in this supervised pilot;
the result explicitly declines joint actor/critic and PPO continuation
certification. Either `--require-joint-critic` or `--require-ppo` returns an error
before reading any checkpoint.

## Supported command

From the established experimental checkout, build the CPU utility. Reusing the
test compilation profile avoids duplicating dependency artifacts under the
preserved free-space floor; `cargo build` does not execute tests or simulations:

```sh
env PATH=/workspace/.cargo/bin:$PATH CARGO_HOME=/workspace/.cargo RUSTUP_HOME=/workspace/.rustup CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 cargo build --profile test -p alife_game_app --features cpu-training --config profile.test.package.alife_training.opt-level=2 --bin verify_actor_private_recovery
```

The owner-local command is:

```sh
target/debug/verify_actor_private_recovery --manifest /workspace/language-recovery-handoff-v33/manifest.json --manifest-sha256 57e3f76942e9d620ab0573f47e531c4bc5d26e667e89c06380ee7c7f09f8ff6e --profile actor_history
```

The manifest digest pins the existing paths, hashes and dependency identities.
The accepted profiles are `original_snapshot`, `four_arm_snapshots` and
`actor_history`; the default is `actor_history`. The last profile reads32 existing
files totaling9,240,739 bytes. There is no historical directory scan or scoring
stream decode. The manifest-selected original is the compiler/foundation and
provenance dependency, rather than a fresh founder substituted for an actor.
Manifest and every selected input are size/hash checked, cached read-only, then
read and hashed again after native admission, including a failed semantic
admission. Output is JSON on stdout; this utility never writes its inputs.

## What native admission means

The embedded actor and every selected saved actor boundary pass
`FoundationTrainerCheckpoint::validate_for` and
`CpuFoundationTrainer::from_checkpoint`. Complete serialized roundtrip equality
is required for weights, both Adam moments, exact integer coordinate ages,
global optimizer step, configuration, stage mask, phenotype and foundation
identity. Checkpoint shapes and native finite/age invariants are retained.
The existing saved gradients are byte-verified dependencies, never dispatched
or used to reconstruct or apply an update.

The embedded original private state and selected current/private positive-start
checkpoints pass `CpuCanonicalHost::from_checkpoint` with their recorded actor.
The native host checks world/config, resident/founder identity, tracked-object
clocks and ownership, memory/language sequence boundaries, neural topology,
sleep/controller consistency, sealed private authority, sampling RNG and the
existing policy binding. Full serialized host checkpoint equality and world
tick equality are required afterward. No policy rebinding operation is called;
existing experimental64-coordinate policy provenance stays as saved.

The audited constructors validate and reconstruct in-memory derived structures.
They do not call a decision, recurrent forward, world tick, credit or Adam step.
An ephemeral process-wide `alife_core::load_only::Scope` provides a second check:
CPU decision/forward/credit/actor Adam/critic proposal and headless world
advance/action entry points refuse work before their first transition while
the scope is active. Worker threads see the same barrier. Any attempted operation
is counted and the verifier fails closed. Normal admission guards and checkpoint
schemas are unchanged. The barrier is inactive during ordinary execution and
is not checkpoint state. It is an internal execution barrier, not an OS sandbox
against arbitrary unrelated code; the utility exposes only the audited restore
call graph.

The process must otherwise be quiescent before entry, with every spawned worker
joined before receipt or drop. `Scope` is an admission flag, not a quiescence
barrier: an operation that read inactive may finish after entry, and a paused
attempt may miss a snapshot or be counted after a later reset. Concurrent
ordinary simulation is unsupported and intentionally errors; the existing
infallible `advance_tick` wrapper panics on that error. Nested/concurrent scope
owners are rejected. See the subsequent
[integration qualification](native-load-only-integration-qualification.md) for
the motor-wrapper coverage correction and lifecycle tests. The isolated V34
owner admission remains a separate measured fact.

The owner-pinned inputs must also have no concurrent writers. Initial canonical
paths are checked for root escape, and byte hashes are compared before/after;
this is not a guarantee of immutable file identity against concurrent path
replacement. The64MiB decoded-size limit covers zstd inputs only. Plain JSON
inputs are trusted and recursion limits are disabled for native checkpoints;
this utility is not hardened for arbitrary hostile manifests/checkpoints.
Several atomic reads can occur per ordinary tick. No performance overhead was
measured, so passing functional tests does not establish zero cost.

Successful admission certifies these saved actor/private states under this native
source, without consuming a decision. It does not demonstrate new learning,
successful future gameplay, cross-platform bit identity, complete pre436 gradient
history, an independent backup, or joint actor/critic/PPO continuity. Original
V32 scored results and its rejection remain unchanged.

## Focused source validation

```sh
env PATH=/workspace/.cargo/bin:$PATH CARGO_HOME=/workspace/.cargo RUSTUP_HOME=/workspace/.rustup CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p alife_game_app --features cpu-training --config profile.test.package.alife_training.opt-level=2 --bin verify_actor_private_recovery
env PATH=/workspace/.cargo/bin:$PATH CARGO_HOME=/workspace/.cargo RUSTUP_HOME=/workspace/.rustup CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p alife_game_app --features cpu-training --config profile.test.package.alife_training.opt-level=2 --bin verify_actor_private_recovery -- --ignored --exact tests::exact_native_admission_rejects_mismatch_and_forbids_all_work --test-threads=1
```

The process-wide barrier test runs in isolation. It constructs a static public
newborn fixture at tick0 while the barrier is already active, then restores
nonzero in-memory Adam serialization sentinels including integer ages above
2^24. These sentinels are test data, not training. Valid actor/private restore
must be exact and make zero attempted execution/update calls. Invalid moment
length/age/sign, mismatched actor weights and altered private sampling authority
must be rejected. Only after this zero-attempt assertion does the test deliberately
attempt blocked decision, world advance, replay forward and Adam APIs and check
that state stays exact. Cross-thread and remaining barrier categories are
checked without creating a missing critic, fabricated credit or world outcome.
Pure tests cover corrupt bytes, missing dependencies, traversal, digest
requirements, unsupported experiment flags and early joint/PPO refusal.

## Measured result

These V34 measurements were made at
`26963404226f5905c1a9db81836a2fe663ded834`. They are preserved as historical
receipts; subsequent integration qualification uses public fixtures and does
not rerun the saved recovery closure.

All four focused tests passed: three pure tests and one isolated native fixture.
The standalone binary then admitted the pinned owner `actor_history` closure in
3.67 seconds. Fourteen actor checkpoints passed complete Adam roundtrip equality:
the embedded original at436, Untouched at436, and each saved437..440 boundary
for ZeroGradient4, Balanced4 and rejected MatchedFactual4. Six private checkpoints
passed exact host roundtrip equality: the original at tick460, known positive
start at458 and four current ends at479. These are restored clock values, not new
ticks. All seven forbidden-operation attempt counts were0. No critic was created.

All32 selected files plus the manifest were hash-checked before and after;
all were unchanged. The standalone joint and PPO request commands each returned
an explicit refusal before any checkpoint admission. Free space after admission
was961,724,416 bytes, above the482,148,352-byte protected floor. No payload was
copied or archived. The11,192,280-byte utility has SHA256
`dbdd08dd0651c1136c930fe32d1c034675b9e53040101aca0f91497731c1a89d`.

The full local native receipt is
`/workspace/language-native-load-only-v34/owner-native-admission.json`, SHA256
`a696f8acb916eb262aa88aaed50283504fbf265e26b0f0aad97e4913303dcd0a`.
The same directory contains exact command, build, source-fixture, refusal and
source/executable identity receipts. The public-safe numerical summary is
[`native-actor-private-load-only-results.json`](native-actor-private-load-only-results.json).
It contains source identities and validation metadata, no acquired weights,
Adam payload, private state or model input.

This utility was compiled from the experimental source containing the new
inactive-by-default barrier, using the same native constructors as the pinned
pilot. It is not the old measured V32 executable. The original native identities
remain authoritative and passed unchanged under this source. All trained/private
payloads remain local; independent backup is still unverified. No trained state
transfer, archive, new simulation, external model call, PR automation or brain
promotion occurred. There is no behavioral or learning claim from load-only
success, and V32 remains rejected.
