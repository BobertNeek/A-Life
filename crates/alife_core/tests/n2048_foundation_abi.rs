//! Contract acceptance for the frozen N2048 foundation and language ABI.

use std::collections::BTreeSet;

use alife_core::{
    AuxiliaryDecoderPlan, BrainCapacityClass, BrainGenome, BrainPhenotype, CanonicalDigestBuilder,
    CompiledSynapseKind, DecoderHeadKind, DevelopmentState, FoundationAbiBinding,
    LanguageCodebookV1, LanguageTokenClass, LanguageTokenId, LifetimePlasticityBand, LobeKind,
    N2048FoundationLayoutV1, NormalizedScalar, PersistentAddressMap, PhenotypeCompiler,
    PhenotypeCompilerInputs, SensorProfile, SpeechActKind, SpeechDecoderLayoutV1, Tick,
};

fn compile(class: BrainCapacityClass, seed: u64) -> BrainPhenotype {
    let genome = BrainGenome::scaffold(seed, class.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    PhenotypeCompiler::compile(
        &genome,
        &class,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap()
}

fn auxiliary_decoder_digest(
    head: DecoderHeadKind,
    input_width: u16,
    output_width: u16,
    start: u32,
    count: u32,
) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(b"alife.phenotype.auxiliary-decoder.v1");
    digest.write_u16(1);
    digest.write_u8(head.raw());
    digest.write_u16(input_width);
    digest.write_u16(output_width);
    digest.write_u32(start);
    digest.write_u32(count);
    digest.finish256()
}

#[test]
fn n2048_layout_routes_and_decoder_partitions_are_exact() {
    let expected_lobes = [
        (LobeKind::SensoryGrounding, 0, 256),
        (LobeKind::MetabolicDrive, 256, 128),
        (LobeKind::AuditorySpeech, 384, 128),
        (LobeKind::GlyphVision, 512, 128),
        (LobeKind::LexiconConcept, 640, 256),
        (LobeKind::CoreAssociation, 896, 448),
        (LobeKind::EpisodicMemory, 1_344, 256),
        (LobeKind::WorkingMemory, 1_600, 128),
        (LobeKind::MotorArbitration, 1_728, 224),
        (LobeKind::HomeostaticRegulation, 1_952, 96),
    ];
    let layout = N2048FoundationLayoutV1::lobe_layout();
    for (kind, start, len) in expected_lobes {
        let region = layout.region(kind).unwrap();
        assert_eq!(
            (region.start, region.len, region.enabled),
            (start, len, true)
        );
    }
    assert_eq!(layout.total_neurons(), 2_048);

    let expected_routes = [
        (LobeKind::SensoryGrounding, LobeKind::CoreAssociation, 3_584),
        (LobeKind::AuditorySpeech, LobeKind::CoreAssociation, 1_536),
        (LobeKind::GlyphVision, LobeKind::CoreAssociation, 1_536),
        (
            LobeKind::MetabolicDrive,
            LobeKind::HomeostaticRegulation,
            1_024,
        ),
        (
            LobeKind::HomeostaticRegulation,
            LobeKind::CoreAssociation,
            1_024,
        ),
        (
            LobeKind::HomeostaticRegulation,
            LobeKind::MotorArbitration,
            768,
        ),
        (LobeKind::CoreAssociation, LobeKind::MotorArbitration, 3_072),
        (
            LobeKind::MotorArbitration,
            LobeKind::MotorArbitration,
            1_536,
        ),
        (LobeKind::CoreAssociation, LobeKind::WorkingMemory, 1_536),
        (LobeKind::WorkingMemory, LobeKind::CoreAssociation, 1_536),
        (LobeKind::CoreAssociation, LobeKind::EpisodicMemory, 1_536),
        (LobeKind::EpisodicMemory, LobeKind::CoreAssociation, 1_536),
        (LobeKind::CoreAssociation, LobeKind::LexiconConcept, 1_536),
        (LobeKind::LexiconConcept, LobeKind::CoreAssociation, 1_536),
        (LobeKind::LexiconConcept, LobeKind::WorkingMemory, 768),
        (LobeKind::WorkingMemory, LobeKind::LexiconConcept, 512),
    ];
    let route_specs = N2048FoundationLayoutV1::route_specs();
    assert_eq!(route_specs.len(), expected_routes.len());
    assert_eq!(
        route_specs
            .iter()
            .map(|route| route.synapse_count())
            .sum::<u32>(),
        24_576,
    );
    let expected_policies = [
        (0, 3_584, 0),
        (0, 1_536, 0),
        (0, 1_536, 0),
        (0, 1_024, 0),
        (0, 1_024, 0),
        (0, 768, 0),
        (0, 2_048, 1_024),
        (0, 1_536, 0),
        (0, 0, 1_536),
        (0, 0, 1_536),
        (0, 0, 1_536),
        (0, 0, 1_536),
        (0, 0, 1_536),
        (0, 0, 1_536),
        (0, 0, 768),
        (0, 0, 512),
    ];
    for ((spec, expected), policy) in route_specs
        .iter()
        .zip(expected_routes)
        .zip(expected_policies)
    {
        assert_eq!(
            (spec.source_lobe(), spec.target_lobe(), spec.synapse_count()),
            expected,
        );
        assert_eq!(
            (
                spec.section_policy().count(LifetimePlasticityBand::Fixed),
                spec.section_policy().count(LifetimePlasticityBand::Slow),
                spec.section_policy().count(LifetimePlasticityBand::Fast),
            ),
            policy,
        );
        assert_eq!(spec.section_policy().total_synapses(), spec.synapse_count());
    }
    let core_motor = route_specs
        .iter()
        .find(|route| {
            route.source_lobe() == LobeKind::CoreAssociation
                && route.target_lobe() == LobeKind::MotorArbitration
        })
        .unwrap();
    assert!(
        core_motor
            .section_policy()
            .count(LifetimePlasticityBand::Slow)
            > 0
    );
    assert!(
        core_motor
            .section_policy()
            .count(LifetimePlasticityBand::Fast)
            > 0
    );

    let phenotype = compile(BrainCapacityClass::n2048(), 0x2048_F00D);
    assert_eq!(phenotype.lobe_layout(), &layout);
    assert_eq!(phenotype.projections().len(), 18);
    for (index, (spec, projection)) in route_specs
        .iter()
        .zip(phenotype.projections().iter())
        .enumerate()
    {
        assert_eq!(projection.route_index(), index as u16);
        assert_eq!(projection.source_lobe(), spec.source_lobe());
        assert_eq!(projection.target_lobe(), spec.target_lobe());
        assert_eq!(projection.synapse_range().1, spec.synapse_count());
    }
    assert_eq!(
        (
            phenotype.budgets().global.recurrent_synapses,
            phenotype.budgets().global.action_decoder_synapses,
            phenotype.budgets().global.memory_decoder_synapses,
            phenotype.budgets().global.total_synapses,
        ),
        (24_576, 4_096, 4_096, 32_768),
    );
    assert_eq!(phenotype.candidate_decoder().decoder_synapse_count(), 3_072);
    assert_eq!(
        phenotype.speech_decoder().unwrap().decoder_synapse_count(),
        1_024,
    );
    assert_eq!(
        phenotype.memory_decoder().unwrap().decoder_synapse_count(),
        4_096,
    );
}

#[test]
fn language_codebook_is_bounded_compositional_and_address_independent() {
    let codebook = LanguageCodebookV1::canonical();
    codebook.validate_contract().unwrap();
    assert_eq!(codebook.code_count(), 256);
    assert_eq!(codebook.max_heard_tokens(), 16);
    assert_eq!(codebook.max_generated_tokens(), 6);

    let expected_counts = [
        (LanguageTokenClass::SilenceUnknown, 1),
        (LanguageTokenClass::VerbAction, 24),
        (LanguageTokenClass::EcologicalNoun, 64),
        (LanguageTokenClass::DriveInternalState, 16),
        (LanguageTokenClass::ModifierSpatialRelation, 16),
        (LanguageTokenClass::GrammarQuerySocialOperator, 8),
        (LanguageTokenClass::LearnedAliasDialect, 64),
        (LanguageTokenClass::NameSocialBinding, 32),
        (LanguageTokenClass::ReservedExperimental, 31),
    ];
    for (class, expected) in expected_counts {
        let actual = (0_u16..256)
            .map(|raw| LanguageTokenId::new(raw).unwrap())
            .filter(|token| codebook.classify(*token) == class)
            .count();
        assert_eq!(actual, expected, "wrong count for {class:?}");
    }
    assert!(LanguageTokenId::new(256).is_err());
    assert!(serde_json::from_value::<LanguageTokenId>(serde_json::json!(256)).is_err());
    let symbols = (0_u16..256)
        .map(|raw| codebook.pronounceable_symbol(LanguageTokenId::new(raw).unwrap()))
        .collect::<BTreeSet<_>>();
    assert_eq!(symbols.len(), 256);
    assert!(symbols.iter().all(|symbol| {
        !symbol.is_empty() && symbol.bytes().all(|byte| byte.is_ascii_lowercase())
    }));

    for (raw, act) in [
        (0, SpeechActKind::Declare),
        (1, SpeechActKind::Request),
        (2, SpeechActKind::Respond),
        (3, SpeechActKind::QueryWhat),
        (4, SpeechActKind::QueryWhy),
        (5, SpeechActKind::ExpressState),
        (6, SpeechActKind::Acknowledge),
        (7, SpeechActKind::Refuse),
    ] {
        assert_eq!(act.raw(), raw);
        assert_eq!(SpeechActKind::try_from_raw(raw).unwrap(), act);
    }
    assert!(SpeechActKind::try_from_raw(8).is_err());
    assert_eq!(SpeechDecoderLayoutV1::INPUT_WIDTH, 32);
    assert_eq!(SpeechDecoderLayoutV1::OUTPUT_WIDTH, 32);
    assert_eq!(SpeechDecoderLayoutV1::SPEECH_ACT_COUNT, 8);
    assert_eq!(SpeechDecoderLayoutV1::TOKEN_BIT_COUNT, 8);
    assert_eq!(SpeechDecoderLayoutV1::RECURRENT_CONTROL_COUNT, 14);
    let candidate_motor_width = u32::from(N2048FoundationLayoutV1::CANDIDATE_FAMILY_COUNT)
        * u32::from(N2048FoundationLayoutV1::CANDIDATE_MOTOR_UNITS_PER_FAMILY);
    assert_eq!(
        SpeechDecoderLayoutV1::MOTOR_SOURCE_OFFSET,
        candidate_motor_width,
    );
    assert_eq!(
        SpeechDecoderLayoutV1::MOTOR_TARGET_OFFSET,
        SpeechDecoderLayoutV1::MOTOR_SOURCE_OFFSET + u32::from(SpeechDecoderLayoutV1::INPUT_WIDTH),
    );
    assert!(
        SpeechDecoderLayoutV1::MOTOR_TARGET_OFFSET + u32::from(SpeechDecoderLayoutV1::OUTPUT_WIDTH)
            <= 224,
    );
    assert_eq!(
        SpeechDecoderLayoutV1::speech_act_output(SpeechActKind::Refuse),
        7,
    );

    let wire = serde_json::to_value(&codebook).unwrap();
    let round_trip: LanguageCodebookV1 = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(round_trip, codebook);
    let text = serde_json::to_string(&wire).unwrap().to_ascii_lowercase();
    assert!(!text.contains("neuron"));
    assert!(!text.contains("ordinal"));
    assert!(!text.contains("gpu"));
    assert!(!text.contains("offset"));
}

#[test]
fn persistent_addresses_are_unique_packing_independent_and_blake3_bound() {
    let phenotype = compile(BrainCapacityClass::n2048(), 0xADD2_0480);
    let map = phenotype.persistent_address_map();
    map.validate_against(&phenotype).unwrap();
    assert_eq!(map.neurons().len(), 2_048);
    assert_eq!(map.projections().len(), phenotype.projections().len());
    assert_eq!(map.synapses().len(), phenotype.synapses().len());
    assert_eq!(
        map.decoders().len(),
        phenotype
            .synapses()
            .iter()
            .filter(|row| matches!(row.kind(), CompiledSynapseKind::Decoder(_)))
            .count(),
    );
    assert_eq!(map.digest(), map.recompute_digest().unwrap());

    let mut repacked_wire = serde_json::to_value(map).unwrap();
    let first = repacked_wire["neurons"][0]["packed_index"]
        .as_u64()
        .unwrap();
    let second = repacked_wire["neurons"][1]["packed_index"]
        .as_u64()
        .unwrap();
    repacked_wire["neurons"][0]["packed_index"] = serde_json::json!(second);
    repacked_wire["neurons"][1]["packed_index"] = serde_json::json!(first);
    let repacked: PersistentAddressMap = serde_json::from_value(repacked_wire).unwrap();
    assert_eq!(repacked.recompute_digest().unwrap(), map.digest());
    assert!(repacked.validate_against(&phenotype).is_err());

    let neuron_addresses = map
        .neurons()
        .iter()
        .map(|entry| entry.address())
        .collect::<BTreeSet<_>>();
    let projection_addresses = map
        .projections()
        .iter()
        .map(|entry| entry.address())
        .collect::<BTreeSet<_>>();
    let synapse_addresses = map
        .synapses()
        .iter()
        .map(|entry| entry.address())
        .collect::<BTreeSet<_>>();
    assert_eq!(neuron_addresses.len(), map.neurons().len());
    assert_eq!(projection_addresses.len(), map.projections().len());
    assert_eq!(synapse_addresses.len(), map.synapses().len());

    let other = compile(BrainCapacityClass::n2048(), 0xADD2_0481);
    assert_ne!(map.digest(), other.persistent_address_map().digest());
    assert_eq!(map.digest().algorithm(), "BLAKE3-256");
}

#[test]
fn auxiliary_decoder_plan_cannot_rebind_its_authenticated_head_to_another_range() {
    let phenotype = compile(BrainCapacityClass::n2048(), 0xA0D1_2048);
    let speech = phenotype.speech_decoder().unwrap();
    let forged_start = phenotype.memory_decoder().unwrap().decoder_synapse_start();
    let mut wire = serde_json::to_value(speech).unwrap();
    wire["decoder_synapse_start"] = serde_json::json!(forged_start);
    wire["canonical_digest"] = serde_json::to_value(auxiliary_decoder_digest(
        speech.head(),
        speech.input_width(),
        speech.output_width(),
        forged_start,
        speech.decoder_synapse_count(),
    ))
    .unwrap();

    let forged: AuxiliaryDecoderPlan = serde_json::from_value(wire).unwrap();
    assert!(forged.validate_against(&phenotype).is_err());
}

#[test]
fn foundation_and_language_mismatch_reject_before_phenotype_construction() {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0xAB10_2048, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let binding = FoundationAbiBinding::canonical_for_capacity(&capacity).unwrap();
    let inputs = PhenotypeCompilerInputs::try_new_with_foundation_abi(
        genome,
        &capacity,
        development,
        SensorProfile::PrivilegedAffordanceV1,
        binding,
    )
    .unwrap();
    let canonical = serde_json::to_value(&inputs).unwrap();

    for pointer in [
        "/foundation_abi/capacity_class_id",
        "/foundation_abi/layout_id",
        "/foundation_abi/layout_digest/0",
        "/foundation_abi/language_codebook/canonical_digest/0",
    ] {
        let mut forged = canonical.clone();
        let target = forged
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("missing test pointer {pointer}"));
        let value = target.as_u64().unwrap();
        *target = serde_json::json!(value ^ 1);
        assert!(serde_json::from_value::<PhenotypeCompilerInputs>(forged).is_err());
    }
}

#[test]
fn phenotype_identity_binds_language_address_route_and_plasticity_digests() {
    let phenotype = compile(BrainCapacityClass::n2048(), 0xD165_E57A);
    assert_ne!(phenotype.route_abi_digest().bytes(), &[0; 32]);
    assert_ne!(phenotype.plasticity_abi_digest().bytes(), &[0; 32]);
    assert_ne!(
        phenotype.persistent_address_map().digest().bytes(),
        &[0; 32]
    );
    assert_ne!(
        phenotype.language_codebook().canonical_digest().bytes(),
        &[0; 32]
    );

    let canonical = serde_json::to_value(&phenotype).unwrap();
    for pointer in [
        "/route_abi_digest/0",
        "/plasticity_abi_digest/0",
        "/persistent_address_map/digest/0",
        "/language_codebook/canonical_digest/0",
    ] {
        let mut forged = canonical.clone();
        let target = forged.pointer_mut(pointer).unwrap();
        let value = target.as_u64().unwrap();
        *target = serde_json::json!(value ^ 1);
        assert!(serde_json::from_value::<BrainPhenotype>(forged).is_err());
    }
}

#[test]
fn smaller_promoted_classes_retain_valid_compiled_phenotypes() {
    for capacity in [BrainCapacityClass::n512(), BrainCapacityClass::n1024()] {
        let phenotype = compile(capacity, 0x5CA1_AB1E);
        phenotype.validate_against(&capacity).unwrap();
        assert_eq!(phenotype.brain_class_id(), capacity.id());
        assert_eq!(
            phenotype.persistent_address_map().neurons().len(),
            capacity.execution().max_neurons() as usize,
        );
        assert_eq!(
            phenotype.language_codebook().id(),
            LanguageCodebookV1::canonical().id(),
        );
    }
}
