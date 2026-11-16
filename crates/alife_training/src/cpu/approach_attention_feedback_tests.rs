//! Manual readout calibration preserves acquired state; synthetic tests, not efficacy.
use super::*;
use alife_core::experience::ChannelPhysicalOutcome;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_world::*;
fn fixture(profile: SensorProfile) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    fixture_kind(profile, false)
}
fn fixture_kind(profile: SensorProfile, hazard: bool) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    let grip = true;
    let asset = FoundationWeightAsset::decode_canonical(include_bytes!("../../../../docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation")).unwrap();
    let mut genome: CreatureGenome = serde_json::from_slice(include_bytes!("../../../../docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation.genome.json")).unwrap();
    for architecture in [
        &mut genome.brain.construction.maternal_architecture,
        &mut genome.brain.construction.paternal_architecture,
    ] {
        *architecture = architecture.with_memory_candidate_bound_v4(1.0).unwrap();
        if grip {
            *architecture = architecture.with_grip_proprioception_v5().unwrap();
        }
    }
    // Fresh hereditary decoder-plan identity requires a matching foundation ABI.
    // Preserve every weight bit and coordinate; never reinterpret a plan-one asset.
    let seed_expression = genome.express().unwrap();
    let seed_development = foundation_construction_development(
        &seed_expression.brain_genome,
        &BrainCapacityClass::n2048(),
        &seed_expression.development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let (fresh, _) = PhenotypeCompiler::compile_n2048_anchored_v2_graph(
        seed_expression.brain_genome.clone(),
        seed_development,
        profile,
    )
    .expect("fresh inherited architecture graph");
    let rebound = FoundationWeightAsset::from_trained_weights(
        &fresh,
        asset.weights().to_vec(),
        asset.manifest().training_stage(),
    )
    .expect("fresh foundation ABI binding");
    assert_eq!(asset.weights().len(), rebound.weights().len());
    assert!(asset
        .weights()
        .iter()
        .zip(rebound.weights())
        .all(|(a, b)| a.to_bits() == b.to_bits()));
    assert_eq!(
        asset.manifest().address_map_digest(),
        rebound.manifest().address_map_digest()
    );
    genome = genome
        .with_n2048_foundation_candidate(rebound.clone())
        .unwrap();
    let asset = rebound;
    let mut config = CanonicalNewGameConfig::phase3(2306, 1).unwrap();
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = asset.manifest().sensor_profile();
    let mut game =
        create_canonical_new_game_with_n2048_experimental_candidate(&config, &asset).unwrap();
    let owner = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    let expressed = genome.express().unwrap();
    let development = foundation_construction_development(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &expressed.development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let (p, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        expressed.brain_genome.clone(),
        development,
        asset.clone(),
    )
    .unwrap();
    assert_eq!(p.cognitive_architecture().grip_proprioception(), grip);
    // Entirely fresh synthetic world and biological record, not an acquired-body conversion.
    let builder = HeadlessScenarioBuilder::new(2306).agent("subject", owner, Vec3f::ZERO);
    let builder = if hazard {
        builder.hazard("near", Vec3f::new(0.25, 0., 0.), 0.05)
    } else {
        builder.food("near", Vec3f::new(0.25, 0., 0.), 0.4)
    };
    let mut world = builder.build().unwrap();
    let biology = BiochemistryState::new(&expressed, Tick::ZERO).unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::new(
                owner,
                world.entity_id("subject").unwrap(),
                genome.clone(),
                expressed,
                biology,
                Tick::ZERO,
            )
            .unwrap(),
        )
        .unwrap();
    game.world = world;
    game.creatures[0].genome_id = genome.id;
    game.creatures[0].mind.tick = biology.tick;
    game.creatures[0].mind.homeostasis = biology.homeostasis;
    game.creatures[0].development_tick = biology.development.last_update_tick;
    game.creatures[0].weights.genetic_fixed_digest =
        PortableAssetDigest::for_bytes(&serde_json::to_vec(&genome).unwrap()).0;
    let mask = crate::StageTrainableMask::recurrent_only(&p).unwrap();
    let trainer = CpuFoundationTrainer::new(p, asset, mask, crate::AdamWConfig::default()).unwrap();
    (
        CpuCanonicalHost::new_cohort(game, &trainer, 2306).unwrap(),
        trainer,
    )
}
fn serialized(host: &CpuCanonicalHost, trainer: &CpuFoundationTrainer) -> Vec<u8> {
    serde_json::to_vec(&host.checkpoint(trainer).unwrap()).unwrap()
}

#[test]
fn attach_exact_identity_legacy_cold_and_once_only_cost() {
    for apply in [false, true] {
        let (mut old, t) = fixture(SensorProfile::GroundedObjectSlotsV1);
        assert!(old
            .fork_with_approach_attention_feedback_for_research(&t)
            .is_err());
        old.enable_joint_ingest_direct2295(&t, apply).unwrap();
        old.enable_candidate_opportunity_coverage2313(&t).unwrap();
        let before = old.checkpoint(&t).unwrap();
        let trainer = serde_json::to_vec(&t.checkpoint()).unwrap();
        let mut new = old
            .fork_with_approach_attention_feedback_for_research(&t)
            .unwrap();
        let cp = new.checkpoint(&t).unwrap();
        assert_eq!(cp.schema_version, 17);
        assert_eq!(cp.candidate_opportunity_coverage2313, Some(2));
        let mut norm = cp.clone();
        norm.schema_version = 16;
        norm.candidate_opportunity_coverage2313 = Some(1);
        assert_eq!(
            serde_json::to_vec(&norm).unwrap(),
            serde_json::to_vec(&before).unwrap()
        );
        assert!(new
            .fork_with_approach_attention_feedback_for_research(&t)
            .is_err());
        let mut cold = CpuCanonicalHost::from_checkpoint(cp, &t).unwrap();
        let mut legacycold = CpuCanonicalHost::from_checkpoint(before.clone(), &t).unwrap();
        for _ in 0..4 {
            let a = new.step(&t, 1.).unwrap();
            let b = cold.step(&t, 1.).unwrap();
            assert_eq!(
                serde_json::to_vec(&a.decisions).unwrap(),
                serde_json::to_vec(&b.decisions).unwrap()
            );
            assert_eq!(serialized(&new, &t), serialized(&cold, &t));
            for row in &a.decisions {
                let f = row.approach_attention_feedback2325.as_ref().unwrap();
                assert_eq!(
                    f.frame_digest,
                    row.patch.pre_action().perception().frame_digest()
                );
                assert!(f.work_units >= 13);
                let c = row.patch.pre_action().cognitive_context.as_ref();
                let frame = row.patch.pre_action().perception();
                let expected = c.unwrap().attention.budget_receipt.work_units
                    + frame.grip_proprioception().map_or(0, |r| r.work_units())
                    + frame
                        .candidate_opportunity_coverage()
                        .map_or(0, |r| r.work_units())
                    + f.work_units;
                assert_eq!(
                    row.patch.cognitive_work().unwrap().focal_target_ops,
                    expected
                );
                let record = new
                    .world()
                    .organism_registry()
                    .get(row.organism_id)
                    .unwrap();
                let (policy, debit, energy_before, energy_after) =
                    row.cognitive_cost_capture.unwrap();
                let phenotype = record.phenotype();
                let expected_debit = policy
                    .energy_debit(row.patch.cognitive_work().unwrap())
                    .unwrap()
                    * phenotype.body.metabolic_turnover
                    * (ContinuousLocus::midpoint_value(0.52, 0.60)
                        - phenotype.chemistry.brain_atp_efficiency)
                        .exp2();
                assert_eq!(debit, expected_debit.min(energy_before));
                assert_eq!(record.cognitive_energy_debit(), debit);
                assert!(energy_after <= energy_before);
                let (mut body_before, body_after) = row.cognitive_body_cost_capture.unwrap();
                assert_eq!(body_before.energy, energy_before);
                assert_eq!(
                    body_before.debit_energy_pro_rata(expected_debit).unwrap(),
                    debit
                );
                assert_eq!(
                    serde_json::to_vec(&body_before).unwrap(),
                    serde_json::to_vec(&body_after).unwrap()
                );
                assert_eq!(body_after.energy, energy_after);
                assert_eq!(record.cognitive_work(), row.patch.cognitive_work().unwrap());
                let encoded = serde_json::to_vec(row).unwrap();
                let decoded: CpuCollectedDecision = serde_json::from_slice(&encoded).unwrap();
                assert_eq!(encoded, serde_json::to_vec(&decoded).unwrap());
            }
            let a = old.step(&t, 1.).unwrap();
            let b = legacycold.step(&t, 1.).unwrap();
            assert_eq!(
                serde_json::to_vec(&a.decisions).unwrap(),
                serde_json::to_vec(&b.decisions).unwrap()
            );
            assert_eq!(serialized(&old, &t), serialized(&legacycold, &t));
            assert!(a
                .decisions
                .iter()
                .all(|r| r.approach_attention_feedback2325.is_none()));
        }
        assert_eq!(trainer, serde_json::to_vec(&t.checkpoint()).unwrap());
        for (schema, adapter) in [(16, Some(2)), (17, Some(1)), (17, None), (17, Some(3))] {
            let mut bad = before.clone();
            bad.schema_version = schema;
            bad.candidate_opportunity_coverage2313 = adapter;
            assert!(CpuCanonicalHost::from_checkpoint(bad, &t).is_err());
        }
    }
}
#[test]
fn successful_approach_only_generic_binding_and_hysteresis() {
    for hazard in [false, true] {
        let (mut host, t) = fixture_kind(SensorProfile::GroundedObjectSlotsV1, hazard);
        host.enable_joint_ingest_direct2295(&t, true).unwrap();
        host.enable_candidate_opportunity_coverage2313(&t).unwrap();
        let row = host.step(&t, 1.).unwrap().decisions.remove(0);
        let frame = row.patch.pre_action().perception();
        let mut action = row.action.clone();
        let index = frame
            .candidates()
            .iter()
            .position(|c| c.family == CandidateActionFamily::Approach && c.target.entity.is_some())
            .unwrap();
        let c = &frame.candidates()[index];
        action.motor_candidates[0] = Some(index as u16);
        let mut joint = row.patch.outcome().joint.clone().unwrap();
        joint.channel_outcomes.clear();
        let mut physical = joint.execution;
        physical.target_entity = c.target.entity;
        let evidence = ExecutedChannelEvidenceV1::new(
            c.action_id,
            c.target.entity,
            true,
            BodyEventDelta::zero(),
        )
        .unwrap();
        joint.channel_outcomes.push(
            ChannelPhysicalOutcome::new(MotorChannel::Locomotion, physical)
                .unwrap()
                .with_execution_evidence(evidence)
                .unwrap(),
        );
        let before = HysteresisState::default();
        let f = approach_attention_feedback(frame, &action, &joint, host.world(), before).unwrap();
        assert!(f.nominated);
        assert_eq!(f.after.retained_ticks, 1);
        assert_eq!(f.after.switch_margin, before.switch_margin);
        let mut retained = f.after;
        retained.retained_ticks = 9;
        let same =
            approach_attention_feedback(frame, &action, &joint, host.world(), retained).unwrap();
        assert_eq!(same.after.retained_ticks, 9);
        assert_eq!(f.work_units, same.work_units + 2);
        let mut missing = host.world().clone();
        missing
            .editor_remove_object(c.target.entity.unwrap())
            .unwrap();
        assert!(
            !approach_attention_feedback(frame, &action, &joint, &missing, before)
                .unwrap()
                .nominated
        );
        if !hazard {
            let mut consumed = host.world().clone();
            let eat = ActionCommand::structured(
                frame.organism_id(),
                HeadlessActionIds::EAT,
                ActionKind::Interact,
                c.target,
                Intensity::new(1.).unwrap(),
                DurationTicks::new(1),
                Confidence::new(1.).unwrap(),
                0,
                None,
                None,
                None,
            )
            .unwrap();
            assert!(consumed.apply_command(&eat).unwrap().execution.succeeded);
            assert!(consumed.entity(c.target.entity.unwrap()).unwrap().consumed);
            assert!(
                !approach_attention_feedback(frame, &action, &joint, &consumed, before)
                    .unwrap()
                    .nominated
            );
        }
        let policy = attention_selection_policy_for(t.phenotype());
        let old_context = row.patch.pre_action().cognitive_context.as_ref().unwrap();
        let absent = old_context
            .attention
            .peripheral_summaries
            .iter()
            .filter(|s| Some(s.identity) != f.after.previous_identity)
            .cloned()
            .collect::<Vec<_>>();
        let next = select_focal_targets(
            frame.organism_id(),
            old_context.attention.sequence_id,
            frame.tick(),
            &absent,
            f.after,
            policy,
        )
        .unwrap();
        assert!(!next
            .focal_targets
            .contains(&f.after.previous_identity.unwrap()));
        joint.channel_outcomes[0]
            .execution_evidence
            .as_mut()
            .unwrap()
            .succeeded = false;
        let failed =
            approach_attention_feedback(frame, &action, &joint, host.world(), before).unwrap();
        assert!(!failed.nominated);
        assert_eq!(failed.after, before);
        joint.channel_outcomes[0]
            .execution_evidence
            .as_mut()
            .unwrap()
            .succeeded = true;
        joint.channel_outcomes[0]
            .execution_evidence
            .as_mut()
            .unwrap()
            .command_target = Some(WorldEntityId::new(999).unwrap());
        assert!(approach_attention_feedback(frame, &action, &joint, host.world(), before).is_err());
        action.motor_candidates[0] = frame
            .candidates()
            .iter()
            .position(|c| c.family == CandidateActionFamily::Avoid)
            .map(|i| i as u16);
        assert!(
            !approach_attention_feedback(frame, &action, &joint, host.world(), before)
                .unwrap()
                .nominated
        );
        for (i, c) in frame.candidates().iter().enumerate().filter(|(_, c)| {
            c.family == CandidateActionFamily::Approach && c.action_id.raw() != 101
        }) {
            action.motor_candidates[0] = Some(i as u16);
            let skipped =
                approach_attention_feedback(frame, &action, &joint, host.world(), before).unwrap();
            assert!(!skipped.nominated);
            assert_eq!(skipped.after, before);
            assert!(c.action_id.raw() != 101);
        }
        assert!(frame.candidates().iter().any(|c| c.action_id.raw() == 103));
        action.motor_candidates[0] = None;
        assert_eq!(
            approach_attention_feedback(frame, &action, &joint, host.world(), before)
                .unwrap()
                .work_units,
            13
        );
    }
}
#[test]
fn approach_feedback_preserves_sleep_and_consolidation() {
    let (mut h, t) = fixture(SensorProfile::GroundedObjectSlotsV1);
    h.enable_joint_ingest_direct2295(&t, true).unwrap();
    h.enable_candidate_opportunity_coverage2313(&t).unwrap();
    let own = h
        .world()
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    // Synthetic compatibility setup only, not an actual research intervention.
    h.request_recovery_sleep(own).unwrap();
    let before = serialized(&h, &t);
    let mut warm = h
        .fork_with_approach_attention_feedback_for_research(&t)
        .unwrap();
    let mut cold = CpuCanonicalHost::from_checkpoint(warm.checkpoint(&t).unwrap(), &t).unwrap();
    let mut sleeps = 0;
    let mut consolidations = 0;
    for _ in 0..256 {
        let a = warm.step(&t, 1.).unwrap();
        let b = cold.step(&t, 1.).unwrap();
        sleeps += a.sleeping_organisms;
        consolidations += a.sleep_consolidations;
        assert_eq!(
            serde_json::to_vec(&a.decisions).unwrap(),
            serde_json::to_vec(&b.decisions).unwrap()
        );
    }
    assert!(sleeps > 0 && consolidations > 0);
    assert_eq!(serialized(&warm, &t), serialized(&cold, &t));
    assert_eq!(serialized(&h, &t), before);
}
