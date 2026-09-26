# Gameplay system review — 2026-09-26

Review order agreed with the user:
1. Senses, neural inputs, actions, consequences.
2. Food, digestion, energy, hunger, fatigue, sleep.
3. Learning, chemical credit, memory, prediction, consolidation.
4. Development, reproduction, newborns, ageing, death.
5. Player care, communication, visible feedback.
6. Saves, checkpoints, archives, lineage.
7. Ecology, tooling, performance.

## First group: sensory and action boundary repairs

Requirements: AOA-SENSE-001/002, AOA-MOTOR-001 through 007,
AOA-AUTH-001 through 005, AOA-MEM-001 through 003, AOA-PERSIST-003.
This is a source/contract review, not proof of learned navigation or visible play.

For GroundedTerrainVisionV1, retain the existing 220-degree view, independent
head swivel, opaque occlusion, transparent sight passage, and coarse depth fan.
Add short forward steps and body turns to existing motor channels. Their
candidates are unscored, targetless, and geometrically parameterized. Movement
uses the same world collision and outcome path as other walking. Hold-gaze lets
the brain retain its chosen head offset while walking or turning its body.
Candidate features distinguish head and body actions; at most three visible
objects receive five choices each, leaving the frame within the 32-choice ABI.

Anonymous smell uses two local samples of four physical odor bands. Strength
depends on distance and expressed chemical sensitivity. It supplies no source
identity or precise target. This is a bounded radial approximation; walls do not
model scent diffusion. Touch, held-object contact, ground contact, heat, and
injury are independent of visible object slots. Existing expressed sensor gains
calibrate this profile. Source-specific chemical and temperature descriptors are
contact observations rather than remote chemical identification.

Absolute position is neutralized in this profile's GPU neural body payload and
both predictor input/successor vectors. Authoritative frames, physics, rendering,
and receipts retain true coordinates. Velocity and orientation remain available.
Older profiles retain their existing semantics.

The 96-value memory query previously rejected profile 3 and could not represent
Look correctly. Formerly reserved lanes 83/84 now identify Look/terrain vision;
85–95 remain reserved. Existing valid legacy feature bytes and offsets remain
unchanged. Terrain candidates can be encoded, validated against their final
frame, and serialized through the existing query contract.

Focused verification covers opaque/transparent sight, terrain depth/collision,
targetless motor translation and retained gaze, invisible smell/contact,
candidate limits, memory round trips, and GPU-upload coordinate neutralization.
Legacy object-slot and memory-query checks protect existing profile behavior.
GPU-runtime compilation verifies predictor integration. No training or founder
promotion is part of this repair.

## Remaining integration and later groups

New-game and care-training defaults still use GroundedObjectSlotsV1. The trained
founder binds that profile in its foundation recipe. Adaptation must explicitly
bind a new foundation identity/profile and handle its optimizer state; do not
silently relabel the old founder or claim these senses are already learned.
The user's chosen path is adapting existing weights, then varied food-location
navigation, with food seen before it becomes hidden in the first wall lesson.

Visible head/gaze feedback remains unverified and belongs to group 5. Use Astra
for animation/model changes as requested. No renderer/model work was performed.

Carry these findings into later reviews:
- Group 3: trace Inspect/Vocalize outcome valuation before concluding that
  diagnostic homeostatic deltas become actual biological reward.
- Group 4: motor-bundle ambient mating opportunity still uses 1.0, while the
  passive tick uses inherited mate_response. Review this mismatch.
- Feeding: review reach/obstruction and carried-object ownership semantics
  before changing them. Distance alone does not prove a legal meal.

Groups 3–7 have not yet received this sequential end-to-end review. The biological
10–30 minute food-free survival gate remains acceptance work; it is not a lesson
to reward extended inactivity.

## Second group: food and recovery review

Status: source trace completed; the user chose immediate meal energy for the
alpha and separate hunger/tiredness. The waking/recovery and walking-capacity
repair below is implemented. No new wall-time survival trial is claimed.
Requirements: AOA-TIME-002, AOA-BODY-003/004, AOA-BIO-011/015/016/018,
AOA-SLEEP-006, AOA-PERF-002.

The user's final sleep decision is tiredness reduction plus greatly slowed
metabolism, not energy restoration. The official [C3 Genetics Kit tutorials](https://lisdude.com/cdn/CreaturesGenKitTutorials.pdf)
describe physical-state-to-chemical emitters and chemical-to-body receptors with
inherited thresholds, gains, and response settings. This guides our mechanism;
the exact stock C3 sleeping metabolic multiplier has not been verified. No exact
C3 metabolic equivalence is claimed.

Source findings before this repair:
- `headless.rs::execute_eat` requires an existing target within EAT_RADIUS,
  checks its food kind/consumed state, consumes it once, records ecology,
  and emits measured nutrition and damage. It does not explicitly check
  an intervening solid barrier or carried-object ownership in that path.
- `OutcomeProfile::food` supplies both nutrition and an immediate signed energy
  gain. `BodyState::apply_event` additionally distributes nutrition directly
  to organ reserves, scaled by inherited metabolic efficiency. The existing
  nutrient-to-ATP graph does not mediate those immediate body gains.
- The default genetic graph emits nutrient and reduces hunger on nutrition.
  Nutrient reacts into brain ATP. Its decay also removes material; body energy
  accounting and this chemical conversion are separate representations, not
  a single staged digestion chain.
- Passive upkeep, organ upkeep, action expenditure, repair, and actual neural
  work spend reserve. Inherited turnover scales body expenditure. The configured
  normal simulation cadence is 20 ticks/second. A reserve-horizon formula alone
  cannot establish 10–30 minute survival under actual activity.
- Default hunger and fatigue emitters both respond to energy deficit. No default
  basal waking or exertion source independently accumulates fatigue/sleep
  pressure. Thus a well-fed, active creature has no complete independent
  tiredness path in this reference graph.
- Motor translation uses inherited embodiment controllability/proprioception.
  Those gains do not read current reserve, injury, or fatigue. Biological
  lethargy has not been wired into this physical limit.
- The sleep scheduler observes chemical fatigue/sleep pressure and emergency
  recovery triggers. The live staged tick emits recovery while non-awake, but
  excludes completed sleep waiting for a durability permit. Recovery is real
  biology, not a teacher reward. Default rest has no direct body energy gain;
  the old no-repair-receptor fallback still adds sleep energy. The default graph
  also emits ATP from sleep recovery and relaxes material ATP to its inherited
  baseline. The live caller uses this as neural availability and separately
  charges measured cognitive work to body reserve. Automatic ATP availability
  is therefore not proof of free body energy; adding another debit without
  replacing that accounting would risk charging twice.

Implemented repair: retain immediate meal energy as the user requested.
New Game's existing chromosome calibration installs an explicit waking circuit
on both homologs. Awake emits fatigue and sleep pressure; their inherited
retention, gains, developmental expression, and existing sleep recovery govern
accumulation and relief. The old energy-deficit-to-fatigue emitter is removed
from this calibrated circuit; hunger still responds to reserve deficit. These
are mutable/recombinable graph parameters, not a fixed sleep timer.

The same chromosome update installs a sleep-state signalling chemical and an
inherited SleepMetabolicRate receptor. Its default lowers basal and organ upkeep
to about 10% once the sleeping signal arrives; the first interval reads previous
chemistry, and wake resumes normal upkeep immediately. The emitter's inherited
expression floor keeps this working in newborns too. Tissue repair and measured
cognitive work still cost real reserve. The sleep-specific ATP emitter is removed
from these new genes; sleep supplies no food or body-energy gain. Existing ATP
availability/baseline and measured-work accounting remain otherwise unchanged.
The energy-deficit-to-ATP emitter's inherited sensitivity is reduced to avoid
losing neural availability at ordinary body reserve after removing that sleep
boost. A focused check verifies availability with half-full reserve; this is
not a learned feeding or GPU recovery-loop acceptance trial.

A typed LocomotorCapacity receptor reads fatigue and bounds translation in the
existing factorized motor adapter. Current locomotor reserve relative to its
inherited initial level, and organ integrity, further bound that physical
capacity. The selected primitive and target are preserved; no rest action is
selected on the brain's behalf. Other channels remain available for looking,
handling nearby food, or communicating.

The legacy reference constructor, stored concentrations, saved chromosomes,
and exact existing founder assets are preserved. Graphs without the new receptor
retain their previous motor behavior. The existing trained founder must acquire
the explicit new genes during its adaptation; loading it alone does not install
this circuit. Immediate digestion is an intentional simplification, not a new
metabolism controller. No new teacher reward encourages prolonged inactivity.

Further repairs: grab and eat now validate the existing solid-object segment
and terrain obstacle boxes, regardless of optical opacity or gaze. A blocked
attempt has no contact, consumption, or nutrition. Eating a nearby meal held by
another organism remains legal for offering food; grabbing another organism's
possession retains its existing ownership restriction. Food is still consumed
at most once. This is straight-segment contact in the current point-body model,
not articulated arm/mouth reach or a heightfield intersection proof.

The no-repair-receptor legacy fallback no longer manufactures sleep energy.
Its existing inherited repair capacity now restores tissue only as far as local
reserve can pay, matching the resource rule of the chemical repair path. Saved
chromosome bytes are unchanged; this intentionally fixes old runtime behavior.
Two existing unit-test callers missing the sleep-metabolism argument were also
repaired, and the existing repair/accounting test covers the legacy fallback.

Outstanding in this group: heightfield/articulated contact limits; normal-speed
survival with actual cognition/activity; player-visible lethargy/sleep feedback. Do not
mark the whole group accepted from the focused checks alone.

Existing evidence in the training regimen records a 24,000-tick inherited-body
check and a live N2048 policy depleting reserve at tick 14,224. These older records
are not current wall-time acceptance proof. After the repair, use one focused
food-to-energy/accounting check and one hunger/fatigue-to-motor/recovery check,
reusing existing tests. Full player-visible survival acceptance remains separate.

## Third group: learning, chemical credit, memory, prediction, consolidation

Requirements: AOA-LEARN-001 through 010, AOA-MEM-001 through 003/008,
AOA-SLEEP-002/003/005/007. Source review and focused contract evidence are
separate from post-training behavioral and GPU retention acceptance.

The live motor transaction measures canonical before/after biology, including
actual cognitive work cost, before sealing experience. Host reward stays zero.
OutcomeCreditPacket derives homeostatic improvement, fresh injury valuation,
and genetically scaled disappointment from that measured transition. Hormonal
receptors modulate a measured consequence rather than treating baseline chemical
concentrations as an action reward. GPU online plasticity and bounded sleep replay
consume those separate lanes and exact causal eligibility; they do not use the
legacy reward field as an answer key. No learning-rule replacement was needed
for this part of the traced path.

Found and repaired: active candidate-memory construction still used the zero
legacy reward field for signed value and salience, and treated every failed
attempt's raw frustration as severe danger. New memories now derive a signed
summary from the existing measured homeostatic, injury, and disappointment
components. The distinct learning lanes are retained. Legacy diagnostic patches
keep their historical values; stored memories are not silently rewritten. The
same summary repairs episodic diagnostic and topology emotional associations.

The app also incorrectly used target latent lane zero (signed hunger change)
as expected valence. It now reads the explicit family-value lane with its own
source and confidence. A negative hunger change remains negative in the target
prediction, while a relieving meal can be remembered as positive. One focused
extension of the existing memory retrieval suite checks meal relief, mild
blocked-attempt aversion, and save/restore preservation of the new memories.

Prediction freezes a forecast before applying the world action and compares it
with the grounded successor; physical outcome is not supplied as a teacher
label. Sleep replay is bounded and validates event identity, eligibility spans,
cycle/generation, and digest. The app prepares memory compaction on a clone and
uses the existing staged sleep rollback/durability path. Source inspection alone
does not prove a learned choice survives sleep, restore, and normal play.
Existing GPU choice-retention and sleep-atomicity checks are the later focused
acceptance path; no new GPU campaign was launched in this review.

Groups 4 through 7 remain unreviewed. Do not describe all seven groups as repaired
or accepted.

Validation for this repair packet: one world contact-legality check passed;
the existing core inherited-upkeep/repair check, extended for legacy sleep,
passed; the existing candidate-memory retrieval suite passed 19/19 with one
new measured-consequence check. The GPU-enabled app compiled with existing
unused/dead-code warnings. Core boundary, documentation (77/77), and diff checks
passed. No GPU dispatch, visible playtest, or wall-time survival trial was run.
