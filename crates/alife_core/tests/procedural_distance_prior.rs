//! Birth-prior revision and exact legacy recompilation, independent of vision.
use alife_core::*;

fn founder(seed: u64) -> CreatureGenome {
    CreatureGenome::early_mammal_founder(
        seed,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N1024_ID).unwrap(),
    )
    .unwrap()
}
fn legacy_founder(seed: u64) -> CreatureGenome {
    let mut creature = founder(seed);
    creature.brain.construction.maternal_distance_gain = 1.0;
    creature.brain.construction.paternal_distance_gain = 1.0;
    creature
}
fn inputs(genome: BrainGenome) -> PhenotypeCompilerInputs {
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    PhenotypeCompilerInputs::try_new(
        genome,
        &BrainCapacityClass::n1024(),
        development,
        SensorProfile::GroundedTerrainVisionV1,
    )
    .unwrap()
}
fn digest<T: serde::Serialize>(value: &T) -> String {
    blake3::hash(&serde_json::to_vec(value).unwrap())
        .to_hex()
        .to_string()
}

#[test]
fn legacy_inputs_recompile_to_prechange_golden_phenotype_bytes() {
    // Captured before this revision from main 05c25089bd586ffa88516ae2d35f4820ddb0ab65,
    // N1024 early-mammal seed96001 at full maturity. Keep the actual persisted
    // inputs, not a current-compiler-generated imitation of the legacy fixture.
    let bytes = include_bytes!("fixtures/legacy_distance_inputs.json");
    let old: PhenotypeCompilerInputs = serde_json::from_slice(bytes).unwrap();
    assert_eq!(old.genome().procedural_distance_gain, 1.0);
    assert_eq!(serde_json::to_vec(&old).unwrap(), bytes);
    assert_eq!(
        digest(&old),
        "beda33aab386969681204f3bae4df340968866987ba6b6a0441343c1774c6c0b"
    );
    let phenotype =
        PhenotypeCompiler::compile_validated(&old, &BrainCapacityClass::n1024()).unwrap();
    assert_eq!(
        phenotype.phenotype_hash(),
        PhenotypeHash([
            9859142718175891090,
            10415550312981859185,
            336908772012686224,
            12860956408481087365,
        ])
    );
    assert_eq!(
        digest(&phenotype),
        "709826f5837e2ec2691cb9627423f32e70736848e6d09e4b9ec761c42b3368c4"
    );
}

#[test]
fn legacy_creature_expression_and_serialization_remain_exact() {
    let legacy = legacy_founder(96001);
    let bytes = serde_json::to_vec(&legacy).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("distance_gain"));
    assert_eq!(
        digest(&legacy),
        "49404e85ae5e13e9fb5862ba7a3bc0ffc07961d528a7efc53ae7a43dc95b5291"
    );
    let restored: CreatureGenome = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored, legacy);
    let expressed = restored.express().unwrap().brain_genome;
    assert_eq!(expressed.procedural_distance_gain, 1.0);
    assert_eq!(
        digest(&expressed),
        "4adf1ab4ed4e812fd5bb7f098cc59302ac3ed11b282c8413da0e395c8b9c5407"
    );
    assert_eq!(
        digest(&inputs(expressed)),
        "beda33aab386969681204f3bae4df340968866987ba6b6a0441343c1774c6c0b"
    );
}

#[test]
fn new_birth_gain_is_explicit_bound_and_identity_bearing() {
    let fresh = founder(96001);
    let genome = fresh.express().unwrap().brain_genome;
    assert_eq!(genome.procedural_distance_gain, 6.0);
    assert_eq!(
        BrainGenome::scaffold(96001, BrainCapacityClass::N1024_ID).procedural_distance_gain,
        1.0
    );
    let new = inputs(genome.clone());
    let old = inputs(legacy_founder(96001).express().unwrap().brain_genome);
    assert_ne!(new.canonical_digest(), old.canonical_digest());
    let restored: PhenotypeCompilerInputs =
        serde_json::from_slice(&serde_json::to_vec(&new).unwrap()).unwrap();
    assert_eq!(new, restored);
    let compiled =
        PhenotypeCompiler::compile_validated(&new, &BrainCapacityClass::n1024()).unwrap();
    assert_eq!(
        compiled,
        PhenotypeCompiler::compile_validated(&restored, &BrainCapacityClass::n1024()).unwrap()
    );
    for value in [0.0, 0.99, 16.01, f32::NAN, f32::INFINITY] {
        let mut invalid = genome.clone();
        invalid.procedural_distance_gain = value;
        assert!(invalid.validate_contract().is_err());
        let mut invalid_creature = fresh.clone();
        invalid_creature.brain.construction.maternal_distance_gain = value;
        assert!(invalid_creature.validate_contract().is_err());
    }
    for value in [1.0, 16.0] {
        let mut valid = genome.clone();
        valid.procedural_distance_gain = value;
        valid.validate_contract().unwrap();
    }
    let mut tampered = serde_json::to_value(new).unwrap();
    tampered["genome"]["procedural_distance_gain"] = serde_json::json!(5.0);
    assert!(serde_json::from_value::<PhenotypeCompilerInputs>(tampered).is_err());
}

#[test]
fn distance_prior_inherits_one_allele_from_each_parent_and_expresses_mean() {
    let mut maternal = founder(96001);
    maternal.brain.construction.maternal_distance_gain = 2.0;
    maternal.brain.construction.paternal_distance_gain = 4.0;
    let mut paternal = founder(96002);
    paternal.brain.construction.maternal_distance_gain = 8.0;
    paternal.brain.construction.paternal_distance_gain = 10.0;
    for seed in 1..=16 {
        let child = CreatureGenome::reproduce(&maternal, &paternal, seed).unwrap();
        let a = child.brain.construction.maternal_distance_gain;
        let b = child.brain.construction.paternal_distance_gain;
        assert!([2.0, 4.0].contains(&a));
        assert!([8.0, 10.0].contains(&b));
        assert_eq!(
            child
                .express()
                .unwrap()
                .brain_genome
                .procedural_distance_gain,
            (a + b) * 0.5
        );
    }
}

#[test]
fn legacy_reproduction_retains_prechange_random_stream_and_child_bytes() {
    // Same pre-change executable as the input golden captured this offspring.
    let child =
        CreatureGenome::reproduce(&legacy_founder(96001), &legacy_founder(96002), 96003).unwrap();
    assert_eq!(
        digest(&child),
        "5e7bae6f91ed2d6775b22578ccf2ef321ae97ec9b517c7d47212c55755b3e261"
    );
    assert_eq!(
        child
            .express()
            .unwrap()
            .brain_genome
            .procedural_distance_gain,
        1.0
    );
}

#[test]
fn exact_foundation_weights_are_not_recalibrated_by_birth_gain() {
    let capacity = BrainCapacityClass::n2048();
    let mut genome = BrainGenome::scaffold(96001, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let baseline = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedTerrainVisionV1,
    )
    .unwrap();
    let mut weights: Vec<_> = baseline
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect();
    let distance = baseline.synapses().iter().position(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate && c.input_lane() == 2)).unwrap();
    weights[distance] = 0.625; // A learned signed choice must survive, not receive a birth prior.
    let asset = FoundationWeightAsset::from_trained_weights(
        &baseline,
        weights.clone(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let bytes = asset.encode_canonical().unwrap();
    let mut identities = Vec::new();
    for gain in [1.0, 6.0] {
        genome.procedural_distance_gain = gain;
        let (compiled, inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome.clone(),
            development.clone(),
            asset.clone(),
        )
        .unwrap();
        assert!(compiled
            .synapses()
            .iter()
            .zip(&weights)
            .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits()));
        let restored: PhenotypeCompilerInputs =
            serde_json::from_slice(&serde_json::to_vec(&inputs).unwrap()).unwrap();
        assert_eq!(
            PhenotypeCompiler::compile_validated(&restored, &capacity).unwrap(),
            compiled
        );
        identities.push(compiled.phenotype_hash());
    }
    assert_ne!(identities[0], identities[1]); // Explicit provenance remains distinguishable.
    assert_eq!(asset.encode_canonical().unwrap(), bytes);
}
