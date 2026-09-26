use alife_core::*;

fn founder(seed: u64) -> CreatureGenome {
    CreatureGenome::early_mammal_founder(
        seed,
        FoundationGeneticIdentity::new(22, 1, 1, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap()
}

#[test]
fn valuation_genes_survive_birth_and_persistence_and_cannot_drift() {
    let mut maternal = founder(301);
    let mut paternal = founder(302);
    let legacy_wire = serde_json::to_value(&maternal).unwrap();
    assert!(legacy_wire["chemistry"]["graph"]["maternal"]
        .get("value_profile")
        .is_none());
    let legacy: CreatureGenome = serde_json::from_value(legacy_wire).unwrap();
    let legacy_state = BiochemistryState::new(&legacy.express().unwrap(), Tick(600)).unwrap();
    let legacy_state_wire = serde_json::to_value(legacy_state).unwrap();
    assert!(legacy_state_wire.get("value_profile").is_none());
    let restored_legacy: BiochemistryState = serde_json::from_value(legacy_state_wire).unwrap();
    assert_eq!(restored_legacy, legacy_state);
    let mut profile = BiologicalValueProfile::default();
    profile.drives[0] *= 0.5;
    profile.hormones[3] = 0.1;
    profile.injury = 0.4;
    profile.disappointment = 0.01;
    for genome in [&mut maternal, &mut paternal] {
        for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
            genome.chemistry.graph = genome
                .chemistry
                .graph
                .clone()
                .with_value_profile(side, profile)
                .unwrap();
        }
    }
    let child = CreatureGenome::reproduce(&maternal, &paternal, 303).unwrap();
    let restored: CreatureGenome =
        serde_json::from_slice(&serde_json::to_vec(&child).unwrap()).unwrap();
    let phenotype = restored.express().unwrap();
    assert_eq!(phenotype.chemistry.biochemical.value_profile(), profile);
    let before = BiochemistryState::new(&phenotype, Tick(600)).unwrap();
    let saved: BiochemistryState =
        serde_json::from_slice(&serde_json::to_vec(&before).unwrap()).unwrap();
    assert_eq!(saved, before);
    saved.validate_against(&phenotype).unwrap();
    let mut different = phenotype.clone();
    different.chemistry.biochemical = founder(304).express().unwrap().chemistry.biochemical;
    assert!(saved.validate_against(&different).is_err());
    let mut after = before;
    after.homeostasis.drives.hunger -= 0.1;
    after.homeostasis.hormones.oxytocin += 0.1;
    after.homeostasis.drives.pain += 0.2;
    let transition = MeasuredPhysiologyTransition::new(before, after).unwrap();
    assert!((transition.homeostatic_improvement() - (0.1 / 14.0 + 0.01)).abs() < 1e-6);
    assert!((transition.aversive_value() - 0.08).abs() < 1e-6);
    profile.disappointment = f32::NAN;
    assert!(maternal
        .chemistry
        .graph
        .clone()
        .with_value_profile(AlleleSide::Maternal, profile)
        .is_err());
}

#[test]
fn every_drive_and_hormone_has_the_same_gene_scaled_delta_contract() {
    let mut wire = serde_json::to_value(BiologicalValueProfile::default()).unwrap();
    for (name, count) in [
        ("drives", DriveSnapshot::CHANNEL_COUNT),
        ("hormones", EndocrineSnapshot::CHANNEL_COUNT),
    ] {
        for channel in 0..count {
            let mut delta = serde_json::to_value(HomeostaticDelta::zero()).unwrap();
            let names: Vec<_> = if name == "drives" {
                vec![
                    "hunger",
                    "fatigue",
                    "fear",
                    "pain",
                    "loneliness",
                    "curiosity",
                    "brain_atp",
                    "temperature_stress",
                    "reproductive_drive",
                ]
            } else {
                vec![
                    "adrenaline",
                    "cortisol",
                    "dopamine",
                    "oxytocin",
                    "serotonin",
                    "acetylcholine",
                    "learning_modulator",
                    "developmental_hormone",
                    "sleep_pressure",
                ]
            };
            if channel < names.len() {
                delta[name][names[channel]] = serde_json::json!(-0.2);
            } else {
                delta[name]["extension"][channel - names.len()] = serde_json::json!(-0.2);
            }
            wire[name] = serde_json::json!(vec![0.0; count]);
            wire[name][channel] = serde_json::json!(0.5);
            let profile: BiologicalValueProfile = serde_json::from_value(wire.clone()).unwrap();
            let delta: HomeostaticDelta = serde_json::from_value(delta).unwrap();
            assert!(
                (profile.value_change(delta, 0.0) + 0.1).abs() < 1e-6,
                "{name} {channel}"
            );
            wire[name][channel] = serde_json::json!(1.0);
            let doubled: BiologicalValueProfile = serde_json::from_value(wire.clone()).unwrap();
            assert!((doubled.value_change(delta, 0.0) + 0.2).abs() < 1e-6);
        }
        wire[name] = serde_json::json!(vec![0.0; count]);
    }
}

#[test]
fn hormonal_tone_modulates_consequences_without_inventing_reward() {
    let phenotype = founder(305).express().unwrap();
    let state = BiochemistryState::new(&phenotype, Tick(600)).unwrap();
    let mut receptors = state.neural_receptor_frame(&phenotype).unwrap();
    for activation in &mut receptors.activations {
        activation.signal = 1.0;
    }
    for (pain, benefit, failure, expected) in [
        (0.0, 0.0, 0.0, [0.0, 0.0]),
        (0.0, 0.2, 0.0, [0.2, 0.0]),
        (0.3, 0.0, 0.0, [0.0, 0.3]),
        (0.0, -0.2, 0.03, [0.0, 0.23]),
        (0.3, 0.2, 0.0, [0.2, 0.3]),
    ] {
        let sample = NeuromodulatorSample::from_components(0.9, pain, benefit, failure, 0.0)
            .unwrap()
            .with_biochemical_receptors(&receptors)
            .unwrap();
        for (actual, expected) in sample.frame().lanes()[6..].iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-6);
        }
        if pain == 0.0 && benefit == 0.0 && failure == 0.0 {
            // Surprise remains separate and may have a genetically chosen
            // effect. Compare with the same sample before hormonal modulation.
            let unmodulated =
                NeuromodulatorSample::from_components(0.9, 0.0, 0.0, 0.0, 0.0).unwrap();
            assert_eq!(
                phenotype
                    .brain_genome
                    .plasticity_parameters()
                    .receptor_profile()
                    .project(&sample.frame())
                    .unwrap(),
                phenotype
                    .brain_genome
                    .plasticity_parameters()
                    .receptor_profile()
                    .project(&unmodulated.frame())
                    .unwrap()
            );
            let neutral = NeuromodulatorSample::from_components(0.0, 0.0, 0.0, 0.0, 0.0)
                .unwrap()
                .with_biochemical_receptors(&receptors)
                .unwrap();
            assert_eq!(
                phenotype
                    .brain_genome
                    .plasticity_parameters()
                    .receptor_profile()
                    .project(&neutral.frame())
                    .unwrap(),
                0.0
            );
        }
    }
}

#[test]
fn ordinary_biological_events_have_proportionate_values() {
    let phenotype = founder(306).express().unwrap();
    let before = BiochemistryState::new(&phenotype, Tick(600)).unwrap();
    let measure = |event| {
        let after = before.advance(Tick(601), event, &phenotype).unwrap();
        let transition = MeasuredPhysiologyTransition::new(before, after).unwrap();
        (
            transition.homeostatic_improvement() - transition.aversive_value(),
            after,
        )
    };
    let (neutral, _) = measure(BodyEventDelta::zero());
    let mut values = Vec::new();
    for (name, event, sign) in [
        (
            "injury",
            BodyEventDelta {
                damage: 0.4,
                ..BodyEventDelta::zero()
            },
            -1.0,
        ),
        (
            "social",
            BodyEventDelta {
                social_contact: 0.2,
                ..BodyEventDelta::zero()
            },
            1.0,
        ),
        (
            "recovery",
            BodyEventDelta {
                sleep_recovery: 0.2,
                ..BodyEventDelta::zero()
            },
            1.0,
        ),
        (
            "temperature",
            BodyEventDelta {
                temperature_stress: 0.4,
                ..BodyEventDelta::zero()
            },
            -1.0,
        ),
        (
            "mating cue",
            BodyEventDelta {
                mating_opportunity: 0.4,
                ..BodyEventDelta::zero()
            },
            0.0,
        ),
    ] {
        let (value, _) = measure(event);
        let relative = value - neutral;
        if sign == 0.0 {
            assert!(relative.abs() < 1e-6, "{name}: {relative}");
        } else {
            assert!(relative * sign > 0.0, "{name}: {relative}");
        }
        println!("{name}: {relative:.6} relative to neutral");
        values.push(relative);
    }
    assert!(
        -values[0]
            > phenotype
                .chemistry
                .biochemical
                .value_profile()
                .disappointment
                * 5.0
    );
    assert!(-values[0] > values[2]);
}
