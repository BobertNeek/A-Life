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

## Current-main integration checkpoint — 2026-10-20

Integrated main 96a8375c813a12c9ec85e66f34d65bad24285931 (main tree
cc7fb3dd6e6a6b4ff3b7077d63ac3668243a933d) onto the standalone PR109 branch.
The sole text conflict was the Cargo binary list; both rehearsal and group50
registrations are retained. Protected freshness files and CPU/Argon work have
no changes against this main. PR112 and the blocked Library transfer were not
retried. No accepted policy7400 bytes or optimizer state were changed.

Selected only the own-package continuation changes from preserved ancestry
branch 691ba2c84524ccaf110e97875d19afa016791847: schema 2 binds the actual
behavioral receipt and exact parent/manifest hashes, requires both one-step Adam
age increments, and publishes its complete marker last. The shared-admission API
and its separate route work were not imported. Unmeasured holdout/untrainable
rewards are now explicitly null in `training_reward_sum`.

At the source publication checkpoint:

- Foundation-training all-target check passed.
- Training library: 36 tests passed.
- Simulator rehearsal library: seven tests passed, including two-link ancestry,
  altered/unsealed receipt rejection and exact source-byte binding.
- Standalone CLI: three tests passed.
- Foundation-training/gpu-tests test sources compiled without GPU execution;
  one existing unused helper warning remains outside the changed files.
- Strict foundation-training all-target Clippy passed.
- Both `rehearse_n2048_care` and `inspect_archived_success` built successfully.
- R2 integration review and the focused CI-workflow review found no blocking
  implementation issues. Workflow YAML/path parity and compile-only GPU commands
  passed local validation. The new job runs device-free rehearsal contracts and
  compiles the ignored GPU fixture; remote execution remains pending.

The complete `bash scripts/check.sh --full` aggregate was running at this first
publication; its final result and final-head CI are tracked in PR109. Parent-owned
hardware restore continuity and accepted source-to-A-to-B continuation are still
pending. Required ages are 16684/19800, 16685/19801 and 16686/19802 with actual
updates only and unchanged source hashes. Qualification must report whether any
perturbed success/failure retries occur and honestly record their absence.

These mechanical checks, R2 and final-head CI define remaining standalone merge
acceptance. Matched learning/retention measurements gate benefit claims rather
than delivery of this opt-in experiment. CPU/common admission and ordinary care
resume are separate work. The parent coordinates the existing PC owner and owns
eventual merge/post-merge checks; no duplicate PC or external review job was
launched here.
