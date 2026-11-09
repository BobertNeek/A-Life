# Hosted grip typecheck V5: preserve all inherited workflows

This is a fresh source-only hosting proposal, not a grip-source revision or a
qualification result. V4 and all prior packets remain byte-for-byte preserved.
The denied V4 deletion design is abandoned, not retried through another route.
No GitHub write, branch, job, native compilation or study was created here.

## Why preservation is safe for the proposed two operations

Pinned main c3aec16df33861f1316d127736f5f60eea327ede has tree
9140d525fe1af16fed6539c3178a1a45e1fb04d0 and 15 workflow files. Their exact
Git blob IDs, modes, byte lengths and SHA256 hashes are bound in
WORKFLOW-PRESERVATION.json. All remain inherited unchanged. The only workflow
addition is .github/workflows/m2-grip-hosted-typecheck-v5.yml.

TRIGGER-AUDIT.json contains every exact event/filter mapping and immutable
source link. INHERITED-WORKFLOW-FIXTURES.json stores the exact text of all 15
pinned Git blobs only as inert JSON, never as executable workflow additions.
Static checks parse these actual YAML bytes and verify their blob/SHA256 hashes. Thirteen inherited workflows subscribe to push: eleven only to
main, Group50 to main or group50-ci/**, and Jules to codex/**. None matches
qualification/m2-grip-hosted-typecheck-v5. The remaining two workflows have no
push event. None subscribes to create, schedule, workflow_run,
repository_dispatch or any other event besides push/pull_request/manual
workflow_dispatch. No inherited job, checkout or supporting script is reached.

Path filters additionally exclude the new packet/workflow paths wherever
present. CI and Jules have no push path filter, so this proposal deliberately
relies on their branch exclusion. New-branch diffs may include a broad history,
and GitHub can fail open on a path-filter timeout; neither can override an
explicit branch mismatch. The new V5 workflow rejects branch creation at the
job gate independently of its ACTIVATE.json path filter and runner admission.

The guarantee covers exactly creating this new qualification branch directly
at the prepared commit, then its sole direct-child marker addition. It does
not disable existing workflows or prevent someone independently using manual
dispatch, opening a PR, merging, renaming refs or changing files. Do none of
those: a PR could start CI and Jules. Unrelated repository/organization
integrations are outside this workflow-file audit; do not claim global silence.

## Publication and activation contract

Parent must first verify the current accepted main identity/tree, availability
of the exact new branch name, and any uncertain prior write outcomes. Build an
additions-only prepared commit directly parented at pinned main. Combine every
update-archive member with every immutable REUSE-MANIFEST blob at its exact V5
destination. Every main tree entry stays identical. No inherited source,
workflow or supporting script is edited, deleted, renamed or replaced.

Verify the entire prepared diff contains only added docs packet files and the
single new V5 workflow; verify all inherited entries, all 16 workflow paths and
blob/mode identities; verify ACTIVATE.json is absent. Create the branch directly
at that already-verified prepared commit. No main ref update, PR, merge,
workflow dispatch, branch reuse or initial pointer to another tree.

Only after verifying the remote prepared head/tree may a separately authorized
operation add ACTIVATE.json as the only changed path of one direct-child commit.
The template keeps a placeholder until that exact prepared head is known.
Job gates require exact repository/branch, a non-created/non-deleted/non-forced
push and attempt 1. Runner admission requires matching event before/after,
exact single parent, sole marker addition, exact activation JSON, sealed packet
hashes and the unchanged 15 inherited workflow modes/blobs/bytes plus exactly
one V5 workflow. Missing, changed or extra workflows fail before toolchain setup.
Old markers, reruns and source-study workflows remain ineligible.

## Narrow runner change, unchanged compile-only qualification

Relative to V4, run_hosted.py changes only V5 plan/schema identifiers and replaces
the sole-workflow inventory rule with exact inherited workflow preservation.
RUNNER-SEMANTICS.json proves every other function and all compiler/stage/limit
constants unchanged, including the rest of admit(). The seal still binds all
packet inputs and the workflow binds the seal digest. Source-manifest.json is
byte-identical to V4 and binds all 865 inputs, including the main-owned world
target. All 149 V6 registered paths, recovery chain and Cargo.lock are unchanged.
No source payload, target deletion or compilation-scope relaxation is added.

Official Rust 1.99.1. Sequential cargo check --locked -j1, defaults retained:
1. alife_core --all-targets
2. alife_world --all-targets
3. alife_gpu_backend --all-targets --features training-rollout
4. alife_runtime --all-targets --features training-rollout
5. alife_training --lib --test grip_options_host --features cpu-backend
6. alife_game_app --all-targets --features cpu-training

No test body, study recipe, neural forward, world command, body advance,
gameplay, training or GPU execution. Cargo build scripts and procedural macros
may execute as compilation requires. Other training examples/integrations and
Bevy frontend remain unqualified. No behavioral success is claimed.

Same fresh private target/Cargo/work storage, compiler identity checks,
contents:read, pinned checkout, no persisted credentials, caches or retries.
90-minute wall; 6 GiB sampled session RSS; 12 GiB accounted storage; 2 GiB free
floor; 3 GiB selected sysroot cap. RSS at 250 ms and storage at most every two
seconds are monitored bounds, not hard cgroup quotas. Native output remains
private: 32 MiB combined pipe budget, one safe JSON summary at most 32 KiB,
static stage/error code and normalized bound path/line only. No raw logs,
private state, credentials, checkpoints, artifacts or caches are uploaded.

## Local evidence and remaining work

Only local Git object reads, Python/static/synthetic checks and deterministic
source-only sealing are performed. Prior packet preservation is rechecked.
The new local packet is capped at 750,000 bytes in a separate directory; the prior
grip packet stays below 16 MiB,
new staging below 3 MiB, and local free-space floor 4 GiB. No existing files or
target directories are deleted to make room. Large unchanged source inputs are
referenced, not duplicated. Publication, activation and hosted native checks
remain pending and must not be described as passed.
