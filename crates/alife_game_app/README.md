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
by the round owner lock. Live/recorded priors, GPU/group/cooperative-result source
admission, custom mixed-skill episodes, automatic curriculum progression and
trainer-removed efficacy assessment are outside V1.

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
matched. An operator still must choose an authorized source and supply the
trainer-removed assessment; neither command performs automatic promotion.

## Ownership

- `alife_world` owns perception facts, unscored candidates, legality, targets, action execution, outcomes, and stable world identity.
- `alife_runtime` and `alife_gpu_backend` own production neural state and GPU execution.
- This crate schedules those systems and translates read-only state for Bevy.
- UI and render code never own simulation truth.
