# Core game loop delivery plan

Date: 20 September 2026. Source reviewed: `abeb8d84`.

This is a proposed implementation sequence, not a completion report or a new
architecture. The [controlling architecture](architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md),
especially sections 1.1, 29, 32, 35 and 38, remains authoritative. No game code,
training, or runtime validation was performed while preparing the original plan
on 20 September. Subsequent implementation and evidence are recorded below.

## The experience

Build a modern Creatures successor around persistent individuals: notice a need,
help or teach, watch the creature respond, see it make a later choice, and return
to the same individual with its experience intact. Later, raise a small community,
observe relationships and inherited differences, breed descendants, and preserve
useful abilities through explicit breed promotion.

The player's recurring decisions should be concrete: whom to help, what to offer,
where to put it, what to teach, when to leave a creature alone, and eventually
which lineages to develop. Creatures remain capable of independent action and
imperfect responses. A spoken command is perception, not remote control.

**First playable:** one creature, a useful habitat, care, one useful learned
behavior, sleep, and exact save/reload; then support the default small population.
**Generational playable:** relationships, viable offspring, lineage continuity,
and named breed promotion. This follows the care loop.

## Lessons from the Docking Station demonstration

Two local recordings were inspected as frame sequences, including a two-minute
demonstration sampled every two seconds. They had no audio. They showed creature
selection, an egg dispenser and eggs, room navigation, machinery and contextual
panels, lift travel, and speech about needs, actions, and other creatures.
They do not establish the underlying learning mechanism or long-term retention.

Local references, kept outside Git:

- `C:\Users\PC\Videos\Captures\docking-station-study-2026-09-20-01.mp4`
- `C:\Users\PC\Videos\Captures\docking-station-demonstration-2026-09-20-194533.mp4`

| Observed lesson | Application to A-Life |
| --- | --- |
| Creatures and useful objects occupy a compact, readable habitat. | Start with reachable food, space to rest, and a few useful objects before expanding the map. |
| Devices produce tangible results, such as an egg in the habitat. | Care tools operate on real world objects; food placement has an understandable source/tool and visible result. |
| Speech beside creatures refers to needs, actions, and individuals. | Show readable live condition and attention near the creature; distinguish genuine speech from UI explanations. |
| Portraits, appearances, and markers identify individuals. | Keep selection/follow easy, with persistent names and recognizable appearance. |
| Contextual panels explain the inspected object. | Use short hover help, direct actions, and local failure messages rather than requiring debug mode. |
| Activity continues while panels are open and the player changes rooms. | Preserve autonomy and continuity during inspection and camera movement, subject to explicit pause rules. |

The hand interactions proposed below are design applications, not claims that
every gesture or its learning effect was demonstrated. Retain the approved
Hearthling/Creatures/Black & White direction. Do not copy Docking Station assets
or require its particular rooms, machines, or 2D presentation.

## Original source baseline: extend the existing game

| Source finding | Required work |
| --- | --- |
| `new_game_lifecycle.rs::stage_phase3_new_game` loads `builtin_nano512_v1`; the N512 exporter builds a procedural bootstrap. | Evaluate retained candidates and explicitly integrate a suitable founder into ordinary New Game. |
| `production_voxel_renderer/ux_input.rs` calls `GpuLiveBrainRuntime::place_player_food`; `live_food_projection.rs` displays canonical objects. | Improve the existing care path, rather than building another demo. |
| `bevy_shell.rs::reconcile_production_presentation` refreshes live needs and identities. | Improve interpretation and feedback; do not rebuild the needs model. |
| `production_conversation_lineage_ui.rs::send_player_speech` uses height zero; hearing uses 3D distance. | Ground player speech on the actual terrain and establish that the intended creature hears it. |
| Sleep/checkpoint paths, `phase3_save_load_continuity.rs`, and candidate retention experiments exist. | Establish continuity for the actual playable founder; experimental success is not automatically New Game success. |
| World lifecycle, managed breeding, archives, and rendered births/retirements exist. | Repair the ordinary route where it fails, rather than reimplementing reproduction. |
| Ordinary errors redirect to F3 and some effect triggers remain launch-state based. | Explain care outcomes and failures in the player view using live state. |

These are the original 20 September source findings, before the repairs below.
The September 2 status page and older performance receipts are not current-build
playtest results.

## Implementation status — 21 September 2026

The production baseline at `decc7ebc` died at tick 164, approximately 16 seconds
after starting. Reserve exhaustion is the working causal inference; this
observation alone does not establish a complete cause or the intended care pace.

Integrated code now grounds player speech on authoritative positions, repairs
terrain food placement, supports selecting and moving loose food while preserving
its identity, and shows basic controls and care outcomes in the ordinary UI.
An explicit founder-candidate bridge, inherited log metabolic turnover, and a
saved temporary age-death flag are also integrated. New games disable age-only
death by default; explicit CLI enable/disable switches apply only to New Game.
Loading preserves the saved policy, and legacy saves with no flag retain their
previous age-death behavior. Maturation and starvation, injury, and organ-failure
deaths remain applicable.

The focused biological accounting check passed: 12,000 unfed active steps retain
positive reserves, the initial depletion estimate is within 10–30 normal-speed
minutes, and feeding restores material reserves. The existing world lifespan
regression also passed: disabling age-only death preserves development while
zero energy and failed organ health still cause death. Core boundary checks pass.

The four focused app checks also passed: spatial speech in flat/elevated worlds,
food placement and movement with identity/rollback checks, New Game CLI policy,
and founder/age-policy save roundtrip. A test-only private-helper call found during
compilation was corrected to use public asset identity fields. Six focused tests
were run in total; no broad test suite was run.

The updated Vulkan/RTX 3050 candidate trial completed with GPU-authoritative
cognition. It remained alive through the final trace at tick 918 / 49.45 seconds
of game elapsed time; a middle interval advanced at approximately 18.9 ticks/s.
The last natural save at tick 887 retained one living creature, energy 0.778,
health 0.370, and the disabled age-death setting. No food was consumed; the food
was carried. Four Rest-labelled trace rows had blocked joint motor outcomes;
the trace does not isolate Rest from concurrent channels, so it does not prove
Rest itself failed. Recovery/consolidation cycles occurred. This is survival
past the original blocker, not a completed care loop.

The trial video is local at
`target/artifacts/core-loop-candidate-20260920-212115/candidate-window.mp4`.
Native input was unavailable because Computer Use window discovery failed after
its documented recovery. The lower care-control strip was outside the useful
captured region, so its visual acceptance remains unverified. No observed
10–30-minute survival, teaching, natural restorative rest, or fresh-process
continuity of learned behavior is claimed. The minimum verification policy below
still applies.

The next bounded diagnosis after that trial was candidate choice versus motor-channel execution.
Source inspection confirms held food remains perceived and edible; the saved
apple was 1.0015 units away, inside the 1.25 eating radius. No representative Eat
choice appeared after pickup, but aggregate trace outcomes cannot isolate each
channel. Expose the existing manipulation command/receipt in the bounded trace
before changing legality or training. The measured health loss is explained by
32 hazard-contact ticks, not starvation. Hunger saturation is canonical inherited
chemical signalling, not proof of empty reserves; its calibration is a separate
care-readability issue. Do not compensate with another reserve multiplier or
forced feeding.

### Supervised preparation for brain training

The current bounded assignment is to repair the action path and biological
foundation, then discuss training before starting it. These are preparation
steps, not completion of the playable milestones below.

The existing world motor receipts now appear in the passive action trace beside
the requested motor channels. Representative candidate labels and aggregate
outcomes remain explicitly distinguished from individual channel execution.
This adds observation only: no new action selector, feeding controller, neural
readback, or persisted simulation state.

New founders inherit a lower EnergyDeficit-to-Hunger emitter gain and a small
basal OrganRepair receptor response. Hunger now distinguishes provisioned from
depleted reserves; existing damage-limited repair continues after acute pain
fades and spends body reserves. Both alleles carry the changes. Existing saved
genomes remain unchanged. Rest no longer grants an unearned positive energy
event; its existing recovery signal remains. Legacy genomes without the repair
receptor retain their historical core sleep-energy fallback.

The existing starting hazard was on the second founder's spawn position. It is
now away from the spawn group. The existing habitat regression verifies all
eight supported founder positions in flat and Highlands terrain, reachable
food, harmless idle ticks, and damage only after actual hazard contact.

Two focused world regressions passed after these changes: inherited biological
accounting (including 12,000 active unfed steps, hunger, Rest, and paid repair)
and starting habitat safety. This is deterministic production-rate accounting,
not an observed ten-minute GPU play session.

The same review found a second, independently evolving ATP ledger in the GPU
scheduler. Its fixed awake debit could exhaust a full budget in approximately
five seconds despite healthy canonical biology. Normal production now connects
to canonical biochemical BrainATP as a tick-bound affordability
projection. Measured neural work still pays the existing world-owned body
energy cost. The existing monotonic tick cursor prevents retry refills, durable
sleep holds preserve their budget, and restore retains the recorded boundary.
Explicit continuous-wake laboratory protocols retain their existing policy.
No second chemical compartment or sleep controller was added.
Requirement trace: AOA-BIO-001, AOA-TIME-003, AOA-PERF-002.

Seven selected regressions pass: the two world checks above, real-GPU ATP
budget/debit/restore guards, live canonical ATP binding, sleep retry atomicity,
motor-receipt trace sealing, and New Game founder/save roundtrip. The binding
test initially failed before its first tick because its old bare-agent fixture
was not admitted as a registered organism. That existing test now uses ordinary
New Game admission and passes. Only the corrected check was rerun. Core boundary
checks pass. No broad suite or training was run.

The repaired production trial at `6f8bb82` exited successfully on Vulkan / RTX
3050 with GPU-authoritative cognition. Its last trace was tick 1174 at 59.79
seconds; the 10–50 second interval ran at 19.900 ticks/s. The natural save at
tick 1111 retained health 1.0, injury 0, energy 0.76368, hunger 0.42703, and
biochemical ATP 0.44820. Two sleep cycles replaced the earlier repeated rapid
cycles. This short observation supports the biological pacing check; it is not
an observed 10–30 minute survival claim.

The trace recorded five Manipulation Eat requests and five matching executed
commands. Both food attempts occurred approximately 4–6 units from the food,
outside the 1.25 eating radius; the other requests targeted an obstacle twice
and a hazard. No `Consumed` contact occurred, food was never carried, and no Rest
primitive was requested. This run demonstrates a choice/approach competence
gap, not a dropped eating command. Do not repair it with forced consumption or
a second food-seeking policy. Training remains unstarted pending discussion.

Trial artifacts are local under
`target/artifacts/core-loop-repair-20260921-172321`. Desktop recording was
occluded by Chrome and does not establish game visuals or input usability.
A separate 20-second visual-only run used the existing in-engine screenshot
capture, without source changes. Its 96 captures show changing poses, position,
and needs, with the entire care-control strip visible at 1280x720. Rendered
positions track the authoritative positions with expected interpolation (maximum
horizontal gap 0.742 units, below one movement step). The resulting nominal
8 FPS frame sequence is a visual record, not wall-time/performance evidence.
It is under `visual-only-20260921-181602` inside the trial artifact directory.
Actual player input and fresh-process learned-state continuity remain unverified.

## Delivery sequence

### 0. Establish the ordinary play baseline

Use a fresh one-creature save in the existing production frontend. Preserve
existing saves. Try select/follow, food placement, speech, pause/resume, and
save/reload in one short session. Note the source, founder asset, save, adapter,
and backend using existing output.

Stop at the first blocking failure, identify its causal boundary, repair it,
and resume. Separate observed defects from untested behavior. Do not start a
general architecture audit, research corpus, or new test harness.

**Exit:** a reproducible player scenario and the first actual blocker. Launching
and displaying a creature alone do not establish the care loop.

### 1. Close basic care with a capable founder

Inspect the evidence for the best retained candidate. Try reachable food,
movement, eating, physiological recovery, and rest in the production terrain.
Distinguish failure to perceive or choose from failure to move, execute, or
benefit from an action. Repair that boundary before doing more training.

When a candidate succeeds, connect its explicit foundation identity and
compatible ABI to New Game, births, archive, and restore. Preserve builtin
identity checks and existing saves. If none succeeds, use bounded training for
the specific missing behavior and repeat the same small scenario. Do not assume
historic N2048 training proves current Nano512 competence.

Arrange existing resources into a compact safe starting habitat. Food must be
reachable and visibly placed, carried, consumed, and renewed where applicable.
Provide an opportunity to rest and recover. No hidden food-seeking script,
forced neural action, or silent physiological refill.

**Required care pacing (user update, 21 September):** calibrate ordinary
no-food survival toward **10–30 minutes** of wall time at normal speed (1x),
with **20 minutes** as the supervisor's working target. At 20 world ticks/second,
these observations correspond to 12,000–36,000 ticks, targeting 24,000 ticks.
Survival must emerge from inherited reserves, metabolism, activity, and organ
condition, not a countdown or guaranteed grace period. This is a calibration
requirement, not an achieved result or a prescription for a blanket cost scalar.
Reserve tuning does not establish cognition or replace real feeding benefits.

The user requires **no fixed age-only death**. Creatures still mature; aging may
affect biological function and maintenance, leading to organ failure. Starvation,
injury, and other world consequences can still kill them. This biological
lifespan choice is separate from the food-reserve target. While biological aging
is unfinished, a temporary program flag may disable age-only death while retaining
starvation, injury, and organ-failure deaths; no inherited immortality trait or
senescence rewrite is required now.

**Exit:** a hungry creature independently reaches and eats food, its real
body/chemistry benefits, and fatigue can lead to rest and recovery. It can cope
with food moved to another reachable position. Preserve basic exact restore
while integrating the founder; continuity is not postponed to a later rewrite.

Reuse New Game, world action/terrain, GPU runtime, and candidate/survival paths.

### 2. Make care direct and readable

Build a small set of interactions on existing world commands:

- Select, follow, and name an individual; keep its identity visible.
- Offer/place food through a clear hand/tool action. A simple food source can
  reuse existing placement authority; elaborate machinery is unnecessary.
- Pick up and reposition suitable loose objects with world validation and
  persistent identity. Creature carrying is a separate later choice.
- Add one gentle touch with a real sensed/contact consequence and observable
  response, without directly writing reward, chemistry, or neural state.
- Provide short help and immediate success/failure feedback near the action.

Expose hunger, fatigue, distress, sleep, attention, and observable action in
ordinary language. UI explanations describe authoritative facts; they must not
invent private thoughts or present translated speech as proof of understanding.
Drive relevant poses/effects from live state. Keep neural detail in expert UI.

**Exit:** a player can identify a need, offer help, see a response, and understand
refusal or failure without debug mode. Picking, speech focus, and pause cooperate.

Reuse production input, live projections, conversation UI, and the needs panel.
For substantive visual redesign, use the required Image 2/high-reasoning
blueprint and compare the rendered game. Small visual fixes need one focused
visual check, not a prototype or a unit-test suite.

### 3. Make one lesson change later independent behavior

Fix terrain-grounded speech, hearing range, and addressee feedback first. Choose
one useful lesson within the working perception/action repertoire, such as
approaching/eating a recognizable food or responding to a simple learned cue.

Let the player speak, arrange objects, demonstrate where supported, and give
grounded social feedback. Add only the interaction missing for this lesson.
A school editor, broad vocabulary, and an external language model are unnecessary.

Observe behavior before the lesson and after teaching stops. Move the relevant
object to distinguish a useful skill from a memorized position. Use a small
matched untrained comparison only if chance or a founder preference makes the
change ambiguous. Counters, acknowledgments, and one lucky action are insufficient.

**Exit:** the individual demonstrates a useful acquired change without the
teacher choosing its actions. Distinguish inherited ability from lifetime learning.

Reuse ordinary speech/perception and GPU learning; use structured education only
if the chosen lesson needs it. Reuse the existing retention scenario where suitable.

### 4. Preserve the taught creature through sleep and return visits

This is integrated acceptance of continuity maintained throughout earlier steps.
Let the taught creature enter sleep normally, consolidate, and wake. Save, close
the process, reopen the save, and observe the learned behavior in a fresh placement.
Include terrain and loose objects in the restored world.

Preserve identity, acquired weights/memory, body/chemistry, sleep, lineage, and
relevant player-visible state. Show save progress and failures clearly. A failed
load preserves the current game; missing cognitive dependencies cannot silently
produce a replacement creature. Reuse existing transaction checks.

**Exit: first playable.** Create, care, teach, observe an independent response,
sleep/wake, save, exit, reload, and observe retained ability through ordinary controls.

### 5. Support a small, understandable community

Extend the working scenario to the default six creatures. Provide reachable
resources and resting opportunities, quick selection, readable nearby speech,
and understandable competition/social contact. Expose actual individual
preferences and relationships; never manufacture them from random flavor text.

Add one reusable non-food activity, such as a simple toy, if the existing
object/action path supports it. Give the player another teaching opportunity
without building an elaborate content system.

Measure performance during this same play session. Proposed initial targets on
the named supported machine: 1080p, at least 30 FPS during ordinary play, roughly
20 world ticks/second at 1x, and control feedback within 250 ms. Record frame tails,
memory, and save stalls as well as averages. These are tunable proposed budgets,
not demonstrated results or architecture rules. Fix observed bottlenecks only.

**Exit:** several distinct creatures remain sustainable and legible while the
player helps one. Birth/death cannot leave stale selection, visible identities,
or GPU residents. Do not scale to hundreds before this group is pleasant to play.

### 6. Complete the generational loop

Expose the existing breeding path through parent selection, compatibility and
readiness feedback, and lineage inspection. Verify creature-chosen reproduction
where the habitat permits it. Ordinary breeding uses genotypes and fresh child
cognition, not a silent copy of acquired adult memory.

Make birth visible, with fresh identity and recognizable inherited differences.
A nursery or egg-like presentation is optional and follows actual lifecycle
authority. Include development, death, and a readable lasting life record.
Save/reload preserves the family, lineage, objects, and archives.

**Exit:** raise viable offspring from known parents, observe meaningful inherited
variation, and return to the same family after restarting. Preserve archive
before GPU admission and final life archive before retirement.

### 7. Preserve useful abilities as a named breed

Implement the [approved promotion scope](reviews/2026-09-11-breed-promotion-scope.md):
select one capable individual, name an immutable breed revision, and create a
fresh descendant from compatible retained abilities. Leave the source unchanged.
Exclude explicit personal episodes, relationships, individual/place references,
current chemistry/activation, and transient learning state.

Reuse foundation packaging and archive/import paths with a small extraction/reset
bridge. Cohort distillation and perfect removal of incidental weight associations
are not prerequisites.

**Exit: generational playable.** A fresh descendant retains one useful skill in
a fresh situation, starts with fresh personal records, and loads independently
of the source adult. Explicit promotion remains distinct from ordinary mating.

## Minimum necessary verification

User constraint: use the minimum number of tests needed to establish each change.
Testing is a means to get this loop working, not a separate expansion project.

| Change | Smallest meaningful verification |
| --- | --- |
| Plan/documentation only | Existing documentation check and whitespace/diff review. No Cargo, GPU, or game run. |
| Small UI/copy/visual change | Focused real-input or rendered check. No new unit test unless a specific logic defect warrants one. |
| World/action defect | One existing focused regression that exercises the causal failure; extend it only if it cannot catch the defect. |
| Food-reserve care pacing | One accelerated deterministic biological/accounting check using production rates, plus a short live timing confirmation of normal-speed tick cadence. No 20-minute GPU soak by default; these checks do not replace the behavioral care trial or proof of real feeding benefits. |
| GPU choice/learning/founder integration | The smallest existing real-GPU scenario for the affected path. Readbacks and counters alone cannot prove behavior. |
| Persistence, identity, or transaction change | Smallest existing roundtrip/rollback coverage for the affected state, plus the ordinary save/reopen step when that route changed. |
| Integrated milestone | One connected play session using the ordinary controls; combine learning, sleep, and restore observations in it. |

Before running a check, name the failure it can reveal. Reuse valid evidence and
existing fixtures. Add a new test only when no existing check can expose the
concrete defect or protect the changed persistent/public contract. No tests that
merely mirror implementation. No new harness, broad suite, benchmark matrix,
soak, ablation study, or comprehensive compliance campaign by default.

Run one focused check, repair failures, and rerun only affected checks. Do not
repeat a passing check without a relevant change or unresolved concern. Expand
only when an observed failure crosses another boundary. Run Cargo/GPU work
serially and honor an occupied shared build slot.

Use one evolving acceptance scenario across milestones, rather than separate
duplicated journeys. Record short video and existing causal output when needed;
keep large artifacts outside Git. Acceptance must still distinguish learned
behavior from chance, exact continuity from reconstruction, and GPU action from
fallback. Missing evidence stays Unknown or Blocked, not an excuse for more gates.

## Boundaries and follow-on work

Requirement trace: AOA-GOAL-004; AOA-GAME-001/002/007/008/009;
AOA-TEACH-004/005/008; AOA-OBS-001/004; AOA-PERSIST-001/002/004;
AOA-FAIL-002; AOA-REP-001/002; AOA-ASSIM-003 through AOA-ASSIM-009.

Keep neural choice/learning GPU-authoritative, outcomes world-owned, and
presentation read-only. No CPU neural fallback, hidden teacher authority,
scripted survival policy, or replacement of saved individuals.

Implement one bounded slice at a time, ending with visible production progress.
EI1 research promotion, larger populations/brains, elaborate schools, sharing,
robot transplantation, natural assimilation, and more art breadth remain later
work. EI1 stays Blocked until its own gates pass; it does not block the smaller
care loop. Packaging and release approval remain separate from these milestones.
