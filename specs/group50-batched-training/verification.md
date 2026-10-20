# Group-50 verification

- Mode: 2, implementation with a focused contract and independent review.
- Review class actual: R2, read-only implementation and follow-up review.
- Baseline implementation: `8813166353a450a5738fc0009655a8995e704a66`.
- Merged freshness PR108 (`bda3b65be5fa6f01865b850640adf2e8d01f4990`)
  as `59f8eab5a5345ab3f98eb15df32c476adf4414bf`, then aligned admission with
  its immutable history/recovery contract and surviving mixed-care lineage.
- Spec status: CPU contracts validated; hardware qualification deferred.
- Result: published draft PR111; no real-GPU qualification or speedup claim.
- Loop recipe / judge rubric: none.

## Acceptance coverage

| Contract | Evidence | Result |
|---|---|---|
| Start fifty canonical private organisms directly | Population, unique identity, grounded positions, plot walls/resources and shared policy CPU contract | Pass |
| Native learned state and exact integer optimizer ages | Complete actor/value readmission, moments, ages above2^24, hashes and corrupted/incomplete handoff cases | Pass on repository-derived fixtures |
| Preserve known exclusions and honest freshness | Strict admission, explicit unknown freshness, recursive inherited exclusions, exact ledger imports, duplicate-key rejection | Pass |
| Private recurrent windows and bounded replay | Sleep/organism spans, terminal quota, bound arithmetic, accepted-row/epoch counters; strict compile | Pass CPU contracts |
| Fifty hardware rows, recurrence/logits, masked old probabilities | Production hardware path plus opt-in `group50_gpu` fixture; initial/sleep replay tolerance1e-4 absolute+relative | Compiles; real GPU run blocked |
| Central update/export/rebind/native readmission | Opt-in hardware smoke, sixteen decisions for all fifty organisms | Compiles; real GPU run blocked |
| Actual accepted private package | Read-only Library materialization attempted through the supported resolved-reference flow | Blocked by transfer denial; no source bytes available |

## Checks

Run in the cloud, using the selected environment's Cargo configuration. No PC or
paid provisioning was used. The broad suites below passed on the baseline source.
After merging PR108, affected group/recovery, compile and lint checks were rerun.

| Command | Result |
|---|---|
| `cargo test --locked -p alife_game_app --features foundation-training --lib foundation_training_cycle::` | Pass56, including all13 group contracts |
| `cargo test --locked -p alife_game_app --features foundation-training --lib foundation_training::` | Pass63; one existing ignored hardware test |
| `cargo test --locked -p alife_training --lib` | Pass26 |
| `cargo test --locked -p alife_training --test blocked_legacy_pipelines --test checkpoint_validation --test trainer_contracts` | Pass7 |
| `cargo clippy --locked -p alife_game_app --features foundation-training --all-targets -- -D warnings` | Pass |
| `cargo check --locked -p alife_game_app --features foundation-training,gpu-tests --test group50_gpu` | Pass; compilation only |
| `cargo fmt --all --check` | Pass |
| `bash scripts/check_core_boundaries.sh` | Pass |
| `bash scripts/docs_check.sh` | Pass77/77 |
| `git diff --check` | Pass |

Postmerge integration checks, on the final admission implementation:

| Command | Result |
|---|---|
| `python -B -m unittest discover -s scripts -p test_mixed_care_food_night.py -v` | Pass88 |
| `cargo test --locked -p alife_game_app --features foundation-training --lib foundation_training_cycle::group::` | Pass15 |
| `cargo clippy --locked -p alife_game_app --features foundation-training --all-targets -- -D warnings` | Pass |
| `cargo check --locked -p alife_game_app --features foundation-training,gpu-tests --test group50_gpu` | Pass; compilation only |
| Formatting, core boundaries, docs and diff checks | Pass; docs77/77 |

## Review and limits

R2 review identified and verified fixes for native source/history exclusions,
post-quota death handling, legacy coordinate grounding, conservative device scratch
budgeting, complete original-package validation, GPU-test feature gating and the
ordering of initial trainer memory admission. A final delta review of read-only
source validation, initial per-organism replay comparisons, native output
readmission and handoff documentation reported no findings. Integration review
identified and verified the mixed-care latest-predecessor guard for both modes,
including acceptance of hash-bound group descendants. Tests reject older sources,
missing/mismatched immutable metadata, changed recovery evidence and changed ledger
bindings; direct latest predecessors and two group descendant links pass.
The reused Python functions perform reads/hashes only, with bytecode writes
suppressed; history bytes and absolute paths are bound before any reservation.
Final integration review reported no findings. Open P0/P1/P2/P3
findings: none. Artifact lint: pass; no unresolved clarification markers.

Follow-up admission portability fix: the direct mixed-tail check uses filesystem
identity (`Path.samefile`) plus the exact source hashes. Rust's Windows
canonicalization can produce an extended-length path spelling that differs from
the ledger's ordinary spelling; lexical equality could reject the same directory.
Hash-first short-circuiting retains the group-descendant fallback. All15 group
contracts passed again, and a focused R2 review reported no findings. Actual Windows
execution remains with the qualification owner. References:
[Rust canonicalization](https://doc.rust-lang.org/std/fs/fn.canonicalize.html),
[Python path identity](https://docs.python.org/3/library/pathlib.html#pathlib.Path.samefile).

The exact-head Broadleaf CI failures on `785d392c` were deterministic: the four
group50 test/compile workflow lines changed the current CI SHA256, while the plant
audit still pinned the earlier workflow. A failed-job retry of that source would
fail again. The follow-up adds a new CI-only integration profile and validates the
unchanged October17 integration profile as its pinned predecessor. No historical
profile, art, workflow, Rust, training or GPU source is rewritten by that follow-up.
Both plant/shipping audits and all74 composition,4 preview-rotation and3 integration
tests pass locally. The parent authorized publication of this audit-only successor;
the PC qualification candidate remains pinned to
`785d392c926f50bff2354f954c8029bc82221214`. This does not relabel the existing
binary as built from the audit successor. The replay writer reads checkout HEAD,
so the qualification checkout should retain that actual runtime source stamp.

Byte-identity receipt against that runtime commit: every other tracked path has
the same Git mode and blob. The content manifests below use path-sorted UTF-8 JSON
arrays `[path, git_mode, byte_count, content_sha256]`, compact separators and
`ensure_ascii=true`; each digest is identical for the baseline and successor.
Only the checker, new integration profile and this verification note differ.

| Unchanged scope | Files | Bytes | Content-manifest SHA256 |
| --- | ---: | ---: | --- |
| All other tracked paths | 1508 | 141250765 | `f209484fc9b464184999d64612f3227315a8fc7ec6a2949d57088fb3c4f88ff5` |
| All Rust files | 659 | 16437373 | `bb62401bea4fdd02a993801c159312f49bb97beac05f415218221fe2ef2c5ac2` |
| GPU backend and all shader files | 91 | 2412871 | `83ed410e456042943ee03fd784e1c248876378962ac2f4cb92f8c26f02d71082` |
| All asset-directory files | 270 | 86425518 | `adf4ddf81bf9ecc4020b4940cca6528add618edaa6deb78e95730af69d407833` |
| All workflow files | 9 | 44861 | `58f56de064e1ef24a54e3470ea262524e4e89847836d9367f2b1969b479a04b0` |

The unchanged workflow itself has SHA256
`815c3ad05b83457f9708aa0ab128ec05d9d18ae51c0cda43c89ddadcd8d7e808`.
The unchanged October17 predecessor integration profile retains SHA256
`ebbdbeba4c652e2b4aad897cd1d228506a80155b574a0081a328640e1da7c4a3`.
The new profile has SHA256
`72b6d10627c5b11bd5c8936e4b4f4c1ea7c870be70a17138dcf511c878ce5474`.
A focused independent R2 review found no blockers or weakened source pins.

The cloud exposes no GPU device. The hardware smoke, fifty-row recurrence/private
state checks, frozen old-policy GPU comparison, central update/export/rebind and
throughput measurement have not run. No seed was allocated. The accepted private
checkpoint download was denied by the executor's proxy with HTTP403 after the
Library helper returned a signed transfer. One explicitly consumer-local helper
retry also returned no local path. Archive, manifest and member hash verification
and readmission of that package remain unperformed. No download bypass was used.

## Ponytail review

- Existing: reuses canonical world transactions, production class arenas, replay
  codec, masked joint actions, actor cohort accumulation and accepted critic loss.
- Native: preserves the actor/value checkpoints, integer ages and founder weights;
  no checkpoint reset or replacement is introduced.
- Shrink: one detached BPTT graph, bounded captured features and narrow read-only
  trainer sizing APIs; no separate neural executor or serial fifty-session wrapper.
- Safety: strict history and immutable seed reservations remain necessary; all
  failures remain training outcomes. Shared-world correlation stays disclosed.

Final decision: source/tests/review are published for integration. Keep the PR
draft until coordinated real-adapter qualification, accepted source validation and
the required CI checks are reviewed. Operator commands, resource scopes and safe
failure handling are in `docs/training/group50-2026-10-19.md`.

## Batch capture and cohort replay successor

Baseline is instrumented `4cca46b49e1f8a295811b18a1fe6b9ac68496366`, kept separate
from PR111's `52559f55` head. The PC owner's read-only evidence resolves the prior
timeout: the generation1 base manifest appeared at61.218521 seconds, with50 tick0
organisms, no non-null GPU brains/runtime and no assets. Its referenced base file
was50,513,021 bytes. The subsequent160 assets include50 each of active memory,
topology sidecars and exact cognition for those same50 IDs at tick0, continuing
through118.190058 seconds. The120.228-second attempt therefore demonstrated
ongoing initial checkpoint capture. It did not establish that capture returned,
exact publication/rebind finished, or any training tick ran. Its seed remains
consumed; no retry or new GPU run was performed here.

The staged training path previously performed one mutable-state wait before and
after each awake organism. Nominal50x16 collection requires17 captured decisions
per life, hence1700 state captures and850 separately synchronized replay records.
The new capture API preflights the complete requested capability inventory, copies
private mutable/uncached structural ranges into bounded shared staging packets,
maps each packet once and restores request order. The existing single-row capture
remains the real-GPU differential reference. Neither path advances neural state on
the CPU. The caller supplies canonical frame tick labels; existing staged receipt
checks retain action, generation, private context and old-policy ownership.

Group replay now writes one pending bundle with independent compressed/authenticated
records and an indexed, hash-bound footer. One successful data sync precedes exclusive
durable publication after collection completes. Unix creation syncs directory
ancestry, and publication syncs the containing directory; Windows follows the
project's write-through rename protocol without replacing existing content.
Recovery verifies the index and every record; pending bytes never become a native
checkpoint or free a consumed seed. Partial encoding poisons the pending cohort.
Ordinary serial writers retain their record-file protocol. Progress distinguishes
staged rows from the sealed cohort; actual readback rows/submissions/bytes and cohort
sync/commit counts are recorded without a speedup claim.

Focused independent R2 review identified and closed module visibility and ancestor
directory durability issues; the final delta review found no remaining blockers.
Real GPU comparison, N2048 update/export/rebind and Windows execution remain
unqualified in this GPU-free cloud. CPU fixtures do not certify their behavior.

Focused validation passed:9 cohort crash/integrity tests,15 group/native-checkpoint
contracts,3 replay codec/I/O tests and1 packet-order/bounds test. The existing CPU
timing benchmark remains ignored. The fifty-row real-GPU single-vs-batch fixture
and the complete N2048 group50 handoff fixture both compile with explicit GPU-test
features; neither executed. Strict Clippy passed for the game library/group binary
and for the GPU library/differential fixture. Formatting, diff lint and docs77/77
passed. That first publication changed no workflows and skipped broad CI/review
jobs as requested.

## Integrated source and focused CI follow-up

The successor now integrates PR111's exact `52559f55` dependency without modifying
its branch. Current main was fetched explicitly and remained `bda3b65b`; it is
already an ancestor. The unchanged pinned `ci.yml` and historical art profiles are
verified against the PR111 dependency. The combined plant audit, shipping art audit,
three integration tests, boundaries and77 documentation checks pass after integration.

The new unpinned `group50-training.yml` registers the28 focused CPU contracts,
both GPU fixture compilations and scoped strict source checks. Its dedicated
`group50-ci/**` push path avoids duplicate broad jobs; it also discovers relevant
main PRs/pushes after integration. This job cannot qualify GPU behavior.
The native hardware fixture now accepts an explicit seed admission selection,
defaulting to strict and rejecting unknown values before execution. It asserts
exact policy/freshness provenance in the cohort and all fifty episodes. This
changes no seed admission rules or immutable history.

The existing PC task remains the sole hardware owner. It must reconcile the
running `4cca46b4` build before building the integrated source, retain source/cache
paths, and use observed phase progress plus resource headroom instead of repeating
the old120-second timeout. The consumed seed202610174119 and accepted policy7400
packages/Adam remain immutable. No PC command or new task was issued here.

## Qualified predecessor and current-main integration

The coordinated Windows RTX3050/Vulkan receipt qualifies source
`617063b85f45cdcd657744fe3e94902d11b6e23c`, tree
`344f3f8e68cae2f64024703ee2d565856ad810b8`. Its50-by16 run exited successfully
after180.721 seconds, with28 focused Windows CPU contracts, native input admissions,
output readmission,850 replay records,800 valid learning decisions and50 private
replay comparisons passing. Actor/value Adam advanced by2/100 updates; export and
next-cohort rebind passed. There were1700 snapshots in34 readback submissions,
one replay data sync and one durable commit. Protected-state checks matched.
These observations establish neither a comparative speedup nor learning benefit.
The separate batch-versus-single GPU fixture was not run for that receipt.

Current main `37de3cc299728d1914e575e5dd6ebfa2ddea1e56` includes PR113's changed
founder/decoder semantics, which the predecessor receipt predates. PR111 now
integrates that main at `90a8b86a473e826a6a344ad7d6b0553cf75b1fc3`; PR114 merges
this updated parent. No conflict resolution or new neural implementation was
needed. The three shared files retain group admission and memory planning alongside
the common newborn floor, signed decoder/Adam metadata, exact private-foundation
replacement and Play/Grab vocabulary/replay identity. All22 other main-change
blobs are retained exactly. The capture, cohort persistence and staged-tick source
from the qualified predecessor is byte-retained outside the vocabulary merge.

PR112's head is not an ancestor. All five paths changed by that branch remain
byte-identical to main or absent; its unapproved plasticity implementation is not
part of this stack. The pinned CI workflow and historical art profiles are retained.
PR114 is the complete integration candidate for main; PR111 remains draft while
the combined source is qualified. An independent merge of the predecessor is not
covered by the combined candidate's future hardware receipt.

The old receipt must not be relabelled as qualification of the integrated source.
The existing PC owner must run the fifty-row batch-versus-single readback fixture
and bounded native50-by16 handoff contract against the final integrated source,
with measured resource headroom and separate qualification outputs. Retention and
training-benefit experiments remain optional follow-ups, not implementation merge
requirements. Accepted checkpoint/optimizer files are not replaced by qualification
outputs. Only exact-head checks and the matching hardware contracts can close the
integration gate.

Focused independent R2 integration review at `858a48a7` found no blocking source
issues. It independently confirmed the22 main blobs,13 retained batch-successor
crate blobs, exclusion of all PR112 paths, signed decoder/private-weight/integer-age
contracts and clean merge. Formatting, architecture boundaries, documentation77/77,
both art audits and three historical-pin integration tests pass on the combined
tree. The final source's CPU/compile/CI results are recorded on PR114; retargeting
the PR alone is not evidence that any check ran. The integrated source still needs
both explicit GPU contracts before the hardware gate can close.
