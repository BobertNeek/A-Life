//! Opt-in layout-v2 CPU sampling/credit contracts and real world motor execution.
//! Deliberately supplied logits isolate factorization, not learned navigation.
//! The physical tests use ordinary candidates, embodiment, collision and biology.
//! Synthetic checkpoint sentinels test preservation, not acquired competence.
#![cfg(feature = "cpu-backend")]

use alife_core::*;
use alife_training::cpu::*;
use alife_training::{
    AdamWConfig, PpoConfig, PpoJointAction, StageTrainableMask, TrainingInitialState,
};
use alife_world::*;

const SUBJECT: OrganismId = OrganismId(1);
const PROFILE: SensorProfile = SensorProfile::GroundedTerrainVisionV1;

fn genome(independent_gaze: bool) -> BrainGenome {
    let capacity = BrainCapacityClass::n2048();
    let mut genome = BrainGenome::scaffold(0x0A11_2026, capacity.id());
    if independent_gaze {
        let parameters = (*genome.cognitive_architecture())
            .with_independent_orientation_v2(2)
            .unwrap();
        genome = genome.with_cognitive_architecture(parameters).unwrap();
    }
    genome
}

fn phenotype(independent_gaze: bool) -> BrainPhenotype {
    let capacity = BrainCapacityClass::n2048();
    let genome = genome(independent_gaze);
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        PROFILE,
    )
    .unwrap()
}

fn world(blocked: bool) -> HeadlessWorld {
    let mut world = HeadlessScenarioBuilder::new(0x0A11_2026)
        .agent("subject", SUBJECT, Vec3f::ZERO)
        .build()
        .unwrap();
    if blocked {
        let terrain = WorldTerrain::new(
            TerrainData {
                width: 9,
                depth: 9,
                origin_x: -4.0,
                origin_z: -4.0,
                spacing: 1.0,
                heights: vec![0.0; 81],
                // Initial body radius .2 clears this wall; a forward interval does not.
                obstacles: vec![[0.21, -2.0, 0.8, 2.0, 0.0, 2.0]],
                water_level: None,
            },
            LocomotionLimits::default(),
        )
        .unwrap();
        world
            .enable_terrain_for_new_game(terrain, Vec3f::ZERO)
            .unwrap();
    }
    let genome = CreatureGenome::early_mammal_founder(
        0x0A11_2026,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N2048_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let entity = world.entity_id("subject").unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::newborn(SUBJECT, entity, genome, phenotype, Tick::ZERO).unwrap(),
        )
        .unwrap();
    world
}

fn frame(world: &mut HeadlessWorld) -> PerceptionFrame {
    let homeostasis = world
        .organism_registry()
        .get(SUBJECT)
        .unwrap()
        .biochemistry()
        .homeostasis;
    world
        .perception_frame(SUBJECT, world.tick(), PROFILE, homeostasis)
        .unwrap()
}

// Controlled sampler fixtures retain native command/feature semantics and frame
// provenance. Subsetting/reindexing is test-only, never a gameplay input filter.
fn subset(frame: &PerceptionFrame, ids: &[ActionId]) -> PerceptionFrame {
    let candidates = ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let mut candidate = *frame
                .candidates()
                .iter()
                .find(|c| c.action_id == *id)
                .unwrap();
            candidate.candidate_index = index as u16;
            candidate
        })
        .collect();
    PerceptionFrame::new(
        SUBJECT,
        frame.tick(),
        PROFILE,
        frame.sensory().clone(),
        frame.body(),
        *frame.homeostasis(),
        candidates,
        frame.profile_provenance(),
        frame.grounded_object_slots().to_vec(),
    )
    .unwrap()
}

fn bundle(
    p: &BrainPhenotype,
    frame: &PerceptionFrame,
    action: &PpoJointAction,
) -> MotorCommandBundle {
    let candidate = frame.candidates()[action.representative as usize];
    let command = candidate
        .to_command(SUBJECT, candidate.sensor_confidence)
        .unwrap();
    let slots = action.motor_candidates.map(|i| i.map_or(0, |i| i + 1));
    let bundle = factorized_motor_bundle_for_candidates(
        SUBJECT,
        ExperienceSequenceId(frame.tick().raw() + 1),
        frame.tick(),
        frame,
        slots,
        &p.candidate_decoder().factorized_motor_channels(p).unwrap(),
        &command,
        action.representative,
        None,
        false,
    )
    .unwrap();
    JointActionSelectionV1::new(slots, &bundle)
        .unwrap()
        .validate_bundle(&bundle)
        .unwrap();
    bundle
}

fn primitive(bundle: &MotorCommandBundle, channel: MotorChannel) -> Option<ActionId> {
    bundle
        .channels
        .iter()
        .find(|c| c.channel == channel)
        .map(|c| c.primitive)
}

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= 2e-6, "{actual} != {expected}");
}

#[test]
fn move_representative_gets_independent_gaze_only_in_opt_in_layout() {
    let old = phenotype(false);
    let new = phenotype(true);
    let f = subset(
        &frame(&mut world(false)),
        &[
            HeadlessActionIds::STEP_FORWARD,
            HeadlessActionIds::LOOK_LEFT,
            HeadlessActionIds::LOOK_RIGHT,
        ],
    );
    // Saturation is intentional only to make this plumbing unit test deterministic.
    let logits = [1000.0, 0.0, -1000.0];
    for (p, independent) in [(&old, false), (&new, true)] {
        let a = sample_cpu_joint_action(p, &f, &logits, 42, 0, 1.0).unwrap();
        assert_eq!(a.representative, 0);
        assert_eq!(a.motor_candidates[0], Some(0));
        assert_eq!(a.motor_candidates[1], independent.then_some(1));
        assert_eq!(a.motor_masks[1], if independent { 0b110 } else { 0 });
        let b = bundle(p, &f, &a);
        assert_eq!(
            primitive(&b, MotorChannel::Locomotion),
            Some(HeadlessActionIds::STEP_FORWARD)
        );
        assert_eq!(
            primitive(&b, MotorChannel::Orientation),
            independent.then_some(HeadlessActionIds::LOOK_LEFT)
        );
    }
}

#[test]
fn look_representative_keeps_independent_locomotion_in_both_layouts() {
    let f = subset(
        &frame(&mut world(false)),
        &[
            HeadlessActionIds::STEP_FORWARD,
            HeadlessActionIds::LOOK_LEFT,
        ],
    );
    for p in [phenotype(false), phenotype(true)] {
        let a = sample_cpu_joint_action(&p, &f, &[0.0, 1000.0], 42, 0, 1.0).unwrap();
        assert_eq!(a.representative, 1);
        assert_eq!(a.motor_candidates[..2], [Some(0), Some(1)]);
        let b = bundle(&p, &f, &a);
        assert_eq!(
            primitive(&b, MotorChannel::Locomotion),
            Some(HeadlessActionIds::STEP_FORWARD)
        );
        assert_eq!(
            primitive(&b, MotorChannel::Orientation),
            Some(HeadlessActionIds::LOOK_LEFT)
        );
        if p.cognitive_architecture().schema_version() == 1 {
            assert_eq!(
                a.motor_masks[1], 0,
                "legacy gaze is globally forced, not banned"
            );
        }
    }
}

#[test]
fn hold_gaze_is_an_explicit_choice_and_absent_look_does_not_invent_one() {
    let p = phenotype(true);
    let native = frame(&mut world(false));
    let f = subset(
        &native,
        &[
            HeadlessActionIds::STEP_FORWARD,
            HeadlessActionIds::HOLD_GAZE,
        ],
    );
    let a = sample_cpu_joint_action(&p, &f, &[1000.0, 0.0], 42, 0, 1.0).unwrap();
    assert_eq!(a.motor_candidates[1], Some(1));
    assert_eq!(
        primitive(&bundle(&p, &f, &a), MotorChannel::Orientation),
        Some(HeadlessActionIds::HOLD_GAZE)
    );
    let no_look = subset(&native, &[HeadlessActionIds::STEP_FORWARD]);
    let a = sample_cpu_joint_action(&p, &no_look, &[0.0], 42, 0, 1.0).unwrap();
    assert_eq!(a.motor_masks[1], 0);
    assert_eq!(a.motor_candidates[1], None);
    assert_eq!(bundle(&p, &no_look, &a).channels.len(), 1);
    assert_eq!(a.old_joint_log_probability, 0.0);
}

#[test]
fn independent_gaze_joint_probability_and_ppo_credit_count_each_factor_once() {
    let p = phenotype(true);
    let f = subset(
        &frame(&mut world(false)),
        &[
            HeadlessActionIds::STEP_FORWARD,
            HeadlessActionIds::LOOK_LEFT,
            HeadlessActionIds::LOOK_RIGHT,
        ],
    );
    let logits = [1.0_f32, 0.4, -0.2];
    let config = PpoConfig {
        entropy_coefficient: 0.0,
        ..Default::default()
    };
    let normalizer = logits.iter().map(|x| x.exp()).sum::<f32>();
    let gaze_normalizer = logits[1].exp() + logits[2].exp();
    let mut representatives = [false; 3];
    for counter in 0..64 {
        let a = sample_cpu_joint_action(&p, &f, &logits, 42, counter, 1.0).unwrap();
        let representative = a.representative as usize;
        representatives[representative] = true;
        let look = a.motor_candidates[1].unwrap() as usize;
        let mut expected_logp = logits[representative] - normalizer.ln();
        if representative == 0 {
            expected_logp += logits[look] - gaze_normalizer.ln();
        }
        // Look representatives force Orientation; the sole Move contributes log(1).
        near(a.old_joint_log_probability, expected_logp);
        let objective = cpu_ppo_objective(&logits, &a, 1.0, config).unwrap();
        near(objective.joint_log_probability, expected_logp);
        near(objective.ratio, 1.0);
        for i in 0..3 {
            let mut expected = logits[i].exp() / normalizer - f32::from(i == representative);
            if representative == 0 && i != 0 {
                expected += logits[i].exp() / gaze_normalizer - f32::from(i == look);
            }
            near(objective.logit_gradients[i], expected);
            // Differentiate the actual unclipped PPO loss with the recorded old
            // path probability held fixed. Both factors share these logits.
            let epsilon = 0.002_f32;
            let mut plus = logits;
            let mut minus = logits;
            plus[i] += epsilon;
            minus[i] -= epsilon;
            let hi = cpu_ppo_objective(&plus, &a, 1.0, config).unwrap();
            let lo = cpu_ppo_objective(&minus, &a, 1.0, config).unwrap();
            assert!(!hi.clipped && !lo.clipped);
            let finite_difference = (hi.policy_loss - lo.policy_loss) / (2.0 * epsilon);
            assert!(
                (objective.logit_gradients[i] - finite_difference).abs() < 2e-4,
                "coordinate {i}: analytic {} != finite difference {finite_difference}",
                objective.logit_gradients[i]
            );
        }
        // Conditional Orientation credit is nontrivial even when its addition to
        // the representative softmax gradient nearly cancels. This is credit for
        // the sampled latent representative path, not the marginal bundle, which
        // may also arise with a Look representative. No somatic outcome is faked.
        if representative == 0 {
            let representative_credit = logits[look].exp() / normalizer;
            let conditional_credit = objective.logit_gradients[look] - representative_credit;
            near(
                conditional_credit,
                logits[look].exp() / gaze_normalizer - 1.0,
            );
            assert!(conditional_credit < -0.3);
            let path_probability = expected_logp.exp();
            let marginal_bundle_probability = path_probability + representative_credit;
            assert!(marginal_bundle_probability > path_probability + 0.1);
        }
    }
    assert!(representatives.into_iter().all(|seen| seen));
}

#[test]
fn joint_receipts_roundtrip_and_reject_cross_channel_or_command_tampering() {
    let p = phenotype(true);
    let f = subset(
        &frame(&mut world(false)),
        &[
            HeadlessActionIds::STEP_FORWARD,
            HeadlessActionIds::LOOK_LEFT,
        ],
    );
    let a = sample_cpu_joint_action(&p, &f, &[1000.0, 0.0], 42, 0, 1.0).unwrap();
    let decoded: PpoJointAction = serde_json::from_slice(&serde_json::to_vec(&a).unwrap()).unwrap();
    assert_eq!(decoded, a);
    decoded.validate(1.0).unwrap();
    let mut wrong = a.clone();
    wrong.motor_candidates[1] = Some(0);
    assert!(wrong.validate(1.0).is_err());
    let mut missing = a.clone();
    missing.motor_candidates[1] = None;
    assert!(missing.validate(1.0).is_err());
    let b = bundle(&p, &f, &a);
    let proof = JointActionSelectionV1::new(a.motor_candidates.map(|i| i.map_or(0, |i| i + 1)), &b)
        .unwrap();
    let restored: MotorCommandBundle =
        serde_json::from_slice(&serde_json::to_vec(&b).unwrap()).unwrap();
    proof.validate_bundle(&restored).unwrap();
    let mut changed = restored;
    changed
        .channels
        .iter_mut()
        .find(|c| c.channel == MotorChannel::Orientation)
        .unwrap()
        .primitive = HeadlessActionIds::LOOK_RIGHT;
    assert!(proof.validate_bundle(&changed).is_err());
    assert!(a.validate(0.5).is_err());
}

#[test]
fn sampled_joint_commands_move_and_look_with_native_head_limits_and_hold() {
    let p = phenotype(true);
    let mut w = world(false);
    let entity = w.entity_id("subject").unwrap();
    let body_yaw = w.entity(entity).unwrap().body_yaw;
    let mut saturated = None;
    for step in 0..16 {
        let f = subset(
            &frame(&mut w),
            &[
                HeadlessActionIds::STEP_FORWARD,
                HeadlessActionIds::LOOK_LEFT,
            ],
        );
        let a = sample_cpu_joint_action(&p, &f, &[1000.0, 0.0], 42, step, 1.0).unwrap();
        let before = w.entity(entity).unwrap().position;
        let receipt = w
            .apply_registered_motor_bundle(&bundle(&p, &f, &a), entity)
            .unwrap();
        assert!(receipt.succeeded);
        assert!(w.entity(entity).unwrap().position.x > before.x);
        let body = w.entity(entity).unwrap();
        assert_eq!(
            body.body_yaw, body_yaw,
            "Look is a head swivel, not a body turn"
        );
        assert!(body.head_yaw > 0.0 && body.head_yaw <= 70.0_f32.to_radians());
        assert_eq!(receipt.channel_receipts.len(), 2);
        assert!(receipt
            .channel_receipts
            .iter()
            .any(|r| r.command.channel == MotorChannel::Locomotion
                && r.physical.displacement.x > 0.0));
        assert_eq!(
            receipt.biology_after.tick.raw(),
            receipt.biology_before.tick.raw() + 1
        );
        if step >= 12 {
            if let Some(previous) = saturated {
                assert_eq!(body.head_yaw, previous);
            }
            saturated = Some(body.head_yaw);
        }
        assert_eq!(w.try_advance_tick().unwrap(), receipt.outcome_tick);
    }
    near(saturated.unwrap(), 70.0_f32.to_radians());
    let head = w.entity(entity).unwrap().head_yaw;
    let f = subset(
        &frame(&mut w),
        &[
            HeadlessActionIds::STEP_FORWARD,
            HeadlessActionIds::HOLD_GAZE,
        ],
    );
    let a = sample_cpu_joint_action(&p, &f, &[1000.0, 0.0], 42, 16, 1.0).unwrap();
    let before = w.entity(entity).unwrap().position;
    let receipt = w
        .apply_registered_motor_bundle(&bundle(&p, &f, &a), entity)
        .unwrap();
    assert!(receipt.succeeded);
    assert!(w.entity(entity).unwrap().position.x > before.x);
    assert_eq!(w.entity(entity).unwrap().head_yaw, head);
}

#[test]
fn blocked_locomotion_keeps_sampled_look_and_records_each_physical_channel() {
    let p = phenotype(true);
    let mut w = world(true);
    let entity = w.entity_id("subject").unwrap();
    let f = subset(
        &frame(&mut w),
        &[
            HeadlessActionIds::STEP_FORWARD,
            HeadlessActionIds::LOOK_LEFT,
        ],
    );
    let a = sample_cpu_joint_action(&p, &f, &[1000.0, 0.0], 42, 0, 1.0).unwrap();
    let probability = a.old_joint_log_probability;
    let before = w.entity(entity).unwrap().position;
    let receipt = w
        .apply_registered_motor_bundle(&bundle(&p, &f, &a), entity)
        .unwrap();
    assert!(
        !receipt.succeeded,
        "locomotion must actually encounter the wall"
    );
    assert_eq!(w.entity(entity).unwrap().position, before);
    assert!(w.entity(entity).unwrap().head_yaw > 0.0);
    let moved = receipt
        .channel_receipts
        .iter()
        .find(|r| r.command.channel == MotorChannel::Locomotion)
        .unwrap();
    let looked = receipt
        .channel_receipts
        .iter()
        .find(|r| r.command.channel == MotorChannel::Orientation)
        .unwrap();
    assert_eq!(moved.physical.contact, PhysicalContactKind::Blocked);
    assert_eq!(moved.physical.displacement, Vec3f::ZERO);
    assert_eq!(looked.physical.contact, PhysicalContactKind::None);
    assert!(looked.physical.energy_cost.raw() > 0.0);
    assert_eq!(a.old_joint_log_probability.to_bits(), probability.to_bits());
    a.validate(1.0).unwrap(); // Outcome failure never erases or resamples a factor.
}

#[test]
fn cpu_state_roundtrip_preserves_sentinels_and_rejects_other_layout() {
    let old = phenotype(false);
    let new = phenotype(true);
    assert_ne!(old.phenotype_hash(), new.phenotype_hash());
    let initial = TrainingInitialState {
        activations: vec![0.1; new.neuron_count() as usize],
        activity_ema: vec![0.2; new.neuron_count() as usize],
        metabolic_load: vec![0.03; new.neuron_count() as usize],
        dendrites: DendriticBranchSet::default(),
    };
    let mut checkpoint = CpuOrganismState::new(&new, SUBJECT, initial)
        .unwrap()
        .checkpoint()
        .unwrap();
    // Explicit synthetic nonzero sentinels exercise codec/state compatibility.
    checkpoint.fast[0] = 0.125;
    checkpoint.lifetime[1] = -0.0625;
    checkpoint.eligibility[2] = 0.25;
    checkpoint.last_decision_tick = Some(9);
    checkpoint.last_outcome_tick = Some(10);
    checkpoint.dispatch_generation = 11;
    checkpoint.weight_generation = 12;
    let bytes = serde_json::to_vec(&checkpoint).unwrap();
    let restored =
        CpuOrganismState::from_checkpoint(&new, serde_json::from_slice(&bytes).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_vec(&restored.checkpoint().unwrap()).unwrap(),
        bytes
    );
    assert!(CpuOrganismState::from_checkpoint(&old, checkpoint.clone()).is_err());
    let mut broken = checkpoint;
    broken.eligibility.pop();
    assert!(CpuOrganismState::from_checkpoint(&new, broken).is_err());

    let source = FoundationWeightAsset::from_phenotype_for_genetic_birth(&new).unwrap();
    // The procedural phenotype is intentionally unbound. A foundation trainer
    // requires its ordinary typed asset binding, as in the other CPU tests.
    // This is n2048, not an n512/grounded-encoder compatibility workaround.
    let genome = genome(true);
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let (bound_new, _) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, source.clone())
            .unwrap();
    assert_eq!(bound_new.sensor_encoder(), new.sensor_encoder());
    assert_eq!(bound_new.synapses(), new.synapses());
    let trainer = CpuFoundationTrainer::new(
        bound_new.clone(),
        source.clone(),
        StageTrainableMask::recurrent_only(&bound_new).unwrap(),
        AdamWConfig::default(),
    )
    .unwrap();
    let mut founder = trainer.checkpoint();
    founder.optimizer_step = 17;
    founder.first_moment[0] = 0.125;
    founder.second_moment[0] = 0.25;
    founder.update_ages[0] = 17;
    let bytes = serde_json::to_vec(&founder).unwrap();
    let restored = CpuFoundationTrainer::from_checkpoint(
        bound_new,
        source.clone(),
        serde_json::from_slice(&bytes).unwrap(),
    )
    .unwrap();
    assert_eq!(serde_json::to_vec(&restored.checkpoint()).unwrap(), bytes);
    assert!(founder.validate_for(&old, source.digest()).is_err());
}
