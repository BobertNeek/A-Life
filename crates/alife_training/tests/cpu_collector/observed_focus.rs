//! Native CPU capability evidence on fresh public fixtures, never a learned
//! competence or physical-GPU qualification. No optimizer step is performed.
use super::*;
use alife_runtime::canonical_preparation::*;

#[test]
fn native_selection_can_reach_an_observed_far_focus_without_rewriting_the_brain() {
    let (game, trainer, inputs) =
        new_game_with_profile_fixture(None, SensorProfile::GroundedTerrainVisionV1);
    let p = trainer.phenotype();
    let founder_before = trainer.checkpoint();
    let record = game.world.organism_registry().iter().next().unwrap();
    let own = record.organism_id();
    let tick = game.world.tick();
    let mut world = HeadlessScenarioBuilder::new(60_045)
        .agent("observer", own, Vec3f::ZERO)
        .build()
        .unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::new(
                own,
                world.entity_id("observer").unwrap(),
                record.genome().clone(),
                record.phenotype().clone(),
                *record.biochemistry(),
                record.birth_tick(),
            )
            .unwrap(),
        )
        .unwrap();
    let mut far = None;
    for i in 0..16 {
        let id = world
            .spawn_toy(
                &format!("object-{i}"),
                Vec3f::new(1.0 + i as f32 * 0.3, 0.0, 0.2),
                true,
            )
            .unwrap();
        world.editor_set_optical_opacity(id, 0.0).unwrap();
        far = Some(id);
    }
    let far = far.unwrap();
    let mut moving = world.entity(far).unwrap().grounded_physical;
    moving.velocity = Vec3f::new(4.0, 0.0, 0.0);
    world.set_grounded_physical_properties(far, moving).unwrap();
    let (draft, binding) = world
        .perception_frame_draft_with_bindings(
            own,
            tick,
            p.sensor_profile(),
            record.biochemistry().homeostasis,
        )
        .unwrap();
    assert_eq!(draft.grounded_object_slots().len(), 16);
    let focal =
        StableFocusIdentity::TrackedObject(draft.grounded_object_slots()[15].tracked_object_id);
    let profile = draft.profile_provenance().identity();
    let memory = MemorySidecarState::new_profiled(
        own,
        profile,
        MemoryBankConfig::new(256, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    let topology =
        TopologySidecar::new_profiled(own, profile, TopologicalMapConfig::default()).unwrap();
    let predictor = predictor_for_phenotype(p).unwrap();
    let receptors = record
        .biochemistry()
        .neural_receptor_frame(record.phenotype())
        .unwrap();
    let effects = NeuralReceptorEffects::from_frame(
        &receptors,
        &NeuralReceptorPhenotype::compile(p).unwrap(),
    )
    .unwrap();
    let mut input = CanonicalPreparationInput {
        draft,
        candidate_binding: None,
        sequence_id: ExperienceSequenceId(1),
        homeostasis: record.biochemistry().homeostasis,
        hysteresis: HysteresisState::default(),
        policy: attention_selection_policy_for(p),
        neural_receptors: receptors,
        receptor_effects: effects,
    };
    let before = prepare_canonical_frame(&input, &memory, &topology, &predictor, false)
        .result
        .unwrap();
    input.candidate_binding = binding;
    let after = prepare_canonical_frame(&input, &memory, &topology, &predictor, false)
        .result
        .unwrap();
    for prepared in [&before, &after] {
        assert_eq!(
            prepared
                .memory_recall
                .cognitive_context()
                .unwrap()
                .attention
                .focal_targets[0],
            focal
        );
    }
    assert!(!before
        .frame
        .candidates()
        .iter()
        .any(|c| c.target.entity == Some(far)));
    assert_eq!(
        after
            .frame
            .candidates()
            .iter()
            .filter(|c| c.target.entity == Some(far))
            .count(),
        6
    );

    // Exercise the real collector capture boundary as well as the shared helper.
    let mut host_game = game.clone();
    host_game.world = world.clone();
    let mut host = CpuCanonicalHost::new_cohort(host_game, &trainer, 42).unwrap();
    let row = host.step(&trainer, 1.0).unwrap().decisions.remove(0);
    let frame = row.patch.pre_action().perception();
    assert_eq!(
        frame
            .candidates()
            .iter()
            .filter(|c| c.target.entity == Some(far))
            .count(),
        6
    );
    assert_eq!(
        row.patch
            .decision()
            .neural_evidence()
            .unwrap()
            .phenotype_hash,
        p.phenotype_hash()
    );

    let evaluate = |prepared: &CanonicalPreparedFrame| {
        let context = encode_cpu_frame(
            p,
            &prepared.frame,
            &prepared.memory_recall,
            &effects,
            vec![true; p.projections().len()],
            Vec::new(),
        )
        .unwrap();
        let n = p.neuron_count() as usize;
        let initial = alife_training::TrainingInitialState {
            activations: vec![0.0; n],
            activity_ema: vec![0.0; n],
            metabolic_load: vec![0.0; n],
            dendrites: DendriticBranchSet::default(),
        };
        let mut neural = CpuOrganismState::new(p, own, initial).unwrap();
        neural
            .advance(&trainer, context, &effects)
            .unwrap()
            .candidate_logits
    };
    let before_logits = evaluate(&before);
    let after_logits = evaluate(&after);
    let mut witnessed = None;
    for counter in 0..256 {
        let a =
            sample_cpu_joint_action(p, &before.frame, &before_logits, 42, counter, 1.0).unwrap();
        let b = sample_cpu_joint_action(p, &after.frame, &after_logits, 42, counter, 1.0).unwrap();
        assert!(a
            .motor_candidates
            .iter()
            .flatten()
            .all(|i| before.frame.candidates()[*i as usize].target.entity != Some(far)));
        if let Some(i) = b.motor_candidates[0] {
            let c = &after.frame.candidates()[i as usize];
            if c.action_id == HeadlessActionIds::APPROACH && c.target.entity == Some(far) {
                let body = world.entity_id("observer").unwrap();
                let start = world.entity(body).unwrap().position;
                let mut before_world = world.clone();
                let before_displacement = a.motor_candidates[0].map_or(0.0, |index| {
                    let c = &before.frame.candidates()[index as usize];
                    before_world
                        .apply_command(&c.to_command(own, Confidence::new(1.0).unwrap()).unwrap())
                        .unwrap();
                    let end = before_world.entity(body).unwrap().position;
                    (end.x - start.x).hypot(end.z - start.z)
                });
                let outcome = world
                    .apply_command(&c.to_command(own, Confidence::new(1.0).unwrap()).unwrap())
                    .unwrap();
                assert!(outcome.execution.succeeded);
                let end = world.entity(body).unwrap().position;
                let displacement = (end.x - start.x).hypot(end.z - start.z);
                assert!(displacement > 0.0);
                witnessed = Some((counter, before_displacement, displacement));
                break;
            }
        }
    }
    let witnessed =
        witnessed.expect("bounded native sampling must demonstrate newly available approach");
    eprintln!(
        "CPU fixture seed=42 counter={} original_displacement={} focal_displacement={}",
        witnessed.0, witnessed.1, witnessed.2
    );
    assert_eq!(
        serde_json::to_vec(&trainer.checkpoint()).unwrap(),
        serde_json::to_vec(&founder_before).unwrap()
    );
    // Same accepted compiler inputs/topology remain admissible; no migration.
    assert_eq!(
        PhenotypeCompiler::compile_validated(&inputs, &BrainCapacityClass::n2048()).unwrap(),
        *p
    );
}
