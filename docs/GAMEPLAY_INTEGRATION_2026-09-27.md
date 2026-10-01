# Gameplay integration — 2026-09-27

Scope: the user's requested founder adaptation, navigation teacher/curriculum,
player care/reward, and visible embodiment. Repair existing world, biochemical,
trainer, input, and animation paths. This is not founder promotion or an overnight
training campaign. Source assets and unrelated Blender/cache files are preserved.

## Founder and training

Requirements: AOA-FOUND-001, AOA-SENSE-001/002, AOA-AUTH-001 through 005,
AOA-GEN-001/007/008, AOA-PERSIST-001/003.

Explicit `--adapt-terrain` accepts the sealed object-slot cohort, reconstructs its
native inherited graph, compiles terrain sensing, and checks every source/target,
route, synapse kind, and persistent address-map digest. Every learned foundation
weight is retained bit-for-bit. The new asset has a different sensing identity;
its receipt binds source and target digests and the inherited founder seed. Old
actor moments and value estimates are deliberately reset. No lifetime memory,
body state, or saved genome is silently rewritten.

Fresh terrain cohorts use current calibrated genetic chemistry, including waking
tiredness, reduced sleeping upkeep, and care signals. N2048 candidate New Game,
pilot, warm-up, initial/resumed PPO, and successor admission derive their profile
from the actual asset. Warm-up manifests may name `source_asset` so imitation can
retain the adapted weights. Current lesson campaigns reject an unadapted legacy
source. The old builtin Nano512 remains available, not silently promoted/replaced.

`--teacher-adapted SOURCE OUTPUT TICKS WORLD_SEED LESSON` produces demonstrations
from the adapted foundation, with its original inherited seed and new world seed.

## Introductory navigation

Requirements: AOA-WORLD-006, AOA-SENSE-001/002, AOA-MOTOR-001/004/007,
AOA-LEARN-010, AOA-GAME-002.

The teacher's state is separate from the learner. It knows the scenario's food
identity for demonstration selection, but chooses hidden-food movement only from
a previously observed food vector, sensed movement/orientation, and the local range fan. It
cannot select an invisible food candidate. With no sighting it scans its head,
recenters, and turns its body. Remembered food distance/bearing is updated by
actual interval displacement, so a detour does not turn into endless movement
along the original bearing. It ingests only at sensed contact. Clearance causes
body turns and short forward detours; reacquired food updates the remembered
heading. The introductory detour uses a bounded short clearance sequence, not a
general path planner or a runtime NPC policy. Unused orientation holds gaze.

Food direction, distance, and lateral offset vary by seed. In the first wall
lesson a closing gate begins away from the sight line, then moves into it after
two world ticks; its placement leaves room beyond the initial short approaches.
This deliberately teaches sighting → occlusion → detour → reacquisition → meal.
Teacher acceptance requires initial visibility, a later missing food observation,
actual movement, consumption, and no blocked actions. The teacher-free cohort
uses the same scenery and gate; no teacher state or answers enter the learner.
Other lessons search with terrain senses after hazard separation or brief recovery.

## Care and deliberate reward

Requirements: AOA-GAME-001/002/007, AOA-SENSE-001, AOA-BIO-011/018,
AOA-AUTH-001 through 005, AOA-LEARN-010, AOA-PERSIST-003.

The user selected gentle touch, optional play, and a separate deliberate praise
reward. Ordinary controls: select a creature; C touches gently; J offers/repositions
one reusable physical plaything; K praises. Text entry blocks these shortcuts.
The Hand acts at the selected world position. Touch requires physical reach and
unblocked contact; praise requires local range and an unblocked source path.
Dead/missing targets are rejected. Play offers ordinary investigation/contact
opportunities and never selects the creature's action or forces enjoyment.

The world owns a bounded, saved pending care stimulus per living individual.
Duplicate presses before one tick do not stack their dose. Normal action biology
or passive biology consumes it exactly once at the next boundary, inside existing
rollback authority. Tactile/auditory channels carry the corresponding cue. No UI
copy of chemistry advances independently, and no host reward/gradient is injected.

Touch uses existing social-contact emitters. New founder alleles express this
care circuit in newborns too, instead of waiting for mature expression. Praise
uses an inherited PlayerReward emitter into a short regulatory pulse (chemical
21, retention .25, default release .6), an inherited endocrine Extension0 receptor,
and the inherited value-profile weight .3. Both homologs receive the construction
genes; ordinary genetic expression/recombination/mutation remain in charge.
Positive pulse arrival contributes measured reinforcement. Pulse clearance ends
reward without punishing the next action. Existing eligibility supplies temporal
credit; this does not label or force a particular next action. The player panel
shows actual praise concentration alongside separate energy/tiredness/needs.

## Visible embodiment

Requirements: AOA-INV-003/011, AOA-AUTH-002, AOA-MOTOR-004, AOA-GAME-007.

Astra repaired the existing skinned Hearthling projection. Authoritative body yaw
sets model facing even during a turn without translation. Authoritative head
offset replaces the authored idle yaw after animation sampling, retaining nod
and roll. Repeated projection does not accumulate yaw. The existing gait follows
actual displacement, so biological locomotor limits reduce visible stride travel;
real sleep still selects the existing sleep clip. No GLB, manifest digest, motor
controller, or biology was replaced by animation state.

## Verification and limits

Executed evidence:

- Combined foundation-training/Bevy/production-assets binaries build successfully.
- Explicit adaptation preserved all 32,768 weight bits and checked identical
  synapse coordinates. The unpromoted asset and receipt are bundled at
  `assets/founders/terrain-care-n2048-v1`.
- Final navigation layouts at seeds 539366000 and 539366002 completed in 20 and
  22 decisions, respectively. Each passed sighting/occlusion/reacquisition gates,
  consumed one meal, and had no blocked actions. GPU replay maximum logit error
  was 0.0000038146973. Receipts: `terrain-teacher-20260927-004/005` under
  `target/founder-training`.
- Hazard avoidance completed in 13 decisions at seed 539366010; recovery
  completed in four decisions at seed 539366013 after ordinary biological
  fatigue preconditioning. Both consumed one meal and passed their distinct
  lesson gates and replay parity. These are bounded introductory checks,
  not the complete 32-demonstration corpus.
- A four-decision teacher-free PPO cycle sealed its exported checkpoint and
  verified the next cohort's decision capture and optimizer rebound:
  `target/founder-training/terrain-handoff-20260927-006`. It had no blocked
  actions or deaths; this small integration check consumed no meal.
- Focused existing world tests passed for gene-controlled touch/praise, one-time
  consumption on direct and passive action paths, pending-care save restoration,
  and praise clearance without negative reinforcement. The aligned food-bearing
  regression passed. The focused animation yaw/gaze test and shipping GLB
  animation validator passed.
- A 90-second GPU-required ordinary New Game smoke with the bundled N2048
  candidate exited successfully on the RTX 3050. The real Hearthling rendered.
  Evidence: `target/founder-training/terrain-visual-20260927-002` (isolated
  save/assets). The first launch exposed a stale Nano512-only frontend exact-save
  check; it now requires the explicitly selected founder class while retaining
  exact cognitive checkpoint, population, seed, and GPU validation. Desktop
  window discovery/control timed out, so interactive C/J/K and live head-turn
  acceptance remain unverified. This smoke is not sustained gameplay evidence.
  It used an unoptimized, training-enabled debug executable and collected no
  frame timings. The player reported extreme game lag, so its successful exit
  must not be treated as acceptable responsiveness.

The learner check exposed floating-point bearing components a few ULPs outside
the normalized range. Sensing now clamps its calculated normalized components;
strict perception validation remains intact. The trainer reports the failing
preparation stage and fails immediately on rejected perception instead of
silently advancing passive world ticks without collecting decisions. Interrupted
diagnostic runs remain preserved.

Adaptation retains weights; it
does not establish competence with changed senses. Demonstrations prove the
teacher's physical sequence, not learned navigation. Direct praise is an effective
biological stimulus, not proof of a trained response. Wall-time survival, learned
sleep/restart retention, held-out navigation, and ordinary player-visible acceptance
remain separate. No maze curriculum or complete articulated manipulation is claimed.

## Lag follow-up

Requirements: AOA-PERF-001/002/003, AOA-AUTH-002, AOA-PERSIST-001/003.

The reported lag was reproduced with an optimized production frontend, Vulkan,
the same adapted founder, one creature, and 1280x720 MinimumSettings30x30.
The 60-second receipt at `target/founder-training/lag-profile-20260927-001/performance.json`
records 482 frames: median 118.714 ms, p95 164.208 ms, p99 240.544 ms,
maximum 335.687 ms; every measured frame exceeded 100 ms. It completed 483
world ticks (8.043 TPS against configured 20). Runtime tick work consumed 56.5
of the measured 60 seconds; persistence waits and ordinary full-brain snapshots
were zero. This falsifies the claim that a release launch alone resolves lag.

The existing organism validator repeatedly hashes the complete immutable genome
and re-expresses its phenotype for sleep, action, energy accounting, and cognitive
seals. The focused repair retains that proof in the existing organism record.
Genome/phenotype fields are private and have no mutation path after construction.
The proof is process-local, excluded from serialization and canonical equality;
deserialized records must validate genetics afresh. Mutable state, inherited
digest binding, chemistry, sleep, body, lifecycle, and rollback validation remain
on every relevant operation. No action, learning, biological, or tick-rate rule
changes. The existing release-default launcher now forwards explicit founder
and isolated settings paths, and rejects debug performance recording up front.

The matched post-repair 60-second receipt at
`target/founder-training/lag-profile-20260927-002/performance.json`, built from
`3a29daabcd7b16046bea56e1246876483547c14b`, records 5,286 frames
(88.1 FPS): median 6.130 ms, p95 31.086 ms, p99 34.188 ms, maximum
99.135 ms, and zero frames over 100 ms. It completed 1,200 world ticks
(19.998 TPS against configured 20), with no dropped catch-up ticks. Mean
runtime tick work fell from approximately 117 ms to 22.9 ms. GPU cognition
and learning remained active; ordinary full snapshots and persistence waits
remained zero, and shutdown drained successfully.

One focused regression passed for fresh-load validation, canonical equality,
unchanged biological advancement, and rejection of forged genetic bindings
and phenotype data. Release build, core boundary check, documentation check,
and launcher preview/debug-recording guard passed. Windows MCP captured the
game window; attempted keyboard interaction was not visibly verified. This
measurement covers one creature at the graphics floor, not larger populations,
default graphics settings, long-term survival, or player-care acceptance.

## Visible care follow-up

Requirements: AOA-GAME-001/002/007/009, AOA-INV-003/011, AOA-AUTH-002.

The current HUD is a temporary development interface, not an approved visual
design. This repair makes its feedback functional without redesigning it.
Accepted touch and praise now show a queued acknowledgement, including a resume
hint while paused; play acknowledges an offer without claiming enjoyment. Missing
creature selection has a direct explanation. The existing live food projection
also draws the canonical player plaything as a blue ball, follows repositioning,
and removes unavailable objects. It does not create a second toy simulation.

The selected creature panel retains a last confirmed response from the existing
sealed action receipt or an observed change in inherited chemistry. It separates
confirmed consumption, blocked attempts, plaything inspection, and rises
in praise/comfort signals from the player's offered stimulus. No action, affect,
learning reward, or animation is forced. The existing optional bounded action
trace includes body energy, hunger, comfort/praise concentrations, and lifetime
learning update counts to inspect the same live sequence without extra readback.

The two focused care-feedback regressions passed, as did the production release
build, core boundary check, and documentation check. The release run at
`target/founder-training/care-visible-20260927-002/actions.jsonl` confirms a sealed
meal at tick 3 with immediate energy and hunger changes. This was existing spawn
food, not proof of player-placed food or Hand care. The requested manual care
sequence was interrupted by the player's observation that movement is too fast
for practical care; touch, praise, play, and player-placed feeding acceptance
remain pending.

A follow-up separates a current blocked attempt from the retained last care
response, and gives observed praise/comfort increases priority when another
motor channel is blocked. Its focused regression passed; that follow-up has not
yet been rebuilt into the live release.

The live scheduler is paced at 20 world ticks per second at 1x, rather than
unbounded execution. Existing speed controls accelerate the same sequence by
2x through 4x. However, locomotion currently permits up to one world unit per
tick: the observed policy moved approximately 0.7 units per tick, or 14 units
per wall second. Care pacing requires a decision between correcting physical
walking speed while keeping the established biological clock, and rebasing the
whole simulation clock. Neither clock nor locomotion behavior has been changed
in this follow-up.

## Care pacing and playback modes

Requirements: AOA-TIME-001/002/004/005, AOA-AUTH-002, AOA-PERSIST-001/003.

Normal mode keeps the established 20 world ticks per second. Walking now uses a
50 ms physical interval with a two-world-unit-per-second reference ceiling,
rather than a whole unit per decision. The existing inherited embodiment gain,
locomotor chemistry, organ reserve, and integrity still reduce commanded motion.
Movement legality, carried-object movement, and measured feedback use the same
world executor in all hosts. This changes navigation distances per decision;
older navigation scores do not certify behavior under the corrected pacing.
Hunger, energy, tiredness, learning, and ageing rates are not slowed or retuned.

The only modes are `1x`, `max`, and `headless-max`. Keys 1 and 2 select normal
and graphical maximum speed; brackets are aliases for these two modes. Maximum
speed is unpaced, with bounded batches yielding to input/rendering after 8 ms
of tick work (a slow tick finishes before yielding), and immediate presentation.
It is not a fixed 2x, 3x, or 4x multiplier. Pause and single-step remain separate.
Animation time derives from completed authoritative intervals, not a pretend
multiplier saved in settings. Older accelerated UX settings migrate to Max speed.

Key 3 starts headless maximum speed, closes the window, and transfers ownership
of the same live GPU runtime after the graphical schedule finishes. It does not
rebuild the brain or world. Startup can instead use `--run-mode headless-max`, or
`scripts/run_production_voxel_frontend.ps1 -RunMode headless-max`. This path uses
the same canonical admission and tick method without creating rendering plugins.
The headless loop prints a world-specific `.stop` path. Creating that file stops
at a tick boundary, requests the existing manual checkpoint, drains publication,
and requires a completed checkpoint at least as recent as the final world tick.
It writes a compact `.headless.json` receipt beside that checkpoint. A bounded
run can use the existing `--smoke-seconds` / `-SmokeSeconds` option. Restore uses
the existing canonical world/load path; selecting New Game still creates a new
world. A headless run must finish before resuming that world in another process.

Three focused existing regressions passed: movement interval distance, carried
objects, stationary/blocked velocity and joint motor execution; normal cadence,
unpaced scheduling, pause, single-step and debt reset; and UX migration plus all
three persisted modes. The optimized release built successfully. In isolated
one-founder graphical runs, normal mode measured 19.99985 ticks/second over the
stable 19.8-second interval; Max measured 28.0966 ticks/second. Both capped observed
horizontal displacement at 0.100003 units per tick (floating-point rounding).
These are hardware measurements, not promised acceleration factors.

The rebuilt live HUD retained an observed "Ate food" response beside a later
blocked attempt. This was spawn food: direct player touch, praise, play and placed
food still need a manual end-to-end care check. The graphical key-3 ownership
handoff also remains unverified by actual keyboard input.

The initial headless stop uncovered a portable-memory export failure: valid live
sensory arithmetic can produce negative zero, but portable import rejects that
noncanonical encoding. Export now normalizes signed zero consistently with the
existing canonical digest. The existing memory roundtrip regression reproduced
the failure before repair and passes afterward, preserving recall digest and
tamper rejection. The repaired optimized release ran headless from tick 0 to
306 in 8.016 seconds (38.1728 ticks/second), published its final checkpoint and
exited successfully. The ordinary LoadExisting path then restored that checkpoint
at tick 306, continued to tick 350, and published another completed checkpoint.
The saved world identity and Standard2048 class persisted. These runs did not
create a render window. Evidence is under
`target/founder-training/care-paced-release-20260927/headless-fixed/`;
the first receipt is retained as `first-headless-receipt.json`, with the original
checkpoint as `before-resume.json`. Core boundary checks and documentation
assertions passed. This does not certify long food-free survival, navigation under
the smaller movement interval, or the pending direct-input care/handoff checks.

## Manual care evidence at 1x

Requirements: AOA-GAME-001/002/007/009, AOA-BIO-018, AOA-AUTH-002.

The player exercised the ordinary controls in the optimized release at source
`6e927914db1bb0a4bc3853f1ed692d567792faea`. F12 captures and the passive tick trace
are retained under `target/founder-training/care-player-20260927-003/`.
`touch.png` shows the selected founder's confirmed "Comfort signal rose" response.
The player reported the creature approaching placed food. At tick 794 the sealed
world receipt records consumption of player food entity 6: energy changed from
0.767811 to 0.823470 and hunger from 0.421051 to 0.344501. The food object's consumed
flag changed at that boundary, and `player-food.png` retains "Ate food" on the HUD.
This closes the observed touch/feeding path beyond the earlier spawn-food smoke.

The player reported successful offers, but the available trace through tick 1181
contains no praise pulse or representative token interaction. Praise delivery and
the play offer/response therefore remain unverified; a targeted clarification is
pending. Active learning updates are recorded, not proof of retained behavior.
Sky window discovery failed. Windows MCP mouse focus reached the left monitor;
its single-letter shortcuts send Unicode text and did not activate physical-key
care bindings. This is an input-evidence limitation, not a demonstrated chemical
failure. No behavior was changed to manufacture a passing receipt.
