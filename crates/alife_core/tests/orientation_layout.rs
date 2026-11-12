use alife_core::*;
fn phenotype(v2: bool, heads: u8) -> BrainPhenotype {
    let c = BrainCapacityClass::n2048();
    let g = BrainGenome::scaffold(314, c.id());
    let mut value = serde_json::to_value(g.cognitive_architecture()).unwrap();
    value["motor_head_count"] = heads.into();
    let mut a: alife_core::genome::CognitiveArchitectureGenomeParameters =
        serde_json::from_value(value).unwrap();
    if v2 {
        a = a.with_independent_orientation_v2(heads).unwrap();
    }
    let g = g.with_cognitive_architecture(a).unwrap();
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    PhenotypeCompiler::compile(&g, &c, &d, SensorProfile::GroundedObjectSlotsV1).unwrap()
}
#[test]
fn legacy_channel_prefix_and_serialization_stay_v1() {
    let expected = [
        MotorChannel::Locomotion,
        MotorChannel::Manipulation,
        MotorChannel::Vocal,
        MotorChannel::Posture,
    ];
    for n in 1..=4 {
        let p = phenotype(false, n);
        assert_eq!(
            p.candidate_decoder().factorized_motor_channels(&p).unwrap(),
            expected[..n as usize]
        );
        assert_eq!(p.cognitive_architecture().schema_version(), 1);
        let bytes = serde_json::to_vec(&p).unwrap();
        let q: BrainPhenotype = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(q, p);
        assert_eq!(serde_json::to_vec(&q).unwrap(), bytes);
    }
}
#[test]
fn new_layout_is_persisted_distinct_and_roundtrips_without_weight_edits() {
    let old = phenotype(false, 2);
    let new = phenotype(true, 2);
    assert_eq!(
        new.candidate_decoder()
            .factorized_motor_channels(&new)
            .unwrap(),
        vec![MotorChannel::Locomotion, MotorChannel::Orientation]
    );
    assert_ne!(old.phenotype_hash(), new.phenotype_hash());
    assert_eq!(old.synapses(), new.synapses());
    assert_eq!(old.neuron_dynamics(), new.neuron_dynamics());
    assert_eq!(old.sensor_encoder(), new.sensor_encoder());
    assert_eq!(old.candidate_decoder(), new.candidate_decoder());
    let b = serde_json::to_vec(&new).unwrap();
    let q: BrainPhenotype = serde_json::from_slice(&b).unwrap();
    assert_eq!(new, q);
    let mut tampered = serde_json::to_value(&new).unwrap();
    tampered["cognitive_architecture"]["parameters"]["schema_version"] = 1.into();
    assert!(serde_json::from_value::<BrainPhenotype>(tampered).is_err());
}
#[test]
fn v2_supports_only_implemented_channel_prefixes_and_rejects_unknown_layout() {
    for n in 1..=5 {
        let p = phenotype(true, n);
        assert_eq!(
            p.candidate_decoder()
                .factorized_motor_channels(&p)
                .unwrap()
                .len(),
            n as usize
        );
    }
    let g = BrainGenome::scaffold(314, BrainCapacityClass::n2048().id());
    let a = *g.cognitive_architecture();
    assert!(a.with_independent_orientation_v2(0).is_err());
    assert!(a.with_independent_orientation_v2(6).is_err());
    let mut j = serde_json::to_value(a).unwrap();
    j["schema_version"] = 3.into();
    assert!(
        serde_json::from_value::<alife_core::genome::CognitiveArchitectureGenomeParameters>(j)
            .is_err()
    );
}
