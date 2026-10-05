# Cassidy's first eating lesson — 2026-10-05

This change starts from fetched `main` at
`de38cad9dcee6af78e577174be6690730f65435f`, on the isolated local branch
`codex/cassidy-eat-held-food-2026-10-05`. It does not incorporate the personal
PC's unmerged overnight diagnostics or stage-mask experiments. Cassidy separately
authorized branch publication and a draft PR after checks pass. No training,
GPU behavior acceptance, founder promotion, or merge is performed by this task.
The parent coordinator selects tonight's source and single owner.

## Behavior and authority

Held objects follow a near-front body-local grip when the carrier translates or
turns, while head gaze stays independent. Carry displacement includes rotation
over the complete motor interval. Grip placement and movement check solid
geometry before publishing pose. Rotation uses bounded centerline segments at
most five degrees apart, including a thin-wall arc regression; this preserves the
existing point-body world model and is not an exact carried-object volume sweep.
Visibility still uses the existing 110-degree half-view and ordinary
occlusion. Bounded retention and candidate preference apply only to an actually
observed held/contact object; they do not expose invisible objects or semantic
object kinds. Neural candidate capacity and sensor ABI remain unchanged.
The renderer projects authoritative object positions and adds its existing 0.30
vertical lift. Creature X/Z smoothing can briefly differ from a food position
updated from the current frame. No palm socket or possession animation is added;
actual visual hand attachment remains pending an interactive check.

`eat_held_food` establishes legal possession before learner collection using the
ordinary Grab world action. The setup receipt separates that action from the
learner's decisions. A healthy, awake founder with natural hunger receives safe
food; setup neither replenishes reserves nor writes a reward. Assessment requires
the learner's Eat of the exact object it still holds, that channel's actual
Consumed receipt, and an ordinary beneficial body consequence. Navigation,
learner Grab, and speech competence are not prerequisites. A blocked independent
motor channel does not conceal successful ingestion.
Terminal pilot reporting retains the admitted learner's lifecycle/archive death
tick and leaves food distance unset when that learner is unavailable; a remaining
teacher cannot stand in for it. The pilot retains one-shot terminal biology
immediately after the successful tick, before another tick can clear it.

Ordinary spatial teacher speech supplies contextual naming/request words for
the real held object. These cues are independent of vocabulary assessment:
speech labels remain absent, including silence labels, and language-contingent
praise is absent. This first slice does not narrate completed events. A later
completed-action cue must follow a sealed channel-specific outcome into a later
ordinary hearing frame; it must not be injected retrospectively into the decision.

The explicit rich-information lesson retains normal senses and language and
requires the bounded semantic prior. It disables deliberate training dropout and
fails when bounded priming cannot supply a usable hint. Receipts distinguish
preparation from input accepted by the authoritative inference dispatch, record
nonzero private-port encoder delivery per decision, and retain provider failures.
Receipt queues are bounded at 256 entries; each episode uses at most 128 loss
rows plus one bootstrap row, retaining the entire decision-input trace. Duplicate
lexicon rows use the maximum validated salience for a token consistently in
readiness and conversion, so a zero-salience first row cannot conceal delivery. Metrics do not prove
that cognition used a hint. Exact current persistence and GPU-authoritative
cognition remain required.

The applicable controlling requirements are `AOA-WORLD-001..003`,
`AOA-BODY-001/005/006`, `AOA-SENSE-001/002`, `AOA-MOTOR-004/005`,
`AOA-EMB-001/003`, `AOA-LEARN-002/004/010`, `AOA-FOUND-002/006`,
`AOA-SLM-001..004`, `AOA-TEACH-004/005/007`, `AOA-PERSIST-001/002/004`,
and `AOA-OBS-002..005`. Carrying a nearby object in a stable hand position follows
the repository's WWCD guidance. Teacher, biology, semantic prior, and learner
policy remain separate mechanisms.

## Tonight's startup and longer run

Cassidy requested **thousands of distinct examples/episodes**, rather than dozens
or repeated decision rows. The new `scripts/run_eat_held_food_night.py` targets
3,000 fresh `eat_held_food` episodes by default. It does not launch in plan mode.
The 16/128/16 frozen-before/update/frozen-after sequence and its 45-minute cap are
startup qualification only. Subsequent episodes use 16 loss rows by default,
with frozen 16-decision checks after each bounded batch of 16 episodes and a final
assessment reserve. No navigation, Grab lesson, maze or multi-skill schedule is
included. Longer dose does not establish successful learning by itself.

Every episode prepares exactly one genuine held target. Its seeded X/Z location
and body facing vary within four units per horizontal axis of the founder origin; the engine's
horizontal plane is X/Z. At most 32 attempts reject invalid terrain, blocked
body/grip geometry or crowded placements. Ordinary registered Turn and Grab
realize the accepted arrangement, and natural biology establishes need. Receipts
record the sampling seed, attempt count, realized body/grip positions and yaw.
The runner rejects repeated realized position/facing pairs. A unique seed alone
is not counted as proof of spatial variation.

Each completely sealed cycle resumes the preceding exported actor, value head,
optimizer ages and policy version. The runtime validates that exact handoff; the
runner also checks source bytes, ages and checkpoint hashes. **Fresh worlds start
fresh individuals:** recurrent state and personal lifetime memories restart
between these episodes. Ordinary individual lifetime learning remains active
within each episode. This is an offline parameter-training sequence, not one
creature retaining personal memories across resets. Exact world snapshots and
replay remain preserved; no current-format persistence conversion is added.

PC provider and GPU readiness are **Unknown** from this cloud checkout. Tonight's
coordinator must choose one owner, the reviewed commit, a compatible sealed
source and the morning cutoff. Use PowerShell 7 from a clean repository root.
Verify no game, Cargo or training process occupies the lane. Build once using the
existing bounded `Run-Command` pattern in `run_n2048_care_training.ps1`, or an
owner-supervised 20-minute limit that stops only its owned build process tree:

```powershell
cargo build --release --offline --locked -j 2 -p alife_game_app --features foundation-training --bin train_n2048_care
cargo build --offline --locked -j 2 -p alife_semantic --features local-llamacpp --example local_slm_prior_preflight
```

Record Git SHA/tree/status, executable SHA-256, selected source receipt and asset
hash, founder seed, actual adapter, GGUF file hash, service alias, and start/end
times. Check port 18081 before starting the existing local prior launcher; avoid
a second server. Set the served alias and independently calculated model digest:

```powershell
$env:ALIFE_SLM_PRIOR = 'on'
$env:ALIFE_SLM_PRIOR_MODEL = 'OWNER_VERIFIED_SERVED_ALIAS'
$env:ALIFE_SLM_PRIOR_MODEL_SHA256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $ModelPath).Hash.ToLowerInvariant()
# Owner-supervised five-minute process limit; the provider request itself is 30 seconds.
.\target\debug\examples\local_slm_prior_preflight.exe
```

Retain preflight JSON/errors. The supplied digest is a claim:
`model_file_hash_verified=false` is intentional. The independent file hash and
validated structured reply are separate evidence. `/health`, `/v1/models` and
configured-on alone do not establish actual neural input delivery.

Choose `$Source` and an explicit local morning cutoff within the next 24 hours;
its timezone offset is required. `$Run` must be a new directory directly under `target/founder-training`.
The following writes a reviewable plan without running the binary:

```powershell
$Source = 'OWNER_SELECTED_SEALED_SOURCE'
$Run = 'target/founder-training/eat-held-food-NIGHT-UNIQUE'
# Example only: owner must choose tonight's actual morning cutoff and timezone.
$Cutoff = '2026-10-06T08:00:00+00:00'
py -3 scripts/run_eat_held_food_night.py --binary target/release/train_n2048_care.exe --source $Source --output ($Run + '-plan') --cutoff $Cutoff --target-episodes 3000 --batch-episodes 16 --decisions 16
```

After startup readiness is authorized by tonight's single owner, the ready command
uses a separate fresh output and explicit execution flag:

```powershell
py -3 scripts/run_eat_held_food_night.py --binary target/release/train_n2048_care.exe --source $Source --output $Run --cutoff $Cutoff --target-episodes 3000 --batch-episodes 16 --decisions 16 --run
```

`--run` enforces a shared exclusive owner lock, clean source and binary/source
hashes, idle Cargo/game lane, 20-minute per-cycle and 10-minute per-assessment caps,
45-minute startup qualification and the hard morning cutoff. It reserves 15
minutes for final assessment. It uses below-normal Windows process priority,
requires 8 GiB free disk and 2 GiB available RAM, and checks resources while each
owned child runs. After a 30-second startup grace it interrupts when Windows
input resumes within the preceding 30 seconds, including startup and final probes.
Create `$Run/STOP` to stop the owned child. Other processes are never stopped.
Interactive checking is Windows-specific; it does not measure GPU scheduling
priority, VRAM or a guarantee of desktop responsiveness. Resource failures and
hardware errors remain explicit stop conditions.

Startup and batch receipts estimate end-to-end episode throughput, amortized assessment
cost and checkpoint/replay storage, with a 25% timing margin. The estimate is
updated from ordinary windows once measured and marked provisional until at least
three ordinary episodes are sealed. Internal collection/update times remain
diagnostic; throughput includes setup, admission and sealing. If 3,000 episodes cannot fit the
remaining night or disk, `night.json` records that limit and the attainable
estimate; the runner collects what fits and reports **Incomplete**, rather than
relabeling rows or epochs as examples. No throughput or overnight capacity is
promised from cloud configuration. Every episode is a checkpoint boundary;
partial collection never becomes a resume source. Failures retain logs, the last
sealed checkpoint and the partial directory for diagnosis, without automatic
retries. Two batch probes losing previously observed eating stop the sequence.
A baseline failure to eat is an observation, not a manufactured success gate.
To continue a previous held-food night, select its last sealed episode and an
explicit `--seed` greater than that episode's seed. The runner rejects repeating
an earlier seed prefix; the same assessment seed remains held out.

The frozen assessment seed is excluded from the training seed range and reused
before/after. Assessments pause offline optimizer updates while normal biology
and lifetime learning continue. Actual candidate support for Eat of the held
target, independent contextual hearing and per-decision encoder delivery must
be present even when the baseline does not eat. No language response is required.

## Required receipts and interpretation

- `pilot.json`, `cycle.json`, scenario and lesson trace must identify
  `eat_held_food`, the setup Grab, actual food/organism IDs, initial natural need,
  learner commands, per-channel outcomes, and physiological transitions.
  Preserve `held_food_terminal_death_tick` and a null final distance when the
  admitted learner dies, alongside `held_food_terminal_biology` when the ordinary
  retirement path supplies it; death remains a real failed/terminal outcome.
- Teacher cue exposure must be nonzero and independent of `vocabulary_token`;
  speech supervision and language praise must be zero. Do not require a spoken
  response to accept eating.
- The semantic prior must have `rich_information_required=true`, zero
  `dropout_frames`, failures, prime timeouts, and missing-delivery decisions.
  `decision_frames_with_prior` must equal `decision_frames`. Retain every bounded
  `decision_inputs` receipt with nonzero private lanes and actual encoder delivery.
  Model identity/configuration alone does not establish provider availability;
  preserve validated provider replies, cache provenance, and any failure records.
- `night.json` separates distinct prepared episodes, captured decision rows, trained
  loss rows, bootstrap rows, epoch repeats and actor/value optimizer updates.
  `cycle.json` separates captured held-food meals from trained-row held-food meals:
  a meal on the final bootstrap is observed but its action receives no loss.
  Admission ticks, setup actions and post-consumption idle rows add no examples.
- Count held-food success only from matching learner-selected Eat, real possession,
  matching channel consumption, and measured body benefit. Wrong-target Eat,
  attempts, ground-food meals after Drop, and setup Grab do not qualify.
- Preserve replay agreement metrics, actor/value/optimizer handoff, exact world
  checkpoint and sealed source digests. Inspect a current-format checkpoint
  restore before increasing run length. No compatibility conversion is implied.

One bounded night can establish readiness and provide learning observations at
the measured attainable dose.
It does not establish retained competence, transfer, speech competence, a service-off
gate, or founder promotion. Grab-only and reach-only relocation lessons remain later
slices; combinations follow their independent assessments.

## CPU validation

Passed in the cloud with Rust 1.98.1:

- `cargo fmt --all -- --check` and `git diff --check`.
- `cargo check --workspace --all-targets`, also with
  `--features alife_game_app/foundation-training`.
- `cargo test --workspace --all-targets --features alife_game_app/foundation-training`:
  **1,714 passed, 20 ignored, zero failures** across 203 batches. GPU test features were disabled.
- Focused foundation suites passed **28 tests**, including randomized valid setup,
  blocked sampling, old lesson behavior, exact optimizer handoff and bootstrap dose.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Core dependency/source boundaries and docs assertions passed (**77/77**).
- Provider-preflight example compilation passed. **11** overnight-runner CPU
  fixtures and **three** existing training-dose Python tests passed, including
  plan-only thousands, native receipt shape, exact resume/dose counts, cutoff
  cleanup of an owned disposable CPU child, and final assessment after interruption.

Focused carry checks passed eight new regressions, the existing interval-motion
regression, and all 14 tests in the revised world harness. Six semantic-prior CPU
regressions passed in the aggregate, including duplicate tokens through the native
128-lane encoder of a hearing-enabled N2048 fixture. The terminal-distance regression passed
alone and in the aggregate with the fixture's required birth/life archive links.
It tests reporting and retirement binding, without claiming a natural GPU death.

Additional strict feature lint,
`cargo clippy --workspace --all-targets --features alife_game_app/foundation-training -- -D warnings`,
still reports **15 existing findings**: six constant-chunk suggestions, one range
pattern, three existing argument-count findings, three existing modulus patterns,
the boxed demonstrator type, and test-module placement. Their source/signatures
were checked against `de38cad9`; the expanded change still reports those same
15 findings, with no new findings. No baseline compiler run or lint suppression is
claimed. The new cue-modulus finding was fixed. Legacy cleanup remains outside
this change. Local logs and the source comparison receipt are retained under
`target/artifacts/cassidy-20261005/`.

The initial debug/incremental build exhausted the writable overlay; only generated
incremental caches and an unused task-installed toolchain were removed. Validation
then used the saved lean CPU environment. The first expanded aggregate encountered
three generated Python bytecode caches under the source-audit directory; only
those generated files were removed, and the complete rerun passed with bytecode
generation disabled. An old carry regression was updated to
assert the new body-local grip and actual interval displacement.

Actual GPU dispatch, learned eating, replay on real hardware, PC model availability,
visual hand attachment, Windows process/resource behavior, and nighttime commands are **Unrun**
in this cloud task.
