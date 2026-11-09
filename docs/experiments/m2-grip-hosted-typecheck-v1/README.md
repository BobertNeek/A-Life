# Separate grip hosted typecheck v1: source prepared, NOT activated

This is a new grip-only source packet, branch and workflow. It does not reuse,
retry, publish or activate the uncertain reflex V5 study. No GitHub write,
upload, native build, test body, or hosted run was performed to prepare it.

## Immutable input recovery

1. Fresh checkout of accepted main c3aec16df33861f1316d127736f5f60eea327ede.
2. The byte-identical inherited main→fbf **library** closure, SHA-256
   be1fff0fde16f474c118f9b37ec95b526b52c6f644216b1d7d0c12eee0372a74,
   from three separately hashed UTF-8 parts. Its 120 restored files are verified.
   This closure alone never claimed all-targets coverage.
3. The new 17-file nominal source supplement, 136,753 bytes. It restores 15
   integration source files and two source metadata includes not in old closure.
4. Immutable full grip V4 patch, 649,925 bytes, SHA-256
   acf8cc50353ba76cb7adf5af1db5845ee6e6b2e2261d0ee823821428e2e4d054.
   All 144 registered candidate paths are checked against the frozen map.
5. Verify all 864 bound source/config/include files and Cargo.lock before any
   compiler command, then reverify after each stage. Actual local rustc dep-info
   must be a subset of those bound inputs and cover every local compiled root.

The nominal source is exact fbf2a517f93098b5d936878a3d26bbb8d0391c38 plus V4,
not an assertion that uncompiled unrelated main files equal the entire fbf tree.
No current-main substitution, old engine rlibs, shared Cargo cache, or target
reuse is allowed. A pre-existing work directory fails admission.

## Targets and features

Every stage uses `cargo check --locked -j1`, never `cargo test`, and retains
Cargo's defaults. Explicit features are intentional, not disabled to force a
pass. The package defaults happen to be empty at this pinned source.

1. alife_core --all-targets
2. alife_world --all-targets
3. alife_gpu_backend --all-targets --features training-rollout
4. alife_runtime --all-targets --features training-rollout
5. alife_training --lib --test grip_options_host --features cpu-backend
6. alife_game_app --all-targets --features cpu-training

Training's cpu-backend includes receipt-lineage. Game-app's cpu-training includes
foundation-training and gpu-runtime, so the changed CPU host, foundation-training
and gpu-live-runtime modules are compiled. No Bevy frontend claim is made.

Training is intentionally NOT blanket --tests: the unrelated legacy-wire target
requires cpu-resident-pre-motor-v1.json.zst, absent from main. That binary fixture
is not transported, reconstructed, stubbed, or selected. Only grip_options_host
and the real library are checked; all other training integration tests and
examples remain unqualified. The source manifest identifies all excluded literal
includes in unselected example targets. Actual dep-info verification will fail
closed if any omitted input is unexpectedly compiled.

No test body, recipe, neural forward, world command, body advance, training loop,
gameplay, or GPU dispatch executes. Cargo dependency/local build scripts and
procedural macros execute as compilation requires. Cargo may build check-time
binaries but the runner never invokes them. This is typechecking, not behavioral
qualification or a performance result.

## Static metadata and public asset boundary

The supplement includes these source metadata files:

- docs/experiments/bounded-founder-v1/legacy-wire-capture.json: 5,142 bytes. Only
  hashes, lengths and public source capture provenance for contract tests. No
  captured input/phenotype/selector bytes, private life or checkpoint contents.
- docs/experiments/motor-lobe-native-m0-v1/mediation-m0-manifest.json: 31,226 bytes.
  Source-authored fixed M0 topology, parameter configuration and source-only
  admission status, not trained state.

Existing public main-owned compiled-in foundation/fixture/image/mesh/world
assets are hash-bound and read from the exact main checkout. They are never
copied into this archive or replaced. No binary or opaque fixture is in the
transport. The inherited three recipe JSONs are compile-time source data, never
interpreted or executed by the runner.

## Toolchain and resource envelope

Uses the already verified official Rust 1.99.1 minimal installation recipe from
pinned dtolnay/rust-toolchain b725eda525362e7cf34af9a7243cac1e3f01a7ec, with the
unaltered audited action source and provenance. The official action is not run
as a composite because it prints native installer output. The private adapter
performs the exact official rustup installation, then validates:

- rustc 1.99.1 (58cf8f9c3 2026-10-13), full commit
  58cf8f9c35b355199727627d07ae34a4e2846d67, LLVM 23.1.1;
- cargo 1.99.1 (5f94df478 2026-08-27), x86_64-unknown-linux-gnu;
- exact standard sysroot layout and compiler/Cargo binary hashes.

The prior V3 setup succeeded with these identities. That historical setup is
provenance, not evidence this new grip job has run or will pass. There is no
bootstrap fallback, default-toolchain mutation, install retry or native retry.

One workflow job, serial -j1, incremental disabled and dev/test symbols disabled.
Workflow wall limit is 90 minutes. The runner independently limits itself to
90 minutes, stops at the first failure, and samples total child-session RSS
(including descendants plus supervisor) at 250 ms with a 6 GiB ceiling.
Disk checks at most every two seconds cap the checked-out packet+source,
private work/target/Cargo/temp, selected toolchain and rustup download/temp
storage at 12 GiB, with 2 GiB free floors. The selected sysroot additionally
has a 3 GiB cap. Polling ceilings are monitored bounds, not cgroup hard quotas;
brief overshoot between samples is possible. File-size/core-dump limits are set.

Native stdout/stderr are drained through bounded pipes, not saved as raw logs.
Combined native output has a 32 MiB session budget. A single final JSON summary
is capped at 32 KiB. Compiler diagnostics are strictly projected to error code,
normalized bound crates/<package>/... path, line and static stage. No rendered
message, snippet, assertion data, private state or raw stderr is emitted. At
most 24 diagnostic rows are retained. No artifacts, caches, binaries, logs or
checkpoints are uploaded. No credentials/secrets enter child environments.

## Publication and explicit single-use activation

Publication is owned by the coordinating parent; this packet performs none.
Remote branch existence/nonexistence and prepared-head identity are NOT verified
locally. Before any write, verify the proposed name is unused and resolve any
ambiguous prior creation outcome. Never blindly retry uncertain creation. This
archive is source preparation, not publication or runnable approval.
Use publication.json's exact destinations and source-only archive manifest.
Build the prepared commit directly from exact main with all 15 inherited
.github/workflows entries removed **on this new branch only**, and only this
new workflow present. Create the remote branch pointing directly at that fully
prepared tree, with ACTIVATE.json absent. Never first create it at main or an
old study tree. This prevents generic historical push workflows from running
on branch creation or activation. Do not merge these branch-local deletions.

The proposed branch is qualification/m2-grip-hosted-typecheck-v1. The proposed
workflow is .github/workflows/m2-grip-hosted-typecheck-v1.yml. Audit the published
prepared commit's workflow tree and all packet hashes, then obtain its immutable
40-character commit ID. This ID cannot be supplied from local source preparation.

Replace the prepared-head placeholder in ACTIVATE.template.json with that
verified commit, and create ACTIVATE.json as the sole addition in exactly one
child commit. Do not alter any other file in that activation commit. The marker
binds the immutable source manifest and sealed packet. Workflow/runner admit
only BobertNeek/A-Life, the dedicated exact branch, push, non-created,
non-deleted, non-forced event, one direct parent equal to event.before, only
that single marker addition, and run_attempt 1. Reruns and marker modification
or reuse are ineligible. The runner also checks that the new workflow is the
only workflow in the activation tree. No workflow_dispatch or pull_request
trigger, secrets, write permission, persisted credential or artifact step exists.

The template is not activation. Source is ready, but prepared-head publication
and activation are pending. Old workflows/studies are not a fallback.

## Local checks performed

prepare_source.py only reads local Git blobs and applies text patches in memory.
It verifies the inherited closure, nominal supplement, complete include/source
hashes and V4 map. static_checks.py performs Python syntax/AST, synthetic
activation/diagnostic/closure/feature tests only. No rustc/cargo or engine test
execution is part of either preparation check. STATIC-CHECK.json and
ARCHIVE-MANIFEST.json record local results and every archive member's hash.
