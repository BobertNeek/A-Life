# Disposable scenario batch handoff (opt-in)

**Fresh-founder dispatch is blocked by default.** The existing native `--fixture`
constructor performs offline training, so it does not satisfy this task's
assessment-only requirement. No training opt-in or retained-source workaround is
provided. A verified construction-only native backend is still required before
this can become an unattended end-to-end assessment loop.

`scripts/care_scenario_batches.py` is a durable local handoff protocol for an
**already running, authorized agent**. It does not invoke Codex, a model API,
an agent service, arbitrary shell commands, or training. The separate fixed
`care_scenario_dispatch.py` wrapper handles disposable native dispatch. Automatic agent
invocation is not implemented or proven. This is not a scheduler, unattended
training runner, or a learning/capability claim.

The base seam invokes only completed-archive validation. The optional fixed
dispatcher is designed to use a construction-only fresh founder and run reserved
discovery assessments with pinned native binaries. Its initializer currently
fails before native execution because that backend is unavailable. No credentials are read or
copied. No retained source input is accepted by the dispatcher. No retained
creature is selected, updated or promoted; no pool, holdout or reference trial
or training round is dispatched.

## Host preparation

Use a trusted local POSIX filesystem supporting exclusive directory/file creation
and file/directory `fsync`. Ledger records have a symmetric 8 MiB read/write bound. Windows and distributed/cloud-sync filesystem crash
semantics are not qualified. Initialize a **new** root outside all creature,
checkpoint, training and archive directories. Only this root is written.

The host supplies this bounded plan (replace the evaluator hash):

```json
{
  "schema": 1,
  "scope": "disposable_assessment_only",
  "batches": 2,
  "family_count": 2,
  "levels": 2,
  "initial_difficulty": 0,
  "decision_budget": 64,
  "slots": 1,
  "trials_per_family": 3,
  "pool_per_family": 1,
  "holdouts_per_family": 1,
  "seed_start": 1001,
  "sampler_seed": 42,
  "excluded_seeds": [1000],
  "native_evaluator_sha256": "REPLACE_WITH_SHA256_OF_TRUSTED_NATIVE_EVALUATOR"
}
```

The example reserves scene seeds 1001..1020. Plans are capped at 64 discovery
trials per batch (at most 8,192 evaluated decisions at the maximum dose), keeping
the current fixed-fixture archive/control inventory well below the shared
10,000-file inventory bound. Larger aggregate plans reject before any claim. A contiguous range is reserved for
**all planned batches at initialization**: per batch and family, pool seeds first,
holdout seeds next, discovery seeds last. Exclusion overlap and overflow reject.
All scene seeds are distinct. Sampler randomness uses its separate seed.
Pool/holdout seeds are reserved but never dispatched by this seam.

The host must supply all relevant founder/prior-use exclusions and reserve this
range against other runs. The ledger prevents reuse within its plan, but is not
a global seed registry. Never initialize another root with the same range, or
recycle reservations from a failed/abandoned plan.

Family IDs are generated as `family_00`, `family_01`, etc. Assigned difficulty is
family index modulo `levels`. Proposals cannot change this assignment or the
number of levels. Actual empirical hardness still comes from native outcomes;
new maps sharing a tier are not guaranteed to have identical difficulty.

## Controller/designer boundary and single-task loop contract

The outer trusted controller and the Codex recipe designer are separate roles
with separate context. Only `prepare` stdout goes to the designer. Private
proposal handoffs, seeds, paths, source/evaluator identities, archive rows and
host tool outputs must **never** be forwarded to the designer. The outer task
owns native dispatch and archive admission. It may coordinate the loop in its
existing authorized task, but this protocol does not itself create an isolated
designer invocation. No autonomous per-batch invocation/dispatch qualification
is claimed until the actual controller integration has been tested.

1. The outer trusted controller initializes once:
   `python3 scripts/care_scenario_batches.py --root NEW_ROOT init --plan PLAN.json`
2. Call `prepare`. Its stdout is the **only model-facing/public request**:
   `python3 scripts/care_scenario_batches.py --root ROOT prepare`
   This contains fixed recipe rules, canonical IDs, assigned tiers and, after
   the first batch, only verified trial/success counts and before/after difficulty.
   No concrete seeds, paths, source/checkpoint hashes, credentials, archive rows
   or untrusted free-form text are included. The fixed food-variety formula is
   a scenario rule, not an actual seed.
3. The designer authors one strict JSON recipe from the public request only.
   The outer trusted controller submits that proposal once:
   `python3 scripts/care_scenario_batches.py --root ROOT propose --recipe RECIPE.json`
   Proposal and compiled bank are sealed, and stdout returns only metadata.
   The private bank and discovery tasks stay inside the ledger until the outer
   controller claims dispatch.
4. Initialize the fixed dispatcher once, after ledger initialization and before
   any dispatch. All paths are host/controller inputs, never designer inputs:
   `python3 scripts/care_scenario_dispatch.py --run-root NEW_RUN_ROOT init --ledger-root ROOT --care-binary CARE --native-evaluator NATIVE --founder-seed FOUNDER_SEED`
   This currently fails closed before any native command or founder claim.
   There is no supported construction-only native source initializer. The
   mocked contract requires an exclusive founder claim before initialization,
   with source inventory, executable pins and ledger-plan binding sealed. It
   cannot accept a retained source or opt in to training.
5. Future supported-constructor contract, currently exercised only with mocks:
   for each accepted proposal, the outer controller would run:
   `python3 scripts/care_scenario_dispatch.py --run-root RUN_ROOT run`
   It durably claims `dispatched` before starting any new assessment, then runs
   only reserved discovery trials and feeds their complete archives to `assess`.
   Every batch uses the same frozen disposable source. There is no training or
   Current/promotion path. All commands are fixed native argv with time/output
   bounds, no shell interpolation. Failures and uncertain outcomes are preserved
   and never redispatched. Native construction remains authoritative for movement.
   The existing sampler admits complete archives, matching reserved seed,
   family/layout, dose and frozen identity. It rechecks executable, ledger,
   archive and source evidence before feedback commits. Pool/holdout observations,
   duplicate or missing trials, mixed sources and changed evidence reject.
   A controller may separately use `assess --native-evaluator NATIVE --archive ...`
   to admit independently verified completed archives after a dispatch; this is
   validation only, never permission to re-execute an uncertain life.
6. Repeat from `prepare` until the fixed batch count is exhausted. The existing
   sampler adjusts difficulty at most one tier. All-success, all-failure and
   no-frontier batches retain measured outcomes; none is rerolled or represented
   as training progress. Candidate training selections are recorded but not run.

The batch handoff contract does not require per-batch user interaction when
the task has authority and valid inputs, but fresh initialization is currently
blocked and the complete autonomous loop is not qualified. This seam neither grants authority nor
ensures an agent remains alive to continue.

## Durability and interrupted work

Each batch operation first exclusively creates and fsyncs a numbered stage directory,
claiming it before proposal reads or native validation. It writes/fsyncs the
record and completion seal, then fsyncs the directory. Founder initialization
has a separate exclusive, fsynced claim bound into dispatch records. Records bind predecessor
hashes and preserve host inputs, exact proposal-input hash, compiled bank,
native receipt/inventory pins, and sampler results. Existing evidence is never
overwritten. Concurrent workers cannot claim the same stage twice.

Failed validation or interrupted writes leave an incomplete claim that blocks all
later operations. There is **no automatic recovery, deletion, retry, reroll or
seed reuse**. Scoped SIGTERM/SIGHUP handlers kill and reap the current native
process group before returning failure, and restore previous handlers. SIGKILL
and power loss cannot run cleanup: any surviving child or partial output is
uncertain and must never trigger automatic replay. Read-only `status` verifies complete records and rejects incomplete
ones. Preserve the root and report the blocker for an explicit decision.

After interruption with a sealed `dispatched` stage, never redispatch its saved
instructions: execution may already have started. Only consume independently
known complete archives for that batch. If any completion is uncertain or any
archive is missing, stop. A sealed `assessed` stage can proceed only after its separate completed-batch
marker and inventory validate. Missing completion blocks `prepare` before it
returns any next designer request. This handles interruption between the ledger
assessment commit and batch-seal write without replaying lives.

The hash chain is **not an authenticated external anchor**. It detects ordinary
changes but cannot detect complete trailing-stage deletion, whole-ledger rollback
or intentional rewrite with recalculated hashes. Never restore an older ledger
and resume dispatch. Strong anti-rollback requires an independent monotonic
reservation/dispatch service, outside this seam. Local root permissions and the
host's seed reservations must be trusted.

## Verification

The fixed dispatcher is assessed separately from designer invocation. A passing
mock suite is not an end-to-end autonomous agent or live-native qualification.

`python3 -m unittest discover -s scripts -p test_care_scenario_batches.py -v`

`python3 -m unittest discover -s scripts -p test_care_scenario_dispatch.py -v`

Tests mock native admission and cover two-batch progression, public data bounds,
seed/holdout separation, fixed ladder assignments, exclusive claims, incomplete
writes, failed proposals, repeated finalization, tampering, evaluator pins,
mixed source/dose and cross-batch identity. They do not qualify a live model or
native assessment run.

## Historical native smoke and correction

One bounded prototype smoke on 2026-11-04 created a disposable founder and ran
three frozen discovery trials at two decisions each, with read-only native
admission afterward. All three survived; none recorded nourishment success.
Difficulty remained zero. No retained creature, pool/holdout/reference trial,
training round after initialization, or promotion was used.

The constructor used for that smoke was **not training-free**: its source receipt
records **2 actor optimizer updates and 2 value optimizer updates**, starting
from zero. Earlier prototype `training_enabled=false` initialization receipts
were therefore incorrect. The three later assessment receipts do correctly
record zero optimizer updates. The original evidence is preserved with a separate
correction; the initializer is now disabled rather than repeating that behavior.

This smoke verifies frozen assessment/receipt flow from that disposable trained
fixture, not a no-training source constructor, native learning improvement, or
automatic Codex designer invocation. No further native lives were run after the
constructor issue was identified.
