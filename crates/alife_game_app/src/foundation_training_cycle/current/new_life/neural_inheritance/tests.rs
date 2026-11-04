use super::super::tests::{game, metadata_life, terminal};
use super::*;
use alife_core::*;

fn learned_topology(p: &BrainPhenotype) -> GpuLiveTopologyCheckpointV1 {
    for source in 0..16 {
        let mut state = alife_gpu_backend::GpuV11CausalState::for_phenotype(p).unwrap();
        let mut branches = state.dendritic_branches().branches().to_vec();
        branches[0].output_gain *= 0.75;
        state
            .set_dendritic_branches(DendriticBranchSet::new(branches).unwrap())
            .unwrap();
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
        checkpoint
            .structural
            .discover_candidates(&[CoactivationEvidence {
                region: 0,
                source: 20,
                target: 2,
                coactivation: 1,
                eligibility: 1,
                concept_gap_support: 0,
            }])
            .unwrap();
        if let Ok(top) = GpuLiveTopologyCheckpointV1::recompile_for_phenotype(p, checkpoint) {
            if top.total_synapse_count == p.synapses().len() as u32 + 1 {
                return top;
            }
        }
    }
    panic!("no valid disposable learned edge");
}

pub(crate) fn complete_neural_metadata(
    mut life: CurrentLifeCheckpointV1,
    p: &BrainPhenotype,
    learned: bool,
) -> CurrentLifeCheckpointV1 {
    let mut save = PortableSaveFile::from_json_str(&life.world_json).unwrap();
    let mut brain = save.creatures[0].gpu_brain.clone().unwrap();
    let old_packet: serde_json::Value = serde_json::from_slice(
        &read_asset(&life, &save, brain.exact_cognitive_state.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
    let topology = if learned {
        learned_topology(p)
    } else {
        GpuLiveTopologyCheckpointV1::recompile_for_phenotype(
            p,
            alife_gpu_backend::GpuV11Checkpoint::canonical_for_phenotype(p).unwrap(),
        )
        .unwrap()
    };
    let exact = ExactCognitiveCheckpointState {
        schema_version: V11_EXACT_COGNITIVE_STATE_SCHEMA_VERSION,
        organism_id: brain.organism_id,
        checkpoint_tick: Tick::ZERO,
        cognitive_context: CognitiveContextFrame::empty(
            brain.organism_id,
            ExperienceSequenceId(1),
            Tick::ZERO,
        )
        .unwrap(),
        predictor: Default::default(),
        selected_motor_bundle: None,
        cognitive_work: CognitiveWorkReceipt::zero(),
        sleep_state: SleepState::awake_at(Tick::ZERO),
        last_sleep_work: None,
        dendritic_branches: topology.v11_checkpoint.dendritic_branches.clone(),
        structural_plasticity: topology.v11_checkpoint.structural.clone(),
        structural_edit_receipts: Vec::new(),
        last_sleep_report: None,
        private_semantic_prior: None,
        private_training_sampling: Some(
            serde_json::from_value(old_packet["private_training_sampling"].clone()).unwrap(),
        ),
    };
    brain.schema_version = GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION;
    brain.live_structural_topology = Some(
        put(
            &mut life,
            &mut save,
            "live-structural-topology",
            &serde_json::to_vec(&topology).unwrap(),
        )
        .unwrap(),
    );
    brain.exact_cognitive_state = Some(
        put(
            &mut life,
            &mut save,
            "exact-cognitive-state",
            &exact.encode().unwrap(),
        )
        .unwrap(),
    );
    for layer in [GPU_BRAIN_WEIGHT_LAYER_LIFETIME, GPU_BRAIN_WEIGHT_LAYER_FAST] {
        let n = topology.total_synapse_count as usize;
        let mut bank0 = vec![0; n];
        let mut bank1 = vec![0; n];
        if learned {
            for i in 0..n {
                bank0[i] = ((i % 19) as f32 * 0.001).to_bits();
                bank1[i] = (-((i % 17 + 1) as f32) * 0.002).to_bits();
            }
            // The native private-bank codec rejects negative zero. Global
            // actor/Adam fixtures separately exercise that permitted bit pattern.
            bank0[0] = 0;
            bank1[n - 1] = 1;
        }
        let mut bank = PortableDualWeightBankV1 {
            schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
            layer_raw: layer,
            phenotype_hash: p.phenotype_hash(),
            synapse_count: topology.total_synapse_count,
            active_generation: if learned { 71 } else { 1 },
            active_bank: u8::from(learned),
            bank_0_bits: bank0,
            bank_1_bits: bank1,
            canonical_digest: [0; 4],
        };
        bank.canonical_digest = bank.recompute_canonical_digest().unwrap();
        bank.validate().unwrap();
        let reference = put(
            &mut life,
            &mut save,
            if layer == GPU_BRAIN_WEIGHT_LAYER_LIFETIME {
                "lifetime"
            } else {
                "fast"
            },
            &serde_json::to_vec(&bank).unwrap(),
        )
        .unwrap();
        if layer == GPU_BRAIN_WEIGHT_LAYER_LIFETIME {
            brain.lifetime_weights = reference;
        } else {
            brain.fast_weights = reference;
        }
        brain.active_weight_generation = bank.active_generation;
        brain.active_weight_bank = bank.active_bank;
    }
    let mut eligibility = PortableEligibilityBanksV1 {
        schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
        phenotype_hash: p.phenotype_hash(),
        recurrent_count: topology.recurrent_synapse_count,
        decoder_count: topology.decoder_synapse_count,
        active_generation: 1,
        inactive_generation: 0,
        active_bank: 0,
        recurrent_bank_0_bits: vec![0; topology.recurrent_synapse_count as usize],
        recurrent_bank_1_bits: vec![0; topology.recurrent_synapse_count as usize],
        decoder_bank_0_bits: vec![0; topology.decoder_synapse_count as usize],
        decoder_bank_1_bits: vec![0; topology.decoder_synapse_count as usize],
        canonical_digest: [0; 4],
    };
    eligibility.canonical_digest = eligibility.recompute_canonical_digest().unwrap();
    eligibility.validate().unwrap();
    brain.eligibility = put(
        &mut life,
        &mut save,
        "eligibility",
        &serde_json::to_vec(&eligibility).unwrap(),
    )
    .unwrap();
    save.creatures[0].gpu_brain = Some(brain);
    life.world_json = serde_json::to_string(&save).unwrap();
    life
}

#[cfg(feature = "gpu-tests")]
pub(crate) fn native_learned_fixture(
    life: CurrentLifeCheckpointV1,
    p: &BrainPhenotype,
) -> CurrentLifeCheckpointV1 {
    let mut life = complete_neural_metadata(life, p, true);
    let mut save = PortableSaveFile::from_json_str(&life.world_json).unwrap();
    alife_runtime::retain_current_gpu_checkpoint_manifest_entries(&mut save);
    let closure = save
        .assets
        .entries
        .iter()
        .map(|e| e.relative_path.clone())
        .collect::<std::collections::BTreeSet<_>>();
    life.assets_base64.retain(|path, _| closure.contains(path));
    life.world_json = serde_json::to_string(&save).unwrap();
    reanchor_newborn_journal(&mut life, &save).unwrap();
    life
}

fn fixture() -> (CurrentTrainingCheckpointV1, CurrentLifeCheckpointV1) {
    let mut source = terminal();
    let terminal_event = source.terminal_disposition.take().unwrap();
    let p = source.handoff.clone().admit().unwrap().phenotype;
    let life = complete_neural_metadata(source.life.take().unwrap(), &p, true);
    source.same_individual.as_mut().unwrap().private_life_sha256 =
        sha(&serde_json::to_vec(&life).unwrap());
    source.life = Some(life);
    let source = source
        .with_terminal_disposition(
            &terminal_event.baseline_current_sha256,
            &terminal_event.request_sha256,
            terminal_event.session_nonce,
            terminal_event.terminal_event,
        )
        .unwrap();
    let newborn = game(
        8000,
        2,
        source
            .handoff
            .compiler_inputs
            .n2048_candidate_asset()
            .unwrap(),
    );
    let life = metadata_life(&newborn, &source.handoff, 88);
    let life = complete_neural_metadata(life, &p, false);
    (source, life)
}

#[test]
fn retained_neural_round_trip_preserves_structure_all_banks_and_canonical_adam() {
    let (source, birth) = fixture();
    let original = source.encode().unwrap();
    let (life, proof) = inherit(&source, birth).unwrap();
    assert_eq!(
        proof.live_synapse_count as usize,
        proof.canonical_actor_coordinates + 1
    );
    let imported = source
        .seal_new_life_with_retained_neural(&original, 8000, 2, life, proof)
        .unwrap();
    let decoded = CurrentTrainingCheckpointV1::decode(&imported.encode().unwrap()).unwrap();
    assert_eq!(
        serde_json::to_vec(&decoded.handoff).unwrap(),
        serde_json::to_vec(&source.handoff).unwrap()
    );
    let source_neural = RetainedNeural::read(&source).unwrap();
    verify(&source_neural, decoded.life.as_ref().unwrap()).unwrap();
    assert_eq!(
        source_neural
            .topology
            .v11_checkpoint
            .structural
            .connection_count(),
        1
    );
    assert_eq!(
        source_neural
            .topology
            .v11_checkpoint
            .structural
            .candidate_count(),
        1
    );
    let saved =
        PortableSaveFile::from_json_str(&decoded.life.as_ref().unwrap().world_json).unwrap();
    let world = saved.restore_headless_world().unwrap();
    let record = world.organism_registry().iter().next().unwrap();
    assert_eq!(record.organism_id(), OrganismId(2));
    assert_eq!(world.tick(), Tick::ZERO);
    assert!(record.biochemistry().body.energy > 0.5);
    assert_eq!(
        saved.creatures[0]
            .gpu_brain
            .as_ref()
            .unwrap()
            .memory
            .summary
            .record_count,
        0
    );
    assert_eq!(source.encode().unwrap(), original);
    #[cfg(feature = "cpu-training")]
    {
        use alife_training::{cpu::*, CriticResidualRow};
        let p = source.handoff.clone().admit().unwrap().phenotype;
        let asset = source
            .handoff
            .compiler_inputs
            .n2048_candidate_asset()
            .unwrap();
        let mut before = CpuFoundationTrainer::from_checkpoint(
            p.clone(),
            asset.clone(),
            source.handoff.actor.clone(),
        )
        .unwrap();
        let mut after =
            CpuFoundationTrainer::from_checkpoint(p, asset.clone(), decoded.handoff.actor.clone())
                .unwrap();
        let gradient = vec![0.001; source.handoff.actor.weights.len()];
        before.apply_gradients(&gradient).unwrap();
        after.apply_gradients(&gradient).unwrap();
        assert_eq!(
            serde_json::to_vec(&before.checkpoint()).unwrap(),
            serde_json::to_vec(&after.checkpoint()).unwrap()
        );
        let mut critic_before =
            CpuValueHead::from_checkpoint(source.handoff.value.clone()).unwrap();
        let mut critic_after =
            CpuValueHead::from_checkpoint(decoded.handoff.value.clone()).unwrap();
        let features = vec![
            vec![0.1; source.handoff.value.feature_count as usize],
            vec![0.2; source.handoff.value.feature_count as usize],
        ];
        let rows = [CriticResidualRow {
            current: 0,
            next: 1,
            reward: 0.3,
            discount: 0.9,
        }];
        critic_before
            .residual_step(
                &features,
                &rows,
                source.handoff.actor.config,
                0.5,
                source.handoff.policy_version + 1,
            )
            .unwrap();
        critic_after
            .residual_step(
                &features,
                &rows,
                source.handoff.actor.config,
                0.5,
                source.handoff.policy_version + 1,
            )
            .unwrap();
        assert_eq!(
            serde_json::to_vec(&critic_before.checkpoint()).unwrap(),
            serde_json::to_vec(&critic_after.checkpoint()).unwrap()
        );
    }
}

#[test]
fn retained_neural_preserves_multiple_candidates_including_their_complete_state() {
    let (mut source, birth) = fixture();
    let retained = RetainedNeural::read(&source).unwrap();
    let event = source.terminal_disposition.take().unwrap();
    let mut topology = retained.topology;
    let evidence = (31..34)
        .map(|source| CoactivationEvidence {
            region: 0,
            source,
            target: source - 29,
            coactivation: 1,
            eligibility: 1,
            concept_gap_support: 0,
        })
        .collect::<Vec<_>>();
    topology
        .v11_checkpoint
        .structural
        .discover_candidates(&evidence)
        .unwrap();
    assert_eq!(topology.v11_checkpoint.structural.candidate_count(), 4);
    topology.canonical_digest = topology.recompute_canonical_digest().unwrap();
    let expected = serde_json::to_vec(&topology.v11_checkpoint.structural).unwrap();
    let mut exact = retained.exact;
    exact.structural_plasticity = topology.v11_checkpoint.structural.clone();
    let mut life = source.life.take().unwrap();
    let mut save = PortableSaveFile::from_json_str(&life.world_json).unwrap();
    let mut brain = save.creatures[0].gpu_brain.clone().unwrap();
    brain.live_structural_topology = Some(
        put(
            &mut life,
            &mut save,
            "live-structural-topology",
            &serde_json::to_vec(&topology).unwrap(),
        )
        .unwrap(),
    );
    brain.exact_cognitive_state = Some(
        put(
            &mut life,
            &mut save,
            "exact-cognitive-state",
            &exact.encode().unwrap(),
        )
        .unwrap(),
    );
    save.creatures[0].gpu_brain = Some(brain);
    life.world_json = serde_json::to_string(&save).unwrap();
    source.same_individual.as_mut().unwrap().private_life_sha256 =
        sha(&serde_json::to_vec(&life).unwrap());
    source.life = Some(life);
    let source = source
        .with_terminal_disposition(
            &event.baseline_current_sha256,
            &event.request_sha256,
            event.session_nonce,
            event.terminal_event,
        )
        .unwrap();
    let original = source.encode().unwrap();
    let (life, proof) = inherit(&source, birth).unwrap();
    let child = source
        .seal_new_life_with_retained_neural(&original, 8000, 2, life, proof)
        .unwrap();
    let decoded = CurrentTrainingCheckpointV1::decode(&child.encode().unwrap()).unwrap();
    let inherited = RetainedNeural::read(&decoded).unwrap();
    assert_eq!(
        inherited
            .topology
            .v11_checkpoint
            .structural
            .candidate_count(),
        4
    );
    assert_eq!(
        serde_json::to_vec(&inherited.exact.structural_plasticity).unwrap(),
        expected
    );
    assert_eq!(source.encode().unwrap(), original);
}

#[test]
fn retained_neural_rejects_tampered_mapping_partial_assets_and_identity_reuse() {
    let (source, birth) = fixture();
    let p = source.handoff.clone().admit().unwrap().phenotype;
    let retained = RetainedNeural::read(&source).unwrap();
    let mut altered = retained.topology.clone();
    altered.genetic_weight_bits[0] ^= 1;
    altered.canonical_digest = altered.recompute_canonical_digest().unwrap();
    assert!(validate_topology(&altered, &p).is_err());
    let mut pending = retained.topology.clone();
    pending.v11_checkpoint.pending_lifetime_synapse = Some(alife_gpu_backend::AddLifetimeSynapse {
        source: 30,
        target: 3,
        route: 0,
        initial_weight: 0.1,
        evidence: CoactivationEvidence {
            region: 0,
            source: 30,
            target: 3,
            coactivation: 32,
            eligibility: 16,
            concept_gap_support: 1,
        },
    });
    assert!(validate_topology(&pending, &p).is_err());
    let mut wrong_identity = retained.topology.clone();
    wrong_identity.phenotype_hash.0[0] ^= 1;
    wrong_identity.canonical_digest = wrong_identity.recompute_canonical_digest().unwrap();
    assert!(validate_topology(&wrong_identity, &p).is_err());
    let mut partial = source.clone();
    partial.life.as_mut().unwrap().assets_base64.clear();
    assert!(RetainedNeural::read(&partial).is_err());
    let mut old_id = birth.clone();
    let mut save = PortableSaveFile::from_json_str(&old_id.world_json).unwrap();
    save.creatures[0].gpu_brain.as_mut().unwrap().organism_id = OrganismId(1);
    old_id.world_json = serde_json::to_string(&save).unwrap();
    assert!(inherit(&source, old_id).is_err());
    let original = source.encode().unwrap();
    let (mut inherited, proof) = inherit(&source, birth).unwrap();
    let save = PortableSaveFile::from_json_str(&inherited.world_json).unwrap();
    let brain = save.creatures[0].gpu_brain.as_ref().unwrap();
    let entry = save
        .assets
        .entries
        .iter()
        .find(|entry| entry.asset_id == brain.fast_weights.asset_id)
        .unwrap();
    inherited.insert_asset(entry.relative_path.clone(), b"{}");
    assert!(verify(&retained, &inherited).is_err());
    assert_eq!(source.encode().unwrap(), original);
    assert_eq!(
        proof.canonical_actor_coordinates,
        source.handoff.actor.weights.len()
    );
}
