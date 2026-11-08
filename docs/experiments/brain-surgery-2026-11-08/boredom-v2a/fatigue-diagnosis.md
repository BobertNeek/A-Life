# Fatigue diagnosis from the closed matrix

This is an explanation of the existing 24-tick tired fixtures, not a new recovery
experiment or a retuned claim. No interval was rerun. The separately calibrated
wake/recovery chemistry from the other workstream was not applied here.

## What was measured

All eight tired episodes have exactly the same fatigue concentration and derived
fatigue trajectory: 0.800000011920929 at tick 0 to 0.7990403175354004 at tick 24.
The recorded per-interval graph concentration equals the homeostatic fatigue
projection. Developmental age advances from 0 to 24; maturation, biochemical
expression and development last-update tick remain exactly zero. Development
cadence is 60 ticks, so there is no developmental refresh in this interval.

Across every arm, stream 17 selects Rest on 23/24 intervals, with Body.sleeping
true after 14/24 intervals. Stream 29 selects Rest on 24/24, with Body.sleeping
true after 13/24. The other selected-Rest intervals have actual physical
translation. Each one loses recovery under the common Rest-motion correction.
The host scheduled-sleep counter is zero in all eight episodes. It is not a
count of the transient physiological Body.sleeping bit from a Rest command.

The graph's float32 decay recurrence c_next = c_previous * float32(0.99995)
reproduces every measured fatigue concentration exactly. The finite-scalar
interval export records the prior/successor concentration, derived fatigue,
expression, maturation, sleep-pressure hormone, body sleeping bit, selected
commands and measured per-channel displacement for every interval.

## Authoritative causal path

- The fixture initializer changes the fatigue species concentration using the
  existing receptor's inverse and re-derives homeostasis from that graph. It
  does not set an independent live drive or alter the graph, its decay,
  developmental clock, receptors or emitters. The initial 0.8 pressure is a
  diagnostic starting condition, not evidence that this newborn accumulated
  mature fatigue naturally.
- The inherited founder constructor already calls with_waking_recovery. This is
  the pre-existing graph, not the separately calibrated candidate. It sets
  fatigue retention to 0.99995 and a small Awake emitter. The reference fatigue
  regulatory baseline is 0.2; its expressed baseline is minimum 0 at zero
  developmental expression. The source's SleepRecovery→fatigue emitter has gain
  -0.70 and expression floor 0. The Awake→fatigue emitter also has floor 0.
- BiochemicalGraphState::advance first decays toward the expressed baseline,
  then multiplies each emitter response by max(developmental expression,
  emitter floor). Here that multiplier is zero for fatigue, on both stationary
  and moving intervals. Thus actual Rest does not lower fatigue through its
  emitter. Slow passive decay explains the entire observed change.
- HeadlessWorld's Rest outcome supplies sleep_recovery 1.0. The common motion
  normalization changes it to 0.0 if any executed channel translates. The final
  BodyEventDelta is not directly persisted in these policy receipts. Its dose
  is therefore labeled source-derived (1.0 or 0.0), based on the actual commands
  and per-channel physical receipts, and checked against measured Body.sleeping.
  It is not mislabeled as a directly recorded scalar.
- BodyState::apply_event sets sleeping from sleep_recovery × metabolic_turnover
  > 0. This body response is not gated by biochemical maturation. Consequently
  stationary Rest can make Body.sleeping true while fatigue emission is zero.
- CpuCanonicalHost uses a separate SleepController for neural sleep/consolidation.
  Its reference fatigue threshold is 0.9, above these 0.8 fixtures; recorded
  sleep pressure stays zero. No host scheduled sleep was entered. The probe
  therefore records an actual neural decision on every world interval.

## Source references

- crates/alife_training/examples/probe_boredom_loop.rs: biology_with_hunger.
- crates/alife_world/src/new_game.rs: calibrate_inherited_founder_biochemistry.
- crates/alife_core/src/biochemical_graph.rs: ChemicalSpecies::expressed_baseline,
  with_waking_recovery, early_mammal_reference, BiochemicalGraphState::advance,
  derive_homeostasis and emitter defaults.
- crates/alife_core/src/biochemistry.rs: DevelopmentReadiness::derive,
  advance_with_neural_emission and BodyState::apply_event.
- crates/alife_world/src/headless.rs: OutcomeProfile::rest and
  normalize_sleep_recovery_for_motion.
- crates/alife_training/src/cpu/host.rs: CpuCanonicalHost::step.
- crates/alife_core/src/sleep.rs: SleepConsolidationConfig::reference and
  SleepController::evaluate_homeostasis.

This explains near-constant fatigue and shows that the corrected Rest-motion
transaction operated. It does not qualify fatigue recovery, maturation, a sleep
cycle, consolidation, or the proposed newer chemistry. Any comparison against
that chemistry would require an explicitly factored, separately authorized plan.
