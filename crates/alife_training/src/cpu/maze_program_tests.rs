//! Manual readout calibration preserves acquired state; synthetic tests, not efficacy.
use super::*;
use alife_core::*;
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
        builder.food("near", Vec3f::new(2.5, 0., 0.), 0.4)
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
    let mask = alife_training::StageTrainableMask::recurrent_only(&p).unwrap();
    let trainer =
        CpuFoundationTrainer::new(p, asset, mask, alife_training::AdamWConfig::default()).unwrap();
    (
        CpuCanonicalHost::new_cohort(game, &trainer, 2306).unwrap(),
        trainer,
    )
}
fn cpbytes(h: &CpuCanonicalHost, t: &CpuFoundationTrainer) -> Vec<u8> {
    serde_json::to_vec(&h.checkpoint(t).unwrap()).unwrap()
}
fn interface() -> (CpuCanonicalHost, CpuFoundationTrainer) {
    let (mut h, t) = fixture(SensorProfile::GroundedObjectSlotsV1);
    let mut edits = vec![(26144u32, f32::from_bits(0x80000000)), (26146, 0.)];
    edits.extend((30048..30088).map(|i| (i, 0.4)));
    // Synthetic fixture uses only declared public g0/Inspect setup. No private
    // selected-program artifact is a compile-time test dependency.
    let calibrated = t.calibrated_readout_copy(&edits).unwrap();
    h.rebase_readout_policy(&t, &calibrated).unwrap();
    let t = calibrated;

    h.enable_joint_ingest_direct2295(&t, true).unwrap();
    h.enable_candidate_opportunity_coverage2313(&t).unwrap();
    let h = h
        .fork_with_approach_attention_feedback_for_research(&t)
        .unwrap()
        .fork_with_active_approach_opportunity_for_research(&t)
        .unwrap();
    (h, t)
}
#[test]
fn maze_attach_and_cold_preserve_all_prior_state() {
    let (old, t) = interface();
    let before = old.checkpoint(&t).unwrap();
    let original = cpbytes(&old, &t);
    let mut h = old.fork_with_maze_program_for_research2346(&t).unwrap();
    let cp = h.checkpoint(&t).unwrap();
    assert_eq!(cp.schema_version, 19);
    assert_eq!(cp.maze_program2346, Some(1));
    let mut n = cp.clone();
    n.schema_version = 18;
    n.maze_program2346 = None;
    assert_eq!(serde_json::to_vec(&n).unwrap(), original);
    assert!(h.fork_with_maze_program_for_research2346(&t).is_err());
    let mut cold = CpuCanonicalHost::from_checkpoint(cp, &t).unwrap();
    for _ in 0..4 {
        let a = h.step(&t, 1.).unwrap();
        let b = cold.step(&t, 1.).unwrap();
        assert_eq!(
            serde_json::to_vec(&a.decisions).unwrap(),
            serde_json::to_vec(&b.decisions).unwrap()
        );
        assert_eq!(cpbytes(&h, &t), cpbytes(&cold, &t));
        assert!(a
            .decisions
            .iter()
            .all(|r| r.patch.pre_action().perception().maze_heading2346()));
    }
    assert_eq!(original, cpbytes(&old, &t));
    let mut bad = before;
    bad.schema_version = 19;
    assert!(CpuCanonicalHost::from_checkpoint(bad, &t).is_err());
}
#[test]
fn maze_calibration_is_exact_finite_bounded_and_explicit() {
    let (old, t) = interface();
    for e in [
        vec![],
        vec![(698, f32::NAN)],
        vec![(698, 17.)],
        vec![(0, 1.)],
        vec![(698, 1.), (698, 2.)],
        vec![(32768, 1.)],
    ] {
        assert!(t.calibrated_maze_program_copy2346(&e).is_err());
    }
    let before = t.checkpoint();
    let next = t
        .calibrated_maze_program_copy2346(&[(698, 2.), (14288, 0.75), (15629, 0.75)])
        .unwrap();
    let mut expected = before.clone();
    expected.weights[698] = 2.;
    expected.weights[14288] = 0.75;
    expected.weights[15629] = 0.75;
    assert_eq!(
        serde_json::to_vec(&expected).unwrap(),
        serde_json::to_vec(&next.checkpoint()).unwrap()
    );
    let mut rejected = old.clone();
    let original = cpbytes(&old, &t);
    assert!(rejected.rebase_maze_program_policy2346(&t, &next).is_err());
    assert_eq!(original, cpbytes(&rejected, &t));
    let mut h = old.fork_with_maze_program_for_research2346(&t).unwrap();
    let pre = h.checkpoint(&t).unwrap();
    h.rebase_maze_program_policy2346(&t, &next).unwrap();
    let after = h.checkpoint(&next).unwrap();
    let mut normalized = after.clone();
    normalized.founder_weight_digest = pre.founder_weight_digest;
    assert_eq!(
        serde_json::to_vec(&pre).unwrap(),
        serde_json::to_vec(&normalized).unwrap()
    );
    let mut cold = CpuCanonicalHost::from_checkpoint(after, &next).unwrap();
    let a = h.step(&next, 1.).unwrap();
    let b = cold.step(&next, 1.).unwrap();
    assert_eq!(
        serde_json::to_vec(&a.decisions).unwrap(),
        serde_json::to_vec(&b.decisions).unwrap()
    );
    assert_eq!(cpbytes(&h, &next), cpbytes(&cold, &next));
    assert_eq!(
        serde_json::to_vec(&expected).unwrap(),
        serde_json::to_vec(&next.checkpoint()).unwrap()
    );
}

#[test]
fn maze_bridge_graph_semantics_are_native_not_label_ranges() {
    let (_, t) = interface();
    let p = t.phenotype();
    for &(i, source, target) in &MAZE_RECURRENT_EDGES2346 {
        let e = &p.synapses()[i];
        assert_eq!((e.source(), e.target()), (source, target));
        assert!(matches!(e.kind(), CompiledSynapseKind::Recurrent));
    }
    for (i, family) in [
        (10366, CandidateActionFamily::Approach),
        (10423, CandidateActionFamily::Approach),
        (10362, CandidateActionFamily::Inspect),
        (10419, CandidateActionFamily::Inspect),
    ] {
        let target = p.synapses()[i].target();
        let families = p
            .synapses()
            .iter()
            .filter_map(|s| match s.kind() {
                CompiledSynapseKind::Decoder(c)
                    if s.source() == target && c.head() == DecoderHeadKind::ActionCandidate =>
                {
                    Some(c.family())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(!families.is_empty());
        assert!(families.iter().all(|&f| f == family));
    }
    assert!(p
        .sensor_encoder()
        .assignments()
        .iter()
        .any(
            |a| a.source_group() == SensorEncoderSourceGroup::SensoryChannel
                && a.source_index() == 39
                && a.target_neuron() == 49
        ));
}
#[test]
fn actual_step_collision_then_sleep_cold_and_once_awake_sensor() {
    let (old, t) = interface();
    let mut h = old.fork_with_maze_program_for_research2346(&t).unwrap();
    let own = h
        .world()
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    let near = h.world().entity_id("near").unwrap();
    h.world_mut()
        .editor_move_object(near, Vec3f::new(-100., 0., 0.))
        .unwrap();
    h.world_mut()
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "synthetic-contact-wall".into(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position: Vec3f::new(0.6, 0., 0.),
            nutrition: 0.,
            hazard_pain: 0.,
            radius: 0.35,
            token_id: None,
        })
        .unwrap();
    let edits = t
        .phenotype()
        .synapses()
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match s.kind() {
            CompiledSynapseKind::Decoder(c)
                if c.head() == DecoderHeadKind::ActionCandidate
                    && c.family() == CandidateActionFamily::Approach =>
            {
                Some((
                    i as u32,
                    if matches!(c.input_lane(), 1 | 23) {
                        16.
                    } else {
                        0.
                    },
                ))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let next = t.calibrated_maze_program_copy2346(&edits).unwrap();
    h.rebase_maze_program_policy2346(&t, &next).unwrap();
    let original_adam = serde_json::to_vec(&next.checkpoint()).unwrap();
    let mut direct = h.world().clone();
    let cmd = ActionCommand::structured(
        own,
        ActionId(105),
        ActionKind::Move,
        ActionTarget::NONE,
        Intensity::new(1.).unwrap(),
        DurationTicks::new(1),
        Confidence::new(1.).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap();
    let direct_result = direct.apply_command(&cmd).unwrap();
    println!(
        "DIRECT_STEP {}",
        serde_json::to_string(&direct_result.execution).unwrap()
    );
    let first = h.step(&next, 1.).unwrap();
    let row = &first.decisions[0];
    let joint = row.patch.outcome().joint.as_ref().unwrap();
    println!(
        "SAMPLED {:?} JOINT {}",
        row.action,
        serde_json::to_string(joint).unwrap()
    );
    assert!(joint
        .channel_outcomes
        .iter()
        .any(|c| c.channel == MotorChannel::Locomotion
            && c.physical.contact == PhysicalContactKind::Blocked
            && c.execution_evidence
                .is_some_and(|e| e.primitive.raw() == 105)));
    let work = row.patch.cognitive_work().unwrap();
    let rec = h.world().organism_registry().get(own).unwrap();
    assert_eq!(rec.cognitive_work(), work);
    let raw = alife_core::cognitive_work::CognitiveWorkCostPolicy::enabled(1.0e-6)
        .unwrap()
        .energy_debit(work)
        .unwrap();
    let ph = rec.phenotype();
    let expected = raw
        * ph.body.metabolic_turnover
        * (ContinuousLocus::midpoint_value(0.52, 0.60) - ph.chemistry.brain_atp_efficiency).exp2();
    assert_eq!(expected.to_bits(), rec.cognitive_energy_debit().to_bits());
    let f = row.patch.pre_action().perception();
    let extra = row.active_approach_opportunity2327.as_ref().unwrap();
    let base = row
        .patch
        .pre_action()
        .cognitive_context
        .as_ref()
        .unwrap()
        .attention
        .budget_receipt
        .work_units
        + extra.intent_work_units
        + extra.selection.work_units
        + row
            .approach_attention_feedback2325
            .as_ref()
            .unwrap()
            .work_units
        + f.grip_proprioception().map_or(0, |s| s.work_units())
        + f.candidate_opportunity_coverage()
            .map_or(0, |s| s.work_units());
    assert_eq!(work.focal_target_ops, base + 331);
    let history = h.checkpoint(&next).unwrap().residents[0]
        .maze_blocked2346
        .clone()
        .unwrap();
    assert!(history.blocked);
    assert_eq!(history.primitive.raw(), 105);
    h.request_recovery_sleep(own).unwrap();
    let cp = h.checkpoint(&next).unwrap();
    let mut cold = CpuCanonicalHost::from_checkpoint(cp, &next).unwrap();
    let mut sleeping = 0;
    let mut delivered = false;
    for _ in 0..512 {
        let a = h.step(&next, 1.).unwrap();
        let b = cold.step(&next, 1.).unwrap();
        assert_eq!(
            serde_json::to_vec(&a.decisions).unwrap(),
            serde_json::to_vec(&b.decisions).unwrap()
        );
        assert_eq!(cpbytes(&h, &next), cpbytes(&cold, &next));
        if a.decisions.is_empty() {
            sleeping += 1;
            assert_eq!(
                h.checkpoint(&next).unwrap().residents[0].maze_blocked2346,
                Some(history.clone())
            );
            continue;
        }
        let f = a.decisions[0].patch.pre_action().perception();
        let receipt = f.maze_blocked_sense2346().unwrap();
        assert_eq!(receipt.source_sequence, history.source_sequence);
        assert!(receipt.blocked);
        assert!(f.sensory().channels.tactile_contact[7] > 0.);
        let second = h.step(&next, 1.).unwrap();
        if let Some(row) = second.decisions.first() {
            assert_ne!(
                row.patch
                    .pre_action()
                    .perception()
                    .maze_blocked_sense2346()
                    .unwrap()
                    .source_sequence,
                history.source_sequence
            );
        }
        delivered = true;
        break;
    }
    assert!(sleeping > 0 && delivered);
    assert_eq!(
        original_adam,
        serde_json::to_vec(&next.checkpoint()).unwrap()
    );
}
