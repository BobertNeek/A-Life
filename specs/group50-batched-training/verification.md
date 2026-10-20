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
