use alife_core::*;
#[test]
fn anchored_v2_has_distinct_identity_and_exact_asset_roundtrip() {
    let g = BrainGenome::scaffold(0x2048_2026, BrainCapacityClass::N2048_ID);
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let (v1, _) = PhenotypeCompiler::compile_n2048_experimental_graph(
        g.clone(),
        d.clone(),
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let (v2, _) = PhenotypeCompiler::compile_n2048_anchored_v2_graph(
        g.clone(),
        d.clone(),
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    assert_ne!(v1.phenotype_hash(), v2.phenotype_hash());
    assert_ne!(v1.route_abi_digest(), v2.route_abi_digest());
    assert_eq!(v1.synapses().len(), v2.synapses().len());
    assert!(v1
        .synapses()
        .iter()
        .zip(v2.synapses())
        .all(|(a, b)| a.source() == b.source()
            && a.target() == b.target()
            && a.route_index() == b.route_index()
            && a.kind() == b.kind()));
    let source = FoundationWeightAsset::decode_canonical(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/foundations/experimental-whole-care-bold-v1.alife-foundation"
    )))
    .unwrap();
    let asset = FoundationWeightAsset::from_trained_weights(
        &v2,
        source.weights().to_vec(),
        source.manifest().training_stage(),
    )
    .unwrap();
    assert!(asset.is_n2048_anchored_v2());
    assert_ne!(
        asset.manifest().foundation_id(),
        source.manifest().foundation_id()
    );
    let bytes = asset.encode_canonical().unwrap();
    assert_eq!(
        FoundationWeightAsset::decode_canonical(&bytes).unwrap(),
        asset
    );
    let (bound, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        g.clone(),
        d.clone(),
        asset.clone(),
    )
    .unwrap();
    assert!(bound
        .synapses()
        .iter()
        .zip(source.weights())
        .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits()));
    let v2_recipe = N2048ExperimentalGraphV1::for_asset(&asset).unwrap();
    assert!(PhenotypeCompilerInputs::try_new_with_foundation_selection(
        g.clone(),
        &BrainCapacityClass::n2048(),
        d.clone(),
        SensorProfile::GroundedObjectSlotsV1,
        FoundationAbiSelection::N2048ExperimentalGraphV1(v2_recipe)
    )
    .is_err());
    let v1_recipe = N2048ExperimentalGraphV1::fresh(SensorProfile::GroundedObjectSlotsV1).unwrap();
    assert!(PhenotypeCompilerInputs::try_new_with_foundation_selection(
        g.clone(),
        &BrainCapacityClass::n2048(),
        d.clone(),
        SensorProfile::GroundedObjectSlotsV1,
        FoundationAbiSelection::N2048ExperimentalGraphV2(v1_recipe)
    )
    .is_err());
    assert!(PhenotypeCompiler::compile_n2048_foundation_candidate(g, d, asset).is_err());
}

#[test]
fn anchored_v2_offspring_keep_port_meanings_and_bind_child_abis() {
    let identity = FoundationGeneticIdentity::new(
        FoundationId::N2048_EXPERIMENTAL_ANCHORED_V2.raw(),
        1,
        FoundationCompatibilityFamilyId::N2048_EXPERIMENTAL_ANCHORED.raw(),
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    let raw_mother = CreatureGenome::early_mammal_founder(701, identity).unwrap();
    let e = raw_mother.express().unwrap();
    let d = e
        .development_state_at(Tick::new(u64::from(
            e.development.maturation_duration_ticks,
        )))
        .unwrap();
    let (p, _) = PhenotypeCompiler::compile_n2048_anchored_v2_graph(
        e.brain_genome,
        d,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let v1 = FoundationWeightAsset::decode_canonical(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/foundations/experimental-whole-care-bold-v1.alife-foundation"
    )))
    .unwrap();
    let source = FoundationWeightAsset::from_trained_weights(
        &p,
        v1.weights().to_vec(),
        v1.manifest().training_stage(),
    )
    .unwrap();
    let mother = raw_mother
        .with_n2048_foundation_candidate(source.clone())
        .unwrap();
    let father = CreatureGenome::early_mammal_founder(702, identity)
        .unwrap()
        .with_n2048_foundation_candidate(source.clone())
        .unwrap();
    let ports = |p: &BrainPhenotype| {
        p.sensor_encoder()
            .assignments()
            .iter()
            .map(|a| (a.source_group().raw(), a.source_index(), a.target_neuron()))
            .collect::<std::collections::BTreeSet<_>>()
    };
    let reference = ports(&p);
    let reserved = reference
        .iter()
        .map(|x| x.2)
        .collect::<std::collections::BTreeSet<_>>();
    for seed in 703..713 {
        let child = CreatureGenome::reproduce(&mother, &father, seed).unwrap();
        let e = child.express().unwrap();
        let d = e
            .development_state_at(Tick::new(u64::from(
                e.development.maturation_duration_ticks,
            )))
            .unwrap();
        let (fresh, _) = PhenotypeCompiler::compile_n2048_anchored_v2_graph(
            e.brain_genome.clone(),
            d.clone(),
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let child_ports = ports(&fresh);
        for added in child_ports.difference(&reference) {
            assert!(
                !reserved.contains(&added.2),
                "child extra reused a reference target: {added:?}"
            );
        }
        assert!(p
            .synapses()
            .iter()
            .zip(fresh.synapses())
            .all(|(a, b)| a.source() == b.source()
                && a.target() == b.target()
                && a.route_index() == b.route_index()
                && a.kind() == b.kind()));
        let rebound = FoundationWeightAsset::from_trained_weights(
            &fresh,
            source.weights().to_vec(),
            source.manifest().training_stage(),
        )
        .unwrap();
        let (bound, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            e.brain_genome,
            d,
            rebound,
        )
        .unwrap();
        assert!(bound
            .synapses()
            .iter()
            .zip(source.weights())
            .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits()));
        println!(
            "seed={seed} parent_ports={} child_ports={} lost={} added={} bits_preserved={}",
            reference.len(),
            child_ports.len(),
            reference.difference(&child_ports).count(),
            child_ports.difference(&reference).count(),
            source.weights().len()
        );
    }
}

#[test]
fn anchored_v2_reproduction_binds_child_weights_for_multiple_generations() {
    let identity = FoundationGeneticIdentity::new(
        FoundationId::N2048_EXPERIMENTAL_ANCHORED_V2.raw(),
        1,
        FoundationCompatibilityFamilyId::N2048_EXPERIMENTAL_ANCHORED.raw(),
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    let raw = CreatureGenome::early_mammal_founder(701, identity).unwrap();
    let e = raw.express().unwrap();
    let d = e
        .development_state_at(Tick::new(u64::from(
            e.development.maturation_duration_ticks,
        )))
        .unwrap();
    let (p, _) = PhenotypeCompiler::compile_n2048_anchored_v2_graph(
        e.brain_genome,
        d,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let v1 = FoundationWeightAsset::decode_canonical(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/foundations/experimental-whole-care-bold-v1.alife-foundation"
    )))
    .unwrap();
    let source = FoundationWeightAsset::from_trained_weights(
        &p,
        v1.weights().to_vec(),
        v1.manifest().training_stage(),
    )
    .unwrap();
    let mut mother = raw.with_n2048_foundation_candidate(source.clone()).unwrap();
    let mut father = CreatureGenome::early_mammal_founder(702, identity)
        .unwrap()
        .with_n2048_foundation_candidate(source.clone())
        .unwrap();
    for generation in 0..10 {
        let child = CreatureGenome::reproduce(&mother, &father, 703 + generation).unwrap();
        let asset = child.n2048_foundation_candidate.as_ref().unwrap();
        assert!(asset.is_n2048_anchored_v2());
        let mutations: std::collections::HashMap<usize, (f32, f32)> = child
            .provenance
            .mutations
            .iter()
            .filter_map(|r| match r {
                MutationRecord::GeneticWeight {
                    index,
                    before,
                    after,
                } => Some((usize::from(*index), (*before, *after))),
                _ => None,
            })
            .collect();
        assert!(!mutations.is_empty());
        let left = mother
            .n2048_foundation_candidate
            .as_ref()
            .unwrap()
            .weights();
        let right = father
            .n2048_foundation_candidate
            .as_ref()
            .unwrap()
            .weights();
        for (i, &actual) in asset.weights().iter().enumerate() {
            if let Some(&(before, after)) = mutations.get(&i) {
                assert!(
                    before.to_bits() == left[i].to_bits() || before.to_bits() == right[i].to_bits()
                );
                assert_eq!(actual.to_bits(), after.to_bits());
                assert_ne!(before.to_bits(), after.to_bits());
            } else {
                assert!(
                    actual.to_bits() == left[i].to_bits() || actual.to_bits() == right[i].to_bits()
                );
            }
        }
        let e = child.express().unwrap();
        let d = e
            .development_state_at(Tick::new(u64::from(
                e.development.maturation_duration_ticks,
            )))
            .unwrap();
        let (bound, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            e.brain_genome,
            d,
            asset.clone(),
        )
        .unwrap();
        assert_eq!(bound.sensor_profile(), SensorProfile::GroundedObjectSlotsV1);
        println!(
            "generation={generation} ports={} inherited_weight_bits={}",
            bound.sensor_encoder().assignments().len(),
            asset.weights().len()
        );
        father = mother;
        mother = child;
    }
    let e = mother.express().unwrap();
    let d = e
        .development_state_at(Tick::new(u64::from(
            e.development.maturation_duration_ticks,
        )))
        .unwrap();
    let (p, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        e.brain_genome,
        d,
        mother.n2048_foundation_candidate.clone().unwrap(),
    )
    .unwrap();
    let mut changed = source.weights().to_vec();
    for value in &mut changed[..128] {
        *value += 0.125;
    }
    let asset = FoundationWeightAsset::from_trained_weights(
        &p,
        changed,
        source.manifest().training_stage(),
    )
    .unwrap();
    let mutated = mother
        .clone()
        .with_n2048_foundation_candidate(asset)
        .unwrap();
    assert!(father.compatible_n2048_foundation_assets(&mutated));
    let mixed = CreatureGenome::reproduce(&father, &mutated, 999).unwrap();
    let repeated = CreatureGenome::reproduce(&father, &mutated, 999).unwrap();
    assert_eq!(mixed, repeated);
    let encoded = serde_json::to_vec(&mixed).unwrap();
    let decoded: CreatureGenome = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(mixed, decoded);
    decoded.validate_contract().unwrap();
    let values = mixed.n2048_foundation_candidate.as_ref().unwrap().weights();
    let paternal = father
        .n2048_foundation_candidate
        .as_ref()
        .unwrap()
        .weights();
    let maternal = mutated
        .n2048_foundation_candidate
        .as_ref()
        .unwrap()
        .weights();
    assert!(values[..128]
        .iter()
        .zip(&paternal[..128])
        .any(|(x, y)| x.to_bits() == y.to_bits()));
    assert!(values[..128]
        .iter()
        .zip(&maternal[..128])
        .any(|(x, y)| x.to_bits() == y.to_bits()));
    let mutations: std::collections::HashMap<usize, (f32, f32)> = mixed
        .provenance
        .mutations
        .iter()
        .filter_map(|r| match r {
            MutationRecord::GeneticWeight {
                index,
                before,
                after,
            } => Some((usize::from(*index), (*before, *after))),
            _ => None,
        })
        .collect();
    assert!(!mutations.is_empty());
    assert!(mutations
        .values()
        .all(|(before, after)| (after - before).abs() <= MAX_MUTATION_DELTA + f32::EPSILON));
    let mut no_mutation_a = father.clone();
    let mut no_mutation_b = mutated.clone();
    no_mutation_a.reproduction.mutation_rate = ContinuousLocus::mean(0.0, 0.0).unwrap();
    no_mutation_b.reproduction.mutation_rate = ContinuousLocus::mean(0.0, 0.0).unwrap();
    let unmutated = CreatureGenome::reproduce(&no_mutation_a, &no_mutation_b, 1001).unwrap();
    assert!(!unmutated
        .provenance
        .mutations
        .iter()
        .any(|r| matches!(r, MutationRecord::GeneticWeight { .. })));
    let mut high_a = father.clone();
    let mut high_b = mutated.clone();
    high_a.reproduction.mutation_rate = ContinuousLocus::mean(1.0, 1.0).unwrap();
    high_b.reproduction.mutation_rate = ContinuousLocus::mean(1.0, 1.0).unwrap();
    let high = CreatureGenome::reproduce(&high_a, &high_b, 1002).unwrap();
    assert_eq!(
        high,
        CreatureGenome::reproduce(&high_a, &high_b, 1002).unwrap()
    );
    assert_eq!(high.provenance.mutations.len(), MAX_MUTATION_RECORDS);
    let indices: Vec<usize> = high
        .provenance
        .mutations
        .iter()
        .filter_map(|r| match r {
            MutationRecord::GeneticWeight { index, .. } => Some(usize::from(*index)),
            _ => None,
        })
        .collect();
    assert!(indices.iter().filter(|&&i| i < 8192).count() > 300);
    assert!(indices.iter().filter(|&&i| i >= 24_576).count() > 300);
    let high_expressed = high.express().unwrap();
    let high_development = high_expressed
        .development_state_at(Tick::new(u64::from(
            high_expressed.development.maturation_duration_ticks,
        )))
        .unwrap();
    let (high_phenotype, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        high_expressed.brain_genome,
        high_development,
        high.n2048_foundation_candidate.clone().unwrap(),
    )
    .unwrap();
    for head in [
        DecoderHeadKind::ActionCandidate,
        DecoderHeadKind::MemoryContext,
    ] {
        assert!(indices.iter().any(|&i| matches!(high_phenotype.synapses()[i].kind(), CompiledSynapseKind::Decoder(coord) if coord.head() == head)), "{head:?} was starved at the mutation-record cap");
    }
    for (i, &actual) in values.iter().enumerate() {
        if let Some(&(before, after)) = mutations.get(&i) {
            assert!(
                before.to_bits() == paternal[i].to_bits()
                    || before.to_bits() == maternal[i].to_bits()
            );
            assert_eq!(actual.to_bits(), after.to_bits());
        } else {
            assert!(
                actual.to_bits() == paternal[i].to_bits()
                    || actual.to_bits() == maternal[i].to_bits()
            );
        }
    }
}
