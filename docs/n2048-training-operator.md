# N2048 training operator

Use PowerShell 7 from the authoritative repository root. The launcher has separate fidelity preparation and bounded campaign modes. Neither a completed teacher corpus nor a successful training cycle by itself proves learned care.

Terrain-vision adaptation is explicit. Before resuming the old object-slot founder,
run `train_n2048_care --adapt-terrain SOURCE_COHORT NEW_DIRECTORY`. This accepts a
sealed old cycle, preserves every weight at identical inherited synapse coordinates,
creates a different profile-bound asset, and records `adaptation.json`. Old Adam
moments and the value head are reset because observations and biology changed.
The first resumed cohort installs current genetic founder chemistry (separate
tiredness, slowed sleep metabolism, and player praise). Source assets, saved
individuals, and old receipts remain unchanged. Campaign `-Source` accepts the
adaptation directory. It is an unpromoted candidate.

The verified adapted checkpoint is bundled at
`assets/founders/terrain-care-n2048-v1`. Use that directory, or a sealed terrain
warm-up/cycle derived from it, for the next campaign. Do not resume the old
object-slot checkpoint directly into these lessons.

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

After rebuilding the release CLI, `scripts/collect_n2048_care_lessons.ps1 -Source assets/founders/terrain-care-n2048-v1 -Output target/founder-training/NEW_CORPUS`
collects the balanced 32-lesson terrain corpus from this explicit source. Every
lesson stops at its measured meal, within a 128-decision cap. Its existing checks
now accept needed scan/body-turn actions between avoidance or recovery and eating,
and bind every demonstration and the warm-up manifest to the adapted asset.

```powershell
Set-Location -LiteralPath 'D:\A life'
& 'C:\Program Files\PowerShell\7\pwsh.exe' -NoProfile -File .\scripts\run_n2048_care_training.ps1 -Mode Status
& 'C:\Program Files\PowerShell\7\pwsh.exe' -NoProfile -File .\scripts\run_n2048_care_training.ps1 -Mode Prepare
```

Before `Prepare`, the supervisor must release the Cargo/GPU lane. Do not run another Cargo command, game, GPU test, or training process alongside it. The launcher takes an exclusive file lock and rejects existing project processes; that lock only coordinates callers that respect it. It never kills processes.

Preparation builds `train_n2048_care` with `foundation-training`, runs exact serial gates (including the PPO GPU objective and selective merge), requires one executed passing test per gate, and runs the supported `--pilot` command for a fixed 32 ticks. Cargo uses existing default features and dependencies offline, two build jobs, and the development profile. This is correctness evidence, not a release-performance measurement. No source edits, downloads, legacy trainer fallback, or full test suites are performed.

Evidence lives in a fresh `target/founder-training/prepare-*` directory. `source/` records HEAD, the complete tracked binary diff, and hashes of nonignored new files under source/docs/scripts/assets directories. Artifact folders, Blender backups, and Python caches are excluded. Source identity is checked between phases and after the pilot. Each command has separate stdout/stderr logs. `status.json` is atomically published both in that directory and at `target/founder-training/status.json`.

`Prepared` means the listed gates and pilot succeeded for that exact source, executable, and receipt. The executable enforces replay tolerances; the launcher additionally rejects missing, negative, or nonfinite receipt metrics. Matching successful evidence is reused. `Failed` means stop and inspect the named phase and logs with the supervisor. Do not change tolerances, skip a gate, repeatedly retry, delete receipts, or modify source just to change its fingerprint. An interrupted `Running` receipt also needs supervisor diagnosis. Check status with the first command above.

`-Mode Campaign -Source target/founder-training/SEALED_CYCLE_DIRECTORY -DurationHours 8 -CycleTicks 256` rebuilds the release trainer from the current source, then runs one teacher-free lesson world per cohort. The next campaign allocates half its cohorts to obstacle navigation, interleaved with feeding, hazard avoidance, and brief recovery. Every cohort uses a distinct seed and records its actual food and obstacle positions; a repeated food position fails the campaign. Food and routes vary in direction, distance, and lateral offset. It carries the actor, value head, and optimizer state forward, seals the latest checkpoint after each cycle, keeps provisional best receipts per lesson, and stops at its deadline or a failed handoff. `-Source` may also point to a sealed warm-up. `-MaxCycles 1 -CycleTicks 16` gives a small coordinator check. It never promotes a founder. The CLI can also run and resume individual bounded cycles. For a focused operator check, choose fresh output directories and run:

```powershell
cargo run -p alife_game_app --features foundation-training --bin train_n2048_care -- --cycle target/founder-training/cycle-001 4
target/debug/train_n2048_care.exe --resume-cycle target/founder-training/cycle-001 target/founder-training/cycle-002 4
target/debug/train_n2048_care.exe --inspect-cycle target/founder-training/cycle-001
```

The cycle collects ordinary GPU decisions, streams Zstd-compressed binary replay records with a 4 GiB disk ceiling and 8 GiB process-memory check, predicts frozen values, calculates GAE across natural sleep gaps using actual world time, applies PPO in 256-tick waking windows with up to 128 burn-in ticks, exports a canonical N2048 asset, and verifies its admission into a fresh cohort. Structural synapses remain frozen replay context while inherited weights train. The first waking decision after each sleep must pass exact GPU production/replay parity. The sealed experience patch retains its existing JSON wire inside each binary record. The second command restores the exported actor, Adam state, and GPU value head, and increments the policy version. `--seed N` may change the world seed on resume while preserving the original inherited founder seed and graph; `cycle.json` records both. `--inspect-cycle` reads and checks sealed replay records, including finalized records from an interrupted collection. Both training commands fail rather than overwrite an output directory. `cycle.json` and its actor/value checkpoints are integration evidence, not a full campaign checkpoint or a promoted founder. The cycle accepts at most 36,000 waking decisions. A production delayed-food run has now closed biological death as a terminal PPO trajectory with zero value bootstrap, exported a trained asset, and admitted its next cohort. A delayed-food invocation still exits as a failed behavior gate unless the organism survives and eats after food returns.

For a held-out check of a frozen checkpoint, run `--resume-cycle` from that checkpoint with a seed outside the campaign sequence, read the pre-update actions and outcomes in `cycle.json`, and do not use the evaluation run's exported checkpoint as the next training source. This reuses production collection without letting evaluation updates affect the candidate. Reserve the long delayed-food gate for post-training acceptance.

`--food-after-world-tick N` in cycle mode temporarily moves the existing food resource to a distant legal world position, then returns it at world tick `N`. `scenario.json` records both positions, and `cycle.json` records the availability tick, first meal, and energy. It changes no physiology or policy. `--teacher-pilot OUTPUT [TICKS] --seed WORLD_SEED --founder-seed-base GENOME_SEED --lesson feeding|hazard_avoidance|obstacle_navigation|recovery` produces source-bound compact demonstration replay in the same format as the cycle. The optional `--food-position X Z` applies to feeding only; the second coordinate is horizontal Z, not elevation. `--cycle` and `--resume-cycle` also accept `--lesson` to place the same scenery around a teacher-free policy; a delayed-food gate cannot be combined with a lesson. Each teacher lesson must complete its measured world outcome before replay is accepted. Successful lessons write `lesson-trace.json`; failed lessons write `lesson-diagnostic.json`, both with every chosen action, sensed object slot, physical contact, and measured physiology.

The teacher's action sequence is a causal contract, not a script of desired labels alone:

| Lesson | Perceived trigger and action | World outcome required before the next choice |
| --- | --- | --- |
| Feeding | Approach the sensed food until contact, then Ingest. | Movement reaches food without a block; the ingestion receipt says Consumed and energy rises. |
| Hazard avoidance | Avoid the sensed nearby hazard; when its sensed distance is safe, approach the food behind the founder and ingest. | Distance from the hazard increases, there is no collision or block, and the creature resumes feeding instead of idling indefinitely. |
| Obstacle navigation | First see food; a gate closes after two world ticks. Retain the observed heading, read the coarse range fan, turn toward clearance, take short steps, reacquire food, then ingest at contact. | Initial food sighting, later occlusion, actual movement, a consumed meal, and no blocked action are required. Food/layout vary by seed; no invisible food candidate or obstacle waypoint is used by the terrain teacher. This is an introductory detour, not maze competence. |
| Recovery | Ordinary world aging first produces measurable fatigue; briefly Rest before searching for reachable food and ingesting. | Fatigue falls and the creature resumes movement and eats. Sleep supplies no meal energy; increased ATP is not a reward requirement. |

The teacher selects known world objects, while the learner sees physical object slots and sealed consequences, not the teacher's object labels. World-spawned food, hazards, and obstacles now retain distinct chemical ranges across spawn orders; other physical channels still vary by object. The 32-lesson corpus at `target/founder-training/demonstrations-20260924-006/manifest.json` balances eight examples per lesson, covers both obstacle sides and several fatigue levels, and requires hazard Avoid → Approach → Ingest and recovery Rest → Approach → Ingest sequences. These are demonstrations, not learned-behavior proof.

The first overnight campaign completed 528 teacher-free PPO cohorts on 25 September 2026 and sealed `target/founder-training/campaign-20260925-070114-57b51372/cycle-0527-recovery`. Its obstacle lesson averaged 2.70 meals per cohort and had six zero-meal cohorts; these are training-world observations, not proof of route finding in held-out worlds. Two new rotated teacher routes reached and consumed food without blocked actions. A 16-decision teacher-free coordinator check sealed its handoff and distinct-position receipt, but had no meal and four blocked actions; it is not a competence test. The next terrain campaign must use the explicit adapted source described above, not resume the old object-slot checkpoint directly or use the coordinator check. The September 28 [vision, maze, and vocabulary run design](n2048-vision-maze-vocabulary-run-2026-09-28.md) supersedes the proposed next-run schedule here; its additional lesson and speech integrations are not implemented yet. No run is launched by that design. The 20-minute food-free interval followed by autonomous feeding remains a separate post-training behavior gate, not a reason to teach extended inactivity. The earlier [training regimen](n2048-care-training-regimen-2026-09-21.md) records the underlying method and implementation history.
