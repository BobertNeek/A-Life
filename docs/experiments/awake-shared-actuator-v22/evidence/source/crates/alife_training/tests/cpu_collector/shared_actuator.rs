use super::*;

fn bytes(host: &CpuCanonicalHost, trainer: &CpuFoundationTrainer) -> Vec<u8> {
    serde_json::to_vec(&host.checkpoint(trainer).unwrap()).unwrap()
}

#[test]
fn shared_actuator_quiet_host_is_exact_ordinary_execution() {
    let (mut ordinary, trainer) = fixture();
    let mut candidate = ordinary.snapshot();
    let expected = ordinary.step(&trainer, 1.0).unwrap();
    let actual = candidate
        .step_with_shared_contact_priority(&trainer, 1.0)
        .unwrap();
    assert!(actual.interventions.is_empty());
    assert_eq!(expected.decisions.len(), actual.ordinary.decisions.len());
    assert!(
        bytes(&ordinary, &trainer) == bytes(&candidate, &trainer),
        "quiet state changed"
    );
    assert!(
        serde_json::to_vec(&expected.decisions).unwrap()
            == serde_json::to_vec(&actual.ordinary.decisions).unwrap(),
        "quiet experience changed"
    );
}

#[test]
fn shared_actuator_real_dispatch_consumed_without_false_experience_and_restores() {
    let (game, mut trainer) = new_game_fixture(None);
    // Explicit synthetic optimizer sentinels, not acquired biological learning.
    let mut sentinel = trainer.checkpoint();
    sentinel.optimizer_step = 7;
    sentinel.update_ages[0] = 7;
    sentinel.first_moment[0] = 0.125;
    sentinel.second_moment[0] = 0.25;
    trainer.restore_checkpoint(sentinel).unwrap();
    let founder_before = trainer.checkpoint();
    let (mut host, source) =
        CpuCanonicalHost::new_prepared_cohort(game, &trainer, 0x1818, |world| {
            let (_, entity) = world.organism_entity_ids()[0];
            let position = world.entity(entity).unwrap().position;
            world.editor_spawn_object(WorldEditorSpawnSpec {
                label: "v18-acute-center".into(),
                kind: WorldObjectKind::Hazard,
                organism_id: None,
                position,
                nutrition: 0.0,
                hazard_pain: 0.4,
                radius: 0.02,
                token_id: None,
            })
        })
        .unwrap();
    let before = host.checkpoint(&trainer).unwrap();
    let step = host
        .step_with_shared_contact_priority(&trainer, 1.0)
        .unwrap();
    assert_eq!(step.interventions.len(), 1);
    assert!(step.ordinary.decisions.is_empty());
    let intervention = &step.interventions[0];
    let physical = &intervention.physical;
    let reflex = physical.withdrawal.as_ref().unwrap();
    assert_eq!(
        reflex.direction_basis,
        WithdrawalDirectionBasis::AcuteDamageHeadingFallback
    );
    assert!(physical.allocated_translation_time <= 1.0);
    assert!(
        intervention.cognitive_work.prediction_ops > 0,
        "executed forecast work disappeared"
    );
    assert_eq!(
        intervention.cognitive_work.learning_ops, 1,
        "staged eligibility work disappeared"
    );
    assert_eq!(
        intervention.selected.sequence_id,
        physical.voluntary.bundle.sequence_id
    );
    let after = host.checkpoint(&trainer).unwrap();
    let a = &after.residents[0];
    let b = &before.residents[0];
    assert!(a.neural.fast == b.neural.fast && a.neural.lifetime == b.neural.lifetime);
    assert!(a.neural.eligibility == b.neural.eligibility);
    assert_eq!(a.neural.weight_generation, b.neural.weight_generation);
    assert_eq!(a.next_sequence, b.next_sequence);
    assert!(a.neural.dispatch_generation > b.neural.dispatch_generation);
    assert!(a.memory == b.memory, "false family memory recorded");
    assert!(
        a.predictor == b.predictor,
        "reflex taught a voluntary motor prediction"
    );
    assert!(
        serde_json::to_vec(&a.language_grounding).unwrap()
            == serde_json::to_vec(&b.language_grounding).unwrap()
    );
    let aj = serde_json::to_value(&after).unwrap();
    let bj = serde_json::to_value(&before).unwrap();
    assert!(aj["residents"][0]["replay"] == bj["residents"][0]["replay"]);
    assert!(trainer.checkpoint() == founder_before);
    let saved = serde_json::to_vec(&after).unwrap();
    let mut restored =
        CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&saved).unwrap(), &trainer)
            .unwrap();
    assert!(bytes(&restored, &trainer) == saved);
    // No experience was issued by the consumed dispatch. A new genuine
    // dispatch may issue that sequence, including after ordinary-mode restore.
    host.world_mut().editor_remove_object(source).unwrap();
    restored.world_mut().editor_remove_object(source).unwrap();
    let next = host.step(&trainer, 1.0).unwrap();
    let twin = restored.step(&trainer, 1.0).unwrap();
    assert_eq!(next.decisions.len(), 1);
    assert_eq!(
        next.decisions[0].patch.header().sequence_id.raw(),
        a.next_sequence
    );
    assert!(
        next.decisions[0]
            .patch
            .decision()
            .neural_evidence()
            .unwrap()
            .dispatch_generation
            > intervention.dispatch_generation
    );
    assert!(
        serde_json::to_vec(&next.decisions).unwrap()
            == serde_json::to_vec(&twin.decisions).unwrap()
    );
    assert!(bytes(&host, &trainer) == bytes(&restored, &trainer));
    assert!(trainer.checkpoint() == founder_before);
}

#[test]
fn shared_actuator_repeated_contact_keeps_sequence_unissued_and_releases() {
    let (game, trainer) = new_game_fixture(None);
    let (mut host, source) =
        CpuCanonicalHost::new_prepared_cohort(game, &trainer, 0x1819, |world| {
            let (_, entity) = world.organism_entity_ids()[0];
            let mut position = world.entity(entity).unwrap().position;
            position.x += 0.1;
            world.editor_spawn_object(WorldEditorSpawnSpec {
                label: "v18-repeated-contact".into(),
                kind: WorldObjectKind::Hazard,
                organism_id: None,
                position,
                nutrition: 0.0,
                hazard_pain: 0.02,
                radius: 0.8,
                token_id: None,
            })
        })
        .unwrap();
    let before = host.checkpoint(&trainer).unwrap();
    let mut dispatch = before.residents[0].neural.dispatch_generation;
    for _ in 0..2 {
        let step = host
            .step_with_shared_contact_priority(&trainer, 1.0)
            .unwrap();
        assert!(step.ordinary.decisions.is_empty());
        assert_eq!(step.interventions.len(), 1);
        assert!(step.interventions[0].dispatch_generation > dispatch);
        dispatch = step.interventions[0].dispatch_generation;
        let current = host.checkpoint(&trainer).unwrap();
        assert_eq!(
            current.residents[0].next_sequence,
            before.residents[0].next_sequence
        );
        assert!(current.residents[0].neural.fast == before.residents[0].neural.fast);
    }
    host.world_mut().editor_remove_object(source).unwrap();
    let step = host
        .step_with_shared_contact_priority(&trainer, 1.0)
        .unwrap();
    assert!(
        step.interventions.is_empty(),
        "acute response remained latched"
    );
    assert_eq!(step.ordinary.decisions.len(), 1);
}

#[test]
fn shared_actuator_exhausted_late_meal_retains_exact_ordinary_experience() {
    shared_actuator_meal_at_origin(0.0);
}

#[test]
fn shared_actuator_translated_meal_has_no_phantom_time_or_credit_loss() {
    shared_actuator_meal_at_origin(3.0);
    shared_actuator_meal_at_origin(1000.0);
}

#[test]
fn common_endpoint_real_meal_at_x128_retains_ordinary_credit_and_checkpoint() {
    shared_actuator_meal_at_origin(128.0);
}

#[test]
fn common_endpoint_fresh_half_speed_calibration_keeps_real_meal_credit() {
    for origin in [128.0, 1000.0] {
        shared_actuator_meal_at_origin_with_calibration(origin, true);
    }
}

fn shared_actuator_meal_at_origin(origin_x: f32) {
    shared_actuator_meal_at_origin_with_calibration(origin_x, false);
}

fn shared_actuator_meal_at_origin_with_calibration(origin_x: f32, half_speed: bool) {
    // New declared inherited readout factor after the unchanged founder selected Inspect.
    // Same seed, normal native sampler, no forced measured action or neural state writes.
    let (game, mut trainer, _) =
        new_game_with_readout_fixture(None, SensorProfile::GroundedObjectSlotsV1, true);
    let mut sentinel = trainer.checkpoint();
    sentinel.optimizer_step = 7;
    sentinel.update_ages[0] = 7;
    sentinel.first_moment[0] = 0.125;
    sentinel.second_moment[0] = 0.25;
    trainer.restore_checkpoint(sentinel).unwrap();
    let founder_before = trainer.checkpoint();
    let (mut candidate, food) =
        CpuCanonicalHost::new_prepared_cohort(game, &trainer, 0x1919, |world| {
            let (organism, entity) = world.organism_entity_ids()[0];
            // Explicit sparse opportunity fixture: prevent unrelated canonical food
            // targets from filling the finite manipulation candidate mask.
            world.configure_ecology(EcologyState::default())?;
            for object in world.object_snapshots() {
                if object.id != entity {
                    world.editor_remove_object(object.id)?;
                }
            }
            // Same declared fixture at translated coordinates. Each case is
            // paired against its own exact ordinary execution.
            world.editor_move_object(entity, Vec3f::new(origin_x, 0.0, 0.0))?;
            let mut position = world.entity(entity).unwrap().position;
            position.x += 0.3;
            let food = world.editor_spawn_object(WorldEditorSpawnSpec {
                label: "v19-toxic-held-meal".into(),
                kind: WorldObjectKind::Food,
                organism_id: None,
                position,
                nutrition: 0.6,
                hazard_pain: 0.4,
                // Held grip offset is0.5; radius0.6 provides actual surface
                // contact/taste rather than a distant chemical oracle.
                radius: 0.6,
                token_id: None,
            })?;
            let grab = ActionCommand::structured(
                organism,
                HeadlessActionIds::GRAB,
                ActionKind::Hold,
                ActionTarget::new(Some(food), None),
                Intensity::new(1.0)?,
                DurationTicks::new(1),
                Confidence::new(0.9)?,
                0,
                None,
                None,
                None,
            )?;
            assert!(world.apply_command(&grab)?.execution.succeeded);
            assert_eq!(world.entity(food).unwrap().carried_by, Some(organism));
            if half_speed {
                // Explicit fresh calibration control, not acquired learning.
                // Normal sampler still chooses the complete real bundle.
                let mut record = world.organism_registry().get(organism).unwrap().clone();
                let mut embodiment = record.embodiment().clone();
                let sensors = embodiment.sensor_calibration().to_vec();
                let mut effectors = embodiment.effector_controllability().to_vec();
                let mut offset = 0;
                for port in embodiment.effectors() {
                    let width = usize::from(port.command_lanes);
                    if port.capability == alife_core::EffectorCapability::Translation {
                        effectors[offset..offset + width].fill(-1.0);
                    }
                    offset += width;
                }
                let body = vec![0.0; embodiment.body_schema().len()];
                embodiment.replace_calibration(world.tick(), sensors, effectors, body)?;
                assert_eq!(
                    embodiment.effector_gain(alife_core::EffectorCapability::Translation),
                    0.5
                );
                assert_eq!(embodiment.proprioceptive_gain(), 1.0);
                record.replace_embodiment_state(embodiment)?;
                world.replace_organism_record_exact(record)?;
            }
            Ok::<_, ScaffoldContractError>(food)
        })
        .unwrap();
    let before = candidate.checkpoint(&trainer).unwrap();
    let mut ordinary = candidate.snapshot();
    let expected = ordinary.step(&trainer, 0.01).unwrap();
    let actual = candidate
        .step_with_shared_contact_priority(&trainer, 0.01)
        .unwrap();
    let sampled = expected
        .decisions
        .iter()
        .map(|d| d.patch.decision().selected_action.action_id.raw())
        .collect::<Vec<_>>();
    for decision in &expected.decisions {
        let replay = trainer
            .evaluate_replay(&alife_training::TrainingSequence {
                phenotype_hash: trainer.phenotype().phenotype_hash(),
                initial: decision.initial.clone(),
                ticks: vec![decision.replay_tick.clone()],
                burn_in_ticks: 0,
                memory_candidate_gain: trainer
                    .phenotype()
                    .candidate_decoder()
                    .memory_channel()
                    .map_or(0.0, |m| m.max_candidate_gain()),
            })
            .unwrap();
        for (index, c) in decision.replay_tick.candidates.iter().enumerate() {
            let mut range = (f32::INFINITY, f32::NEG_INFINITY);
            for s in trainer.phenotype().synapses() {
                if matches!(s.kind(), CompiledSynapseKind::Decoder(d)
                    if d.head() == DecoderHeadKind::ActionCandidate && d.family() == c.family)
                {
                    let a = replay.final_activations[0][s.source() as usize];
                    range.0 = range.0.min(a);
                    range.1 = range.1.max(a);
                }
            }
            eprintln!("detached candidate={index} family={:?} taste={} innate={} logit={} source_range={range:?}",
                c.family, c.decoder_inputs[15], c.innate_bias, replay.candidate_logits[0][index]);
        }
        eprintln!("v19 inherited-meal representative={:?} motor_candidate_indices={:?} candidate_count={}", sampled, decision.action.motor_candidates, decision.action.candidate_count);
        eprintln!(
            "v19 inherited-meal channels={:?}",
            decision
                .patch
                .decision()
                .selected_bundle
                .as_ref()
                .map(|b| &b.channels)
        );
    }
    assert!(
        ordinary.world().entity(food).unwrap().consumed,
        "fixed actual sampler did not consume prepared meal; selected={sampled:?}"
    );
    for intervention in &actual.interventions {
        let p = &intervention.physical;
        eprintln!("intervention altered={} exhausted={} no_effect={} voluntary_time={} reflex_time={} withdrawal={:?}",
            p.voluntary_execution_altered, p.reflex_budget_exhausted, p.exhausted_late_no_effect,
            p.voluntary_time_allocated, p.reflex_time_allocated, p.withdrawal);
    }
    assert!(
        actual.interventions.is_empty(),
        "fixture did not produce an unaffected exhausted response"
    );
    assert_eq!(actual.unaltered_responses.len(), 1);
    let response = &actual.unaltered_responses[0];
    assert!(response.exhausted_late_no_effect && response.reflex_budget_exhausted);
    if half_speed {
        assert!(response.voluntary_path > 0.049 && response.voluntary_path <= 0.05 + 1e-6);
        assert_eq!(response.voluntary_time_allocated, 1.0);
        eprintln!(
            "FRESH_HALF_SPEED_MEAL origin={origin_x} actual_path={} time={}",
            response.voluntary_path, response.voluntary_time_allocated
        );
    }
    assert!(response
        .voluntary
        .channel_receipts
        .iter()
        .any(|channel| channel.physical.contact == PhysicalContactKind::Consumed));
    let reflex = response.withdrawal.as_ref().unwrap();
    assert!(reflex.blocked && reflex.displacement == Vec3f::ZERO && reflex.energy_cost == 0.0);
    assert_eq!(response.reflex_time_allocated, 0.0);
    assert!(
        serde_json::to_vec(&expected.decisions).unwrap()
            == serde_json::to_vec(&actual.ordinary.decisions).unwrap()
    );
    assert!(
        bytes(&ordinary, &trainer) == bytes(&candidate, &trainer),
        "meal state/credit differs from ordinary"
    );
    let after = candidate.checkpoint(&trainer).unwrap();
    assert_eq!(
        after.residents[0].next_sequence,
        before.residents[0].next_sequence + 1
    );
    assert_eq!(
        after.residents[0].neural.weight_generation,
        before.residents[0].neural.weight_generation + 1
    );
    assert_eq!(
        after.residents[0].memory.last_observed_sequence_id_raw,
        before.residents[0].next_sequence
    );
    assert!(trainer.checkpoint() == founder_before);
}
