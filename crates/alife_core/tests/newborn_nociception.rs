use alife_core::*;

fn founder(seed: u64) -> CreatureGenome {
    CreatureGenome::early_mammal_founder(
        seed,
        FoundationGeneticIdentity::new(
            FoundationId::N512_V1.raw(),
            1,
            FoundationCompatibilityFamilyId::N512_FOUNDATION.raw(),
            BrainCapacityClass::N512_ID,
        )
        .unwrap(),
    )
    .unwrap()
}

// Exercise the persisted gene boundary. Before the implementation, the unknown
// floor is ignored and the actual newborn damage response remains zero (RED).
fn with_pain_floor(genome: &CreatureGenome, floor: f32) -> CreatureGenome {
    let graph = genome.chemistry.graph.expressed();
    let pain = graph
        .receptors()
        .iter()
        .find(|r| r.target == BiochemicalTargetLocus::Drive(BiochemicalDriveChannel::Pain))
        .unwrap()
        .source;
    let index = graph
        .emitters()
        .iter()
        .position(|e| e.source == BiochemicalSourceLocus::Damage && e.target == pain)
        .unwrap();
    let mut wire = serde_json::to_value(genome).unwrap();
    for side in ["maternal", "paternal"] {
        wire["chemistry"]["graph"][side]["emitters"][index]["developmental_expression_floor"] =
            serde_json::json!(floor);
    }
    let result: CreatureGenome = serde_json::from_value(wire).unwrap();
    result.validate_contract().unwrap();
    result
}

#[test]
fn inherited_nociception_responds_to_real_damage_without_maturing_other_chemistry() {
    let legacy = founder(96001);
    let old_bytes = serde_json::to_vec(&legacy).unwrap();
    println!("legacy_genome_blake3={}", blake3::hash(&old_bytes).to_hex());
    assert_eq!(
        blake3::hash(&old_bytes).to_hex().as_str(),
        "30200cc049a64ad5eebf6ca9809a4cd016bb11028cab3bb69088786f4c90c23c",
        "legacy canonical genome bytes must retain the pre-change RED receipt"
    );
    assert!(!String::from_utf8(old_bytes.clone())
        .unwrap()
        .contains("developmental_expression_floor"));
    assert_eq!(
        serde_json::to_vec(&with_pain_floor(&legacy, 0.0)).unwrap(),
        old_bytes
    );
    let genome = with_pain_floor(&legacy, 1.0);
    let phenotype = genome.express().unwrap();
    let newborn = BiochemistryState::new_with_age(&phenotype, Tick::ZERO, Tick::ZERO).unwrap();
    let damage = BodyEventDelta {
        damage: 0.15,
        ..BodyEventDelta::zero()
    };
    let hurt = newborn
        .advance_with_age(Tick(1), Tick(1), damage, &phenotype)
        .unwrap();
    assert!(hurt.body.injury > newborn.body.injury);
    assert!(
        hurt.homeostasis.drives.pain > 0.0,
        "actual newborn damage must reach the inherited pain drive; observed {}",
        hurt.homeostasis.drives.pain
    );
    assert!((hurt.homeostasis.drives.pain - 0.12).abs() < 1e-6);
    assert_eq!(hurt.development.biochemical_expression, 0.0);
    let emission = NeuralEmissionFrame::new(
        Tick::ZERO,
        1,
        vec![NeuralEmission::new(NeuralEmissionClass::PredictionResidual, 0.9, 0.8).unwrap()],
    )
    .unwrap();
    let stimulated = newborn
        .advance_with_neural_emission(
            Tick(1),
            Tick(1),
            BodyEventDelta {
                nutrition: 1.0,
                ..damage
            },
            Some(&emission),
            &phenotype,
        )
        .unwrap();
    assert_eq!(
        stimulated.graph_state(),
        hurt.graph_state(),
        "nociception must not activate developmental nutrition or neural emitters"
    );
    let quiet = newborn
        .advance_with_age(Tick(1), Tick(1), BodyEventDelta::zero(), &phenotype)
        .unwrap();
    assert_eq!(quiet.homeostasis.drives.pain, 0.0);
    let old_phenotype = legacy.express().unwrap();
    let old_newborn =
        BiochemistryState::new_with_age(&old_phenotype, Tick::ZERO, Tick::ZERO).unwrap();
    let old_hurt = old_newborn
        .advance_with_age(Tick(1), Tick(1), damage, &old_phenotype)
        .unwrap();
    assert_eq!(old_hurt.homeostasis.drives.pain, 0.0);
    let graph = &phenotype.chemistry.biochemical;
    let pain = graph
        .receptors()
        .iter()
        .find(|r| r.target == BiochemicalTargetLocus::Drive(BiochemicalDriveChannel::Pain))
        .unwrap()
        .source;
    for species in graph.species().iter().filter(|s| s.id != pain) {
        assert_eq!(
            hurt.graph_state().concentration(graph, species.id).unwrap(),
            old_hurt
                .graph_state()
                .concentration(&old_phenotype.chemistry.biochemical, species.id)
                .unwrap()
        );
    }
    let dna = serde_json::to_vec(&genome).unwrap();
    let restored: CreatureGenome = serde_json::from_slice(&dna).unwrap();
    assert_eq!(restored, genome);
    assert_eq!(serde_json::to_vec(&genome).unwrap(), dna);
    let other = with_pain_floor(&founder(96002), 1.0);
    let child = CreatureGenome::reproduce(&genome, &other, 96003).unwrap();
    let child_phenotype = child.express().unwrap();
    let child_state =
        BiochemistryState::new_with_age(&child_phenotype, Tick::ZERO, Tick::ZERO).unwrap();
    assert_eq!(
        child_state.homeostasis.drives.pain, 0.0,
        "birth must not inherit parent's acquired pain concentration"
    );
    let child_hurt = child_state
        .advance_with_age(Tick(1), Tick(1), damage, &child_phenotype)
        .unwrap();
    assert!((child_hurt.homeostasis.drives.pain - 0.12).abs() < 1e-6);
    let index = graph
        .emitters()
        .iter()
        .position(|e| e.source == BiochemicalSourceLocus::Damage && e.target == pain)
        .unwrap();
    let mut typed = legacy.clone();
    for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
        typed.chemistry.graph = typed
            .chemistry
            .graph
            .with_emitter_expression_floor(side, index, 1.0)
            .unwrap();
    }
    assert_eq!(
        typed, genome,
        "typed and persisted genes must express identically"
    );
    for floor in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1, 1.1] {
        assert!(legacy
            .chemistry
            .graph
            .clone()
            .with_emitter_expression_floor(AlleleSide::Maternal, index, floor)
            .is_err());
    }
    assert!(legacy
        .chemistry
        .graph
        .clone()
        .with_emitter_expression_floor(AlleleSide::Maternal, usize::MAX, 1.0)
        .is_err());
    let encoded = serde_json::to_value(&genome).unwrap();
    let inherited = serde_json::to_value(&child).unwrap();
    for side in ["maternal", "paternal"] {
        for (i, emitter) in encoded["chemistry"]["graph"][side]["emitters"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            assert_eq!(
                emitter.get("developmental_expression_floor").is_some(),
                i == index
            );
        }
        assert_eq!(
            inherited["chemistry"]["graph"][side]["emitters"][index]
                ["developmental_expression_floor"],
            1.0
        );
    }
    println!(
        "newborn_pain={} old_pain={} quiet_pain={} child_pain={}",
        hurt.homeostasis.drives.pain,
        old_hurt.homeostasis.drives.pain,
        quiet.homeostasis.drives.pain,
        child_hurt.homeostasis.drives.pain
    );
}
