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
combined with `--compare`. The initial agent-selected 15-second collection and
60-second process smoke bounds failed to admit a complete accepted-source attempt.
The revised explicit qualification profile uses 240 seconds from entry for
collection and 360 seconds overall, preserving all 48 decisions and single updates.
See the qualification handoff for measured capture evidence, timestamp-derived
planning rationale and limitations. No hardware retry or Library transfer was
performed here. Small qualification does not establish learning benefit.

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

## Initial runtime construction repair — 2026-10-20

The PC's exact continuity fixture on 491defda failed at initial `fresh_runtime`
construction with `Core(PhenotypeCompile)`, before any tick or restore assertion.
The reported run exited 101 in 8.1445 seconds, zero passed and one failed. Both
native policy7400 admissions passed and all 1,573 protected hashes matched. A/B
never ran. Hosted exact-tree workspace checks had 1,958 passed, zero failed and
22 ignored; those checks did not execute this ignored hardware fixture.

A device-free regression using founder77777 and scene99001 reproduced the same
error in the existing production resident compiler after scene preparation. The
shared bootstrap factory had compiled a raw scaffold, whose food-attraction
constant differs from diploid founder expression by one floating-point ULP. The
resulting innate decoder gain changes its strict action-decoder digest. The
repair constructs canonical New Game with a provisional token, then compiles the
actual expressed genome using the existing normalized construction development.
No decoder/phenotype validation is relaxed, and no admitted asset is transformed.
New procedural bootstrap weights intentionally use the expressed conception-seed
genome rather than the old raw scaffold seed. Historical bootstraps are not
reproducible by treating this repair as their original generator.

The accepted-source runner reads its exact inspected asset and original founder
seed; it never calls the procedural bootstrap factory. Native source admission
already recompiles that expressed founder and validates the actor binding. New
device-free coverage also exercises the runner's actual prepared-scene route
with an inspected synthetic native package and verifies unchanged source bytes.
That fixture is not evidence about the private policy7400 bytes or optimizer ages.
Hardware restore and accepted-source A/B remain pending parent coordination.

The compiled core/world diagnostic measured raw food-attraction bits
`0x3f147ae1` and expressed bits `0x3f147ae2` for the exact failing seeds. The
negative regression also confirms the decoder digest differs while layout and
persistent address-map digests remain equal.

Repair validation: the broad foundation-training application run had 473 passed,
one stale raw-scaffold graph-test setup failure and five ignored. That isolated
test setup now uses the existing canonical source compiler; its unchanged graph
preservation/tamper assertions passed on rerun. Final focused checks passed one
graph test, ten rehearsal tests, three bootstrap tests, 36 training-library tests
and three CLI tests. Strict all-target foundation-training Clippy, full core
boundaries, docs (77/77), formatting, whitespace, protected-path and workflow
YAML/command checks passed. R2 source, graph-test and workflow reviews found no
blocking findings. The workflow now runs the bootstrap and graph regressions.

GPU-enabled test-source checking and both CLI builds were pending at this source
freeze checkpoint; their terminal evidence belongs in PR109 without a new source pin.
No third local workspace aggregate, PC retry or GPU execution was started.
New-head hosted CI and coordinated mechanical qualification remain pending.

## Fixture output ownership repair — 2026-10-20

The PC's single 910078f8 continuity execution completed the setup, restore,
frame/logit/probability, physiology, replay and canonical-world assertions, then
failed at `remove_dir_all` with Windows sharing violation 32. It exited 101 after
28.663 seconds: overall zero passed and one failed. Both native source admissions
passed objective2/policy7400 with full Adam 16684/19800, and the PC reported all
1,635 immutable protected hashes matching. A/B and diagnostic seeds never ran.

The fixture attempted removal while both runtime-owned `LineageLibrary` SQLite
connections were live. The asset store is path-only. No production ownership
defect is established. The test now uses existing persistence shutdown polling
to reach idle, fails on persistence errors/timeouts, and explicitly consumes and
drops both runtimes, store and device before strict deletion. All preceding
assertions and five neural decisions remain. One entry deadline covers progress
and teardown within the existing 60-second window; no budget is extended.

Device-free coverage uses two real archive connections and a drop-order witness
that catches delete-before-release even on POSIX. A second case requires the
original cleanup I/O failure to propagate after resource release. Hardware lock
ownership was not measured from the cloud; full Windows qualification remains
pending the parent's existing PC coordination. No unchanged retry, new hardware
task, accepted-state edit, Library retry or production runtime change was made.

Cleanup repair checks: all 12 rehearsal tests passed, including both new lifecycle
regressions. Strict foundation-training all-target Clippy and GPU-enabled test
source checking passed; the latter reports the same pre-existing unused helper
warning outside the changed files. Core boundaries, docs (77/77), formatting and
protected-path checks passed. Independent R2 source/lifecycle review found no
blocking findings, including the assertion that persistence drain preserves world
time. The exact ignored fixture's executable compilation was pending at this
source freeze checkpoint; its terminal evidence belongs in PR109 without another
source change. Compilation does not establish Windows hardware qualification.

## Qualification budget diagnosis — 2026-10-20

Exact e21749e8 Windows continuity passed one test in 28.653 seconds. Accepted-source
A then failed after 28.706 seconds before a complete attempt, with no update or
continuation; B never ran. Its capture ledger measured 9.6569172 seconds for one
exact capture. File timestamps suggest a roughly 120-second setup/capture estimate
for four attempts; this estimate does not measure decision or optimizer cost.
The parent authorized explicit 240-second collection / 360-second total planning
allowances without changing attempts, holdouts, masks, source identity or Adam.

New diagnostics persist stage timing and actual decision counts through failure,
including zero-row attempt receipts. Completion finalization follows persistence
drain and release of GPU/archive owners; a failed stage or expired allowance,
including publication overrun, removes the continuation marker. R2 also identified
an abrupt-stop edge between marker write and final timing: candidate validation
now precedes marker publication, and qualification-source admission additionally
requires completed in-budget timing with matching actual decision counts. A
device-free package-format regression exercises the marker-left-but-unfinished
case; these synthetic format tests do not claim neural execution. Default mode and
comparison ceilings remain unchanged. Corrected-pin hardware A/B remains pending;
no learned-benefit or unseen-generalization claim follows from this repair.

Final repair source checks: the full foundation-training application library
passed 486 tests with zero failures and five existing ignored timing tests,
including all 22 simulator rehearsal tests and the real package-format
interruption regressions. Eight lawful-world/policy/sequence tests, strict
all-target foundation-training Clippy, boundaries, docs77/77, formatting and
protected-path checks passed. Independent R2 cleared the interruption finding
and found no remaining source blockers. Executable compilation and new-head
hardware/CI evidence are reported separately in PR109; no PC run or new seed was
launched from this source environment.
