# alife_game_app

`alife_game_app` owns the application shell, launch policy, runtime scheduling, controls, foundation training runner, diagnostics, and voxel presentation for A-Life.

## Player controls

Normal play shows a population/pause indicator, the selected creature's physiology HUD, and the five-control care dock (`E` place food, `C` gentle touch, `J` place toy, `K` praise, `Enter` speech composer). `F2` names the selected creature. `F1` opens keyboard help. `F3` toggles debug mode (performance, brain and memory details, speech diagnostics, and world overlays). `R` restores the starting camera view.

For a bounded behavior trace, set `ALIFE_ACTION_TRACE_PATH` to a new JSONL file in an existing directory. It records up to 10,000 distinct presented world ticks, including actions, targets, outcomes, sleep state, positions, and food state, using existing presentation data without extra neural readback.

Sleep readiness follows fatigue, sleep pressure, and neural recovery. Deaths complete the archive transaction before GPU and visual retirement.

## Production path

The launcher uses an optimized release build by default (`-BuildProfile dev` selects an unoptimized build for iteration):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -NewGame -Population 1
```

Build the local Windows package with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package_windows_production_voxel.ps1
```

The production shell requires GPU-authoritative neural execution. A missing or failed GPU neural path is reported as unavailable; it does not silently switch to CPU neural math.

The shell restores and ticks a live `GpuLiveBrainRuntime`. The active renderer uses its layered-grid terrain meshes, starts creature presentation from the selected save, projects live authoritative positions, adds newborn presentation records, and consumes the runtime retirement queue. During foundation training (`foundation-training` feature), mutable GPU state before and after each tick is captured across all active creatures in two batched GPU submissions (`capture_training_state_batch`).

## Validation and diagnostic commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- validate-production-assets
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-closed-loop-acceptance --help
cargo run -p alife_game_app --features gpu-tests --bin alife_game_app -- gpu-closed-loop-soak --help
```

## Bounded foundation evaluation outcomes

Native pilot receipts (`pilot.json` and `lesson-trace.json` or
`lesson-diagnostic.json`) include `evaluation_observation` beside the unchanged
`lesson_completed` flag. It records the requested ticks, actual tick calls,
captured waking steps, last world-tick boundary, neural phases, body sleep and
terminal death evidence for the admitted learner.

`sleep_censored` means collection stopped at a neural sleep boundary before the
lesson completed. Recovery requires a later awake meal, so this result does not
establish inability to wake. Ordinary body Rest alone is insufficient evidence
of neural sleep. `not_completed` means the criterion was false within the stated
budget or lesson boundary; `terminal_death` requires lifecycle/archive evidence;
`missing_waking_capture` remains unexplained rather than being assumed asleep.
Historical receipts without this field have an unknown stop reason. Collection,
replay and acceptance criteria are unchanged. No neural waking progress is
claimed by this reporting correction.

## Relocated-food motion evidence

Native GPU relocated-food pilot/assessment and cycle captures extend the existing
`food_relocation.rows` diagnostics. Each row binds the organism, sequence,
decision/outcome ticks, frame digest and dispatch generation to:

- `body_before_decision` and `body_after_outcome`: canonical world entity,
  position, interval velocity, `body_yaw`, `head_yaw` and `gaze_yaw`. Yaws are
  radians in the horizontal X/Z plane: zero faces +X, positive turns toward +Z.
  `head_yaw` is the offset from the body; `gaze_yaw` is their sum, as used by
  perception. These are capture boundaries, not intermediate channel poses.
- `selected_commands`: every issued channel's complete command, including
  direction, intensity, duration, target, stand-off distance and bounded payload.
  The representative `selected_action` can differ from concurrent locomotion.
- `applied_motor_channels`: the matching world's adapted commands in execution
  order, with phase, physical outcome and measured channel observation. World
  embodiment/locomotor adaptation can change intensity or direction. The complete
  `channel_outcomes` also remain bound to the sealed patch.

The capture reads existing world state and CPU-side receipts only when this
lesson's diagnostic collector is active; it adds no neural inputs or GPU
readback. General `lesson-trace.json` already includes the perception frame's
gaze quaternion but retains a smaller selected-command projection; sealed replay
patches already retain complete issued bundles. The new rows join that command
evidence to world headings and applied receipts absent from headless relocation
captures. Join records
using their causal identifiers. Missing historical fields or unavailable body/
motor evidence mean unknown, and saved receipts are not backfilled. Frame-level
visibility and motion alone cannot establish memory failure, command conflict
or sub-tick causality. These diagnostics do not measure learning efficacy or
performance.

## Feeding nourishment in mixed-care reports

The native GPU `feeding` cycle writes optional `feeding_food` evidence in
`cycle.json`: the learner/target `food_outcome`, captured/trained row counts and
`nourished_meal_rows`. The observer requires the learner's held, unconsumed target,
its actual selected Eat consumption and measured energy gain or hunger reduction
without fresh health, injury or pain harm. Pickup, release and regrab do not count
as meals. The same target may regrow; each additional meal still requires a new
pickup, consumption and measured benefit. This records evidence without changing
biology, reward or curriculum.
The pilot already reports `food_outcome`; native CPU care separately reports
`captured_nourished_meals`, `trained_nourished_meals` and
`bootstrap_nourished_meals` in `care-episode.json`.

The mixed-care host writes `schedule.json` from accepted ledger receipts.
Feeding's `generic_consumed_events` remains independent of
`exact_nourishing_consumption_events`; the trained exact count excludes bootstrap
rows. Older missing/null `feeding_food` evidence remains unknown. If any accepted
Feeding episode is unknown, both exact totals are `null`, with
`exact_nourishment_measured_episodes`, `exact_nourishment_unknown_episodes` and
`known_*exact_nourishing_consumption_events` exposing the measured subset. A
measured zero is zero. Historical generic consumption is never converted into
meals, and archived receipts are not rewritten. Compact host validation checks
count/row consistency; it cannot independently reconstruct native physiology
or target ownership from those counts.

### Continuous-world meal acquisition and assistance associations

Continuous update and final receipts add `food_meal_provenance_captured` and
`food_meal_provenance_trained`. These are fixed-size aggregate categories for
exact-target nourishing meals linked to an observed `Eat` pickup, an observed
`Grab` pickup, or an unobserved acquisition. `Eat` can acquire loose food before
a later `Eat` consumes it; zero explicit `Grab` is not zero pickup. A link is
cleared by a release, missing ownership or a gap in captured observations.
The trained aggregate filters **meal rows**, not acquisition rows, and excludes
bootstrap. Generic consumed events remain separate and cannot create meal links.

Each category reports overlapping prior-session cue-emission, addressed teacher
food/eat-token hearing and same-target food-placement associations, plus tick
ranges to the meal outcome. Emission history covers this collector's care cues,
not all possible teacher speech. Emission is not hearing; heard tokens are not
target-grounded. Initial placement is included in association history and counted
separately as `care_initial_food_placements`; later moves retain the existing
`care_food_repositions` counter. Histories survive PPO windows and same-entity
regrowth, so a window without a **new** cue or move can still follow assistance.

These are temporal associations, not causal attribution or a mastery test.
History before collector start/restart is **unknown**, including previously
assisted food and carried food restored from a checkpoint. Consequently
`with_no_observed_session_assistance` does not mean independent feeding.
Telemetry retains only latest per-food provenance, fixed aggregate counters and
meal links within the already bounded replay window; no per-meal archive or
checkpoint-format change is introduced. Rewards, biology and learner behavior
are unchanged.

From the repository root, derive a fresh summary from an existing ledger and
its exact saved run directory without dispatching a trainer or writing files:

```sh
python -B - ABS/LANE/.mixed-care-seed-reservations.json ABS/RUN <<'PY'
import json
import sys
from pathlib import Path
sys.path.insert(0, "scripts")
import run_mixed_care_serial as mixed
ledger = mixed.read_ledger(Path(sys.argv[1]))
output = str(Path(sys.argv[2]).resolve(strict=True))
run = next(run for run in ledger["runs"] if run["output"] == output)
print(json.dumps(mixed.summary(run), indent=2))
PY
```

This reads the retained receipt evidence; it does not repeat native checkpoint
admission or establish learning efficacy. A saved `schedule.json` from an older
host retains its original reporting behavior until a new derived report is made.

## One serial CPU care round

Build once, then use the same executable for planning, dispatch and retry:

```sh
cargo build --locked -p alife_game_app --features cpu-training --bin train_cpu_care --bin train_cpu_care_round
target/debug/train_cpu_care_round plan --spec care-round-spec.json --output sealed-care-plan.json
target/debug/train_cpu_care_round run --plan sealed-care-plan.json
target/debug/train_cpu_care_round inspect --result PATH/TO/NEW_ROUND/learner
```

On Windows use `target/debug/train_cpu_care_round.exe`. The sealed plan file must
live outside the new round output directory. Example recipe (replace paths and
choose seeds from the supplied history):

```json
{
  "schema_version": 1,
  "previous": "PATH/TO/EXPLICIT_NATIVE_CPU_SOURCE",
  "seed_history": "PATH/TO/EXACT_HISTORY.json",
  "output": "PATH/TO/NEW_ROUND",
  "known_exclusions": false,
  "temperature": 1.0,
  "prior": "deterministic_v1",
  "window_ticks": 32,
  "burn_in": 8,
  "effective_batch_size": 1,
  "record_diagnostic_replay": false,
  "collectors": [
    {"collector_run_id": "00-held", "seed": 551001, "lesson": "eat_held_food", "decisions": 32},
    {"collector_run_id": "01-reachable", "seed": 551002, "lesson": "eat_reachable_food", "decisions": 32}
  ]
}
```

Planning performs read-only native source/history admission and seals exact
source, evidence, executable, provider and assignment identities. It selects no
checkpoint and reserves no seed. V1 requires a completed native CPU directory
with its full `training-handoff.json`, a grounded natural-senses policy, and the
existing `mixed-care-seed-history` format. Strict mode retains the operator's
complete-history attestation. Explicit `known_exclusions: true` retains unknown
historical freshness and the existing qualified-checkout recovery checks; it
never upgrades incomplete evidence. Run that mode from the source checkout.
The plan pins that checkout's validator/helper bytes and cited recovery evidence;
qualified manifest/segment verification is repeated before dispatch and learning.
The original import graph must leave one nesting level and two import slots for
the round's history wrapper and immutable lane snapshots.

Run dispatches 1..16 collectors in canonical run-ID order, each with 1..128 loss
decisions plus a real bootstrap decision. Every episode starts from the same
frozen founder/actor/critic state and uses a fresh private in-memory deterministic
rule prior with the existing training dropout. Ambient `ALIFE_SLM_*` settings and
live model/cache state do not select the round's prior. Held/reachable setup,
ordinary perception, biology, waking learning/sleep and the existing food
ownership observer remain active; a generic consumption event is not reported
as the learner's own pickup or nourished meal.

Prior failures, including schema-valid replies with no usable hints, are logged
and retained in the episode's metrics. The affected decisions continue unaided
and may be sealed without replacement hints or repeated blocking retries.
The pinned provider identity must still match before collector sealing.

Each `collectors/RUN_ID` retains its sealed shard, native private checkpoint,
prior-life archive, actual care traces and source ancestry. The lane records a
terminal `collected` attempt with zero offline optimizer updates, never an
accepted-founder advance. Per-dispatch histories retain the immutable original
history and exact ledger snapshots. Once every collector is complete, the
existing coordinator runs one central learner and writes the full recoverable
receipt at `learner/cooperative-round.json`; stdout is only a compact summary.
PPO uses its existing defaults plus the explicit temperature. The result is
experimental and is not a native `previous` source or a promotion.

Retry the same plan and executable to reuse completed collectors and a completed
learner result. A failed/unsealed private life is preserved and its seed remains
excluded; the adapter does not restart it or choose a replacement seed. A
surviving whole-episode owner lock is never deleted automatically. Stop and
reconcile the prior owner through the existing care reconciliation procedure;
incomplete life resume remains unsupported. Returned collection errors archive
consumer-owned prior lives alongside the native recovery checkpoint; a process
kill can occur before that boundary and does not guarantee a complete pair.
Concurrent invocations are refused
by the round owner lock. Live/recorded priors, GPU/group source admission,
custom mixed-skill episodes, automatic curriculum progression and exact-life
resume remain unsupported. A completed cooperative result can now be explicitly
exported as a fresh-life CPU founder source, as described below.

This is serial **collection from a frozen source**, a dispatch parity control.
The existing ordinary CPU serial care path learns between episodes and supplies
the separate serial-learning comparison. Match starting checkpoint, seeds,
prior/dropout, lesson doses, replay windows, actual paired optimizer updates and
KL stops; equal rows alone do not match the optimizer budget. This adapter does
not establish positive transfer or synergy. Those require trainer-removed
mixed-skill and preservation outcomes against that matched learning control.

For that separate learning control, supply the same explicit initial source and
history with a separate new lane. An equivalent two-lesson `serial-schedule.json`
is `{"schema":1,"steps":[{"lesson":"eat_held_food","episodes":1,"decisions":32},
{"lesson":"eat_reachable_food","episodes":1,"decisions":32}]}`. From a clean
checkout, the existing scheduler invocation is:

```sh
ALIFE_SLM_PRIOR=on ALIFE_SLM_PRIOR_BACKEND=deterministic python scripts/run_cpu_care_serial.py \
  --binary target/debug/train_cpu_care --previous PATH/TO/EXPLICIT_NATIVE_CPU_SOURCE \
  --lane PATH/TO/SEPARATE_SERIAL_LANE --seed-history PATH/TO/EXACT_HISTORY.json \
  --schedule serial-schedule.json --seeds 551001:551002 --temperature 1.0 --run
```

Plan both arms from the same immutable evidence before either runs; use isolated
lane histories. The scheduler retains its existing prior/cache behavior, so run
it in an isolated process/work directory with explicit prior settings. Compare
actual optimizer steps and KL-stop receipts before treating the budgets as
matched. An operator still must choose an authorized source. The following bounded
comparison command also executes teacher/prior-removed assessment. Neither path
performs automatic promotion.


## Continue and compare two CPU rounds

Export a completed learner result into a new, explicitly selected founder source:

```sh
target/debug/train_cpu_care_round export-source \
  --source ABS/EXACT_NATIVE_SOURCE --result ABS/ROUND/learner --output ABS/NEW_FOUNDER
```

The exporter verifies the raw source handoff and the sealed result before writing.
It carries the complete output actor/value Adam histories, frozen coordinates,
compiler inputs, policy version, fixture taint and all member seed exclusions.
The existing CPU filenames contain an explicit `native_cpu_cooperative_founder`
receipt and retained round proof. Export runs no biology or optimizer and may be
retried only against the same exact inputs/bytes. It selects no accepted lineage.
A next round uses `ABS/NEW_FOUNDER` as `previous` with source-bound history.
Every new episode is a fresh private life. Earlier lives remain archived under
their collection policies; recurrent, prior and biological state are never
transplanted into a changed policy. This is not exact-life resume.

The executable comparison runs exactly two cooperative rounds and an independent
serial mixed-curriculum learning arm from the same explicit initial source:

```sh
python -B scripts/compare_cpu_care_rounds.py --spec ABS/comparison.json
python -B scripts/compare_cpu_care_rounds.py --spec ABS/comparison.json --run
```

The first command validates and prints a plan without starting executables or
creating outputs. Example recipe (all paths must be absolute; choose seeds using
the supplied existing complete history):

```json
{
  "schema": 1,
  "previous": "/ABS/EXPLICIT_NATIVE_CPU_SOURCE",
  "seed_history": "/ABS/EXISTING_COMPLETE_HISTORY.json",
  "output": "/ABS/NEW_COMPARISON",
  "care_binary": "/ABS/A-Life/target/debug/train_cpu_care",
  "round_binary": "/ABS/A-Life/target/debug/train_cpu_care_round",
  "temperature": 1.0,
  "rounds": [
    [
      {"collector_run_id":"00-held","seed":551001,"lesson":"eat_held_food","decisions":32},
      {"collector_run_id":"01-reachable","seed":551002,"lesson":"eat_reachable_food","decisions":32}
    ],
    [
      {"collector_run_id":"00-held","seed":551003,"lesson":"eat_held_food","decisions":32},
      {"collector_run_id":"01-reachable","seed":551004,"lesson":"eat_reachable_food","decisions":32}
    ]
  ],
  "assessments": [
    {"seed":551101,"lesson":"obstacle_navigation","decisions":64},
    {"seed":551102,"lesson":"relocated_food","decisions":64}
  ]
}
```

On Windows use absolute Windows paths and `.exe` executable names. The runner
isolates both arms and their private lives, uses explicit deterministic training
priors, exports each cooperative result and feeds it to the next round. The
serial arm learns between each lesson through the existing native care path.
Using the same recipe seeds across the two independent arms is intentional
experimental replication. Held-out seeds are declared before training and kept
disjoint from every disclosed training/consumed seed.

Matched V1 training doses are 1..32 decisions, with replay32/burn8/batch1: this
keeps each ordinary serial episode in one window. The standalone collector still
supports 1..128. `comparison-report.json` records actual PPO settings, full paired
optimizer-age deltas, loss rows, windows, KL stops, private archive hashes and
paired held-out outcomes. `budget_matched` requires the recorded settings and
actual updates to match; a KL stop or budget mismatch is disclosed rather than
called a matched result. Policy version counters can differ because the arms
group updates differently. Comparison uses actual Adam updates.

Assessment uses the same physical lessons and ordinary perception, biology,
private waking learning and sleep, with teacher actors/utterances and the private
semantic prior disabled. It runs no offline actor/critic update and emits no
learner shard. Own acquisition, consumption and nourished meals remain distinct;
bootstrap outcomes are excluded from the evaluated dose. To assess an explicit
source independently, use `train_cpu_care_round evaluate --spec ABS/assessment.json`
with `previous`, `output`, `seed_history`, `seed`, `decisions`, `temperature` and
`lesson`; the seed must be listed in that history's `assessment_seeds`. Completed
assessment reuse verifies source/history/artifact identity. Unsealed lives are
retained and refused. V1 requires the complete living decision dose and real
bootstrap; death or an incomplete dose retains its checkpoint/trace and aborts
the comparison instead of fabricating a completed outcome. Survival-rate and
variable-duration terminal assessment remain unsupported.

The comparison defaults to an existing strict complete operator-attested history.
For an explicitly limited relative-behavior experiment with incomplete history:

```sh
python -B scripts/compare_cpu_care_rounds.py --spec ABS/comparison.json --known-exclusions
python -B scripts/compare_cpu_care_rounds.py --spec ABS/comparison.json --known-exclusions --run
```

This opt-in requires `complete:false`, exactly
`"seed_admission":{"policy":"known-exclusions","historical_freshness":"unknown"}`,
the existing bounded `missing_history` explanation and valid cited
`recovery_reconciliation`. Derived histories retain those facts unchanged.
The runner reuses the existing recovery validator, source/compiler/objective/full
optimizer admission and every disclosed seed exclusion. Recovery JSON evidence
also contributes disclosed seed ranges to preflight. It pins all cited evidence,
the qualified checkout and five admission helpers, while keeping each arm's
process directory and deterministic prior cache isolated. Both modes retain
within-experiment training/assessment separation and actual matched-budget checks.

Comparison V1 uses new CPU-only lanes with no prior mixed-ledger runs. The supplied
attestation must already describe that isolated fork and bind the empty prior-runs
digest; an attestation for a different lane is rejected. Cited plain immutable
recovery files are supported. Ledger-origin-bound manifests cannot be retargeted
across these fresh lanes; retain their original records and obtain a legitimately
supported fork attestation rather than editing its claims or digest automatically.
Known seed ranges in structured history/import/recovery records remain exclusions.

Unknown-mode plans, command records, care episodes, native serial receipts,
assessment results and comparison reports retain the admission policy and claim
limitation. Generic central learner receipts attest their exact source and
optimizer result through SHA-pinned collector commits; they do not attest
historical freshness. Generic founder exports and inspection summaries remain
explicitly `unasserted`, never silently complete.

Unknown-mode results describe relative behavior conditional on the selected
initial source and specified worlds. They cannot establish historically unseen
seeds, unseen-world generalization or synergy. Shared prior exposure does not
invalidate reporting the paired measurements, but limits those interpretations.
Complete history supports historical seed exclusion; it alone does not establish
novel skill combinations or statistically reliable positive transfer.

Neither mode manufactures completeness or upgrades unknown evidence. Fresh outputs are
required: interrupted comparison runs retain all completed/partial artifacts,
and automatic comparison resume is unsupported. Standalone source export is
limited to 64 MiB aggregate ancestry blobs; comparison ancestry/history inputs
have tighter documented bounds (124 receipts/16 MiB). Original sources remain
untouched if a limit is exceeded. Generic cooperative members carry no inferred
historical-freshness claim. No result is promoted. Fixture outcomes establish
mechanical correctness; positive transfer and composition require adequately
sampled real held-out outcomes against the measured serial-learning control.

## Ownership

- `alife_world` owns perception facts, unscored candidates, legality, targets, action execution, outcomes, and stable world identity.
- `alife_runtime` and `alife_gpu_backend` own production neural state and GPU execution.
- This crate schedules those systems and translates read-only state for Bevy.
- UI and render code never own simulation truth.
