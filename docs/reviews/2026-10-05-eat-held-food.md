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
Receipt queues are bounded at 256 entries; keep tonight's assessments and first cycle
below that bound to retain the entire decision-input trace. Metrics do not prove
that cognition used a hint. Exact current persistence and GPU-authoritative
cognition remain required.

The applicable controlling requirements are `AOA-WORLD-001..003`,
`AOA-BODY-001/005/006`, `AOA-SENSE-001/002`, `AOA-MOTOR-004/005`,
`AOA-EMB-001/003`, `AOA-LEARN-002/004/010`, `AOA-FOUND-002/006`,
`AOA-SLM-001..004`, `AOA-TEACH-004/005/007`, `AOA-PERSIST-001/002/004`,
and `AOA-OBS-002..005`. Carrying a nearby object in a stable hand position follows
the repository's WWCD guidance. Teacher, biology, semantic prior, and learner
policy remain separate mechanisms.

## Tonight's bounded setup and commands

PC provider and GPU readiness are **Unknown** from this cloud checkout. The
owner must verify the reviewed commit, clean source, idle training lane, compatible
sealed source, and founder seed before launching. Use a new output directory for
every command; never resume an interrupted collection or replace the source.
Record Git SHA/tree/status, executable SHA-256, source asset SHA-256, source
receipt, founder seed, world seeds, actual adapter identity, and start/end times.
Build only once in the shared target directory.

On the PC, after the parent selects the owner and exact source, use PowerShell 7
to build the trainer under the same overall process budget:

```powershell
$DeadlineUtc = [DateTime]::UtcNow.AddMinutes(45)
$Cargo = (Get-Command cargo -ErrorAction Stop).Source
$Run = Join-Path (Get-Location).Path ('target/founder-training/eat-held-food-' + [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Path $Run -ErrorAction Stop | Out-Null
function Invoke-BoundedProcess([string]$Program, [string[]]$Arguments, [int]$Seconds, [string]$Log) {
    $remaining = ($DeadlineUtc - [DateTime]::UtcNow).TotalSeconds
    if ($remaining -le 0) { throw 'Overall night-check budget exhausted.' }
    $limit = [int](1000 * [Math]::Min($Seconds, $remaining))
    $info = [Diagnostics.ProcessStartInfo]::new($Program)
    $info.UseShellExecute = $false
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    foreach ($argument in $Arguments) { $info.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $info
    try {
        if (-not $process.Start()) { throw 'Process did not start.' }
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        $timedOut = -not $process.WaitForExit($limit)
        if ($timedOut) { $process.Kill($true); $process.WaitForExit() }
        [IO.File]::WriteAllText("$Log.stdout.log", $stdout.GetAwaiter().GetResult())
        [IO.File]::WriteAllText("$Log.stderr.log", $stderr.GetAwaiter().GetResult())
        if ($timedOut -or $process.ExitCode -ne 0) { throw "Process failed or timed out; retain $Log logs and stop." }
    } finally { $process.Dispose() }
}
Invoke-BoundedProcess $Cargo @('build', '--release', '-p', 'alife_game_app', '--features', 'foundation-training', '--bin', 'train_n2048_care') 1200 (Join-Path $Run 'build')
```

Verify the local GGUF's actual SHA-256 and the llama.cpp server alias. Use the
existing `scripts/start_llamacpp_slm_prior.ps1` with explicit executable/model
paths if the server is absent; the owner starts it tonight, not during day work.
Set `ALIFE_SLM_PRIOR=on`, `ALIFE_SLM_PRIOR_MODEL` to the served alias, and
`ALIFE_SLM_PRIOR_MODEL_SHA256` to the actual file digest. The runtime uses localhost
port 18081. A successful `/health` and `/v1/models` check is an initial check only;
actual validated structured hints and dispatch delivery must appear in the probe.

Run the bounded CPU provider preflight tonight and retain its JSON and errors:

```powershell
Invoke-BoundedProcess $Cargo @('run', '-p', 'alife_semantic', '--features', 'local-llamacpp', '--example', 'local_slm_prior_preflight') 300 (Join-Path $Run 'provider')
```

This sends one real structured request with a 30-second provider timeout and
checks usable validated hints. Its `supplied_model_sha256_claim` comes from the
environment; `model_file_hash_verified=false` is intentional. Keep the independent
GGUF file hash alongside it. This check does not dispatch a neural GPU or establish
brain input delivery; the subsequent frozen probe supplies that receipt.

After setting `$Source` to the selected sealed directory and `$FounderSeed` to its
receipt's founder seed, use one reserved assessment seed before and after the
update, and a distinct learning seed. Assessment pauses offline optimizer updates;
ordinary lifetime associations and biology continue in both assessments.

```powershell
$Trainer = (Resolve-Path target/release/train_n2048_care.exe).Path
$Probe = Join-Path $Run 'frozen-before'
$Cycle = Join-Path $Run 'cycle-000'
$After = Join-Path $Run 'frozen-after'
# Keep $DeadlineUtc, $Run and the helper from the build/preflight setup above.
Invoke-BoundedProcess $Trainer @('--evaluate', $Probe, (Join-Path $Source 'trained.alife-foundation'), 'eat_held_food', '16', '2026100501', "$FounderSeed") 600 (Join-Path $Run 'before')
# Verify real possession, selectable Eat, contextual hearing, delivered prior
# inputs and measured outcome/credit receipts before the next command.
Invoke-BoundedProcess $Trainer @('--resume-cycle', $Source, $Cycle, '128', '--seed', '2026100502', '--lesson', 'eat_held_food') 1200 (Join-Path $Run 'cycle')
# Only a completely sealed cycle may supply this post-update assessment.
Invoke-BoundedProcess $Trainer @('--evaluate', $After, (Join-Path $Cycle 'trained.alife-foundation'), 'eat_held_food', '16', '2026100501', "$FounderSeed") 600 (Join-Path $Run 'after')
```

These are separate supervised steps, not an unattended campaign. Apply a hard
10-minute timeout to each probe and a 20-minute timeout to the single cycle,
with a 45-minute overall limit including build. If a limit or any command fails,
stop the owned process tree, retain logs and partial receipts, mark the result
Incomplete/Failed, and do not retry or widen the budget without diagnosis. A
pre-update failure to eat is a baseline observation; absent action availability,
input delivery, or usable credit evidence requires diagnosis before training.
Check that the sealed ordinary outcome and lifetime credit path match the captured
decision. A baseline with no meal can have no positive ingestion benefit.
Do not invoke the existing multi-skill campaign or an additional broad gate suite
for this first lesson. Earlier prior-off panels used a different environment and
cue configuration and are not an apples-to-apples baseline for this lesson.

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
- Count held-food success only from matching learner-selected Eat, real possession,
  matching channel consumption, and measured body benefit. Wrong-target Eat,
  attempts, ground-food meals after Drop, and setup Grab do not qualify.
- Preserve replay agreement metrics, actor/value/optimizer handoff, exact world
  checkpoint and sealed source digests. Inspect a current-format checkpoint
  restore before increasing run length. No compatibility conversion is implied.

One bounded night can establish readiness and provide a first learning observation.
It does not establish retained competence, transfer, speech competence, a service-off
gate, or founder promotion. Grab-only and reach-only relocation lessons remain later
slices; combinations follow their independent assessments.

## CPU validation

Passed in the cloud with Rust 1.98.1:

- `cargo fmt --all -- --check` and `git diff --check`.
- `cargo check --workspace --all-targets`, also with
  `--features alife_game_app/foundation-training`.
- `cargo test --workspace --all-targets --features alife_game_app/foundation-training`:
  **1,709 passed, 20 ignored, zero failures**. GPU test features were disabled.
- After adding immediate terminal-biology retention, the feature-enabled check
  passed again and the focused foundation-training suite passed **16 tests**.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Core dependency/source boundaries and docs assertions passed (**77/77**).
- Provider-preflight example compilation and three training-dose Python tests passed.

Focused carry checks passed eight new regressions, the existing interval-motion
regression, and all 14 tests in the revised world harness. Five semantic-prior CPU
regressions passed in the aggregate. The terminal-distance regression passed
alone and in the aggregate with the fixture's required birth/life archive links.
It tests reporting and retirement binding, without claiming a natural GPU death.

Additional strict feature lint,
`cargo clippy --workspace --all-targets --features alife_game_app/foundation-training -- -D warnings`,
still reports **15 existing findings**: six constant-chunk suggestions, one range
pattern, three existing argument-count findings, three existing modulus patterns,
the boxed demonstrator type, and test-module placement. Their source/signatures
were checked against `de38cad9`; no baseline compiler run or lint suppression is
claimed. The new cue-modulus finding was fixed. Legacy cleanup remains outside
this change. Local logs and the source comparison receipt are retained under
`target/artifacts/cassidy-20261005/`.

The initial debug/incremental build exhausted the writable overlay; only generated
incremental caches and an unused task-installed toolchain were removed. Validation
then used the saved lean CPU environment. An old carry regression was updated to
assert the new body-local grip and actual interval displacement.

Actual GPU dispatch, learned eating, replay on real hardware, PC model availability,
visual hand attachment, PowerShell execution, and nighttime commands are **Unrun**
in this cloud task.
