//! Controlled physical observations, not learned/autonomous policy fixtures.
use super::*;
use alife_core::*;

struct ControlledObservation {
    patch: ExperiencePatch,
    frame: PerceptionFrame,
    candidate: ActionCandidate,
    next_frame: PerceptionFrame,
    next_candidate: ActionCandidate,
    dose: f32,
}

fn measured_fixture(kind: u8) -> ControlledObservation {
    let own = OrganismId(1);
    let mut world = HeadlessScenarioBuilder::new(921_411)
        .agent("subject", own, Vec3f::ZERO)
        .build()
        .unwrap();
    world.tick = Tick(600);
    let target = if kind == 1 {
        world
            .spawn_toy("observed", Vec3f::new(0.2, 0., 0.), false)
            .unwrap()
    } else {
        world
            .spawn_social_agent(
                "observed",
                OrganismId(2),
                Vec3f::new(if kind == 2 { 2. } else { 0.2 }, 0., 0.),
                0.,
            )
            .unwrap()
    };
    world
        .set_grounded_physical_properties(
            target,
            GroundedPhysicalProperties::deterministic_for_kind(WorldObjectKind::ActivityToy, 2),
        )
        .unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        921_411,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let biology = BiochemistryState::new_with_age(&phenotype, Tick(600), Tick(600)).unwrap();
    let development = phenotype.development_state_at(Tick(600)).unwrap();
    let brain = PhenotypeCompiler::compile_testing_procedural_baseline(
        &phenotype.brain_genome,
        &BrainCapacityClass::n512(),
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let actor = world.entity_id("subject").unwrap();
    world
        .register_organism_record(
            crate::WorldOrganismRecord::new(
                own,
                actor,
                genome,
                phenotype.clone(),
                biology,
                Tick::ZERO,
            )
            .unwrap(),
        )
        .unwrap();
    let frame = world
        .perception_frame(
            own,
            Tick(600),
            SensorProfile::GroundedObjectSlotsV1,
            biology.homeostasis,
        )
        .unwrap();
    let candidate = *frame
        .candidates()
        .iter()
        .find(|c| c.kind == ActionKind::Inspect && c.target.entity == Some(target))
        .expect("observed target inspection candidate");
    let confidence = Confidence::new(0.9).unwrap();
    let command = candidate.to_command(own, confidence).unwrap();
    let seq = ExperienceSequenceId(1);
    // Explicit prescribed selection evidence for the unit fixture: no inference or policy run.
    let pre = PreActionSnapshot::from_cpu_neural_frame(
        seq,
        brain.brain_class_id(),
        brain.phenotype_hash(),
        phenotype.brain_genome.id,
        phenotype.brain_genome.schema_version,
        development,
        frame.clone(),
    )
    .unwrap();
    let decision = DecisionSnapshot::from_cpu_neural_selection(
        seq,
        brain.phenotype_hash(),
        1,
        0,
        &frame,
        NeuralActionSelection {
            candidate_index: candidate.candidate_index,
            logit: 0.,
            confidence,
            active_tiles: 0,
            active_synapses: 0,
        },
        command,
    )
    .unwrap();
    let mut passive = world.clone();
    passive.try_advance_tick().unwrap();
    let passive_after = *passive.organism_registry().get(own).unwrap().biochemistry();
    let receipt = world
        .apply_registered_command(&command, actor, Tick(601))
        .unwrap();
    assert_eq!(receipt.biology_before.tick, Tick(600));
    assert_eq!(receipt.biology_after.tick, Tick(601));
    assert_eq!(receipt.biology_before, biology);
    assert_eq!(
        receipt
            .biology_after
            .homeostasis
            .drives
            .loneliness
            .to_bits(),
        passive_after.homeostasis.drives.loneliness.to_bits()
    );
    world.try_advance_tick().unwrap();
    assert_eq!(world.tick(), Tick(601));
    assert_eq!(
        *world.organism_registry().get(own).unwrap().biochemistry(),
        receipt.biology_after,
        "finalizing registered interval must not advance biology twice"
    );
    println!("initial biological context: case={kind} age={} maturation={} puberty={} puberty_tick={} expression={} loneliness={} energy={} health={} injury={} sleeping={} ticks_per_second={} metabolism_cadence={} development_cadence={} before_tick={} after_tick={} passive_loneliness_delta={}",
        biology.development.age_ticks.raw(), biology.development.maturation, biology.development.puberty_reached,
        phenotype.development.puberty_tick.raw(), biology.development.biochemical_expression,
        biology.homeostasis.drives.loneliness, biology.body.energy, biology.body.health, biology.body.injury,
        biology.body.sleeping, WORLD_TICKS_PER_SECOND, biology.cadence.metabolism_ticks, biology.cadence.development_ticks,
        receipt.biology_before.tick.raw(), receipt.biology_after.tick.raw(),
        passive_after.homeostasis.drives.loneliness - biology.homeostasis.drives.loneliness);
    let transition =
        MeasuredPhysiologyTransition::new(receipt.biology_before, receipt.biology_after).unwrap();
    let outcome = PostActionOutcome::new(
        own,
        seq,
        Tick(601),
        receipt.action_result.execution.succeeded,
        receipt.action_result.execution.physical,
        HomeostaticDelta::zero(),
        SignedValence::ZERO,
        NormalizedScalar::ZERO,
        NormalizedScalar::ZERO,
        SignedValence::ZERO,
        NormalizedScalar::ZERO,
    )
    .unwrap()
    .with_measured_physiology(transition)
    .unwrap();
    let patch = ExperiencePatchBuilder::new(seq)
        .record_pre_action(pre)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap();
    let next_frame = world
        .perception_frame(
            own,
            Tick(601),
            SensorProfile::GroundedObjectSlotsV1,
            receipt.biology_after.homeostasis,
        )
        .unwrap();
    let next_candidate = *next_frame
        .candidates()
        .iter()
        .find(|c| c.kind == ActionKind::Inspect && c.target.entity == Some(target))
        .unwrap();
    ControlledObservation {
        patch,
        frame,
        candidate,
        next_frame,
        next_candidate,
        dose: receipt.action_result.body_event.social_contact,
    }
}

#[test]
fn standalone_need_evidence_records_real_contact_not_a_target_label() {
    let mut deltas = Vec::new();
    let mut doses = Vec::new();
    for kind in 0..3 {
        let ControlledObservation {
            patch,
            frame,
            candidate,
            next_frame,
            next_candidate,
            dose,
        } = measured_fixture(kind);
        let measured = patch.outcome().measured_physiology.as_ref().unwrap();
        let query = NeedConsequenceQueryV1::from_frame(&frame, &candidate).unwrap();
        let mut store = NeedConsequenceStoreV1::new(OrganismId(1), 2).unwrap();
        assert!(matches!(
            store.recall(&query, Tick(601), 10, 2).unwrap().evidence,
            NeedConsequenceEvidenceV1::Unknown
        ));
        assert!(matches!(
            store.observe(&patch).unwrap(),
            NeedConsequenceAdmissionV1::Measured { pruned_records: 0 }
        ));
        let recalled = store.recall(&query, Tick(601), 10, 2).unwrap();
        let NeedConsequenceEvidenceV1::Measured(observations) = &recalled.evidence else {
            panic!("actual measured interval must remain known, including zero")
        };
        assert_eq!(observations.len(), 1);
        let observation = &observations[0];
        let expected = measured.homeostatic_delta.drives.to_array();
        assert_eq!(
            observation.drive_deltas.map(f32::to_bits),
            expected.map(f32::to_bits)
        );
        assert_eq!(
            observation.attribution,
            NeedConsequenceAttributionV1::JointIntervalAssociation
        );
        assert_eq!(observation.joint_command_count, 1);
        assert_eq!(observation.source_sequence_id, patch.header().sequence_id);
        assert!(
            store.observe(&patch).is_err(),
            "no duplicate acquired source count"
        );
        assert_eq!(store.len(), 1);
        let other = frame
            .candidates()
            .iter()
            .find(|c| c.family != candidate.family)
            .unwrap();
        let other_query = NeedConsequenceQueryV1::from_frame(&frame, other).unwrap();
        assert!(matches!(
            store
                .recall(&other_query, Tick(601), 10, 2)
                .unwrap()
                .evidence,
            NeedConsequenceEvidenceV1::Unknown
        ));
        // Artificial query-only pressure substitution is not a new observation.
        let mut h = *frame.homeostasis();
        h.drives.loneliness = (h.drives.loneliness + 0.01).min(1.);
        let changed = PerceptionFrameDraft::new(
            frame.organism_id(),
            frame.tick(),
            frame.sensor_profile(),
            frame.sensory().clone(),
            frame.body(),
            h,
            frame.candidates().to_vec(),
            frame.profile_provenance(),
            frame.grounded_object_slots().to_vec(),
        )
        .unwrap()
        .finalize(frame.context().clone())
        .unwrap();
        let changed_query = NeedConsequenceQueryV1::from_frame(&changed, &candidate).unwrap();
        assert!(matches!(
            store
                .recall(&changed_query, Tick(601), 10, 2)
                .unwrap()
                .evidence,
            NeedConsequenceEvidenceV1::Unknown
        ));
        // Actual next-world query, without resetting need, hormones, pose or age.
        // This exposes the deliberately strict contract's practical recall limit.
        let actual_next_query =
            NeedConsequenceQueryV1::from_frame(&next_frame, &next_candidate).unwrap();
        let actual_next = store.recall(&actual_next_query, Tick(601), 10, 2).unwrap();
        assert!(matches!(
            actual_next.evidence,
            NeedConsequenceEvidenceV1::Unknown
        ));
        println!("actual next-world recall: case={kind} unknown=true same_target={} same_family={} before_need={} next_need={}",
            query.tracked_target() == actual_next_query.tracked_target(),
            query.action_family() == actual_next_query.action_family(),
            frame.homeostasis().drives.loneliness, next_frame.homeostasis().drives.loneliness);
        let encoded = store.to_json().unwrap();
        let restored = NeedConsequenceStoreV1::from_json(&encoded).unwrap();
        assert_eq!(restored.recall(&query, Tick(601), 10, 2).unwrap(), recalled);
        assert_eq!(restored.to_json().unwrap(), encoded);
        println!("controlled need evidence: case={kind} loneliness_delta={} dose={dose} source_bytes={:?} store_bytes={}",
            observation.drive_deltas[4], store.source_encoded_bytes(), encoded.len());
        deltas.push(observation.drive_deltas[4]);
        doses.push(dose);
    }
    assert!(doses[0] > 0.);
    assert_eq!(doses[1], 0.);
    assert_eq!(doses[2], 0.);
    assert!(deltas[0] < deltas[1]);
    assert_eq!(deltas[1].to_bits(), deltas[2].to_bits());
}
