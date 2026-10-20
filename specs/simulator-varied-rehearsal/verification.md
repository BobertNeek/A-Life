# Verification receipt — 2026-10-20

Mode: 2 (feature across offline training and exact runtime restore).
Review class: R2, separate read-only agent. Its actionable implementation findings
were fixed: realistic perturbation geometry, residual-only critic continuation,
ordinary sleep/death boundaries, checkpoint NoProgress handling, delayed terminal
discount, mandatory capture accounting, archive attachment ordering, next-policy
optimizer rebind and the hardware test's sampling counter.

Initial source: fetched main dd79fb026da86c7b4e2b1a6b60176a365a38507c.
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
- Format and whitespace checks passed.

## Evidence still required

The app-level targeted regressions, non-training runtime compile and post-rebase
checks are being completed before the final implementation commit.

Hardware exact-restore continuity, complete source-to-update-to-continuation runs,
and matched discovery/retention comparisons remain **Unknown**. The cloud has no
supported hardware GPU, and the accepted policy7400 private Library checkpoint
has no verified file ID. The ignored hardware continuity regression is executable
only in a separately coordinated hardware session. It compares restored private
context, next sampled policy probabilities, actual canonical physiology and the
next replay sequence using the same sampling counter.

The source implementation and its CPU tests do not establish learning gains,
throughput gains, founder promotion or historical unseen-scene generalization.
The PR remains draft while that explicit end-to-end acceptance evidence is pending.

Risk/rollback: the ordinary restore entry point preserves its prior options; the
new mode is selected only by the training-only constructor. Revert the rehearsal
commits to remove the experiment; source checkpoints are read-only.
