# Core game loop delivery plan

Date: 20 September 2026. Source reviewed: `abeb8d84`.

This is a proposed implementation sequence, not a completion report or a new
architecture. The [controlling architecture](architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md),
especially sections 1.1, 29, 32, 35 and 38, remains authoritative. No game code,
training, or runtime validation was performed while preparing this plan.

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

## Current foundation: extend the existing game

| Source finding | Required work |
| --- | --- |
| `new_game_lifecycle.rs::stage_phase3_new_game` loads `builtin_nano512_v1`; the N512 exporter builds a procedural bootstrap. | Evaluate retained candidates and explicitly integrate a suitable founder into ordinary New Game. |
| `production_voxel_renderer/ux_input.rs` calls `GpuLiveBrainRuntime::place_player_food`; `live_food_projection.rs` displays canonical objects. | Improve the existing care path, rather than building another demo. |
| `bevy_shell.rs::reconcile_production_presentation` refreshes live needs and identities. | Improve interpretation and feedback; do not rebuild the needs model. |
| `production_conversation_lineage_ui.rs::send_player_speech` uses height zero; hearing uses 3D distance. | Ground player speech on the actual terrain and establish that the intended creature hears it. |
| Sleep/checkpoint paths, `phase3_save_load_continuity.rs`, and candidate retention experiments exist. | Establish continuity for the actual playable founder; experimental success is not automatically New Game success. |
| World lifecycle, managed breeding, archives, and rendered births/retirements exist. | Repair the ordinary route where it fails, rather than reimplementing reproduction. |
| Ordinary errors redirect to F3 and some effect triggers remain launch-state based. | Explain care outcomes and failures in the player view using live state. |

These are source findings. The September 2 status page and older performance
receipts are not current-build playtest results.

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
