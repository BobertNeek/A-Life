# N2048 training operator

Cassidy's first primitive is now available as `eat_held_food`. See the
[October 5 eating lesson and bounded night plan](reviews/2026-10-05-eat-held-food.md)
for legal held-food setup, contextual spatial speech without speech labels,
semantic-prior coverage receipts, startup qualification and the separate
held-food runner targeting thousands of fresh episodes with bounded batches.
Its GPU behavior and the PC provider are Unrun/Unknown until tonight's owner
records the checks; it is not added to the automatic multi-skill campaign.

Use PowerShell 7 from the authoritative repository root. The launcher has separate fidelity preparation and bounded campaign modes. Neither a completed teacher corpus nor a successful training cycle by itself proves learned care.

The next proposed [vision, maze, and vocabulary campaign](n2048-vision-maze-vocabulary-run-2026-09-28.md) must use the internal SLM prior during warmup and training, with deliberate dropout, and run headlessly at maximum sustainable speed. The launcher has explicit Care and VisionLanguage curricula. The [September 29 seven-repair report](n2048-seven-repairs-2026-09-29.md) records the current changes and preparation; the [September 28 evidence](n2048-prior-maze-speech-repairs-2026-09-28.md) is historical. No founder training campaign or scheduling is authorized by this operator update.

The final September 29 headless diagnostic passed: `seven-repairs-corpus-20260929-03`
contains 50 demonstrations/4,676 decisions covering eighteen words in both
directions; `seven-repairs-warmup-20260929-03` sealed two diagnostic epochs;
`seven-repairs-cycle-20260929-03` sealed a 32-decision teacher-free GPU update and
native N2048 optimizer handoff. These directories live under `target/founder-training`.
The dated report records source/binary identity, actual prior coverage and timings.
They are unpromoted integration artifacts, not a fully warmed founder or permission
to launch a campaign. The normal operator `status.json` retains campaign history;
this bounded qualification did not overwrite it. Visible 1x toy/care acceptance
remains pending foreground availability. Incomplete corpora ending in `-01`/`-02`
must not be used for warmup.

## Swappable optional language priors (October 10)

The current policy is **log a language-prior failure and continue the genuine
episode without that prior**. Initial and mid-collection priming poll/submit once;
there is no per-decision five-second wait. Provider errors, parse/validation
failures, unavailable workers, bounded asynchronous timeouts, and schema-valid
zero hints produce warning/diagnostic receipts. Failed providers have a shared
one-second submission cooldown. Valid later replies can recover, but an active
hint is never carried into a different bounded current context. Ordinary spatial
teacher hearing, grounded perception, biology, WGSL cognition, acquired weights,
Adam state and sealed checkpoint lineage retain their existing contracts. Invalid
organism/sequence IDs, malformed checkpoint state, numerical errors, resource
limits and ordinary admission/receipt checks remain fatal. Missing prior coverage
is reported honestly rather than used as a food-lesson completion gate.

Select a backend with one environment variable before the ordinary training
command; no lesson or brain code changes are needed:

- `ALIFE_SLM_PRIOR_BACKEND=deterministic` (training default): synchronous bounded
  compositional association hints from current sensed/heard context; no model,
  service, queue or prerecorded outputs. Ordinary gameplay keeps its local default
- `ALIFE_SLM_PRIOR_BACKEND=local`: existing localhost llama.cpp transport;
  `ALIFE_SLM_PRIOR_MODEL` selects its model alias and
  `ALIFE_SLM_PRIOR_MODEL_SHA256` records the declared model digest
- `ALIFE_SLM_PRIOR_BACKEND=luna`: trusted installed Codex CLI with its existing
  ChatGPT login; `ALIFE_SLM_PRIOR_LUNA_EXECUTABLE` is a single executable path
  (default `codex`), and `ALIFE_SLM_PRIOR_LUNA_MODEL` defaults to `gpt-6-luna`
- `ALIFE_SLM_PRIOR_BACKEND=recorded`: immutable bank named by
  `ALIFE_SLM_PRIOR_BANK`; exact current-context hits only, with no provider worker,
  live inference, retry, scenario-name/time lookup, or nearest-context substitution
- `ALIFE_SLM_PRIOR_BACKEND=off`: unaided service-off mode; legacy
  `ALIFE_SLM_PRIOR=off` also takes precedence

Deterministic rules retain at most three candidate associations: heard words
and simple word relations, genuine hunger/fatigue clauses, and conservative
joint shape-plus-chemistry hypotheses. Unknown visuals support generic look
attention; color, scent or grip alone never identifies food or a held item.
Recipes are explicitly uncertain, and ambiguous/novel combinations stay generic.
This is a small rule library, not model output, comprehension or learned competence.

The shared `BoundedSlmPriorProvider` interface accepts only bounded context and
returns validated `LocalSlmPriorOutput`. Live calls run behind the same bounded
asynchronous queue. The Luna adapter fixes read-only, ephemeral Codex arguments,
requires ChatGPT authentication rather than API-key routing, disables agent tools,
and accepts only strict four-field JSON with receiver-vocabulary associations.
It does not log in, read credentials, create tokens, install software, or modify
account settings. Missing executables, unsupported CLI versions, missing login,
quota/model-access errors and timeouts follow the same unaided failure policy.
The default live request timeout is 30 seconds; collection never waits for it.
CLI help was checked with version 0.159.2, but live Luna subscription access and
actual model output are unverified by this change. Owned timeout/error process
trees are cleaned up; guaranteed descendant cleanup after a normal CLI exit is
not established. Windows taskkill behavior is unrun. The optional Luna adapter
needs Windows live qualification before production use. Dropping the shared
queue skips waiting requests; dropping an individual receiver does not cancel
its queued/in-flight call while the queue lives. Already-running calls remain
provider-timeout bounded.

Receipts distinguish backend, configuration identity, model/digest declaration,
context contract, and (for recordings) source provider plus bank digest. Provider
enabled-provider swaps preserve validated developmental gain but discard
incompatible old active hints; malformed checkpoint/controller/packet data still
fails. Off mode keeps a dormant state carrier with no worker or delivered hints,
so on-to-off-to-save-to-on also preserves validated developmental/fade history.
Neural weights, Adam and ordinary checkpoint lineage are unaffected. Runtime-source
changes intentionally select a new live cache identity. Old caches are never
silently migrated or represented as freshly captured responses.

To freeze **existing actual captured responses**, copy provider_identity, model,
model_sha256 and context_contract from the source runtime receipt into an origin
JSON file. The read-only `train_n2048_care --prior-context-contract` command prints
the current compatibility fingerprint without starting a GPU, world, provider or
training. A bank from a different context/prompt/vocabulary contract is rejected.
Export and inspect offline:

```powershell
cargo run -p alife_semantic --features local-llamacpp --example recorded_prior_bank -- export CAPTURED_CACHE.json CAPTURE_ORIGIN.json CURRENT_CONTEXT_CONTRACT NEW_BANK.json
cargo run -p alife_semantic --features local-llamacpp --example recorded_prior_bank -- inspect NEW_BANK.json CURRENT_CONTEXT_CONTRACT
```

Export validates bounded provenance and output shape, refuses to overwrite an
existing bank, and performs no inference. Source provenance remains a declaration,
not independent proof that a model produced an entry; `unverified-model` is an
honest digest marker. Valid zero-salience entries stay zero and misses continue
unaided. No genuine recording bank was generated in these CPU tests. Obtaining
coverage requires capturing real bounded contexts/outputs first; this change does
not prerecord hidden outcomes, fabricate hints or launch a scenario collection.
The October 7 sole-owner launcher is historical and keeps its original cutoff and
listener/process security checks. A future night owner must make provider
preparation optional while preserving those security, source and resource checks.

Architecture trace: AOA-SLM-001/002/003/004/005/006, AOA-CTX-004,
AOA-INV-001/006/008/009, AOA-OBS-005. These implementation choices do not amend
v2.0 or establish learning improvement, GPU qualification or promotion.

CPU validation of this candidate: 21 isolated exact-runtime-module tests, 46
exact semantic-crate unit tests, two offline recording-export tests and 28 night
orchestration tests passed against reused dependency artifacts. Full app and
training CLI source metadata/type checks, formatting, whitespace, static core
boundaries and 77 documentation assertions passed. These are bounded CPU checks,
not a full workspace Cargo build/test/Clippy pass, live account inference,
training, GPU evidence or an actual captured recording bank.

## Tiny local prior trial (September 28)

The selected small-model trial uses [Qwen3.5-0.8B](https://huggingface.co/Qwen/Qwen3.5-0.8B), converted by [ggml-org](https://huggingface.co/ggml-org/Qwen3.5-0.8B-GGUF), in Q8_0. The downloaded file is 833,592,096 bytes, from revision `8fea620810c4afa23dd6443f999a48574c1611a3`, with SHA256 `37ae482d336108d23516fa35e8e0c4126688d81018b87178a18d752a1357814f`. It lives at `models/local/qwen3.5-0.8b-gguf/Qwen3.5-0.8B-Q8_0.gguf`. The previous Qwen3-4B model remains available. Qwen3-Embedding-0.6B is an embedding model, not a generative prior replacement.

The game bridge and launcher now default to this 0.8B model, a 2,048-token context, four threads, one GPU request slot, and reasoning disabled. Explicit overrides remain available. Do not start a second server if port 18081 is already occupied. The verified service uses full GPU offload on the RTX 3050, four CPU threads, a 2,048-token context, one request slot, and reasoning disabled. The launcher's reasoning parser separates the template's empty think tags from JSON; leaving those tags in content broke JSON-constrained generation with this model.

```powershell
pwsh -NoProfile -File .\scripts\start_llamacpp_slm_prior.ps1 -ModelPath .\models\local\qwen3.5-0.8b-gguf\Qwen3.5-0.8B-Q8_0.gguf -ModelAlias alife-qwen3.5-0.8b-prior -GpuLayers 999 -Threads 4 -ContextSize 2048 -ParallelSlots 1
cargo build -p alife_semantic --features local-llamacpp --example local_slm_prior --offline -j 2
.\target\debug\examples\local_slm_prior.exe alife-qwen3.5-0.8b-prior 'heard word food; sees food nearby; hunger high'
```

The example makes one real request through the existing bounded production provider and prints its validated result. It creates no organism and performs no training. Food, toy, and opaque-wall contexts all returned distinct valid associations after removing a hardcoded food/hazard example from the prompt: the original example had caused the tiny model to invent a hazard in the food scene. Revised outputs did not repeat that error in these three samples. Six existing provider/parser checks also passed. Receipts and service configuration are under `target/slm-prior/qwen3.5-0.8b-20260928/`; the revised sample files end in `-prior-revised.json`.

The original CPU responses took roughly 7–9 seconds, with some build contention during collection. A subsequent same-context comparison without the build took 6,341 ms on CPU and 1,359 ms with GPU offload, producing the same validated prior. Reported GPU memory usage rose from 678 MiB to 1,594 MiB (about 0.9 GiB). This single-request check supports using GPU for the current trial; it does not measure concurrent training throughput, compare quality against 4B, or prove creature improvement. Receipts are `food-prior-cpu-warm.json` and `food-prior-gpu.json`. If shared GPU contention later reduces training throughput, `-GpuLayers 0` restores the measured CPU configuration. The subsequent runtime repair connects asynchronous requests, caching, expiry and dropout to separate neural inputs; see the dated repair evidence. No founder training campaign has started.

## Terrain founder and training

Terrain-vision adaptation is explicit. Before resuming the old object-slot founder,
run `train_n2048_care --adapt-terrain SOURCE_COHORT NEW_DIRECTORY`. This accepts a
sealed old cycle, preserves every weight at identical inherited synapse coordinates,
creates a different profile-bound asset, and records `adaptation.json`. Old Adam
moments and the value head are reset because observations and biology changed.
The first resumed cohort installs current genetic founder chemistry (separate
tiredness, slowed sleep metabolism, and player praise). Source assets, saved
individuals, and old receipts remain unchanged. Campaign `-Source` accepts the
adaptation directory. It is an unpromoted candidate.

The legacy adapted checkpoint is bundled at
`assets/founders/terrain-care-n2048-v1`. Its decoder predates inherited survival
salience and fails current exact candidate admission. After rebuilding, explicitly
revise it with `train_n2048_care --refresh-terrain-founder SOURCE NEW_DIRECTORY`.
This validates the legacy source, preserves all 32,768 weight bits and synapse
coordinates, records both decoder identities in `founder-refresh.json`, and checks
ordinary strict admission and exact compiler reconstruction. The new candidate is
unpromoted; optimizer and value state reset. Source files and saved individuals
are retained. Use the revised directory for fresh lessons and warm-up; previous
corpora do not match its new identity. This founder revision does not establish
admission of descendants whose inherited decoder genes differ.

Fresh training uses GroundedTerrainVisionV1. Candidate New Game launches derive
their sensing profile from the selected asset; the legacy builtin Nano512 option
remains available. Select the adapted N2048 asset explicitly for a vision founder.
No trained founder is promoted by changing a default or relabeling an old asset.

For terrain demonstrations, use `train_n2048_care --teacher-adapted ADAPTATION_DIRECTORY OUTPUT_DIRECTORY TICKS WORLD_SEED obstacle_navigation`.
This retains the adapted founder's inherited seed while varying the world layout.
An imitation manifest must name that same foundation in its optional `source_asset`
field (relative to the manifest, or an absolute path); replay still checks every
demonstration's founder identity and foundation digest. Do not feed the old
object-slot corpus into terrain warm-up.

After rebuilding and revising the source, `scripts/collect_n2048_care_lessons.ps1 -Source REVISED_DIRECTORY -Output target/founder-training/NEW_CORPUS`
defaults to the 86-demonstration `VisionLanguage` corpus: 14 care/vision/maze lessons,
36 reception lessons, and 36 production lessons. Each direction visits all eighteen
words twice, including real hunger/tiredness and food/toy subtypes.
`-ExamplesPerWord 1` gives a bounded 50-demonstration preparation corpus. Maze demonstrations
have a 1,024-decision cap, vocabulary 64, and other lessons 128. Meal lessons stop
when they eat; vocabulary records hearing, named actions, and speaking opportunities.
Use `-Curriculum Care` explicitly for the earlier balanced 32-lesson care corpus.
Imitation warmup accepts ten-category manifests, including `EatHeldFood` and
`GrabFood`, and mixes their windows with the other lessons while preserving one
loss budget per demonstration. Existing eight- and nine-category manifests remain
valid only when their omitted later categories have no demonstrations. This is
corpus admission and scheduling support; retained grabbing competence still
requires frozen behavioral evaluation. Warmup initializes its training heads;
use the sealed-cycle continuation path to preserve an existing campaign's actor,
value head and optimizer history.
For an opt-in GrabFood learning experiment, a sealed `--resume-cycle` can add
`--lesson grab_food --sampling-temperature 32 --acquisition-rehearsal-epochs 8`.
After the ordinary PPO update, the actor rehearses only the first captured loss
row that actually acquired food. Its sampled manipulation command is the label;
other motor factors and speech stay unconstrained. Replay retains up to 128
same-segment burn-in rows. Bootstrap and release/regrab events add no labels.
The explicit offline epoch budget is 1..=512. The larger ceiling allows a
separate dose comparison: the [October 8 trials](n2048-grab-rehearsal-dose-2026-10-08.md)
found that 32 updates still left very low likelihood on the successful actions.
Only budgets up to 32 have executed GPU qualification. Higher budgets need their
own bounded GPU trial and frozen GrabFood/Eat comparison; the ceiling does not
establish competence or enable a campaign.
The existing actor optimizer continues and the value checkpoint must remain
byte-identical during rehearsal. The cycle reports these extra offline epochs
separately; they add no distinct lives or real experience. This option changes
no ordinary runtime action policy and is not enabled in the prepared serial
campaign. GPU qualification and frozen GrabFood/Eat retention comparisons are
required before using it for a campaign or claiming improvement.
All records and separate speech labels bind to the same inherited asset and compiler.
Launch training from the qualified Git checkout. Replay source revision comes
from that launch checkout, rather than the compile machine's absolute path;
the operator still pins the source revision and executable hash.

The GrabFood night runner can opt into `--acquisition-rehearsal-epochs 512`
with `--sampling-temperature 32 --sampling-temperature-floor 1`
and `--max-stalled-batches 2`, after the chosen dose passes its private GPU and
frozen behavior comparison. These flags do not change legacy defaults.
Every credited acquisition must complete its reported offline dose, preserving
the value checkpoint and exact actor optimizer delta. A life without credit
performs zero rehearsal updates. The runner checks eating retention at every
batch, and halves exploration temperature only when ordinary-temperature
GrabFood and EatHeldFood both pass. It stops after the requested number of full
batches without frozen grabbing, rather than spending the whole window on an
unchanged failing recipe. Cooling changes collection and matching PPO temperature
together; frozen assessments stay at temperature 1. Extra offline updates remain
separate from distinct world lives and captured experience.

An explicit archived-success experiment can resume the current sealed cycle
with `--archived-success-manifest MANIFEST --archived-success-epochs 16`.
Use `--resume-cycle` with `--lesson grab_food`; archive practice and the
single-acquisition rehearsal flag cannot be combined in one invocation.
The schema1 manifest names 1..128 immutable older event records and their
actual parent checkpoints. Every selected Grab must match a real ownership
transition. Every selected Eat must follow acquisition and consume that same
target. The native loader checks the original record digests, actor/asset
bindings, recurrent continuity, and complete graph compatibility before any
extra update. Only genetic weights and their compiler identity may differ.
The in-memory training sequence then uses the current actor; original records,
policy identities, and rewards remain intact. Historical behavior probabilities
authenticate collection and are never treated as current PPO samples.

Archived practice uses temperature 1, eight-example effective batches, and
1..128 epochs. Acquisition and eating receive equal aggregate learning weight
when both are present. The receipt records the exact examples, manifest digest,
finite losses, completed actor updates, and preserved value checkpoint. These
are extra offline updates, not new lives. Repeat frozen acquisition and eating
checks after this private experiment; a source-bound replay run alone proves
neither transfer nor founder competence.

Collection pins the Git revision and executable hash and stops if either changes.
Keep the checkout and executable fixed until collection completes. Compact replay
schema 2 always writes candidate innate-bias fields, including zero values. Schema
1 records are rejected before body decoding; regenerate those corpora with the
rebuilt executable instead of rewriting their bytes or relaxing identity checks.

The September 28 language/prior encoder changes the compiled phenotype identity,
while retaining the 32,768 inherited weight coordinates. Recollect demonstrations
and make a fresh warm-up/optimizer handoff under the new compiler. Existing prepared
receipts, replay corpora, and old actor checkpoints do not qualify this integration.
The bundled adaptation can seed fresh individuals; do not blindly resume an old
warm-up or cycle across the changed encoder.

```powershell
Set-Location -LiteralPath 'D:\A life'
& 'C:\Program Files\PowerShell\7\pwsh.exe' -NoProfile -File .\scripts\run_n2048_care_training.ps1 -Mode Status
& 'C:\Program Files\PowerShell\7\pwsh.exe' -NoProfile -File .\scripts\run_n2048_care_training.ps1 -Mode Prepare
```

Before `Prepare`, the supervisor must release the Cargo/GPU lane. Do not run another Cargo command, game, GPU test, or training process alongside it. The launcher takes an exclusive file lock and rejects existing project processes; that lock only coordinates callers that respect it. It never kills processes.

Preparation builds the release `train_n2048_care` with `foundation-training`, runs five focused serial checks (phrase GPU gradients, gene-controlled object chemistry, indexed familiarity persistence, maze connectivity, and exact runtime save/resume), requires one executed passing test per check, and runs the supported `--pilot` command for a fixed 32 decisions. Cargo uses existing default features and dependencies offline, with two build jobs. The pilot checks replay fidelity; it does not prove learned gameplay. No source edits, downloads, legacy trainer fallback, or full test suites are performed.

Evidence lives in a fresh `target/founder-training/prepare-*` directory. `source/` records HEAD, the complete tracked binary diff, and hashes of nonignored new files under source/docs/scripts/assets directories. Artifact folders, Blender backups, and Python caches are excluded. Source identity is checked between phases and after the pilot. Each command has separate stdout/stderr logs. `status.json` is atomically published both in that directory and at `target/founder-training/status.json`.

`Prepared` means the listed gates and pilot succeeded for that exact source, executable, and receipt. The executable enforces replay tolerances; the launcher additionally rejects missing, negative, or nonfinite receipt metrics. Matching successful evidence is reused. `Failed` means stop and inspect the named phase and logs with the supervisor. Do not change tolerances, skip a gate, repeatedly retry, delete receipts, or modify source just to change its fingerprint. An interrupted `Running` receipt also needs supervisor diagnosis. Check status with the first command above.

`-Mode Campaign -Source target/founder-training/SEALED_CYCLE_DIRECTORY -DurationHours 8 -CycleTicks 256` rebuilds the release trainer from the current source, then runs one teacher-free lesson world per cohort. The default `VisionLanguage` schedule allocates 40% of cohorts to vision/maze, 40% to vocabulary reception/production, and 20% to feeding/hazard care. Maze cohorts collect at least 1,024 decisions; other cohorts use `CycleTicks`. The explicit `Care` schedule retains the older obstacle/feeding/hazard/recovery mix. Every cohort uses a distinct seed and records its actual food and obstacle positions; a repeated food position fails the campaign. Food and routes vary in direction, distance, and lateral offset. It carries the actor, value head, and optimizer state forward, seals the latest checkpoint after each cycle, keeps provisional care scores and separate frozen navigation/reception/production panels, and stops at its deadline or a failed handoff. `-Source` may also point to a sealed warm-up. `-MaxCycles 1 -CycleTicks 16` gives a small coordinator check. It never promotes a founder. The CLI can also run and resume individual bounded cycles. For a focused operator check, choose fresh output directories and run:

```powershell
cargo run -p alife_game_app --features foundation-training --bin train_n2048_care -- --cycle target/founder-training/cycle-001 4
target/debug/train_n2048_care.exe --resume-cycle target/founder-training/cycle-001 target/founder-training/cycle-002 4
target/debug/train_n2048_care.exe --inspect-cycle target/founder-training/cycle-001
```

The cycle collects ordinary GPU decisions, streams Zstd-compressed binary replay records with a 4 GiB disk ceiling and 8 GiB process-memory check, predicts frozen values, calculates GAE across natural sleep gaps using actual world time, applies PPO in 256-tick waking windows with up to 128 burn-in ticks, exports a canonical N2048 asset, and verifies its admission into a fresh cohort. Structural synapses remain frozen replay context while inherited weights train. The first waking decision after each sleep must pass exact GPU production/replay parity. The sealed experience patch retains its existing JSON wire inside each binary record. The second command restores the exported actor, Adam state, and GPU value head, and increments the policy version. `--seed N` may change the world seed on resume while preserving the original inherited founder seed and graph; `cycle.json` records both. `--inspect-cycle` reads and checks sealed replay records, including finalized records from an interrupted collection. Both training commands fail rather than overwrite an output directory. `cycle.json` and its actor/value checkpoints are integration evidence, not a full campaign checkpoint or a promoted founder. The cycle accepts at most 36,000 waking decisions. A production delayed-food run has now closed biological death as a terminal PPO trajectory with zero value bootstrap, exported a trained asset, and admitted its next cohort. A delayed-food invocation still exits as a failed behavior gate unless the organism survives and eats after food returns.

For a held-out check of a frozen checkpoint, use `--evaluate OUTPUT ASSET LESSON TICKS WORLD_SEED FOUNDER_SEED`. It performs ordinary production collection and replay verification without an optimizer update. Use seeds outside the campaign and demonstration bands. Reserve the long delayed-food gate for post-training acceptance.

`--food-after-world-tick N` in cycle mode temporarily moves the existing food resource to a distant legal world position, then returns it at world tick `N`. `scenario.json` records both positions, and `cycle.json` records the availability tick, first meal, and energy. It changes no physiology or policy. `--teacher-pilot OUTPUT [TICKS] --seed WORLD_SEED --founder-seed-base GENOME_SEED --lesson feeding|hazard_avoidance|obstacle_navigation|recovery` produces source-bound compact demonstration replay in the same format as the cycle. The optional `--food-position X Z` applies to feeding only; the second coordinate is horizontal Z, not elevation. `--cycle` and `--resume-cycle` also accept `--lesson` to place the same scenery around a teacher-free policy; a delayed-food gate cannot be combined with a lesson. Each teacher lesson must complete its measured world outcome before replay is accepted. Successful lessons write `lesson-trace.json`; failed lessons write `lesson-diagnostic.json`, both with every chosen action, sensed object slot, physical contact, and measured physiology.

The teacher's action sequence is a causal contract, not a script of desired labels alone:

| Lesson | Perceived trigger and action | World outcome required before the next choice |
| --- | --- | --- |
| Feeding | Approach the sensed food until contact, then Ingest. | Movement reaches food without a block; the ingestion receipt says Consumed and energy rises. |
| Hazard avoidance | Avoid the sensed nearby hazard; when its sensed distance is safe, approach the food behind the founder and ingest. | Distance from the hazard increases, there is no collision or block, and the creature resumes feeding instead of idling indefinitely. |
| Obstacle navigation | First see food; a gate closes after two world ticks. Retain the observed heading, read the coarse range fan, turn toward clearance, take short steps, reacquire food, then ingest at contact. | Initial food sighting, later occlusion, actual movement, a consumed meal, and no blocked action are required. Food/layout vary by seed; no invisible food candidate or obstacle waypoint is used by the terrain teacher. This is an introductory detour, not maze competence. |
| Recovery | Ordinary world aging first produces measurable fatigue; briefly Rest before searching for reachable food and ingesting. | Fatigue falls and the creature resumes movement and eats. Sleep supplies no meal energy; increased ATP is not a reward requirement. |

The teacher selects known world objects, while the learner sees physical object slots and sealed consequences, not the teacher's object labels. World-spawned food, hazards, and obstacles now retain distinct chemical ranges across spawn orders; other physical channels still vary by object. The 32-lesson corpus at `target/founder-training/demonstrations-20260924-006/manifest.json` balances eight examples per lesson, covers both obstacle sides and several fatigue levels, and requires hazard Avoid → Approach → Ingest and recovery Rest → Approach → Ingest sequences. These are demonstrations, not learned-behavior proof.

The first overnight campaign completed 528 teacher-free PPO cohorts on 25 September 2026 and sealed `target/founder-training/campaign-20260925-070114-57b51372/cycle-0527-recovery`. Its obstacle lesson averaged 2.70 meals per cohort and had six zero-meal cohorts; these are training-world observations, not proof of route finding in held-out worlds. Two new rotated teacher routes reached and consumed food without blocked actions. A 16-decision teacher-free coordinator check sealed its handoff and distinct-position receipt, but had no meal and four blocked actions; it is not a competence test. The next terrain campaign must use the explicit adapted source described above, not resume the old object-slot checkpoint directly or use the coordinator check. The September 28 [vision, maze, and vocabulary run design](n2048-vision-maze-vocabulary-run-2026-09-28.md) supersedes the proposed next-run schedule here; [bounded integration evidence](n2048-prior-maze-speech-repairs-2026-09-28.md) now covers the added paths, while learned competence remains Unknown. No run is launched by that design. The 20-minute food-free interval followed by autonomous feeding remains a separate post-training behavior gate, not a reason to teach extended inactivity. The earlier [training regimen](n2048-care-training-regimen-2026-09-21.md) records the underlying method and implementation history.

## Automatic behavior stages

The VisionLanguage campaign starts at stage 0. Every 40 completed cohorts, while
at least 20 minutes of collection time remain, it assesses the exported policy
without a demonstrator or optimizer: three navigation runs, two different requests
and silence in one identical scene, three production runs, and feeding/hazard care
checks. Navigation and production each require two of three successes; both care
checks must pass. Both requested first responses must be correct and different.
Inspecting every object eventually is insufficient. Production
requires exact meaningful emitted token sequences after the initial hearing
exposure, not a matching first word or HUD translation. An assisted pass then
requires two paired prior-off probes (navigation and production). A failure holds
the stage. These small screens are progression checks, not statistical acceptance.

Stage 0 uses small two-by-two maze trees, a food/toy request contrast, and
food/play/hunger production assessment. Stage 1 uses three-by-three trees, a
get/play request contrast for the same ball, and look/get/tired production.
Stage 2 adds root/fruit request contrasts and food/toy subtype production.
The request comparison changes only words: speaker, object geometry, biology,
founder weights and random seed stay fixed. Initial world signatures are checked.
Demonstrations and training vary actual connectivity using
two tree generators; mirrors and rotations alone do not count as diversity.
Training vocabulary still samples all eighteen words. No score authorizes founder
promotion. The food-free biology and player-visible acceptance gates remain.

Panels and their source digests are recorded separately. Ranking compares scores
only within the same stage and cannot exchange a language regression for meals.
Status retains compact prior metrics; full deliveries stay in sealed receipts.
The prior and stage environments are restored after every panel, including exceptions.

The campaign wall-time budget starts before its build. Collection stops 45 minutes
before the hard deadline, including any unfinished cohort; an interrupted directory
is never used as a next source. The runner then compares the starting source, latest
sealed checkpoint, and the best previously assessed checkpoint for the current stage.
Identical asset digests are assessed once. All candidates face the same current stage.
Navigation, request discrimination, literal speaking and care stay separate: a
candidate can replace the baseline only through componentwise improvement. Ties
and skill tradeoffs retain the baseline. `final-comparison.json`, `selectedCheckpoint`
and `selectedCheckpointPassed` distinguish selection from the latest trained source.
A timeout preserves sealed results and records incomplete assessment. No selection
authorizes founder promotion or replaces the player-visible/food-free gates.

Warmup speech loss gives positive utterances and meaningful silence equal budgets
within each replay window, normalized over their actually supervised outputs.
One completed verb therefore receives the same positive budget as repeated noun
labels in another comparable window. Labels still require physical grounding;
there are no fabricated extra utterances. `warmup.json` records the normalization.

Warmup now gives each complete demonstration one loss budget across all its
windows, interleaves lesson categories, and retains the full-batch gradient
divisor for a partial final batch. A campaign measures its starting asset in
the existing frozen behavior panel before collecting cohorts and stops if all
four skill scores are zero. Short calibration, demonstration collection and
warmup remain available for an untrained source. When the prior is enabled,
campaign and assisted-panel receipts must show delivery without provider
failures or prime timeouts; intentional whole-cohort dropout is distinguished
from an unavailable provider. These checks protect the collection budget;
they do not establish acquired behavior or founder promotion.

The offline teacher, warm-up, cycle, and frozen evaluation worlds now admit the
same grounded terrain rule as the playable game. This converts canonical legacy
X/Y fixture positions into horizontal X/Z positions and keeps movement on the
ground. Older replay corpora and warm-ups collected without terrain admission
are diagnostic artifacts only; collect a fresh corpus and warm-up before the
next campaign. The behavior panel checks two requests and a silent control in
the same scene, plus navigation, literal speech, and care on frozen weights.
Training-world scores alone still cannot certify a founder.

Exact saves retain bounded provider-private fade/context/active-hint state inside
the existing cognitive checkpoint. A provider identity mismatch fails restore
rather than silently changing the creature. In-flight requests are resubmitted;
network requests themselves are not serialized. Disabling the prior intentionally
ignores that private provider state. None of this private state enters a newborn.
