# Separate grip hosted typecheck v2: prepared, NOT published or activated

This new v2 source packet addresses the first grip typecheck failure. Immutable
v1, V4 source, the v1 marker and its single run are preserved. No retry, marker
reuse, GitHub write, upload, native build or test execution occurred here.

## Exact first result

FIRST-RESULT-v1.json is the unchanged safe summary supplied by the coordinator
from the sole schema-matching JSON line in run 42647779547, job 129265618846.
Prepared head: 4a3a39be5f8a4a95f662ed2e7c3a67ed50b62da1.
Activation head: 9f68e9c278f037c845e38354243570db223e5e43.

Official Rust 1.99.1 and all 864 source inputs were verified; core completed.
World stopped on E0004 at crates/alife_world/tests/gpu_memory_grounding_persistence.rs:201.
Elapsed time was 78 seconds; sampled peak RSS 1,603,649,536 bytes, accounted
workspace 1,170,391,003 bytes, native output 168,639 bytes. Zero test bodies,
source forwards, world commands or body advances executed. Later stages did
not complete. No raw Actions log was read or copied. These are v1 results,
not new v2 measurements or a behavioral qualification.

## Exact recovery chain and correction

1. Accepted main c3aec16df33861f1316d127736f5f60eea327ede.
2. Byte-identical main→fbf library closure in its three hashed UTF-8 parts.
3. Unchanged nominal source supplement, covering 15 source files and the two
   source-only metadata includes that the old library closure omitted.
4. Unchanged full grip V4 patch, 649,925 bytes, SHA256
   acf8cc50353ba76cb7adf5af1db5845ee6e6b2e2261d0ee823821428e2e4d054.
5. V4→V5 correction, 2,369 bytes, SHA256
   0ce2a14320e13e31b9727d41d979798910c31e9663dcaca043c0900c5c96774a.

The correction binds before/after hashes for exactly three existing files:
- World persistence fixture: include raw6 in the fixture loop and exhaustively
  map GroundedGripSleepChoiceV1 to a mismatched identity for rejection checks.
- Game-app gpu_evidence.rs: explicitly label the raw5/raw6 research profiles.
  A descriptive label does not admit GPU execution.
- Game-app gpu_evidence/soak.rs: explicitly reject both CPU research profiles.

All 147 registered V5 paths are verified against the parent freeze, source
closure 434abe6295edbb7badfc2bc71b8876ba8cee3dd91e87478f46f3ce408850b150.
Exactly three rows change in the complete 864-file source/input manifest.
The nominal source superset is exact fbf2a517f93098b5d936878a3d26bbb8d0391c38
plus V4 plus this correction. Uncompiled unrelated main files are not claimed
to match the entire historical fbf tree. Actual compiler dep-info must be a
subset of the bound corpus and include every local compiled root.

## Unchanged compilation scope and limits

Each sequential stage uses cargo check --locked -j1 with defaults retained:
1. alife_core --all-targets
2. alife_world --all-targets
3. alife_gpu_backend --all-targets --features training-rollout
4. alife_runtime --all-targets --features training-rollout
5. alife_training --lib --test grip_options_host --features cpu-backend
6. alife_game_app --all-targets --features cpu-training

Training includes its actual CPU host; game-app includes foundation-training
and gpu-runtime. Other training integration targets/examples and Bevy frontend
remain unqualified. The unrelated legacy .json.zst fixture is still absent and
not selected. Existing main-owned binary includes remain hash-bound only, never
uploaded. No source substitution, private state, old engine rlibs or target/cache
reuse is allowed. Native test bodies, recipes, source neural forwards, world
commands, body advances, gameplay, training and GPU execution remain zero.
Cargo build scripts/procedural macros may execute as compilation requires.

Official Rust 1.99.1, commit 58cf8f9c35b355199727627d07ae34a4e2846d67,
2026-10-13, LLVM 23.1.1, and cargo 1.99.1 (5f94df478 2026-08-27) retain the
same pinned official minimal-install recipe and private-output adapter.
No bootstrap, install retry, default-toolchain change or native retry.

One job, 90-minute workflow/runner cap, 6 GiB sampled session RSS, 12 GiB
accounted workspace/target/Cargo/temp plus selected compiler/download storage,
2 GiB free floor and additional 3 GiB selected-sysroot cap. RSS sampling is
250 ms, disk checks at most two seconds; these are monitored, not cgroup hard
quotas. Debug symbols/incremental compilation remain disabled. Stop on first
failure. Native pipes have a 32 MiB combined output budget; only one bounded
32 KiB safe JSON summary is emitted. Diagnostics contain only compiler error
code, normalized bound crate path/line and static stage. No messages/snippets,
raw logs, secrets, checkpoint state, artifacts or caches are uploaded.

static_checks.py verifies that the runner differs from immutable v1 only in
versioned plan/branch/marker/summary IDs and reporting the additional correction
input. Targets, features, monitoring, projection and activation guards are
otherwise byte-identical after those explicit edits are normalized.

## Update-only transport and separate publication

The archive contains only new/changed v2 files. REUSE-MANIFEST.json explicitly
whitelists unchanged inputs already present at immutable v1 **prepared** head
4a3a39be5f8a4a95f662ed2e7c3a67ed50b62da1. Every entry binds source/destination
paths, byte count, SHA256 and Git blob SHA1. Parent must verify these exact source
tree entries before reusing their existing blob objects at v2 destinations.
No moving branch, v1 activation record, workflow or old executable is reused.
Locally these unchanged source files are hardlinked read-only by convention;
all edited files are independent. V1-PRESERVATION.json verifies every v1 file.

Parent creates the new prepared commit from accepted main, not from the v1
activation or prepared commit. Combine all update-archive members with every
whitelisted existing blob, and remove all 15 inherited main workflows on this
new branch only. The only workflow must be the new v2 workflow. Create the
remote branch directly at that already-safe prepared tree with ACTIVATE.json
absent. Never first create it at main or an old study tree. Do not merge these
branch-local workflow deletions into main.

Proposed branch: qualification/m2-grip-hosted-typecheck-v2.
Workflow: .github/workflows/m2-grip-hosted-typecheck-v2.yml.
Packet: docs/experiments/m2-grip-hosted-typecheck-v2.

Remote branch existence and the new prepared head are NOT verified locally.
Parent must check name availability before any write and resolve uncertain
creation outcomes without blind retries. This is source preparation, not
publication or runnable approval. Once the exact prepared remote tree and head
are verified, the separate activation operation adds ACTIVATE.json as the sole
change in one direct-child commit. The template still contains a placeholder.

Admission remains exact repository/branch, push only, not branch creation,
deletion or forced push, attempt 1, exactly one parent matching event.before,
and only the new marker addition. The marker binds prepared head, new seal and
source manifest. The runner verifies the v2 workflow is the only workflow in
the activation tree. No old study, marker, workflow, rerun or fallback is eligible.
Permissions are contents:read; both checkouts disable persisted credentials.
No workflow_dispatch, pull_request, write permission or artifact upload exists.

## Local verification

Only source-byte reconstruction, Python syntax/AST and synthetic/static tests
were run. No rustc/cargo/engine invocation. The update archive hashes every
member in ARCHIVE-MANIFEST.json; omitted reused inputs are separately bound by
REUSE-MANIFEST.json and included in the new runtime seal. New v2 execution and
all hosted outcomes are pending.
