# Group-50 verification

- Mode: 2, implementation with a focused contract and independent review.
- Review class actual: R2, read-only implementation and follow-up review.
- Source implementation: `8813166353a450a5738fc0009655a8995e704a66`.
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
paid provisioning was used. Final implementation source was unchanged during these
checks; the subsequent commit only records verification/operator documentation.

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

## Review and limits

R2 review identified and verified fixes for native source/history exclusions,
post-quota death handling, legacy coordinate grounding, conservative device scratch
budgeting, complete original-package validation, GPU-test feature gating and the
ordering of initial trainer memory admission. A final delta review of read-only
source validation, initial per-organism replay comparisons, native output
readmission and handoff documentation reported no findings. Open P0/P1/P2/P3
findings: none. Artifact lint: pass; no unresolved clarification markers.

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
