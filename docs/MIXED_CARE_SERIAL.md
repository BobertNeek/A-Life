# Configurable mixed-care serial scheduler

`scripts/run_mixed_care_serial.py` runs explicit ordered lesson steps through the
existing native `--resume-cycle PREVIOUS NEXT N --seed S --lesson LESSON` command.
Supported lessons are `eat_held_food`, `eat_reachable_food`, and `feeding`.
Repeating a lesson in later steps is allowed. Each step specifies its own episode
count and decision window (1–128); the total is bounded to 100,000 episodes.
There is no automatic promotion, assessment gate, success target or fixed mixture.

The native trainer restores the entire actor and value Adam histories at every
sealed boundary, including across lesson switches. The scheduler requires the
existing `inspect_archived_success --validate-checkpoint` CPU admission before
dispatch and seal acceptance; the trainer's objective, phenotype, sensor,
checkpoint, exact restore and optimizer admission remain unchanged. This small
runner accepts sealed objective2 cycles only. It exposes no objective transition,
adaptation, reset, rehearsal, temperature or correction operation. An already
corrected source must pass native admission; its actual checkpoint age is used
without crediting historical critic-only work as new PPO updates.

Fresh episodes start new worlds and personal lifetimes. Only the offline trained
parameters and optimizer histories continue. This host orchestration does not
prove gameplay competence or satisfy founder promotion requirements. The ordinary
organism path remains responsible for behavior (AOA-FOUND-004, AOA-FOUND-006 and
AOA-EVO-003); no runtime teaching policy is added.

## Explicit history/import contract

Every plan requires a complete operator-reconciled `mixed-care-seed-history`
document. This is an explicit input contract, **not an automatic migration**:

```json
{
  "schema": 1,
  "kind": "mixed-care-seed-history",
  "complete": true,
  "source_sha256": {
    "cycle.json": "<actual lowercase SHA-256>",
    "trained.alife-foundation": "<actual lowercase SHA-256>",
    "actor-checkpoint.json": "<actual lowercase SHA-256>",
    "value-checkpoint.json": "<actual lowercase SHA-256>"
  },
  "reserved_seed_ranges": [["<first reserved seed>", "<last reserved seed>"]],
  "consumed_seed_ranges": [["<first consumed seed>", "<last consumed seed>"]],
  "assessment_seeds": ["<historical assessment seed>"],
  "imports": [],
  "evidence": "<original allocation, command and receipt identities; completeness basis>"
}
```

Replace seed placeholders with JSON integers; intervals are inclusive. The
consumed ranges must include the source's own seed. Include every lesson and
launcher, all accepted, uncredited, failed, partial and uncertain attempts, and
whole unused allocations. Reserved and consumed ranges are both exclusions;
neither can be inferred from policy age, accepted episode count or Adam age.
`complete: true` is an operator attestation, not independent proof.

If the lane contains `.grab-food-seed-reservations.json`, `imports` must include
`{"path": "<absolute original ledger path>", "sha256": "<exact-byte SHA-256>"}`.
Other explicitly identified Grab ledgers may also be included. Import validates
the Grab-specific kind and allocation provenance and excludes every training
interval and assessment seed, including allocation receipts omitted from summary
intervals. It reads and pins those bytes without modifying the Grab ledger.
A Grab ledger alone is not universal history; the complete mixed-care attestation
must cover other/custom/Eat launchers as well. Changed import bytes stop resume.

**Real historical ledger data was unavailable for this implementation.** Tests
use synthetic histories only. They do not reconcile policy7400's real history.
The accepted policy7400 package, original receipts, 663 unused reserved seeds and
eight older uncredited consumed seeds remain untouched. Real use remains blocked
on an evidence-backed complete history supplied by its owner.

## Executable example schedule (not an approved training dose)

The checked-in [example schedule](examples/mixed-care-schedule.json) requests one
16-decision episode of each lesson in order. These numbers only demonstrate the
configuration format; neither this example nor a 24+8 mixture or 80/90 promotion
gate is an approved dose or policy.

For an operator with an already admitted source, built native binaries and real
reconciled history, this command creates a host-only plan. Set each environment
variable to its actual path, fresh allocation or timezone-qualified cutoff:

```sh
python scripts/run_mixed_care_serial.py \
  --binary "$CARE_TRAINER" --checkpoint-inspector "$CHECKPOINT_INSPECTOR" \
  --source "$SEALED_SOURCE" --seed-history "$RECONCILED_HISTORY" \
  --schedule docs/examples/mixed-care-schedule.json \
  --output target/founder-training/mixed-care-example \
  --seed "$FIRST_FRESH_SEED" --seed-count 6 --cutoff "$CUTOFF"
```

Planning launches no executable and takes no lock or reservation. It checks the
schedule, history and proposed allocation; `plan.json` pins the source handoff
files, optional handoff sidecars, binary and inspector bytes, history bytes and
Git commit. A plan does not guarantee that seeds remain available. The six-seed
reservation in this example includes three optional interruption spares. The
first plan intentionally pins its cutoff and resource/command bounds.

On a clean reviewed checkout, an authorized operator can execute the saved plan:

```sh
python scripts/run_mixed_care_serial.py \
  --output target/founder-training/mixed-care-example --run --start
```

For a new output, adding `--run` to the full planning command also reserves and
executes it. `--start` requires a directory containing only its saved `plan.json`.
After an interruption, use `--run --resume` instead. `--resume` requires an existing
reservation; it never guesses that a missing ledger is safe.
An explicit new `--cutoff "$CUTOFF"` may extend a resumed invocation's time budget
within the next 24 hours, without changing the saved lesson/seed/source plan.

Defaults retain 8 GiB free disk, 2 GiB available memory, 30 seconds of Windows
interactive idle time and a 1,200-second per-command limit. `STOP` in the output
directory stops the owned child. Resumption requires removing the deliberate STOP
and an unexpired cutoff. No startup or final evaluations run automatically.

## Ownership, durable acceptance and coexistence

Execution is restricted to the authoritative repository root and output directories
directly under `target/founder-training`. All runs use its existing exclusive
`.eat-held-food-owner.lock`. The scheduler never removes another owner's lock.
If a hard crash leaves a lock, an operator must verify the owner and every child
are stopped before manually reconciling it; there is no automatic stale takeover.
Trainer and native-inspector children share the same ownership tracking;
cleanup uncertainty for either retains the lock. Resource, STOP, source/binary changes,
command failures and persistence failures stop further dispatch.
The guard runs before every inspector dispatch, including accepted-chain replay
and each current/predecessor check. An inspector's budget is at most 90 seconds
and never exceeds the remaining absolute schedule cutoff.

Under that lock, `.mixed-care-seed-reservations.json` reserves the entire seed
interval before any trainer. Every attempt is persisted as pending before launch.
Only validated seals become accepted, and the next lesson always uses the latest
accepted predecessor. Resume verifies pinned accepted receipts/checkpoints,
revalidates pending seals and derives counts from accepted attempts. A pending
attempt without a seal is abandoned; its seed/output are never retried. An invalid
seal stops reconciliation. A late seal in an abandoned attempt also fails closed.
No reservation is refunded after interruption, failure or zero accepted dose.

New allocations must continue from the latest accepted predecessor across the
lane. Pending attempts must first be reconciled; older allocations cannot resume
after a newer one exists. Exhausted spares require a new explicit allocation and
source-bound history, preserving all previous exclusions. Summary publication
failure cannot duplicate accepted dose: `schedule.json` is derived from the
authoritative ledger, never incrementally trusted on restart.

The existing Grab and held-food night runners check for this mixed ledger at
preflight and again under the shared lock, and refuse to run once it exists.
They do not silently write a universal ledger. Older Care/VisionLanguage scripts,
old checkouts and custom/direct native invocations are outside this protocol;
do not run them in an adopted lane. Any such work requires explicit reconciliation
from original evidence before returning. Copying a lane without its ledger,
deleting/damaging ledger data or out-of-band edits cannot be recovered by guessing.

The ledger reuses the tested bounded JSON writer: file flush/fsync, atomic replace
and POSIX parent-directory fsync. Windows flushes the file and replaces atomically
without a directory fsync. Host fixtures do not certify Windows power-loss behavior.
Documents are capped at 64 MiB, lists at 100,000 entries and seeds at uint64 bounds.

## Count semantics and host verification

`schedule.json` preserves separate per-lesson accepted episode, requested decision,
captured replay row, trained replay row, bootstrap row, completed epoch and actor/
value optimizer-update totals. A living N-decision cycle captures N+1 rows and
trains N; the bootstrap is not additional trained dose. Exact nourishing held/
reachable consumption and its trained-row subset use the native
`held_food_consumption_events` and `trained_held_food_consumption_events` counters.
Feeding reports only generic captured `consumed_events`, which may include its
bootstrap row and cannot exceed the number of captured rows. It is not exact
nourishing success and has no inferred trained-meal
count. No combined success rate or automatic promotion is calculated.

Run the stdlib fixtures and existing affected checks:

The existing CI `test*food_night.py` discovery includes this scheduler suite;
the source-pinned workflow and art provenance remain unchanged.

```sh
python -S -m unittest discover -s scripts -p 'test_mixed_care_food_night.py' -v
python -S -m unittest discover -s scripts -p 'test_summarize_ppo_diagnostics.py' -v
python -S -m unittest discover -s scripts -p 'test*food_night.py' -v
python -S -m unittest discover -s scripts -p 'test_grab_food_seed_reservations.py' -v
bash scripts/check.sh --quick
```

These use fake trainers and native-inspector reports. Cross-lesson chaining,
interruptions, commit/summary failure boundaries, collisions, tampering and count
semantics are host evidence only. No real training, evaluation, neural execution,
GPU or desktop work was performed, and no current-runtime mixed-care success is
claimed.
