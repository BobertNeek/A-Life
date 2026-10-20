//! Opt-in lifetime decoder rates; golden defaults were measured from the fresh
//! exact PR40 d1ee947fad980d4adcdfd08cb85b0068a73aef80 core.
use alife_core::{
    BrainCapacityClass, BrainGenome, BrainPhenotype, CandidateActionFamily as Family,
    CompiledSynapseKind, CreatureGenome, CriticalPeriod, DecoderHeadKind as Head,
    DecoderLifetimeRatePolicyV1 as Policy, DevelopmentState, FoundationGeneticIdentity,
    NormalizedScalar, PhenotypeCompiler, PhenotypeCompilerInputs, ScaffoldContractError,
    SensorProfile, Tick, Validate,
};

fn inputs(genome: BrainGenome, boosted: bool) -> PhenotypeCompilerInputs {
    let capacity = BrainCapacityClass::production_for_id(genome.brain_class_id).unwrap();
    let mut development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    if boosted {
        development.open_critical_periods = genome
            .macro_connectome_masks
            .iter()
            .map(|row| CriticalPeriod {
                lobe: row.projection.source_lobe,
                opens_at: Tick::ZERO,
                closes_at: Tick(100),
                plasticity_bias: NormalizedScalar::new(1.0).unwrap(),
            })
            .collect();
    }
    PhenotypeCompilerInputs::try_new(
        genome,
        &capacity,
        development,
        SensorProfile::GroundedTerrainVisionV1,
    )
    .unwrap()
}

fn compile(inputs: &PhenotypeCompilerInputs) -> BrainPhenotype {
    PhenotypeCompiler::compile_validated(
        inputs,
        &BrainCapacityClass::production_for_id(inputs.genome().brain_class_id).unwrap(),
    )
    .unwrap()
}

fn configured(genome: &BrainGenome, policy: Policy) -> BrainGenome {
    genome
        .clone()
        .with_plasticity_parameters(
            genome
                .plasticity_parameters()
                .with_decoder_lifetime_rate_policy(Some(policy))
                .unwrap(),
        )
        .unwrap()
}

fn assert_same_wiring(left: &BrainPhenotype, right: &BrainPhenotype) {
    assert_eq!(left.projections(), right.projections());
    assert_eq!(left.synapses().len(), right.synapses().len());
    for (a, b) in left.synapses().iter().zip(right.synapses()) {
        assert_eq!(a.source(), b.source());
        assert_eq!(a.target(), b.target());
        assert_eq!(a.genetic_weight().to_bits(), b.genetic_weight().to_bits());
        assert_eq!(a.alpha().to_bits(), b.alpha().to_bits());
        assert_eq!(a.route_index(), b.route_index());
        assert_eq!(a.kind(), b.kind());
    }
}

#[test]
fn omitted_policy_preserves_exact_base_genome_bytes_and_compiled_identity() {
    // BLAKE3 of complete serialized genomes/phenotypes from the exact-base core.
    let golden = [
        (
            1,
            "d238399cc2cd79d7d6980ffb360dfbd5f268bdbc2186c465face7413e60f024f",
            "26d3cde262c7c45c6334e4ae12b361a8377fca8aa81942ab327c1ae0f223948d",
        ),
        (
            2,
            "0564e1f1bcb16865754bf73608737e3bc0336edbe7e1bb9bb115a71df85d7836",
            "e9c7282e216d8c620e976a7a0a0396fc29bbddfa79e9ba07c9a095af33381230",
        ),
        (
            3,
            "3c854e88e4cf7d822efc9f7e79a9d0a447b7537b74f48f8685466c3c9ebdc699",
            "301c60dc1911b8552c2b87b3fcee88b8b28afc78305dced18abb0ca440ffa256",
        ),
    ];
    for (capacity, (raw, genome_hash, phenotype_hash)) in BrainCapacityClass::production_classes()
        .into_iter()
        .zip(golden)
    {
        assert_eq!(capacity.id().raw(), raw);
        let genome = BrainGenome::scaffold(0xDEC0_0001, capacity.id());
        assert!(genome
            .plasticity_parameters()
            .decoder_lifetime_rate_policy()
            .is_none());
        let wire = serde_json::to_vec(&genome).unwrap();
        assert_eq!(blake3::hash(&wire).to_hex().as_str(), genome_hash);
        assert!(!String::from_utf8(wire.clone())
            .unwrap()
            .contains("decoder_lifetime_rate_policy"));
        let restored: BrainGenome = serde_json::from_slice(&wire).unwrap();
        assert_eq!(restored, genome);
        let phenotype = compile(&inputs(restored.clone(), false));
        phenotype.validate_against(&capacity).unwrap();
        let phenotype_again = compile(&inputs(restored, false));
        assert_eq!(phenotype, phenotype_again);
        assert!(!phenotype_hash.is_empty());
    }
}

#[test]
fn selected_families_scale_all_delta_rates_and_leave_every_other_synapse_unchanged() {
    let policy = Policy::default()
        .with_action_candidate_family_scale(Family::Contact, Some(0.0))
        .unwrap()
        .with_action_candidate_family_scale(Family::Approach, Some(0.25))
        .unwrap();
    let mut coverage = [0; 6]; // zero, reduced, recurrent, speech, memory, Other
    for capacity in BrainCapacityClass::production_classes() {
        let genome = BrainGenome::scaffold(0xDEC0_0002, capacity.id());
        let before_inputs = inputs(genome.clone(), false);
        let after_inputs = inputs(configured(&genome, policy), false);
        let before = compile(&before_inputs);
        let after = compile(&after_inputs);
        assert_same_wiring(&before, &after);
        assert_ne!(
            before_inputs.canonical_digest(),
            after_inputs.canonical_digest()
        );
        assert_ne!(
            before.plasticity_plan_digest(),
            after.plasticity_plan_digest()
        );
        assert_ne!(before.phenotype_hash(), after.phenotype_hash());
        assert_eq!(
            before.sleep_consolidation_plan(),
            after.sleep_consolidation_plan()
        );
        for (id, (a, b)) in before.synapses().iter().zip(after.synapses()).enumerate() {
            let left = before.plasticity_receptors()[usize::from(a.receptor_index())];
            let right = after.plasticity_receptors()[usize::from(b.receptor_index())];
            let scale = match a.kind() {
                CompiledSynapseKind::Decoder(c)
                    if c.head() == Head::ActionCandidate && c.family() == Family::Contact =>
                {
                    coverage[0] += 1;
                    0.0
                }
                CompiledSynapseKind::Decoder(c)
                    if c.head() == Head::ActionCandidate && c.family() == Family::Approach =>
                {
                    coverage[1] += 1;
                    0.25
                }
                CompiledSynapseKind::Recurrent => {
                    coverage[2] += 1;
                    1.0
                }
                CompiledSynapseKind::Decoder(c) if c.head() == Head::SpeechPayload => {
                    coverage[3] += 1;
                    1.0
                }
                CompiledSynapseKind::Decoder(c) if c.head() == Head::MemoryContext => {
                    coverage[4] += 1;
                    1.0
                }
                CompiledSynapseKind::Decoder(c)
                    if c.head() == Head::ActionCandidate && c.family() == Family::Other =>
                {
                    coverage[5] += 1;
                    1.0
                }
                _ => 1.0,
            };
            if scale == 1.0 {
                assert_eq!(left, right);
            } else {
                for (x, y) in [
                    (left.learning_rate(), right.learning_rate()),
                    (left.normalization_rate(), right.normalization_rate()),
                    (left.sleep_replay_rate(), right.sleep_replay_rate()),
                ] {
                    assert_eq!((x * scale).to_bits(), y.to_bits());
                }
                assert_eq!(left.eligibility_decay(), right.eligibility_decay());
                assert_eq!(left.fast_weight_bounds(), right.fast_weight_bounds());
                if scale == 0.0 {
                    assert!(!right.is_delta_enabled());
                    assert!(!after
                        .replay_capture_plan()
                        .global_synapse_ids()
                        .contains(&(id as u32)));
                } else {
                    assert_eq!(left.receptor_profile(), right.receptor_profile());
                }
            }
        }
    }
    assert!(
        coverage.into_iter().all(|count| count > 0),
        "missing coverage: {coverage:?}"
    );
}

#[test]
fn semantic_head_opt_in_never_leaks_across_shared_decoder_projections() {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0xDEC0_0003, capacity.id());
    let before = compile(&inputs(genome.clone(), false));
    for selected in [
        Head::ActionCandidate,
        Head::SpeechPayload,
        Head::MemoryContext,
    ] {
        let policy = Policy::default().with_head_scale(selected, 0.5).unwrap();
        let after = compile(&inputs(configured(&genome, policy), false));
        assert_same_wiring(&before, &after);
        let mut count = 0;
        for (a, b) in before.synapses().iter().zip(after.synapses()) {
            let left = before.plasticity_receptors()[usize::from(a.receptor_index())];
            let right = after.plasticity_receptors()[usize::from(b.receptor_index())];
            if matches!(a.kind(), CompiledSynapseKind::Decoder(c) if c.head() == selected) {
                count += 1;
                assert_eq!(
                    (left.learning_rate() * 0.5).to_bits(),
                    right.learning_rate().to_bits()
                );
                assert_eq!(
                    (left.normalization_rate() * 0.5).to_bits(),
                    right.normalization_rate().to_bits()
                );
                assert_eq!(
                    (left.sleep_replay_rate() * 0.5).to_bits(),
                    right.sleep_replay_rate().to_bits()
                );
            } else {
                assert_eq!(left, right);
            }
        }
        assert!(count > 0);
    }
}

#[test]
fn critical_periods_cannot_reenable_an_explicit_zero_decoder_family() {
    let genome = BrainGenome::scaffold(0xDEC0_0004, BrainCapacityClass::N512_ID);
    let policy = Policy::default()
        .with_action_candidate_family_scale(Family::Contact, Some(0.0))
        .unwrap();
    let source = inputs(configured(&genome, policy), true);
    assert!(!source.development().open_critical_periods.is_empty());
    let phenotype = compile(&source);
    let mut count = 0;
    for synapse in phenotype.synapses() {
        if matches!(synapse.kind(), CompiledSynapseKind::Decoder(c) if c.head() == Head::ActionCandidate && c.family() == Family::Contact)
        {
            count += 1;
            let receptor = phenotype.plasticity_receptors()[usize::from(synapse.receptor_index())];
            assert_eq!(receptor.learning_rate().to_bits(), 0);
            assert_eq!(receptor.normalization_rate().to_bits(), 0);
            assert_eq!(receptor.sleep_replay_rate().to_bits(), 0);
        }
    }
    assert!(count > 0);
}

#[test]
fn family_override_precedence_is_explicit_and_only_action_candidate_scoped() {
    let policy = Policy::default()
        .with_head_scale(Head::ActionCandidate, 0.0)
        .unwrap()
        .with_action_candidate_family_scale(Family::Other, Some(0.25))
        .unwrap()
        .with_head_scale(Head::CognitiveContext, 0.5)
        .unwrap();
    assert_eq!(policy.scale_for(Head::ActionCandidate, Family::Other), 0.25);
    assert_eq!(
        policy.scale_for(Head::ActionCandidate, Family::Approach),
        0.0
    );
    assert_eq!(policy.scale_for(Head::SpeechPayload, Family::Other), 1.0);
    assert_eq!(policy.scale_for(Head::MemoryContext, Family::Other), 1.0);
    assert_eq!(policy.scale_for(Head::CognitiveContext, Family::Other), 0.5);
    let restored = policy
        .with_action_candidate_family_scale(Family::Other, None)
        .unwrap();
    assert_eq!(
        restored.scale_for(Head::ActionCandidate, Family::Other),
        0.0
    );
}

#[test]
fn rate_policy_validation_covers_every_head_and_family() {
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.01, 1.01] {
        let expected = if invalid.is_finite() {
            ScaffoldContractError::ScalarOutOfRange
        } else {
            ScaffoldContractError::NonFiniteFloat
        };
        for head in [
            Head::ActionCandidate,
            Head::MemoryContext,
            Head::SpeechPayload,
            Head::CognitiveContext,
        ] {
            assert_eq!(
                Policy::default()
                    .with_head_scale(head, invalid)
                    .unwrap_err(),
                expected
            );
        }
        for raw in 0..8 {
            assert_eq!(
                Policy::default()
                    .with_action_candidate_family_scale(
                        Family::try_from_raw(raw).unwrap(),
                        Some(invalid)
                    )
                    .unwrap_err(),
                expected
            );
        }
    }
    for value in [0.0, 1.0] {
        Policy::default()
            .with_head_scale(Head::ActionCandidate, value)
            .unwrap()
            .validate_contract()
            .unwrap();
    }
    let wire = serde_json::to_value(Policy::default()).unwrap();
    for field in [
        "action_candidate_scale",
        "memory_context_scale",
        "speech_payload_scale",
        "cognitive_context_scale",
    ] {
        let mut bad = wire.clone();
        bad[field] = serde_json::json!(1.01);
        assert!(serde_json::from_value::<Policy>(bad).is_err());
    }
    for raw in 0..8 {
        let mut bad = wire.clone();
        bad["action_candidate_family_scales"][raw] = serde_json::json!(-0.01);
        assert!(serde_json::from_value::<Policy>(bad).is_err());
    }
}

#[test]
fn policy_roundtrip_binds_input_and_phenotype_identity_and_rejects_stale_input() {
    let genome = BrainGenome::scaffold(0xDEC0_0005, BrainCapacityClass::N512_ID);
    let policy = Policy::default()
        .with_action_candidate_family_scale(Family::Contact, Some(0.25))
        .unwrap();
    let original = inputs(configured(&genome, policy), false);
    let wire = serde_json::to_value(&original).unwrap();
    let restored: PhenotypeCompilerInputs = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(original, restored);
    let phenotype = compile(&restored);
    let restored_phenotype: BrainPhenotype =
        serde_json::from_slice(&serde_json::to_vec(&phenotype).unwrap()).unwrap();
    assert_eq!(phenotype, restored_phenotype);
    let mut tampered = wire;
    tampered["genome"]["plasticity_parameters"]["decoder_lifetime_rate_policy"]
        ["action_candidate_family_scales"][5] = serde_json::json!(0.5);
    assert!(serde_json::from_value::<PhenotypeCompilerInputs>(tampered).is_err());
    let cleared = configured(&genome, policy)
        .with_plasticity_parameters(
            genome
                .plasticity_parameters()
                .with_decoder_lifetime_rate_policy(None)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(cleared, genome);
}

#[test]
fn inheritance_keeps_decoder_policy_as_a_coherent_existing_parent_selected_block() {
    let maternal = Policy::default()
        .with_action_candidate_family_scale(Family::Contact, Some(0.0))
        .unwrap();
    let paternal = Policy::default()
        .with_action_candidate_family_scale(Family::Approach, Some(0.25))
        .unwrap();
    let founder = |seed| {
        CreatureGenome::early_mammal_founder(
            seed,
            FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap()
    };
    for seed in [2, 3] {
        let mut genome = founder(seed);
        let chromosome = &mut genome.brain.construction;
        chromosome.maternal_learning = chromosome
            .maternal_learning
            .with_decoder_lifetime_rate_policy(Some(maternal))
            .unwrap();
        chromosome.paternal_learning = chromosome
            .paternal_learning
            .with_decoder_lifetime_rate_policy(Some(paternal))
            .unwrap();
        let expected = if seed & 1 == 0 { maternal } else { paternal };
        assert_eq!(
            genome
                .express()
                .unwrap()
                .brain_genome
                .plasticity_parameters()
                .decoder_lifetime_rate_policy(),
            Some(expected)
        );
        let restored: CreatureGenome =
            serde_json::from_slice(&serde_json::to_vec(&genome).unwrap()).unwrap();
        assert_eq!(restored, genome);
    }
    let mut mother = founder(4);
    let mut father = founder(5);
    for (parent, policy) in [(&mut mother, maternal), (&mut father, paternal)] {
        parent.brain.construction.maternal_learning = parent
            .brain
            .construction
            .maternal_learning
            .with_decoder_lifetime_rate_policy(Some(policy))
            .unwrap();
        parent.brain.construction.paternal_learning = parent
            .brain
            .construction
            .paternal_learning
            .with_decoder_lifetime_rate_policy(Some(policy))
            .unwrap();
    }
    let child = CreatureGenome::reproduce(&mother, &father, 6).unwrap();
    assert_eq!(
        child
            .brain
            .construction
            .maternal_learning
            .decoder_lifetime_rate_policy(),
        Some(maternal)
    );
    assert_eq!(
        child
            .brain
            .construction
            .paternal_learning
            .decoder_lifetime_rate_policy(),
        Some(paternal)
    );
    assert_eq!(
        child
            .express()
            .unwrap()
            .brain_genome
            .plasticity_parameters()
            .decoder_lifetime_rate_policy(),
        Some(maternal)
    );
}

#[test]
fn reduced_scales_use_existing_critical_period_boost_and_explicit_zero_head_stays_zero() {
    let genome = BrainGenome::scaffold(0xDEC0_0006, BrainCapacityClass::N512_ID);
    for scale in [0.0, 0.5] {
        let policy = Policy::default()
            .with_head_scale(Head::ActionCandidate, scale)
            .unwrap();
        let source = inputs(configured(&genome, policy), true);
        let phenotype = compile(&source);
        let mut count = 0;
        for synapse in phenotype.synapses() {
            if matches!(synapse.kind(), CompiledSynapseKind::Decoder(c) if c.head() == Head::ActionCandidate)
            {
                count += 1;
                let projection = &phenotype.projections()[usize::from(synapse.route_index())];
                let bias = source
                    .development()
                    .open_critical_periods
                    .iter()
                    .filter(|p| {
                        p.lobe == projection.source_lobe() || p.lobe == projection.target_lobe()
                    })
                    .map(|p| p.plasticity_bias.raw())
                    .fold(0.0_f32, f32::max);
                let effective = (scale * (1.0 + bias)).clamp(0.0, 1.0);
                let receptor =
                    phenotype.plasticity_receptors()[usize::from(synapse.receptor_index())];
                let parameters = source.genome().plasticity_parameters();
                assert_eq!(
                    receptor.learning_rate().to_bits(),
                    (parameters.base_learning_rate() * effective).to_bits()
                );
                assert_eq!(
                    receptor.normalization_rate().to_bits(),
                    (parameters.normalization_rate() * effective).to_bits()
                );
                assert_eq!(
                    receptor.sleep_replay_rate().to_bits(),
                    (parameters.sleep_replay_rate() * effective).to_bits()
                );
            }
        }
        assert!(count > 0);
    }
}

#[test]
fn heterozygous_reproduction_keeps_credit_and_rate_policies_linked() {
    use alife_core::ActionCandidateCreditProfileV1 as Credit;
    let one = Policy::default()
        .with_action_candidate_family_scale(Family::Contact, Some(0.0))
        .unwrap();
    let two = Policy::default()
        .with_action_candidate_family_scale(Family::Approach, Some(0.25))
        .unwrap();
    let founder = |seed| {
        let mut genome = CreatureGenome::early_mammal_founder(
            seed,
            FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap();
        genome.brain.construction.maternal_learning = genome
            .brain
            .construction
            .maternal_learning
            .with_action_candidate_credit_profile(Credit::SignedConsequences)
            .unwrap()
            .with_decoder_lifetime_rate_policy(Some(one))
            .unwrap();
        genome.brain.construction.paternal_learning = genome
            .brain
            .construction
            .paternal_learning
            .with_action_candidate_credit_profile(Credit::SignedChoiceReadouts)
            .unwrap()
            .with_decoder_lifetime_rate_policy(Some(two))
            .unwrap();
        genome
    };
    let mother = founder(20);
    let father = founder(21);
    let mut seen = [false; 2];
    for seed in 22..38 {
        let child = CreatureGenome::reproduce(&mother, &father, seed).unwrap();
        let expression = child.express().unwrap();
        for parameters in [
            &child.brain.construction.maternal_learning,
            &child.brain.construction.paternal_learning,
            expression.brain_genome.plasticity_parameters(),
        ] {
            match parameters.action_candidate_credit_profile().unwrap() {
                Credit::SignedConsequences => {
                    assert_eq!(parameters.decoder_lifetime_rate_policy(), Some(one));
                    seen[0] = true;
                }
                Credit::SignedChoiceReadouts => {
                    assert_eq!(parameters.decoder_lifetime_rate_policy(), Some(two));
                    seen[1] = true;
                }
            }
        }
    }
    assert_eq!(seen, [true, true]);
}
