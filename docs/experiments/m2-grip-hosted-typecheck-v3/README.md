# Separate grip hosted typecheck v3: source prepared, not activated

New plan/branch/workflow/marker identity. V1/V2 and V4/V5 source remain immutable;
no old attempt or marker is reused. No GitHub write, upload, native compilation,
engine test, gameplay or runtime operation was performed in this preparation.

## Exact previous result

FIRST-RESULT-v2.json is the unchanged parent-provided safe summary from run
42650561444/job 129275443002. Prepared head:
f5ce4d6de7ddf2e04473befefc715cc48bb28f26. Activation head:
bbffb5b0b0856509aed9af11a26e7aeec60f544d.

Official toolchain and 864 inputs verified. Core completed; world stopped on
E0433 at crates/alife_world/src/headless/grip_options.rs lines 212, 213, 219,
220. Missing bincode test dependency, not an import/private-visibility repair.
Elapsed 74 seconds; sampled peak RSS 1,612,570,624 bytes; accounted workspace
1,170,502,961 bytes; native output 193,305 bytes. Zero test bodies, neural
source forwards, world commands or body advances. Later stages remain pending.
No raw Actions log was read or copied. These are prior v2 results, not v3 proof.

## One dependency correction and recovery

Recovery remains accepted main c3aec16df33861f1316d127736f5f60eea327ede → exact
inherited fbf library closure → nominal supplement → unchanged full grip V4 →
unchanged three-file V5 correction → new V6 dependency patch.

V6 patch is 335 bytes, SHA256:
fa468c4ca23f88c2f90f561176283fb1bf0f53c24280fee6c6dcdd093f5ed712.
It changes exactly two bound records:
- crates/alife_world/Cargo.toml: bincode.workspace=true in dev-dependencies.
- Cargo.lock: alife_world gains an edge to already-present bincode 2.0.1.

New Cargo.lock SHA256:
99b01e0353263fd5bbf0261320898e8395d45583f2045e3433d3e2a1f50070eb.
TOML-level comparison verifies no new registry package, version/checksum change,
production dependency change, feature change or Rust source change. All 149 V6
registered paths are independently verified; source closure:
3ce43eb7cfade54c62fdd259caab6133e6c1d30756fc17ae92acd86526fa9548.
The complete nominal manifest still binds 864 source/config/include records.

This is exact nominal fbf2a517f93098b5d936878a3d26bbb8d0391c38 source plus the
explicit recovery chain, not an assertion that unrelated uncompiled main files
equal the entire historical fbf tree. Actual compiler dep-info must remain
within the bound input corpus and include every local compiled source root.

## Unchanged checks and safety envelope

Defaults stay enabled. Sequential cargo check --locked -j1 stages:
1. alife_core --all-targets
2. alife_world --all-targets
3. alife_gpu_backend --all-targets --features training-rollout
4. alife_runtime --all-targets --features training-rollout
5. alife_training --lib --test grip_options_host --features cpu-backend
6. alife_game_app --all-targets --features cpu-training

The actual CPU host and game-app foundation-training/gpu-runtime modules remain
in scope. Other training integrations/examples, the unrelated legacy binary
fixture and Bevy frontend remain unqualified. No fixture data is invented or
stubbed. Existing exact-main binary includes are hash-bound, not uploaded.
There is no test/body/recipe/source-forward/world/gameplay/training/GPU execution.
Cargo build scripts and procedural macros may execute for compilation only.

Same official Rust 1.99.1, rustc commit
58cf8f9c35b355199727627d07ae34a4e2846d67 (2026-10-13), LLVM 23.1.1, and
cargo 1.99.1 (5f94df478 2026-08-27); same pinned official minimal-install recipe
and private-output adapter. No bootstrap/install retry/default mutation.

One job; 90-minute wall cap; 6 GiB sampled session RSS; 12 GiB accounted work,
checkout, target, Cargo, temp and selected compiler/download storage; 2 GiB free
floor; 3 GiB selected-sysroot cap. RSS sampled at 250 ms, disk at most two
seconds: monitored limits, not hard cgroup quotas. Serial builds, no incremental
compilation/debug symbols, fresh private work/target/Cargo home, no engine rlib
or shared cache reuse. Stop at first failure, with no native retry.

Native output stays in bounded pipes and is not stored as raw logs: 32 MiB
combined output budget, one final safe JSON summary at most 32 KiB. Diagnostics
contain only error code, normalized bound crate path/line and static stage.
No message/rendered snippet/private state/secrets or artifact/cache upload.

RUNNER-SEMANTICS.json records an exact normalized v2→v3 runner comparison.
Allowed differences are version identifiers and reporting the additional
correction identity while preserving the V4 and V5 identities in the longer
chain. Targets/features/limits/resource checks/admission/projection/toolchain
and execution boundaries are unchanged. No silent runner repair.

## Reference-only local packet and update-only archive

The local directory intentionally omits unchanged large inputs. It is not a
standalone runnable packet. REUSE-MANIFEST.json whitelists their exact paths,
lengths, SHA256 and Git blob IDs from immutable v2 prepared head
f5ce4d6de7ddf2e04473befefc715cc48bb28f26. Parent must verify these exact remote
source-tree entries and install the existing blobs at the specified v3 paths.
Only preparation tools use packet_inputs.py to resolve verified local v2 bytes;
the hosted runner is unchanged and expects a completely materialized v3 tree.

The update archive contains all new/changed v3 files. Combine every member with
every whitelisted reused blob before publishing. No full V6 patch, large source
snapshot or duplicate registration map is needed: REGISTRATION-DELTA-v6.json
binds the reused V5 map plus two overrides to the parent V6 freeze. All archive
members are hashed; the runtime seal includes both new and reused input files.

Local apparent v3 size is capped at 1 MiB and the entire grip packet at 16 MiB.
No large hardlinks were added. PRIOR-PRESERVATION.json hashes all v1/v2 files
and immutable V4/V5 source evidence; every original archive member is rechecked.

## Separate publication and activation remain pending

Proposed branch: qualification/m2-grip-hosted-typecheck-v3.
Workflow: .github/workflows/m2-grip-hosted-typecheck-v3.yml.
Packet: docs/experiments/m2-grip-hosted-typecheck-v3.

Remote branch availability and the new prepared head are NOT verified locally.
Parent must verify before any write and resolve uncertain creation outcomes
without blind retry. The new prepared commit is parented at accepted main,
not old activation/prepared commits. Remove all 15 inherited main workflows
on this new branch only and install only the new v3 workflow. Create the branch
directly at this already-safe tree, with ACTIVATE.json absent. Never create it
first at main or an old study. Do not merge the branch-local workflow removals.

After verifying every prepared-tree file and immutable head, a separate operation
may add the new ACTIVATE.json as the sole change in one direct child commit.
Template prepared head remains a placeholder. Exact repository/branch, push,
non-created/non-deleted/non-forced event, attempt 1, one exact parent, one new
marker and sole eligible v3 workflow checks are unchanged. V1/V2 markers and
reruns are ineligible. contents:read, credentials not persisted, no secret,
workflow_dispatch/pull_request, write permission or upload step.

Only source recovery, Python syntax/AST and synthetic checks ran locally.
Publication, activation, native v3 typechecking and behavioral proof are pending.
