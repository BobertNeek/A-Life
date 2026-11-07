use alife_core::*;
use std::collections::{BTreeSet, VecDeque};

fn inputs() -> (BrainGenome, DevelopmentState) {
    let genome = BrainGenome::scaffold(0x2048_2026, BrainCapacityClass::N2048_ID);
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    (genome, development)
}
fn compile() -> (BrainPhenotype, PhenotypeCompilerInputs) {
    let (g, d) = inputs();
    PhenotypeCompiler::compile_n2048_experimental_graph(g, d, SensorProfile::GroundedObjectSlotsV1)
        .unwrap()
}
fn distances(p: &BrainPhenotype, source: u32) -> Vec<usize> {
    let mut next = vec![vec![]; p.neuron_count() as usize];
    for s in p
        .synapses()
        .iter()
        .filter(|s| s.kind() == CompiledSynapseKind::Recurrent)
    {
        next[s.source() as usize].push(s.target() as usize);
    }
    let mut d = vec![usize::MAX; next.len()];
    let mut q = VecDeque::from([source as usize]);
    d[source as usize] = 0;
    while let Some(n) = q.pop_front() {
        for &t in &next[n] {
            if d[t] == usize::MAX {
                d[t] = d[n] + 1;
                q.push_back(t);
            }
        }
    }
    d
}
#[test]
fn experimental_native_graph_covers_grounded_inputs_and_exact_budgets() {
    let (p, _) = compile();
    assert_eq!(p.neuron_count(), 2048);
    assert_eq!(p.synapses().len(), 32768);
    assert_eq!(p.budgets().global.recurrent_synapses, 24576);
    assert_eq!(p.budgets().global.active_tiles, 137);
    assert!(p.budgets().global.active_tiles <= 192);
    assert_eq!(p.lobe_layout(), &N2048FoundationLayoutV1::lobe_layout());
    for (projection, receipt) in p.projections().iter().zip(&p.budgets().routes) {
        let (start, len) = projection.synapse_range();
        let tiles: BTreeSet<_> = p.synapses()[start as usize..(start + len) as usize]
            .iter()
            .filter(|s| s.kind() == CompiledSynapseKind::Recurrent)
            .map(|s| (s.source() / 16, s.target() / 16))
            .collect();
        assert_eq!(tiles.len() as u32, projection.active_tile_count());
        assert_eq!(tiles.len() as u32, receipt.active_tiles);
    }
    let motor = p.lobe_layout().region(LobeKind::ActionPlanning).unwrap();
    for a in p.sensor_encoder().assignments() {
        let d = distances(&p, a.target_neuron());
        for m in motor.start..motor.start + motor.len {
            assert!(
                d[m as usize] <= 4,
                "{:?}:{} at {} cannot reach motor {} within four edges",
                a.source_group(),
                a.source_index(),
                a.target_neuron(),
                m
            );
        }
    }
    let core = p
        .lobe_layout()
        .region(LobeKind::TemporalPredictive)
        .unwrap()
        .start;
    for lobe in [
        LobeKind::WorkingContextExecutive,
        LobeKind::MemoryInterface,
        LobeKind::MultimodalAssociation,
    ] {
        let target = p.lobe_layout().region(lobe).unwrap().start;
        assert!(distances(&p, core)[target as usize] <= 1);
        assert!(distances(&p, target)[core as usize] <= 1);
    }
}

#[test]
fn experimental_identity_roundtrips_but_never_admits_as_default_foundation() {
    let capacity = BrainCapacityClass::n2048();
    let (g, d) = inputs();
    let old = PhenotypeCompiler::compile(&g, &capacity, &d, SensorProfile::GroundedObjectSlotsV1)
        .unwrap();
    let old_bytes = serde_json::to_vec(&old).unwrap();
    let (new, new_inputs) = compile();
    assert_ne!(old.phenotype_hash(), new.phenotype_hash());
    assert_ne!(old.route_abi_digest(), new.route_abi_digest());
    assert_ne!(old.plasticity_abi_digest(), new.plasticity_abi_digest());
    assert_ne!(
        old.persistent_address_map().digest(),
        new.persistent_address_map().digest()
    );
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&new).unwrap();
    assert!(asset.is_n2048_experimental_graph());
    assert_ne!(asset.manifest().foundation_id(), FoundationId::N2048_V1);
    assert_ne!(
        asset.manifest().compatibility_family_id(),
        FoundationCompatibilityFamilyId::N2048_FOUNDATION
    );
    let bytes = asset.encode_canonical().unwrap();
    assert_eq!(
        FoundationWeightAsset::decode_canonical(&bytes).unwrap(),
        asset
    );
    assert!(FoundationAbiBinding::canonical_for_foundation_asset(&capacity, &asset).is_err());
    assert!(PhenotypeCompiler::compile_n2048_foundation_candidate(
        g.clone(),
        d.clone(),
        asset.clone()
    )
    .is_err());
    assert!(PhenotypeCompiler::compile_from_foundation_asset(
        &g,
        &capacity,
        &d,
        SensorProfile::GroundedObjectSlotsV1,
        &asset
    )
    .is_err());
    let (bound, bound_inputs) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        g.clone(),
        d.clone(),
        asset.clone(),
    )
    .unwrap();
    asset.validate_against(&bound).unwrap();
    assert!(bound
        .synapses()
        .iter()
        .zip(asset.weights())
        .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits()));
    let round_inputs: PhenotypeCompilerInputs =
        serde_json::from_slice(&serde_json::to_vec(&bound_inputs).unwrap()).unwrap();
    assert_eq!(
        PhenotypeCompiler::compile_validated(&round_inputs, &capacity).unwrap(),
        bound
    );
    let round_p: BrainPhenotype =
        serde_json::from_slice(&serde_json::to_vec(&bound).unwrap()).unwrap();
    assert_eq!(round_p, bound);
    assert_eq!(
        PhenotypeCompiler::compile_validated(&new_inputs, &capacity).unwrap(),
        new
    );
    assert_eq!(
        serde_json::to_vec(
            &PhenotypeCompiler::compile(&g, &capacity, &d, SensorProfile::GroundedObjectSlotsV1)
                .unwrap()
        )
        .unwrap(),
        old_bytes
    );
    let old_asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&old).unwrap();
    assert!(
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(g, d, old_asset)
            .is_err()
    );
}
#[test]
fn changed_recipe_and_topology_are_rejected_at_deserialization() {
    let (p, inputs) = compile();
    let mut value = serde_json::to_value(&inputs).unwrap();
    value["foundation_abi_selection"]["contract"]["recipe_digest"][0] = serde_json::json!(0);
    assert!(serde_json::from_value::<PhenotypeCompilerInputs>(value).is_err());
    let mut value = serde_json::to_value(&p).unwrap();
    value["synapses"][0]["target"] = serde_json::json!(897);
    assert!(serde_json::from_value::<BrainPhenotype>(value).is_err());
}
