use super::*;
use alife_gpu_backend::{GpuLiveTopologyCheckpointV1, GpuV11CausalState};

fn inherited(p: &BrainPhenotype) -> CpuRetainedNeuralInheritance {
    let top = (0..16)
        .find_map(|source| {
            let mut state = GpuV11CausalState::for_phenotype(p).unwrap();
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
            GpuLiveTopologyCheckpointV1::recompile_for_phenotype(p, checkpoint)
                .ok()
                .filter(|t| t.total_synapse_count == p.synapses().len() as u32 + 1)
        })
        .expect("constructed acquired edge");
    let bank = |layer_raw| {
        let mut bank = PortableDualWeightBankV1 {
            schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
            layer_raw,
            phenotype_hash: p.phenotype_hash(),
            synapse_count: top.total_synapse_count,
            active_generation: 1234,
            active_bank: 1,
            bank_0_bits: vec![0; top.total_synapse_count as usize],
            bank_1_bits: vec![0; top.total_synapse_count as usize],
            canonical_digest: [0; 4],
        };
        bank.canonical_digest = bank.recompute_canonical_digest().unwrap();
        bank
    };
    let mut source = CpuRetainedNeuralInheritance {
        lifetime: bank(GPU_BRAIN_WEIGHT_LAYER_LIFETIME),
        fast: bank(GPU_BRAIN_WEIGHT_LAYER_FAST),
        topology: top,
        predictor: Default::default(),
    };
    let map = source.canonical_to_live(p).unwrap();
    for (coordinate, &live) in map.iter().enumerate() {
        source.lifetime.bank_0_bits[live] = ((coordinate % 13) as f32 * 0.0001).to_bits();
        source.lifetime.bank_1_bits[live] = ((coordinate % 17) as f32 * 0.0002).to_bits();
        source.fast.bank_0_bits[live] = ((coordinate % 7) as f32 * 0.0003).to_bits();
        source.fast.bank_1_bits[live] = ((coordinate % 11) as f32 * 0.0004).to_bits();
    }
    source.lifetime.canonical_digest = source.lifetime.recompute_canonical_digest().unwrap();
    source.fast.canonical_digest = source.fast.recompute_canonical_digest().unwrap();
    source
}

#[test]
fn retained_acquired_edge_banks_adam_and_private_restore_survive_newborn_projection() {
    let (mut host, mut trainer) = fixture();
    let mut adam = trainer.checkpoint();
    adam.optimizer_step = 16_777_219;
    for i in 0..adam.weights.len() {
        if adam.stage_mask.is_trainable(i) {
            adam.first_moment[i] = 0.001;
            adam.second_moment[i] = 0.002;
            adam.update_ages[i] = 16_777_219;
        }
    }
    trainer.restore_checkpoint(adam.clone()).unwrap();
    let source = inherited(trainer.phenotype());
    let original = serde_json::to_vec(&source).unwrap();
    assert_eq!(
        source.forward_mapping_probe(&trainer).unwrap()["passed"],
        true
    );
    let map = source.canonical_to_live(trainer.phenotype()).unwrap();
    host.inherit_retained_neural(&trainer, &source).unwrap();
    let cp = host.checkpoint(&trainer).unwrap();
    let r = &cp.residents[0];
    assert_eq!(
        r.neural_topology.sparse_spans,
        source.topology.v11_checkpoint.sparse_spans
    );
    assert_eq!(
        serde_json::to_vec(r.inherited_neural_source.as_ref().unwrap()).unwrap(),
        original
    );
    assert!(r.neural.eligibility.iter().all(|v| *v == 0.));
    for (i, &live) in map.iter().enumerate() {
        assert_eq!(r.neural.fast[i].to_bits(), source.fast.bank_1_bits[live]);
        assert_eq!(
            r.neural.lifetime[i].to_bits(),
            source.lifetime.bank_1_bits[live]
        );
    }
    let bytes = serde_json::to_vec(&cp).unwrap();
    let mut restored =
        CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&bytes).unwrap(), &trainer)
            .unwrap();
    let a = host.step(&trainer, 1.).unwrap();
    let b = restored.step(&trainer, 1.).unwrap();
    assert_eq!(
        serde_json::to_vec(&a.decisions).unwrap(),
        serde_json::to_vec(&b.decisions).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
        serde_json::to_vec(&restored.checkpoint(&trainer).unwrap()).unwrap()
    );
    assert_eq!(trainer.checkpoint(), adam);
    assert_eq!(
        serde_json::to_vec(
            host.checkpoint(&trainer).unwrap().residents[0]
                .inherited_neural_source
                .as_ref()
                .unwrap()
        )
        .unwrap(),
        original
    );
    assert!(host.inherit_retained_neural(&trainer, &source).is_err());
}

#[test]
fn retained_projection_rejects_a_learned_extra_coordinate_without_dropping_it() {
    let (_, trainer) = fixture();
    let mut source = inherited(trainer.phenotype());
    let map = source.canonical_to_live(trainer.phenotype()).unwrap();
    let extra = (0..source.topology.total_synapse_count as usize)
        .find(|i| !map.contains(i))
        .unwrap();
    source.fast.bank_1_bits[extra] = 0.125f32.to_bits();
    source.fast.canonical_digest = source.fast.recompute_canonical_digest().unwrap();
    assert!(source.canonical_to_live(trainer.phenotype()).is_err());
}
