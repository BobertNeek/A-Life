use alife_core::{
    BrainCapacityClass, BrainGenome, BrainPhenotype, DevelopmentState, FoundationWeightAsset,
    NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
};
use alife_training::{
    AdamWConfig, FoundationTrainerCheckpoint, PpoTrainingState, PpoValueHeadCheckpoint,
    StageTrainableMask,
};

fn fixture() -> (
    BrainPhenotype,
    FoundationWeightAsset,
    FoundationTrainerCheckpoint,
) {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x7A11, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let foundation =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        &foundation,
    )
    .unwrap();
    let n = phenotype.synapses().len();
    let checkpoint = FoundationTrainerCheckpoint {
        schema_version: 2,
        phenotype_hash: phenotype.phenotype_hash(),
        source_foundation_digest: foundation.digest(),
        optimizer_step: 1,
        config: AdamWConfig::default(),
        stage_mask: StageTrainableMask::from_synapse_indices(&phenotype, &[17]).unwrap(),
        weights: foundation.weights().to_vec(),
        first_moment: vec![0.0; n],
        second_moment: vec![0.0; n],
        update_ages: vec![0; n],
    };
    (phenotype, foundation, checkpoint)
}

#[test]
fn actor_checkpoint_validation_accepts_valid_payload_without_mutation() {
    let (phenotype, foundation, mut checkpoint) = fixture();
    checkpoint.update_ages[17] = 1;
    checkpoint.first_moment[17] = -0.25;
    checkpoint.second_moment[17] = 0.125;
    let encoded = serde_json::to_vec(&checkpoint).unwrap();
    let checkpoint: FoundationTrainerCheckpoint = serde_json::from_slice(&encoded).unwrap();
    let before = checkpoint.clone();
    checkpoint
        .validate_for(&phenotype, foundation.digest())
        .unwrap();
    assert_eq!(checkpoint, before);
}

#[test]
fn actor_checkpoint_validation_rejects_empty_moments_and_all_restore_contract_failures() {
    let (phenotype, foundation, checkpoint) = fixture();
    let reject = |name: &str, changed: FoundationTrainerCheckpoint| {
        assert!(
            changed
                .validate_for(&phenotype, foundation.digest())
                .is_err(),
            "{name}"
        );
    };
    let mut changed = checkpoint.clone();
    changed.first_moment.clear();
    reject("empty first moments", changed);
    for (name, field) in [
        ("empty mask", "stage_mask"),
        ("empty weights", "weights"),
        ("empty second moments", "second_moment"),
        ("empty update ages", "update_ages"),
    ] {
        let mut json = serde_json::to_value(&checkpoint).unwrap();
        if field == "stage_mask" {
            json[field]["words"] = serde_json::json!([]);
        } else {
            json[field] = serde_json::json!([]);
        }
        reject(name, serde_json::from_value(json).unwrap());
    }
    for (name, mutate) in [
        (
            "schema",
            (|c: &mut FoundationTrainerCheckpoint| c.schema_version = 1)
                as fn(&mut FoundationTrainerCheckpoint),
        ),
        ("phenotype", |c| c.phenotype_hash.0[0] ^= 1),
        ("asset", |c| {
            c.source_foundation_digest = alife_core::Blake3Digest::default()
        }),
        ("config", |c| c.config.learning_rate = 0.0),
        ("age beyond step", |c| c.update_ages[0] = 2),
        ("first moment without age", |c| c.first_moment[0] = 0.1),
        ("second moment without age", |c| c.second_moment[0] = 0.1),
        ("nonfinite weight", |c| c.weights[0] = f32::NAN),
        ("nonfinite first moment", |c| {
            c.first_moment[0] = f32::INFINITY
        }),
        ("nonfinite second moment", |c| {
            c.second_moment[0] = f32::INFINITY
        }),
        ("negative second moment", |c| {
            c.update_ages[0] = 1;
            c.second_moment[0] = -0.1;
        }),
    ] {
        let mut changed = checkpoint.clone();
        mutate(&mut changed);
        reject(name, changed);
    }
    for word in [0, 2] {
        let mut json = serde_json::to_value(&checkpoint).unwrap();
        json["stage_mask"]["words"] = serde_json::json!(vec![word; checkpoint.weights.len()]);
        reject("invalid mask words", serde_json::from_value(json).unwrap());
    }
}

#[test]
fn value_checkpoint_restore_validation_rejects_empty_parameters_without_gpu() {
    let valid = PpoValueHeadCheckpoint {
        feature_count: 2,
        optimizer_step: 1,
        parameters: vec![0.0; 9],
        last_updated_policy_version: Some(7),
    };
    assert!(PpoTrainingState::from_checkpoint(valid.clone()).is_ok());
    let mut changed = valid.clone();
    changed.parameters.clear();
    assert!(PpoTrainingState::from_checkpoint(changed).is_err());
    let mut changed = valid.clone();
    changed.parameters[0] = f32::NAN;
    assert!(PpoTrainingState::from_checkpoint(changed).is_err());
    let mut changed = valid;
    changed.parameters[6] = -0.1;
    assert!(PpoTrainingState::from_checkpoint(changed).is_err());
}
