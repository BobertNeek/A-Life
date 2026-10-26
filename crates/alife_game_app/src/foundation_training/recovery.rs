//! Recovery acceptance from sealed execution and ordinary body consequences.
use alife_core::{ActionKind, ExperiencePatch, MotorChannel, WorldEntityId};

pub(super) fn lesson_completed<'a>(
    food: WorldEntityId,
    patches: impl IntoIterator<Item = &'a ExperiencePatch>,
) -> bool {
    let mut patches = patches.into_iter().peekable();
    let Some(first) = patches.peek() else {
        return false;
    };
    let initial = first.pre_action().homeostasis().drives.fatigue;
    if initial < 0.12 {
        return false;
    }
    let target = initial - 0.02;
    let mut recovered = false;
    let mut fed_after_recovery = false;
    let mut final_fatigue = None;
    for patch in patches {
        let outcome = patch.outcome();
        if super::teacher_outcome_blocked(outcome) {
            return false;
        }
        let physiology = outcome.measured_physiology;
        final_fatigue = physiology.map(|t| t.after.homeostasis.drives.fatigue);
        if rest_executed(patch) {
            // Interruptions and concurrent channels are allowed, but Rest must
            // actually reduce fatigue and reach the original recovery threshold.
            recovered |= physiology.is_some_and(|t| {
                t.after.homeostasis.drives.fatigue < t.before.homeostasis.drives.fatigue
                    && t.after.homeostasis.drives.fatigue <= target
            });
        } else if recovered
            && physiology.is_some_and(|t| !t.after.body.sleeping)
            && super::selected_eat_consumed(
                &patch.decision().selected_action,
                patch.selected_bundle(),
                outcome,
                food,
            )
        {
            // A simultaneous Eat+Rest meal alone is not waking re-engagement.
            fed_after_recovery = true;
        }
    }
    fed_after_recovery && final_fatigue.is_some_and(|fatigue| fatigue <= target)
}

fn rest_executed(patch: &ExperiencePatch) -> bool {
    let outcome = patch.outcome();
    if !outcome.success {
        return false;
    }
    if let Some(bundle) = patch.selected_bundle() {
        bundle.channels.iter().any(|command| {
            command.channel == MotorChannel::Posture
                && command.primitive == ActionKind::Rest.canonical_id()
                && outcome.joint.as_ref().is_some_and(|joint| {
                    joint.channel_outcomes.iter().any(|channel| {
                        channel.channel == MotorChannel::Posture
                            && channel.physical.contact != alife_core::PhysicalContactKind::Blocked
                    })
                })
        })
    } else {
        let action = &patch.decision().selected_action;
        action.kind == ActionKind::Rest
            && action.action_id == ActionKind::Rest.canonical_id()
            && !super::teacher_outcome_blocked(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::super::held_food_lesson_tests::{action_patch, frame, unconfigured};
    use super::*;
    use alife_core::{
        ActionId, Confidence, ExperienceSequenceId, HomeostaticDelta, MeasuredPhysiologyTransition,
        MotorCommandBundle, NormalizedScalar, PhysicalContactKind, PostActionOutcome,
        SignedValence, Tick, Vec3f,
    };
    use alife_world::{HeadlessActionIds as Ids, HeadlessWorld};

    fn prepared(precondition_recovery: bool) -> (HeadlessWorld, WorldEntityId) {
        let mut world = unconfigured(42);
        let scenario = super::super::configure_foundation_scenario(
            &mut world,
            42,
            Some(super::super::FoundationTeacherLesson::Recovery),
            None,
            precondition_recovery,
        )
        .unwrap();
        let (_, entity) = world.organism_entity_ids()[0];
        let origin = world.entity(entity).unwrap().position;
        super::super::move_scenario_object(
            &mut world,
            scenario.food,
            Vec3f::new(origin.x + 0.5, origin.y, origin.z),
        )
        .unwrap();
        if precondition_recovery {
            assert!(frame(&mut world).homeostasis().drives.fatigue >= 0.12);
        }
        (world, scenario.food)
    }

    // Actual CPU world execution and body transitions, with synthetic neural
    // selection scaffolding only. No policy inference, learning, or evaluation.
    fn execute(
        world: &mut HeadlessWorld,
        representative: ActionId,
        target: Option<WorldEntityId>,
        channels: &[(MotorChannel, ActionId, Option<WorldEntityId>)],
    ) -> ExperiencePatch {
        let before = frame(world);
        let (subject, entity) = world.organism_entity_ids()[0];
        let candidate_command = |id, target| {
            before
                .candidates()
                .iter()
                .find(|c| c.action_id == id && c.target.entity == target)
                .unwrap_or_else(|| panic!("missing candidate {id:?} {target:?}"))
                .to_command(subject, Confidence::new(1.0).unwrap())
                .unwrap()
        };
        let sequence = ExperienceSequenceId::new(world.tick().raw() + 1).unwrap();
        let (success, physical, biology_before, biology_after, motor) = if channels.is_empty() {
            let command = candidate_command(representative, target);
            let receipt = world
                .apply_registered_command(&command, entity, Tick(world.tick().raw() + 1))
                .unwrap();
            (
                receipt.action_result.execution.succeeded,
                receipt.action_result.execution.physical,
                receipt.biology_before,
                receipt.biology_after,
                None,
            )
        } else {
            let commands = channels
                .iter()
                .map(|(channel, id, target)| {
                    alife_core::channel_command_for_action(
                        *channel,
                        &candidate_command(*id, *target),
                    )
                    .unwrap()
                })
                .collect();
            let bundle =
                MotorCommandBundle::new(subject, sequence, world.tick(), commands).unwrap();
            let receipt = world
                .apply_registered_motor_bundle(&bundle, entity)
                .unwrap();
            (
                receipt.succeeded,
                receipt.joint.execution,
                receipt.biology_before,
                receipt.biology_after,
                Some((bundle, receipt.joint)),
            )
        };
        world.try_advance_tick().unwrap();
        let outcome = PostActionOutcome::new(
            subject,
            sequence,
            world.tick(),
            success,
            physical,
            HomeostaticDelta::zero(),
            SignedValence::ZERO,
            NormalizedScalar::ZERO,
            NormalizedScalar::ZERO,
            SignedValence::ZERO,
            NormalizedScalar::ZERO,
        )
        .unwrap()
        .with_measured_physiology(
            MeasuredPhysiologyTransition::new(biology_before, biology_after).unwrap(),
        )
        .unwrap();
        action_patch(before, target, outcome, representative, motor)
    }

    fn recover(
        world: &mut HeadlessWorld,
        interrupted: bool,
        factorized: bool,
    ) -> Vec<ExperiencePatch> {
        let initial = frame(world).homeostasis().drives.fatigue;
        let mut patches = Vec::new();
        if interrupted {
            patches.push(execute(world, Ids::LOOK_CENTER, None, &[]));
        }
        for index in 0..6000 {
            patches.push(if factorized {
                execute(
                    world,
                    Ids::LOOK_CENTER,
                    None,
                    &[
                        (MotorChannel::Orientation, Ids::LOOK_CENTER, None),
                        (MotorChannel::Posture, ActionKind::Rest.canonical_id(), None),
                    ],
                )
            } else {
                execute(world, ActionKind::Rest.canonical_id(), None, &[])
            });
            if index > 0 && frame(world).homeostasis().drives.fatigue <= initial - 0.03 {
                return patches;
            }
            if interrupted && index % 3 == 0 {
                patches.push(execute(world, Ids::LOOK_CENTER, None, &[]));
            }
        }
        panic!("ordinary Rest did not reduce fatigue by 0.03");
    }

    fn feed(
        world: &mut HeadlessWorld,
        food: WorldEntityId,
        patches: &mut Vec<ExperiencePatch>,
        posture: Option<ActionId>,
    ) {
        for _ in 0..2 {
            patches.push(if let Some(posture) = posture {
                execute(
                    world,
                    Ids::LOOK_CENTER,
                    None,
                    &[
                        (MotorChannel::Orientation, Ids::LOOK_CENTER, None),
                        (MotorChannel::Manipulation, Ids::EAT, Some(food)),
                        (MotorChannel::Posture, posture, None),
                    ],
                )
            } else {
                execute(world, Ids::EAT, Some(food), &[])
            });
        }
        assert_eq!(
            patches.last().unwrap().outcome().physical.contact,
            PhysicalContactKind::Consumed
        );
    }

    fn changed_outcome(
        patch: &ExperiencePatch,
        change: impl FnOnce(&mut PostActionOutcome),
    ) -> ExperiencePatch {
        let mut outcome = patch.outcome().clone();
        change(&mut outcome);
        if let Some(joint) = &mut outcome.joint {
            joint.execution = outcome.physical;
        }
        if let Some(t) = outcome.measured_physiology {
            outcome = outcome
                .with_measured_physiology(
                    MeasuredPhysiologyTransition::new(t.before, t.after).unwrap(),
                )
                .unwrap();
        }
        let motor = patch.selected_bundle().cloned().zip(outcome.joint.clone());
        let action = &patch.decision().selected_action;
        action_patch(
            patch.pre_action().perception().clone(),
            action.target_entity,
            outcome,
            action.action_id,
            motor,
        )
    }

    // Counterfactual numerical/provenance fixtures retain the executed command
    // and physical receipt, but replace fatigue observations and re-seal their
    // frame/transition bindings. They do not establish causal body recovery.
    fn counterfactual_fatigue(patch: &ExperiencePatch, before: f32, after: f32) -> ExperiencePatch {
        let changed = changed_outcome(patch, |outcome| {
            let transition = outcome.measured_physiology.as_mut().unwrap();
            transition.before.homeostasis.drives.fatigue = before;
            transition.after.homeostasis.drives.fatigue = after;
        });
        let frame = patch.pre_action().perception();
        let mut homeostasis = *frame.homeostasis();
        homeostasis.drives.fatigue = before;
        let frame = alife_core::PerceptionFrameDraft::new(
            frame.organism_id(),
            frame.tick(),
            frame.sensor_profile(),
            frame.sensory().clone(),
            frame.body(),
            homeostasis,
            frame.candidates().to_vec(),
            frame.profile_provenance(),
            frame.grounded_object_slots().to_vec(),
        )
        .unwrap()
        .finalize(frame.context().clone())
        .unwrap();
        let action = &patch.decision().selected_action;
        let outcome = changed.outcome().clone();
        let motor = patch.selected_bundle().cloned().zip(outcome.joint.clone());
        action_patch(
            frame,
            action.target_entity,
            outcome,
            action.action_id,
            motor,
        )
    }

    fn emit_observation(
        world: &HeadlessWorld,
        food: WorldEntityId,
        learner: alife_core::OrganismId,
        observation: &super::super::FoundationEvaluationObservation,
        completed: bool,
    ) -> serde_json::Value {
        let output = std::env::temp_dir().join(format!(
            "alife-recovery-observation-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&output).unwrap();
        super::super::evaluation_observation::write_lesson_receipt(
            &output,
            observation,
            serde_json::json!({
                "lesson": super::super::FoundationTeacherLesson::Recovery,
                "lesson_completed": completed,
                "final_food_distance": super::super::held_food_learner_distance(world, food, learner),
            }),
        ).unwrap();
        let path = output.join(if completed {
            "lesson-trace.json"
        } else {
            "lesson-diagnostic.json"
        });
        let document = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        std::fs::remove_dir_all(output).unwrap();
        document
    }

    #[test]
    fn recovery_sleep_boundary_is_censored_while_later_awake_meal_can_complete() {
        use super::super::{
            FoundationEvaluationObservation as Observation, FoundationEvaluationStopReason as Stop,
        };
        let (mut world, food) = prepared(true);
        let learner = world.organism_entity_ids()[0].0;
        let mut patches = recover(&mut world, false, false);
        assert!(!lesson_completed(food, &patches));
        // These are executed CPU world/body outcomes, not a neural policy test.
        assert!(
            world
                .organism_registry()
                .get(learner)
                .unwrap()
                .biochemistry()
                .body
                .sleeping
        );
        let mut observed = Observation::new(patches.len(), learner);
        observed.before_tick(&world);
        observed.finish(&world, None, patches.len(), Some(false));
        assert_eq!(
            emit_observation(&world, food, learner, &observed, false)["evaluation_observation"]
                ["lesson_outcome"],
            "not_completed"
        );

        // Explicit scheduler-seal fixture for the same missing-capture boundary
        // used by the evaluator. Body Rest by itself never establishes this.
        observed.before_tick(&world);
        world.try_advance_tick().unwrap();
        let mut record = world.organism_registry().get(learner).unwrap().clone();
        record
            .seal_sleep_phase(alife_core::SleepPhase::EnteringSleep, 1, world.tick(), 0)
            .unwrap();
        world.replace_organism_record_exact(record).unwrap();
        observed.stop_reason = Stop::MissingWakingCapture;
        observed.finish(&world, None, patches.len(), Some(false));
        let censored = emit_observation(&world, food, learner, &observed, false);
        assert_eq!(censored["lesson_completed"], false);
        assert_eq!(
            censored["evaluation_observation"]["stop_reason"],
            "neural_sleep"
        );
        assert_eq!(
            censored["evaluation_observation"]["lesson_outcome"],
            "sleep_censored"
        );

        let mut record = world.organism_registry().get(learner).unwrap().clone();
        record
            .seal_sleep_phase(alife_core::SleepPhase::Awake, 1, world.tick(), 0)
            .unwrap();
        world.replace_organism_record_exact(record).unwrap();
        feed(&mut world, food, &mut patches, None);
        assert!(lesson_completed(food, &patches));
        let mut later = Observation::new(patches.len(), learner);
        later.finish(&world, None, patches.len(), Some(true));
        assert_eq!(
            emit_observation(&world, food, learner, &later, true)["evaluation_observation"]
                ["lesson_outcome"],
            "completed"
        );
    }

    #[test]
    fn recovery_terminal_receipt_survives_retirement_without_rebinding_to_a_peer() {
        use super::super::FoundationEvaluationObservation as Observation;
        for keep_peer in [false, true] {
            let (mut world, food) = prepared(true);
            let learner = world.organism_entity_ids()[0].0;
            let patches = vec![execute(&mut world, Ids::LOOK_CENTER, None, &[])];
            assert!(!lesson_completed(food, &patches));
            let mut observed = Observation::new(16, learner);
            observed.before_tick(&world);
            world.try_advance_tick().unwrap();
            let death_tick = world.tick().raw();
            // Legal terminal/archive fixture; this does not claim natural death.
            let mut terminal = world.organism_registry().get(learner).unwrap().clone();
            terminal.mark_dead(world.tick()).unwrap();
            terminal
                .link_birth_manifest(alife_core::Blake3Digest::from_bytes([41; 32]))
                .unwrap();
            terminal
                .link_life_manifest(alife_core::Blake3Digest::from_bytes([42; 32]))
                .unwrap();
            world.replace_organism_record_exact(terminal).unwrap();
            observed.finish(&world, None, patches.len(), Some(false));
            assert_eq!(observed.terminal_death_tick, Some(death_tick));
            world.retire_dead_organism(learner).unwrap();
            if keep_peer {
                world
                    .spawn_social_agent(
                        "remaining-peer",
                        alife_core::OrganismId(99),
                        Vec3f::new(0.0, 0.0, 0.0),
                        0.75,
                    )
                    .unwrap();
            }
            assert_eq!(world.organism_entity_ids().is_empty(), !keep_peer);
            // The production runtime supplies this retained archive death tick.
            observed.finish(&world, Some(death_tick), patches.len(), Some(false));
            let document = emit_observation(&world, food, learner, &observed, false);
            assert_eq!(document["lesson"], "recovery");
            assert_eq!(document["lesson_completed"], false);
            assert!(document["final_food_distance"].is_null());
            assert_eq!(
                document["evaluation_observation"]["lesson_outcome"],
                "terminal_death"
            );
            assert_eq!(
                document["evaluation_observation"]["terminal_death_tick"],
                death_tick
            );
            assert!(document["evaluation_observation"]["final_neural_phase"].is_null());
        }
    }

    #[test]
    fn counterfactual_nonrest_relief_and_later_awake_meal_require_executed_rest() {
        let (mut world, food) = prepared(true);
        let mut patches = vec![execute(
            &mut world,
            Ids::LOOK_CENTER,
            None,
            &[
                (MotorChannel::Orientation, Ids::LOOK_CENTER, None),
                (MotorChannel::Posture, Ids::NO_POSTURE, None),
            ],
        )];
        feed(&mut world, food, &mut patches, None);
        let initial = patches[0].pre_action().homeostasis().drives.fatigue;
        let relieved = initial - 0.03;
        let patches: Vec<_> = patches
            .iter()
            .enumerate()
            .map(|(index, patch)| {
                counterfactual_fatigue(patch, if index == 0 { initial } else { relieved }, relieved)
            })
            .collect();
        assert!(patches.iter().all(|patch| !rest_executed(patch)));
        let first = patches[0].outcome().measured_physiology.unwrap();
        assert!(first.after.homeostasis.drives.fatigue < first.before.homeostasis.drives.fatigue);
        let meal = patches.last().unwrap();
        let transition = meal.outcome().measured_physiology.unwrap();
        assert!(transition.after.homeostasis.drives.fatigue <= initial - 0.02);
        assert!(!transition.after.body.sleeping);
        assert!(super::super::selected_eat_consumed(
            &meal.decision().selected_action,
            meal.selected_bundle(),
            meal.outcome(),
            food,
        ));
        assert!(!lesson_completed(food, &patches));
    }

    #[test]
    fn counterfactual_exact_fatigue_boundaries_accept_rest_crossing_episode_threshold() {
        let (mut world, food) = prepared(true);
        let mut patches = vec![
            execute(&mut world, Ids::LOOK_CENTER, None, &[]),
            execute(
                &mut world,
                Ids::LOOK_CENTER,
                None,
                &[
                    (MotorChannel::Orientation, Ids::LOOK_CENTER, None),
                    (MotorChannel::Posture, ActionKind::Rest.canonical_id(), None),
                ],
            ),
        ];
        feed(&mut world, food, &mut patches, None);
        let initial = 0.12_f32;
        let target = initial - 0.02;
        let intermediate = initial - 0.015;
        let patches: Vec<_> = patches
            .iter()
            .enumerate()
            .map(|(index, patch)| {
                let (before, after) = match index {
                    0 => (initial, intermediate),
                    1 => (intermediate, target),
                    _ => (target, target),
                };
                counterfactual_fatigue(patch, before, after)
            })
            .collect();
        assert_eq!(
            patches[0]
                .pre_action()
                .homeostasis()
                .drives
                .fatigue
                .to_bits(),
            initial.to_bits()
        );
        assert!(!rest_executed(&patches[0]));
        assert!(rest_executed(&patches[1]));
        let rest = patches[1].outcome().measured_physiology.unwrap();
        assert!(rest.after.homeostasis.drives.fatigue < rest.before.homeostasis.drives.fatigue);
        assert!(
            rest.before.homeostasis.drives.fatigue - rest.after.homeostasis.drives.fatigue < 0.02
        );
        assert_eq!(
            rest.after.homeostasis.drives.fatigue.to_bits(),
            target.to_bits()
        );
        let meal = patches.last().unwrap();
        let transition = meal.outcome().measured_physiology.unwrap();
        assert_eq!(
            transition.after.homeostasis.drives.fatigue.to_bits(),
            target.to_bits()
        );
        assert!(!transition.after.body.sleeping);
        assert!(super::super::selected_eat_consumed(
            &meal.decision().selected_action,
            meal.selected_bundle(),
            meal.outcome(),
            food,
        ));
        assert!(lesson_completed(food, &patches));
    }

    #[test]
    fn direct_interrupted_and_factorized_recovery_require_later_waking_meal() {
        let (initial_world, food) = prepared(true);
        let mut completions = Vec::new();
        for (interrupted, factorized) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let mut world = initial_world.clone();
            let mut patches = recover(&mut world, interrupted, factorized);
            assert!(
                !lesson_completed(food, &patches),
                "Rest alone is not re-engagement"
            );
            if factorized {
                assert!(patches
                    .iter()
                    .filter(|p| rest_executed(p))
                    .all(|p| { p.decision().selected_action.action_id == Ids::LOOK_CENTER }));
            }
            feed(
                &mut world,
                food,
                &mut patches,
                factorized.then_some(Ids::NO_POSTURE),
            );
            completions.push(lesson_completed(food, &patches));
            let meal = patches
                .last()
                .unwrap()
                .outcome()
                .measured_physiology
                .unwrap();
            assert!(!meal.after.body.sleeping);
            assert!(meal.after.homeostasis.drives.hunger < meal.before.homeostasis.drives.hunger);
            assert!(!lesson_completed(
                WorldEntityId(food.raw() + 10_000),
                &patches
            ));
        }
        assert_eq!(
            completions, [true; 4],
            "direct, interrupted, factorized, both"
        );
    }

    #[test]
    fn headless_trace_follows_meal_and_rest_without_reusing_stale_outcomes() {
        use crate::gpu_live_runtime::headless_trace::trace_row;
        // Existing offline world fixture: actual legal Eat/Rest and physiology,
        // synthetic neural choice. Full neural sleep is tested separately by the
        // runtime scheduler fixtures, never inferred from body.sleeping here.
        let (mut world, food) = prepared(true);
        let (learner, entity) = world.organism_entity_ids()[0];
        let mut meals = Vec::new();
        feed(&mut world, food, &mut meals, None);
        let meal = meals.last().unwrap();
        let mut summary = crate::LiveBrainTickSummary {
            schema: "offline-trace-fixture",
            schema_version: 1,
            organism_id: learner,
            tick_before: meal.header().world_tick,
            tick_after: meal.outcome().outcome_tick,
            world_tick_before: meal.header().world_tick,
            world_tick_after: meal.outcome().outcome_tick,
            status: alife_core::BrainTickStatus::Normal,
            selected_action_kind: Some(meal.decision().selected_action.kind),
            selected_action_id: Some(meal.decision().selected_action.action_id),
            target_entity: Some(food),
            patch_sealed: true,
            patch_sequence_id: Some(meal.header().sequence_id.raw()),
            patch_success: Some(meal.outcome().success),
            physical_contact: Some(meal.outcome().physical.contact),
            action_failure: None,
            motor_execution: None,
            sealed_patch_count: meals.len(),
            packed_record_count: 0,
            memory_updates: 0,
            topology_updates: 0,
            learning_updates: 0,
            invalid_or_rejected_action_count: 0,
            last_diagnostic: None,
            causal_stages: Vec::new(),
        };
        let signature = world.canonical_signature_digest().unwrap();
        let row = trace_row(&world, &[summary.clone()], &meals, |_| None, 0.0);
        assert_eq!(world.canonical_signature_digest().unwrap(), signature);
        assert_eq!(row["organisms"][0]["organism_id"], learner.raw());
        assert_eq!(row["organisms"][0]["stable_id"], entity.raw());
        let pressure_before_rest = world
            .organism_registry()
            .get(learner)
            .unwrap()
            .biochemistry()
            .homeostasis
            .hormones
            .sleep_pressure;
        assert_eq!(
            row["organisms"][0]["sleep_pressure"],
            serde_json::json!(pressure_before_rest)
        );
        let measured = &row["actions"][0]["outcome"]["measured_physiology"];
        assert!(
            measured["after"]["body"]["energy"].as_f64().unwrap()
                > measured["before"]["body"]["energy"].as_f64().unwrap()
        );
        assert!(world.entity(food).unwrap().consumed);

        let rest = execute(&mut world, ActionKind::Rest.canonical_id(), None, &[]);
        let row = trace_row(&world, &[], std::slice::from_ref(&rest), |_| None, 0.1);
        assert_eq!(row["organisms"][0]["organism_id"], learner.raw());
        assert_eq!(row["organisms"][0]["stable_id"], entity.raw());
        assert_eq!(row["organisms"][0]["body_sleeping"], true);
        assert_eq!(row["actions"], serde_json::json!([]));
        let pressure_after_rest = world
            .organism_registry()
            .get(learner)
            .unwrap()
            .biochemistry()
            .homeostasis
            .hormones
            .sleep_pressure;
        assert!(pressure_after_rest < pressure_before_rest);
        assert_eq!(
            row["organisms"][0]["sleep_pressure"],
            serde_json::json!(pressure_after_rest)
        );

        // Retained latest meal must not appear on later/sleeping summaries, or
        // for a different life/sequence. Omission is unknown, never a new meal.
        for mismatch in 0..4 {
            let original = summary.clone();
            match mismatch {
                0 => summary.patch_sealed = false,
                1 => summary.world_tick_before = world.tick(),
                2 => summary.organism_id = alife_core::OrganismId(learner.raw() + 1),
                _ => summary.patch_sequence_id = Some(meal.header().sequence_id.raw() + 1),
            }
            let row = trace_row(&world, &[summary.clone()], &meals, |_| None, 0.2);
            assert!(row["actions"][0]["outcome"].is_null());
            summary = original;
        }
    }

    #[test]
    fn headless_trace_retains_identity_through_scheduler_consolidation_and_waking() {
        use crate::gpu_live_runtime::headless_trace::trace_row;
        use alife_core::{
            ConsolidationDriverEvent as Event, ConsolidationIntent, ConsolidationJobId,
            ConsolidationStagedOutput, GpuConsolidationRequest, OrganismId, PhenotypeHash,
            ScaffoldContractError, SleepConsolidationConfig, SleepPhase, SleepState,
        };
        // Same deterministic driver events as automatic_sleep_scheduler.rs,
        // queued as scaffolding. Real scheduler/world biology, no GPU weights,
        // neural inference, durable publication, or learned-behavior claim.
        #[derive(Default)]
        struct FixtureDriver(Vec<Event>);
        impl alife_runtime::GpuSleepConsolidationDriver for FixtureDriver {
            fn has_bounded_sleep_phase_data(
                &mut self,
                _: OrganismId,
                _: SleepState,
            ) -> Result<bool, ScaffoldContractError> {
                Ok(false)
            }
            fn progress(
                &mut self,
                _: OrganismId,
                _: SleepState,
                intent: Option<ConsolidationIntent>,
            ) -> Result<Option<Event>, ScaffoldContractError> {
                if let Some(intent) = intent {
                    let mut request = GpuConsolidationRequest {
                        schema_version: alife_core::GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
                        request_flags: 0,
                        cycle_id: intent.cycle_id,
                        phenotype_hash: PhenotypeHash([21, 22, 23, 24]),
                        input_generation: 1,
                        expected_output_generation: 2,
                        input_digest: [31, 32, 33, 34],
                        replay_digest: [11, 12, 13, 14],
                        max_replay_events: 1,
                        max_replay_eligibility_samples: 1,
                        request_digest: [0; 4],
                    };
                    request.request_digest = request.recompute_request_digest()?;
                    let job_id = ConsolidationJobId::try_from_raw(1)?;
                    let mut staged = ConsolidationStagedOutput {
                        job_id,
                        output_generation: 2,
                        output_weight_bank: 1,
                        output_digest: [41, 42, 43, 44],
                        eligibility_reset_generation: 2,
                        output_eligibility_bank: 0,
                        eligibility_output_digest: [51, 52, 53, 54],
                        replay_journal_generation: 2,
                        replay_journal_cursor: 0,
                        replay_journal_event_count: 0,
                        replay_journal_output_digest: [61, 62, 63, 64],
                        staging_digest: [0; 4],
                        promoted_fast_l1_bits: 0.25_f32.to_bits(),
                        replay_induced_fast_l1_bits: 0.125_f32.to_bits(),
                    };
                    staged.staging_digest = staged.recompute_staging_digest(&request, 1, 1)?;
                    self.0 = vec![
                        Event::Committed {
                            cycle_id: intent.cycle_id,
                            output_generation: 2,
                            output_digest: staged.output_digest,
                        },
                        Event::Completed { request, staged },
                        Event::Submitted { request, job_id },
                        Event::Prepared { request },
                        Event::ReplayAssetPersisted {
                            intent,
                            replay_digest: request.replay_digest,
                            replay_event_count: 0,
                            replay_eligibility_sample_count: 0,
                        },
                    ];
                }
                Ok(self.0.pop())
            }
        }
        let (mut world, food) = prepared(true);
        let (learner, entity) = world.organism_entity_ids()[0];
        let mut meals = Vec::new();
        feed(&mut world, food, &mut meals, None);
        let initial_pressure = world
            .organism_registry()
            .get(learner)
            .unwrap()
            .biochemistry()
            .homeostasis
            .hormones
            .sleep_pressure;
        let mut scheduler = alife_runtime::GpuSleepScheduler::new(SleepConsolidationConfig {
            // A shorter observation fixture, never a production threshold edit.
            fatigue_threshold: NormalizedScalar::new(0.12).unwrap(),
            entering_duration: alife_core::DurationTicks::new(1),
            waking_duration: alife_core::DurationTicks::new(1),
            ..SleepConsolidationConfig::reference()
        })
        .unwrap();
        let mut driver = FixtureDriver::default();
        let mut phases = Vec::new();
        let mut pressures = Vec::new();
        let mut committed = false;
        for _ in 0..24 {
            let next = Tick(world.tick().raw() + 1);
            let mut record = world.organism_registry().get(learner).unwrap().clone();
            scheduler
                .scheduled_tick_with_organism(
                    &mut record,
                    alife_core::HomeostaticParameters::reference(),
                    next,
                    &mut driver,
                    false,
                )
                .unwrap();
            world.replace_organism_record_exact(record).unwrap();
            world.try_advance_tick().unwrap();
            let state = scheduler.state();
            let signature = world.canonical_signature_digest().unwrap();
            let row = trace_row(
                &world,
                &[],
                &meals,
                |id| (id == learner).then_some(state),
                0.0,
            );
            assert_eq!(world.canonical_signature_digest().unwrap(), signature);
            assert_eq!(row["organisms"][0]["organism_id"], learner.raw());
            assert_eq!(row["organisms"][0]["stable_id"], entity.raw());
            assert_eq!(row["actions"], serde_json::json!([]));
            // Continuous chemistry must remain visible without waking action
            // outcomes, including Waking's pressure-dependent readiness period.
            let pressure = world
                .organism_registry()
                .get(learner)
                .unwrap()
                .biochemistry()
                .homeostasis
                .hormones
                .sleep_pressure;
            assert_eq!(
                row["organisms"][0]["sleep_pressure"],
                serde_json::json!(pressure)
            );
            pressures.push(pressure);
            let decoded: SleepState =
                serde_json::from_value(row["organisms"][0]["neural_sleep"].clone()).unwrap();
            assert_eq!(decoded, state);
            phases.push(decoded.phase);
            committed |= matches!(
                decoded.consolidation,
                alife_core::ConsolidationState::Committed { .. }
            );
            if decoded.phase == SleepPhase::Awake && decoded.last_consolidated_cycle_id == 1 {
                break;
            }
        }
        assert!(phases.contains(&SleepPhase::EnteringSleep));
        assert!(phases.contains(&SleepPhase::Consolidating));
        assert!(phases.contains(&SleepPhase::Waking));
        assert_eq!(phases.last(), Some(&SleepPhase::Awake));
        assert!(committed);
        assert!(pressures
            .iter()
            .any(|pressure| *pressure < initial_pressure));
        // A later world-owned action is still available to this same life.
        let awake = execute(&mut world, Ids::LOOK_CENTER, None, &[]);
        assert_eq!(awake.header().organism_id, learner);
        assert_eq!(world.organism_entity_ids()[0], (learner, entity));
        assert!(awake.header().world_tick > meals.last().unwrap().outcome().outcome_tick);
        let awake_pressure = world
            .organism_registry()
            .get(learner)
            .unwrap()
            .biochemistry()
            .homeostasis
            .hormones
            .sleep_pressure;
        let row = trace_row(
            &world,
            &[],
            &meals,
            |id| (id == learner).then_some(scheduler.state()),
            0.0,
        );
        assert_eq!(
            row["organisms"][0]["sleep_pressure"],
            serde_json::json!(awake_pressure)
        );
        assert_eq!(row["actions"], serde_json::json!([]));
    }

    #[test]
    fn simultaneous_eat_and_rest_or_food_before_recovery_is_not_reengagement() {
        let (initial_world, food) = prepared(true);
        let mut world = initial_world.clone();
        let mut patches = recover(&mut world, false, true);
        feed(
            &mut world,
            food,
            &mut patches,
            Some(ActionKind::Rest.canonical_id()),
        );
        assert!(
            patches
                .last()
                .unwrap()
                .outcome()
                .measured_physiology
                .unwrap()
                .after
                .body
                .sleeping
        );
        assert!(!lesson_completed(food, &patches));
        patches.push(execute(&mut world, Ids::LOOK_CENTER, None, &[]));
        assert!(
            !lesson_completed(food, &patches),
            "waking later is not a later meal"
        );

        let mut world = initial_world;
        let mut patches = Vec::new();
        feed(&mut world, food, &mut patches, None);
        patches.extend(recover(&mut world, false, false));
        patches.push(execute(&mut world, Ids::LOOK_CENTER, None, &[]));
        assert!(!lesson_completed(food, &patches));
    }

    #[test]
    fn fatigue_thresholds_and_measured_rest_benefit_remain_required() {
        let (mut world, food) = prepared(true);
        let mut patches = vec![execute(
            &mut world,
            ActionKind::Rest.canonical_id(),
            None,
            &[],
        )];
        let initial = patches[0].pre_action().homeostasis().drives.fatigue;
        feed(&mut world, food, &mut patches, None);
        // Counterfactual measured transitions test the numerical boundary;
        // ordinary Rest in this mature body relieves more than 0.02 in a tick.
        let insufficient: Vec<_> = patches
            .iter()
            .map(|p| {
                changed_outcome(p, |outcome| {
                    let t = outcome.measured_physiology.as_mut().unwrap();
                    t.after.homeostasis.drives.fatigue =
                        t.after.homeostasis.drives.fatigue.max(initial - 0.019);
                })
            })
            .collect();
        assert!(
            !lesson_completed(food, &insufficient),
            "a small fatigue decrease is insufficient"
        );

        let (mut world, food) = prepared(true);
        let mut patches = recover(&mut world, false, false);
        feed(&mut world, food, &mut patches, None);
        assert!(lesson_completed(food, &patches));
        let no_rest_measurements: Vec<_> = patches
            .iter()
            .map(|p| {
                changed_outcome(p, |outcome| {
                    if rest_executed(p) {
                        outcome.measured_physiology = None;
                    }
                })
            })
            .collect();
        assert!(!lesson_completed(food, &no_rest_measurements));
        let no_benefit: Vec<_> = patches
            .iter()
            .map(|p| {
                changed_outcome(p, |outcome| {
                    if rest_executed(p) {
                        let t = outcome.measured_physiology.as_mut().unwrap();
                        t.after.homeostasis.drives.fatigue = t.before.homeostasis.drives.fatigue;
                    }
                })
            })
            .collect();
        assert!(!lesson_completed(food, &no_benefit));
        let last = patches.pop().unwrap();
        let mut rebound = patches.clone();
        rebound.push(changed_outcome(&last, |outcome| {
            outcome
                .measured_physiology
                .as_mut()
                .unwrap()
                .after
                .homeostasis
                .drives
                .fatigue = patches[0].pre_action().homeostasis().drives.fatigue;
        }));
        assert!(
            !lesson_completed(food, &rebound),
            "final reduction must remain at least 0.02"
        );
        patches.push(changed_outcome(&last, |o| o.measured_physiology = None));
        assert!(
            !lesson_completed(food, &patches),
            "final measured reduction is mandatory"
        );

        let (mut world, food) = prepared(false);
        for _ in 0..7200 {
            if frame(&mut world).homeostasis().drives.fatigue >= 0.08 {
                break;
            }
            world.try_advance_tick().unwrap();
        }
        assert!(frame(&mut world).homeostasis().drives.fatigue >= 0.08);
        let mut patches = recover(&mut world, false, false);
        feed(&mut world, food, &mut patches, None);
        assert!(patches[0].pre_action().homeostasis().drives.fatigue < 0.12);
        assert!(!lesson_completed(food, &patches));
    }

    #[test]
    fn blocked_actions_missing_posture_receipts_and_representative_only_rest_fail() {
        let (mut world, food) = prepared(true);
        let mut patches = recover(&mut world, false, true);
        feed(&mut world, food, &mut patches, None);
        assert!(lesson_completed(food, &patches));
        for joint in [false, true] {
            let mut blocked = patches.clone();
            blocked[0] = changed_outcome(&blocked[0], |outcome| {
                if joint {
                    outcome.joint.as_mut().unwrap().channel_outcomes[0]
                        .physical
                        .contact = PhysicalContactKind::Blocked;
                } else {
                    outcome.physical.contact = PhysicalContactKind::Blocked;
                }
            });
            assert!(!lesson_completed(food, &blocked));
        }
        let unmeasured_posture: Vec<_> = patches
            .iter()
            .map(|p| {
                changed_outcome(p, |outcome| {
                    if let Some(joint) = &mut outcome.joint {
                        // Empty legacy channel evidence remains sealable;
                        // a partial mismatched bundle is rejected by the ABI.
                        joint.channel_outcomes.clear();
                    }
                })
            })
            .collect();
        assert!(!lesson_completed(food, &unmeasured_posture));

        // Rest as representative still needs its matching execution receipt.
        let (mut world, _) = prepared(true);
        let resting = execute(
            &mut world,
            ActionKind::Rest.canonical_id(),
            None,
            &[(MotorChannel::Posture, ActionKind::Rest.canonical_id(), None)],
        );
        assert!(rest_executed(&resting));
        let patch = changed_outcome(&resting, |outcome| {
            outcome.joint.as_mut().unwrap().channel_outcomes.clear();
        });
        assert!(!rest_executed(&patch));
        let idle = execute(
            &mut world,
            Ids::NO_POSTURE,
            None,
            &[(MotorChannel::Posture, Ids::NO_POSTURE, None)],
        );
        assert!(!rest_executed(&idle));
        let denied = changed_outcome(&resting, |outcome| outcome.success = false);
        assert!(!rest_executed(&denied));
        assert!(!lesson_completed(food, std::iter::empty()));
    }
}
