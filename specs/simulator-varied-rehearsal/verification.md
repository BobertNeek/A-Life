# Verification receipt — 2026-10-20

Mode: 2 (feature across offline training and exact runtime restore).
Review class: R2, separate read-only agent. Its actionable implementation findings
were fixed: realistic perturbation geometry, residual-only critic continuation,
ordinary sleep/death boundaries, checkpoint NoProgress handling, delayed terminal
discount, mandatory capture accounting, archive attachment ordering, next-policy
optimizer rebind and the hardware test's sampling counter.

Initial source: fetched main dd79fb026da86c7b4e2b1a6b60176a365a38507c.
Freshness main bda3b65be5fa6f01865b850640adf2e8d01f4990 was merged at a safe
checkpoint; protected freshness paths have no diff against that main.
Protected freshness paths were not edited. No accepted training checkpoint,
source artifact, actor/value weights or Adam histories were modified by validation.
No PC access, GPU workload, paid provisioning or learned world model was used.

## Verified

- `cargo check -p alife_game_app --features foundation-training --bin rehearse_n2048_care`
  passed after implementing the coordinated restore constructor.
- `cargo test -p alife_training --lib`: 34 passed, including lawful real-world
  variations, near-grab geometry, rejected placements, private biology preservation,
  actual metabolic advancement, diverse retention, stale-policy rejection, seed
  partitioning, sequence masks and delayed-terminal timing.
- `bash scripts/check_core_boundaries.sh --static`: passed.
- `bash scripts/docs_check.sh`: 77/77 assertions passed.
- `cargo test -p alife_game_app --features foundation-training --lib simulator_rehearsal`:
  five passed, including exact source-byte binding, continuation mismatches,
  NoProgress accounting, shared capture ceilings and fail-before-device admission.
  Repeated successfully after merging current main.
- `cargo check -p alife_game_app --features gpu-runtime --lib`: passed with
  training mode disabled.
- `cargo check -p alife_game_app --features foundation-training,gpu-tests --tests`:
  passed; hardware tests were compiled, not run. One existing unused test-helper
  warning was reported outside the changed files.
- `cargo build -p alife_game_app --features foundation-training --bin rehearse_n2048_care`:
  passed. The resulting CLI passed usage, missing-source, invalid-budget and
  comparison-admission smoke checks with no output/device work.
- `cargo clippy --locked -p alife_game_app --features foundation-training --all-targets -- -D warnings`:
  passed after an equivalent conditional-style fix.
- Format and whitespace checks passed. Core/docs checks repeated successfully
  after merging current main.

## Evidence still required

Hardware exact-restore continuity, complete source-to-update-to-continuation runs,
and matched discovery/retention comparisons remain **Unknown**. The cloud has no
supported hardware GPU. A private policy7400 Library identity exists and the PC
owner verified its upload/manifest. CPU/group cloud consumers were blocked from
reading its bytes by proxy CONNECT 403, so consumer byte validation remains
blocked. The private identity and signed URL are not published here. The ignored hardware continuity regression is executable
only in a separately coordinated hardware session. It compares restored private
context, next sampled policy probabilities, actual canonical physiology and the
next replay sequence using the same sampling counter.

The source implementation and its CPU tests do not establish learning gains,
throughput gains, founder promotion or historical unseen-scene generalization.
The PR remains draft and has not been merged while that explicit end-to-end
acceptance evidence is pending. This is an evidence/resource blocker, not an
authorization denial. The final-head remote CI results must also be checked
before any future merge. Automated Jules reviews reported unknown/incomplete
and were not counted as successful reviews.

Risk/rollback: the ordinary restore entry point preserves its prior options; the
new mode is selected only by the training-only constructor. Revert the rehearsal
commits to remove the experiment; source checkpoints are read-only.

## Short qualification follow-up

The parent owns hardware scheduling after group50's bounded test. The explicit
`--qualify` CLI profile admits at most 48 decisions per invocation and cannot be
combined with `--compare`. Its 15-second collection admission limit is soft;
`docs/SIMULATOR_REHEARSAL_QUALIFICATION.md` requires serial execution, separate
prebuilds and an operator-enforced hard stop of at most 60 seconds per execution
(or the PC owner's smaller allowance). No hardware execution or Library transfer
retry was performed here. Small qualification does not establish learning benefit.

Qualification follow-up checks: three `rehearse_n2048_care` binary tests passed;
strict foundation-training all-target Clippy and the CLI build passed. Bounded-mode
missing-source admission and rejection of combined `--qualify --compare` passed
as executable smoke checks with no output/device work. Docs remained 77/77.

## Separate source-ancestry follow-on

The bounded PC candidate remains b7a67384075223be24e5375a7fd3bfe96995042f.
The `feat/rehearsal-source-ancestry` branch is a separate follow-on: schema 2
binds the real behavioral receipt, exact parent file/manifest hashes, source
policy/founder identity, objective versions, GPU execution domain and unchanged
historical freshness. It requires both optimizer ages to advance by the single
actual update. Publication follows the complete receipt. A device-free adapter
returns the full founder/actor/value and verified provenance; it does not reserve
seeds, resolve missing ancestors, restore a life or extend ordinary care resume.

The existing independent R2 reviewer checked only this diff against b7a67384 and
found no blocking correctness or provenance issue. No additional external review
job or hardware workload was launched. Eight simulator-rehearsal library tests
passed, including the two-link source chain, full checkpoint preservation,
unsealed/altered receipt rejection and exact optimizer-age checks. Strict
foundation-training all-target Clippy, formatting, whitespace, docs 77/77 and
static core boundaries passed. A test-only f32/f64 JSON comparison was corrected
to compare the same typed checkpoint encoding before the successful test run.

`docs/SIMULATOR_REHEARSAL_INTEROP.md` records the gameplay asset mechanism and
the remaining shared `FoundationTrainingHandoffV1`/ordinary-care admission work.
The CPU owner owns the common type; CPU/group adapters must use the real receipt
name and preserve both Adam histories, execution provenance and source ancestry.
Schema 1 is not silently upgraded. This follow-on adds no hardware, measured
learning-benefit or cross-backend continuation evidence.
