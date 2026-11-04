#![cfg(feature = "cpu-backend")]
//! Synthetic state-preservation checks, not skill or native GPU evidence.
use alife_core::*;
use alife_gpu_backend::{GpuLiveTopologyCheckpointV1, GpuV11CausalState};
use alife_training::{cpu::retained_interpolation_experiment::*, cpu::*, *};
use alife_world::persistence::*;

fn parent(inputs: PhenotypeCompilerInputs, policy: u64) -> Vec<u8> {
    let p = PhenotypeCompiler::compile_validated(&inputs, &BrainCapacityClass::n2048()).unwrap();
    let asset = inputs.n2048_candidate_asset().unwrap();
    let count = p.synapses().len();
    let actor = FoundationTrainerCheckpoint {
        schema_version: 2,
        phenotype_hash: p.phenotype_hash(),
        source_foundation_digest: asset.digest(),
        optimizer_step: 16_777_219,
        config: AdamWConfig::default(),
        stage_mask: StageTrainableMask::from_synapse_indices(
            &p,
            &(0..count as u32 - 1).collect::<Vec<_>>(),
        )
        .unwrap(),
        weights: asset.weights().to_vec(),
        first_moment: (0..count)
            .map(|i| if i == 0 { -0. } else { 0.001 })
            .collect(),
        second_moment: vec![0.002; count],
        update_ages: (0..count).map(|i| 16_777_219 - (i % 7) as u32).collect(),
    };
    let width = p.neuron_count() as usize + 1;
    let value = PpoValueHeadCheckpoint {
        feature_count: p.neuron_count(),
        optimizer_step: 16_777_221,
        parameters: (0..3 * width)
            .map(|i| if i < width { -0.003 } else { 0.004 })
            .collect(),
        last_updated_policy_version: Some(policy),
    };
    CurrentTrainingCheckpointV1 {
        schema_version: 1,
        seed: policy,
        founder_seed_base: 77,
        seeds: CheckpointSeedsV1 {
            historical_freshness: "unknown".into(),
            excluded_ranges: vec![[policy, policy]],
            pending_range: None,
        },
        handoff: FoundationTrainingHandoffV1::new(
            FoundationTrainingBackendV1::NativeCpu,
            policy,
            inputs,
            actor,
            value,
        )
        .unwrap(),
        life: None,
        same_individual: None,
        terminal_disposition: None,
        training_attempt_origin: None,
        new_life_origin: None,
    }
    .encode()
    .unwrap()
}

fn fixture(active_bank: u8) -> InterpolationExperimentInputs {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x746900, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    let p = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let asset = FoundationWeightAsset::from_trained_weights(
        &p,
        p.synapses().iter().map(|s| s.genetic_weight()).collect(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let (p, pc_inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome.clone(),
        development.clone(),
        asset.clone(),
    )
    .unwrap();
    let mut weights = asset.weights().to_vec();
    // Exercise a recurrent and decoder coordinate, retaining the last frozen.
    weights[0] *= 0.75;
    let decoder = p
        .synapses()
        .iter()
        .position(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(_)))
        .unwrap();
    weights[decoder] += 0.002;
    let cloud_asset =
        FoundationWeightAsset::from_trained_weights(&p, weights, asset.manifest().training_stage())
            .unwrap();
    let (_, cloud_inputs) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, cloud_asset)
            .unwrap();
    let topology = (0..16)
        .find_map(|source| {
            let mut state = GpuV11CausalState::for_phenotype(&p).unwrap();
            state
                .apply_structural_phase(&[CoactivationEvidence {
                    region: 0,
                    source,
                    target: 1,
                    coactivation: 32,
                    eligibility: 16,
                    concept_gap_support: 1,
                }])
                .unwrap();
            let mut checkpoint = state.checkpoint();
            checkpoint.pending_lifetime_synapse = None;
            GpuLiveTopologyCheckpointV1::recompile_for_phenotype(&p, checkpoint)
                .ok()
                .filter(|t| t.total_synapse_count as usize == p.synapses().len() + 1)
        })
        .expect("synthetic acquired edge");
    let bank = |layer_raw| {
        let mut result = PortableDualWeightBankV1 {
            schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
            layer_raw,
            phenotype_hash: p.phenotype_hash(),
            synapse_count: topology.total_synapse_count,
            active_generation: 1234,
            active_bank,
            bank_0_bits: vec![0; topology.total_synapse_count as usize],
            bank_1_bits: vec![0; topology.total_synapse_count as usize],
            canonical_digest: [0; 4],
        };
        result.canonical_digest = result.recompute_canonical_digest().unwrap();
        result
    };
    let mut retained = CpuRetainedNeuralInheritance {
        lifetime: bank(GPU_BRAIN_WEIGHT_LAYER_LIFETIME),
        fast: bank(GPU_BRAIN_WEIGHT_LAYER_FAST),
        topology,
        predictor: Default::default(),
    };
    for (i, live) in retained
        .canonical_to_live(&p)
        .unwrap()
        .into_iter()
        .enumerate()
    {
        retained.lifetime.bank_0_bits[live] = (0.001 * (i % 13) as f32).to_bits();
        retained.lifetime.bank_1_bits[live] = (-0.002 * (1 + i % 17) as f32).to_bits();
        retained.fast.bank_0_bits[live] = (0.003 * (i % 7) as f32).to_bits();
        retained.fast.bank_1_bits[live] = (-0.004 * (1 + i % 11) as f32).to_bits();
    }
    for bank in [&mut retained.lifetime, &mut retained.fast] {
        bank.canonical_digest = bank.recompute_canonical_digest().unwrap();
    }
    retained
        .predictor
        .observe(
            &PredictionTargetReceipt::for_successor(
                OrganismId(7),
                ExperienceSequenceId(1),
                ActionId(1),
                Tick(1),
                [1, 2, 3, 4],
                SemanticStateVector::new(vec![0.5, 0.25]).unwrap(),
                JointMotorCondition::new(vec![MotorChannelFactor {
                    channel: MotorChannel::Locomotion,
                    primitive: ActionId(1),
                    intensity: 0.8,
                    duration_ticks: 2,
                    direction: Vec3f::new(1., 0., 0.),
                    stand_off_distance: 0.,
                    confidence: 0.9,
                    target: None,
                    payload: vec![],
                    coordination_group: 0,
                }])
                .unwrap(),
                SemanticStateVector::new(vec![0.25, 0.75]).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
    assert!(retained.predictor.has_acquired_state());
    InterpolationExperimentInputs {
        pc_current: parent(pc_inputs, 100),
        cloud_current: parent(cloud_inputs, 200),
        pc_retained_projection_json: serde_json::to_vec(&retained).unwrap(),
        cloud_fraction: 0.5,
    }
}

#[test]
fn both_active_banks_acquired_edges_decoder_offsets_and_complete_parents_survive() {
    for active in [0, 1] {
        let inputs = fixture(active);
        let pc = CurrentTrainingCheckpointV1::decode(&inputs.pc_current).unwrap();
        let cloud = CurrentTrainingCheckpointV1::decode(&inputs.cloud_current).unwrap();
        let before: CpuRetainedNeuralInheritance =
            serde_json::from_slice(&inputs.pc_retained_projection_json).unwrap();
        let p = pc.handoff.clone().admit().unwrap().phenotype;
        let before_map = before.canonical_to_live(&p).unwrap();
        let candidate = prepare_retained_interpolation_experiment(inputs.clone()).unwrap();
        assert_eq!(
            serde_json::to_vec(candidate.inputs()).unwrap(),
            serde_json::to_vec(&inputs).unwrap()
        );
        let saved_pc = CurrentTrainingCheckpointV1::decode(&candidate.inputs().pc_current).unwrap();
        let saved_cloud =
            CurrentTrainingCheckpointV1::decode(&candidate.inputs().cloud_current).unwrap();
        assert_eq!(saved_pc.handoff, pc.handoff);
        assert_eq!(saved_cloud.handoff, cloud.handoff);
        assert_eq!(saved_pc.handoff.actor.update_ages[0], 16_777_219);
        assert_eq!(
            saved_pc.handoff.actor.first_moment[0].to_bits(),
            (-0.0f32).to_bits()
        );
        let next = PhenotypeCompiler::compile_validated(
            candidate.compiler_inputs(),
            &BrainCapacityClass::n2048(),
        )
        .unwrap();
        let after = candidate.retained();
        assert_eq!(after.canonical_to_live(&next).unwrap(), before_map);
        assert_eq!(
            after.topology.v11_checkpoint,
            before.topology.v11_checkpoint
        );
        assert_eq!(
            after.topology.decoder_synapse_starts,
            before.topology.decoder_synapse_starts
        );
        assert_eq!(
            after.topology.decoder_weight_global_synapse_ids,
            before.topology.decoder_weight_global_synapse_ids
        );
        assert!(after.topology.decoder_synapse_count > 0);
        let extra = (0..after.topology.total_synapse_count as usize)
            .find(|i| !before_map.contains(i))
            .unwrap();
        assert_eq!(
            after.topology.genetic_weight_bits[extra],
            before.topology.genetic_weight_bits[extra]
        );
        assert_eq!(
            after.topology.alpha_bits[extra],
            before.topology.alpha_bits[extra]
        );
        for (a, b) in [
            (&after.fast, &before.fast),
            (&after.lifetime, &before.lifetime),
        ] {
            assert_eq!(a.bank_0_bits, b.bank_0_bits);
            assert_eq!(a.bank_1_bits, b.bank_1_bits);
            assert_eq!(a.active_bank, active);
            assert_eq!(a.active_generation, b.active_generation);
        }
        assert_eq!(
            serde_json::to_vec(&after.predictor).unwrap(),
            serde_json::to_vec(&before.predictor).unwrap()
        );
        assert_eq!(candidate.receipt().acquired_coordinates, 1);
        assert!(candidate.receipt().changed_genetic_coordinates >= 1);
        let bytes = serde_json::to_vec(&candidate).unwrap();
        let restored: RetainedInterpolationExperiment = serde_json::from_slice(&bytes).unwrap();
        restored.validate().unwrap();
        assert!(CurrentTrainingCheckpointV1::decode(&bytes).is_err());
        let value = serde_json::to_value(&candidate).unwrap();
        assert!(value.get("actor").is_none() && value.get("value").is_none());
        assert!(candidate
            .receipt()
            .optimizer_semantics
            .contains("no-candidate-actor-or-critic"));
    }
}

#[test]
fn interpolation_endpoints_are_exact_and_fraction_is_bounded() {
    let mut inputs = fixture(1);
    for (fraction, bytes) in [
        (0., inputs.pc_current.clone()),
        (1., inputs.cloud_current.clone()),
    ] {
        inputs.cloud_fraction = fraction;
        let expected = CurrentTrainingCheckpointV1::decode(&bytes).unwrap();
        let candidate = prepare_retained_interpolation_experiment(inputs.clone()).unwrap();
        let weights = candidate
            .compiler_inputs()
            .n2048_candidate_asset()
            .unwrap()
            .weights();
        assert_eq!(
            weights.iter().map(|w| w.to_bits()).collect::<Vec<_>>(),
            expected
                .handoff
                .actor
                .weights
                .iter()
                .map(|w| w.to_bits())
                .collect::<Vec<_>>()
        );
    }
    for fraction in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
        inputs.cloud_fraction = fraction;
        assert!(prepare_retained_interpolation_experiment(inputs.clone()).is_err());
    }
}

#[test]
fn rejects_frozen_disagreement_masks_and_changed_coordinate_contracts() {
    let inputs = fixture(1);
    let cloud = CurrentTrainingCheckpointV1::decode(&inputs.cloud_current).unwrap();
    let p = cloud.handoff.clone().admit().unwrap().phenotype;
    let mut weights = cloud.handoff.actor.weights.clone();
    *weights.last_mut().unwrap() += 0.01;
    let asset = FoundationWeightAsset::from_trained_weights(
        &p,
        weights,
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let (_, changed_inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        cloud.handoff.compiler_inputs.genome().clone(),
        cloud.handoff.compiler_inputs.development().clone(),
        asset,
    )
    .unwrap();
    let mut invalid = inputs.clone();
    invalid.cloud_current = parent(changed_inputs, 200);
    assert!(prepare_retained_interpolation_experiment(invalid).is_err());
    let mut changed = cloud.clone();
    changed.handoff.actor.stage_mask = StageTrainableMask::from_synapse_indices(
        &p,
        &(0..p.synapses().len() as u32).collect::<Vec<_>>(),
    )
    .unwrap();
    let mut invalid = inputs.clone();
    invalid.cloud_current = changed.encode().unwrap();
    assert!(prepare_retained_interpolation_experiment(invalid).is_err());
    // A different sensor/decoder graph is an admitted parent but not a successor.
    let capacity = BrainCapacityClass::n2048();
    let genome = cloud.handoff.compiler_inputs.genome().clone();
    let development = cloud.handoff.compiler_inputs.development().clone();
    let other = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedTerrainVisionV1,
    )
    .unwrap();
    let asset = FoundationWeightAsset::from_trained_weights(
        &other,
        other
            .synapses()
            .iter()
            .map(|s| s.genetic_weight())
            .collect(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let (_, other_inputs) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset).unwrap();
    let mut invalid = inputs;
    invalid.cloud_current = parent(other_inputs, 200);
    assert!(prepare_retained_interpolation_experiment(invalid).is_err());
}

#[test]
fn rejects_resealed_layout_or_unrepresentable_added_bank_without_mutating_sources() {
    let inputs = fixture(1);
    let original = serde_json::to_vec(&inputs).unwrap();
    let pc = CurrentTrainingCheckpointV1::decode(&inputs.pc_current).unwrap();
    let p = pc.handoff.admit().unwrap().phenotype;
    let source: CpuRetainedNeuralInheritance =
        serde_json::from_slice(&inputs.pc_retained_projection_json).unwrap();
    for variant in 0..3 {
        let mut projection = source.clone();
        if variant == 0 {
            projection.topology.decoder_synapse_starts[0] += 1;
            projection.topology.canonical_digest =
                projection.topology.recompute_canonical_digest().unwrap();
        } else if variant == 1 {
            projection.topology.genetic_weight_bits[0] = 0.125f32.to_bits();
            projection.topology.canonical_digest =
                projection.topology.recompute_canonical_digest().unwrap();
        } else {
            let map = source.canonical_to_live(&p).unwrap();
            let extra = (0..source.topology.total_synapse_count as usize)
                .find(|i| !map.contains(i))
                .unwrap();
            projection.fast.bank_0_bits[extra] = 0.125f32.to_bits();
            projection.fast.canonical_digest =
                projection.fast.recompute_canonical_digest().unwrap();
        }
        let mut invalid = inputs.clone();
        invalid.pc_retained_projection_json = serde_json::to_vec(&projection).unwrap();
        assert!(prepare_retained_interpolation_experiment(invalid).is_err());
    }
    assert_eq!(serde_json::to_vec(&inputs).unwrap(), original);
}

#[test]
fn cold_validation_rejects_changed_overlay_or_receipt_and_bad_parent_bytes() {
    let inputs = fixture(1);
    let candidate = prepare_retained_interpolation_experiment(inputs.clone()).unwrap();
    for field in ["retained", "receipt"] {
        let mut value = serde_json::to_value(&candidate).unwrap();
        if field == "retained" {
            value[field]["fast"]["bank_0_bits"][0] = serde_json::json!(0.125f32.to_bits());
        } else {
            value[field]["qualification"] = serde_json::json!("qualified");
        }
        let invalid: RetainedInterpolationExperiment = serde_json::from_value(value).unwrap();
        assert!(invalid.validate().is_err());
    }
    let mut invalid = inputs;
    invalid.cloud_current[56] ^= 1;
    assert!(prepare_retained_interpolation_experiment(invalid).is_err());
}
