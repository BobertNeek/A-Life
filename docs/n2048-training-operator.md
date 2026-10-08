# N2048 training operator

Cassidy's first primitive is now available as `eat_held_food`. See the
[October 5 eating lesson and bounded night plan](reviews/2026-10-05-eat-held-food.md)
for legal held-food setup, contextual spatial speech without speech labels,
required semantic-prior input receipts, startup qualification and the separate
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
The existing actor optimizer continues and the value checkpoint must remain
byte-identical during rehearsal. The cycle reports these extra offline epochs
separately; they add no distinct lives or real experience. This option changes
no ordinary runtime action policy and is not enabled in the prepared serial
campaign. GPU qualification and frozen GrabFood/Eat retention comparisons are
required before using it for a campaign or claiming improvement.
All records and separate speech labels bind to the same inherited asset and compiler.
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
