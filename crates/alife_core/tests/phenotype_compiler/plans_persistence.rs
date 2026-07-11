//! Test-only Task 3 compiler, encoder, decoder, and persistence contract ownership.

use super::*;

// Draft-only RED additions for crates/alife_core/tests/phenotype_compiler.rs.
// Append after the seed tests. This snippet intentionally relies on that file's
// existing imports and `compile` helper; it does not replace or edit the live test.

use alife_core::{
    BrainPhenotype, CandidateActionFamily, CandidateDecoderPlan, CompiledSynapse,
    CompiledSynapseKind, DecoderHeadKind, DriveSnapshot, EndocrineSnapshot, LobeKind,
    PhenotypeCompilerInputs, SensorChannelGene, SensorChannelKind, SensorEncoderPlan,
    SensorEncoderSourceGroup,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::ops::Range;

fn compile_genome(
    genome: &BrainGenome,
    development: &DevelopmentState,
    sensor_profile: SensorProfile,
) -> BrainPhenotype {
    let capacity = BrainCapacityClass::production_for_id(genome.brain_class_id).unwrap();
    PhenotypeCompiler::compile(genome, &capacity, development, sensor_profile).unwrap()
}

fn sensor_lane_spec(kind: SensorChannelKind) -> (SensorEncoderSourceGroup, Range<u16>) {
    match kind {
        SensorChannelKind::Vision | SensorChannelKind::GlyphVision => {
            (SensorEncoderSourceGroup::SensoryChannel, 0..16)
        }
        SensorChannelKind::Hearing => (SensorEncoderSourceGroup::SensoryChannel, 16..24),
        SensorChannelKind::Smell | SensorChannelKind::Taste => {
            (SensorEncoderSourceGroup::SensoryChannel, 24..32)
        }
        SensorChannelKind::Touch => (SensorEncoderSourceGroup::SensoryChannel, 32..40),
        SensorChannelKind::Proprioception => (SensorEncoderSourceGroup::Body, 0..13),
        SensorChannelKind::Interoception => (SensorEncoderSourceGroup::Homeostasis, 0..22),
    }
}

fn sensor_gene_is_active(gene: &SensorChannelGene, development: &DevelopmentState) -> bool {
    let passes_maturation =
        f32::from(gene.enabled_at_maturation) <= development.maturation.raw() * 100.0;
    let passes_allowlist = development.active_sensor_channels.is_empty()
        || development.active_sensor_channels.contains(&gene.kind);
    passes_maturation && passes_allowlist
}

fn assignment_count_for_gene(phenotype: &BrainPhenotype, gene: &SensorChannelGene) -> usize {
    let (source_group, source_range) = sensor_lane_spec(gene.kind);
    let region = phenotype
        .lobe_layout()
        .region(gene.target_lobe)
        .expect("an active sensor gene must name a compiled lobe");
    phenotype
        .sensor_encoder()
        .assignments()
        .iter()
        .filter(|assignment| {
            assignment.source_group() == source_group
                && source_range.contains(&assignment.source_index())
                && region.contains_neuron(assignment.target_neuron())
        })
        .count()
}

fn recurrent_coordinate(synapse: &CompiledSynapse) -> Option<(u16, u32, u32)> {
    matches!(synapse.kind(), CompiledSynapseKind::Recurrent).then_some((
        synapse.route_index(),
        synapse.source(),
        synapse.target(),
    ))
}

fn decoder_coordinate(synapse: &CompiledSynapse) -> Option<(u16, u8, u8, u16, u16, u32, u32)> {
    match synapse.kind() {
        CompiledSynapseKind::Recurrent => None,
        CompiledSynapseKind::Decoder(coordinate) => Some((
            synapse.route_index(),
            coordinate.head().raw(),
            coordinate.family().raw(),
            coordinate.input_lane(),
            coordinate.motor_index(),
            synapse.source(),
            synapse.target(),
        )),
    }
}

fn perturb_json_value(value: &mut Value) {
    match value {
        Value::Null => *value = Value::Bool(true),
        Value::Bool(boolean) => *boolean = !*boolean,
        Value::Number(number) => {
            if let Some(unsigned) = number.as_u64() {
                *value = Value::from(unsigned ^ 1);
            } else if let Some(signed) = number.as_i64() {
                *value = Value::from(signed ^ 1);
            } else {
                *value = Value::from(number.as_f64().unwrap() + 0.03125);
            }
        }
        Value::String(text) => text.push_str("-tampered"),
        Value::Array(values) => {
            assert!(!values.is_empty(), "tamper target array must be nonempty");
            perturb_json_value(&mut values[0]);
        }
        Value::Object(fields) => {
            let (_, nested) = fields
                .iter_mut()
                .next()
                .expect("tamper target object must be nonempty");
            perturb_json_value(nested);
        }
    }
}

fn tamper_at(mut serialized: Value, pointer: &str) -> Value {
    let value = serialized
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("missing serialized test path {pointer}"));
    perturb_json_value(value);
    serialized
}

fn decoder_wire_keys(decoder: &Value) -> BTreeSet<&str> {
    decoder
        .as_object()
        .expect("decoder serialization must be a JSON object")
        .keys()
        .map(String::as_str)
        .collect()
}

#[test]
fn identical_inputs_have_identical_serialized_phenotypes_and_seed_changes_weights() {
    for class_id in [
        BrainCapacityClass::N512_ID,
        BrainCapacityClass::N1024_ID,
        BrainCapacityClass::N2048_ID,
    ] {
        let first = compile(class_id, 0xA11F);
        let second = compile(class_id, 0xA11F);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap(),
            "class {class_id:?} must compile to byte-identical serialized evidence",
        );

        let other_seed = compile(class_id, 0xA120);
        let first_weights = first
            .synapses()
            .iter()
            .map(|synapse| {
                (
                    synapse.route_index(),
                    synapse.source(),
                    synapse.target(),
                    synapse.genetic_weight().to_bits(),
                )
            })
            .collect::<Vec<_>>();
        let other_weights = other_seed
            .synapses()
            .iter()
            .map(|synapse| {
                (
                    synapse.route_index(),
                    synapse.source(),
                    synapse.target(),
                    synapse.genetic_weight().to_bits(),
                )
            })
            .collect::<Vec<_>>();
        assert_ne!(
            first_weights, other_weights,
            "the genetic-prior seed must change compiled weight payloads",
        );
        assert_ne!(first.phenotype_hash(), other_seed.phenotype_hash());
    }
}

#[test]
fn recurrent_and_decoder_coordinates_are_sorted_unique_and_in_range() {
    let phenotype = compile(BrainCapacityClass::N512_ID, 0xC001);
    let decoder = phenotype.candidate_decoder();

    let recurrent = phenotype
        .synapses()
        .iter()
        .filter_map(recurrent_coordinate)
        .collect::<Vec<_>>();
    assert!(!recurrent.is_empty());
    assert!(
        recurrent.windows(2).all(|pair| pair[0] < pair[1]),
        "recurrent coordinates must be strictly sorted and unique",
    );
    for &(_, source, target) in &recurrent {
        assert!(source < phenotype.neuron_count());
        assert!(target < phenotype.neuron_count());
    }

    let decoder_coordinates = phenotype
        .synapses()
        .iter()
        .filter_map(decoder_coordinate)
        .collect::<Vec<_>>();
    assert!(!decoder_coordinates.is_empty());
    assert!(
        decoder_coordinates.windows(2).all(|pair| pair[0] < pair[1]),
        "decoder coordinates must be strictly sorted and unique",
    );
    for &(_, head, family, input_lane, motor_index, source, target) in &decoder_coordinates {
        assert_eq!(head, DecoderHeadKind::ActionCandidate.raw());
        assert!(family < 8);
        assert!(input_lane < decoder.flattened_input_lane_count());
        assert!(motor_index < decoder.motor_width());
        assert!(source < phenotype.neuron_count());
        assert!(target < phenotype.neuron_count());
        assert_eq!(target, decoder.motor_start() + u32::from(motor_index));
    }
}

#[test]
fn projections_routes_and_global_budgets_are_exact_partitions() {
    let phenotype = compile(BrainCapacityClass::N512_ID, 0x00B0_D6E7);
    let capacity = BrainCapacityClass::n512();
    let budgets = phenotype.budgets();
    budgets.validate_against(&capacity).unwrap();

    assert_eq!(phenotype.projections().len(), budgets.routes.len());
    let mut cursor = 0_u32;
    for (expected_route_index, (projection, receipt)) in phenotype
        .projections()
        .iter()
        .zip(&budgets.routes)
        .enumerate()
    {
        let expected_route_index = u16::try_from(expected_route_index).unwrap();
        assert_eq!(projection.route_index(), expected_route_index);
        assert_eq!(receipt.route_index, expected_route_index);

        let (start, len) = projection.synapse_range();
        assert_eq!(start, cursor, "projection spans must be gap-free");
        let end = start.checked_add(len).unwrap();
        let slice = &phenotype.synapses()[start as usize..end as usize];
        assert!(
            slice
                .iter()
                .all(|synapse| synapse.route_index() == expected_route_index),
            "a projection span may contain only its own route",
        );
        let source_region = phenotype
            .lobe_layout()
            .region(projection.source_lobe())
            .unwrap();
        let target_region = phenotype
            .lobe_layout()
            .region(projection.target_lobe())
            .unwrap();
        for synapse in slice {
            assert!(source_region.contains_neuron(synapse.source()));
            assert!(target_region.contains_neuron(synapse.target()));
            match projection.projection_type() {
                alife_core::ProjectionType::LateralInhibition => {
                    assert!(synapse.genetic_weight() < 0.0)
                }
                alife_core::ProjectionType::Homeostatic
                | alife_core::ProjectionType::MotorProposal => {
                    assert!(synapse.genetic_weight() >= 0.0)
                }
                _ => {}
            }
        }

        let mut recurrent = 0_u32;
        let mut action_decoder = 0_u32;
        let mut memory_decoder = 0_u32;
        for synapse in slice {
            match synapse.kind() {
                CompiledSynapseKind::Recurrent => recurrent += 1,
                CompiledSynapseKind::Decoder(coordinate) => match coordinate.head() {
                    DecoderHeadKind::ActionCandidate => action_decoder += 1,
                    DecoderHeadKind::MemoryContext => memory_decoder += 1,
                },
            }
        }
        assert_eq!(receipt.recurrent_synapses, recurrent);
        assert_eq!(receipt.action_decoder_synapses, action_decoder);
        assert_eq!(receipt.memory_decoder_synapses, memory_decoder);
        assert_eq!(
            len,
            recurrent
                .checked_add(action_decoder)
                .and_then(|value| value.checked_add(memory_decoder))
                .unwrap(),
        );
        assert_eq!(receipt.active_tiles, projection.active_tile_count());
        cursor = end;
    }

    let global = &budgets.global;
    assert_eq!(cursor as usize, phenotype.synapses().len());
    assert_eq!(global.neuron_count, phenotype.neuron_count());
    assert_eq!(global.total_synapses as usize, phenotype.synapses().len());
    assert_eq!(
        global.total_synapses,
        global
            .recurrent_synapses
            .checked_add(global.action_decoder_synapses)
            .and_then(|value| value.checked_add(global.memory_decoder_synapses))
            .unwrap(),
    );
    assert_eq!(
        budgets
            .routes
            .iter()
            .map(|route| route.recurrent_synapses)
            .sum::<u32>(),
        global.recurrent_synapses,
    );
    assert_eq!(
        budgets
            .routes
            .iter()
            .map(|route| route.action_decoder_synapses)
            .sum::<u32>(),
        global.action_decoder_synapses,
    );
    assert_eq!(
        budgets
            .routes
            .iter()
            .map(|route| route.memory_decoder_synapses)
            .sum::<u32>(),
        global.memory_decoder_synapses,
    );
    assert_eq!(
        budgets
            .routes
            .iter()
            .map(|route| route.active_tiles)
            .sum::<u32>(),
        global.active_tiles,
    );
    assert_eq!(
        budgets
            .routes
            .iter()
            .map(|route| route.immutable_payload_words)
            .sum::<u32>(),
        global.immutable_payload_words,
    );
}

#[test]
fn encoder_has_exact_abi_widths_sorted_assignments_and_every_active_sensor() {
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(0x0E11_C0DE, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let phenotype = compile_genome(&genome, &development, SensorProfile::PrivilegedAffordanceV1);
    let encoder = phenotype.sensor_encoder();
    encoder.validate_against(&phenotype).unwrap();

    assert_eq!(encoder.sensory_lane_count(), 42);
    assert_eq!(encoder.body_lane_count(), 13);
    assert_eq!(encoder.homeostasis_lane_count(), 22);
    assert_eq!(
        encoder.homeostasis_lane_count(),
        (DriveSnapshot::CHANNEL_COUNT + EndocrineSnapshot::CHANNEL_COUNT) as u16,
    );

    let assignment_keys = encoder
        .assignments()
        .iter()
        .map(|assignment| {
            (
                assignment.target_neuron(),
                assignment.source_group().raw(),
                assignment.source_index(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        assignment_keys.windows(2).all(|pair| pair[0] < pair[1]),
        "encoder assignments must be strictly sorted and duplicate-free",
    );

    let active_genes = genome
        .sensor_layout
        .channels
        .iter()
        .filter(|gene| sensor_gene_is_active(gene, &development))
        .collect::<Vec<_>>();
    let active_input_lobes = active_genes
        .iter()
        .map(|gene| gene.target_lobe)
        .collect::<Vec<_>>();
    for assignment in encoder.assignments() {
        let width = match assignment.source_group() {
            SensorEncoderSourceGroup::SensoryChannel => encoder.sensory_lane_count(),
            SensorEncoderSourceGroup::Body => encoder.body_lane_count(),
            SensorEncoderSourceGroup::Homeostasis => encoder.homeostasis_lane_count(),
        };
        assert!(assignment.source_index() < width);
        let target_lobe = phenotype
            .lobe_layout()
            .lobe_by_neuron_index(assignment.target_neuron())
            .expect("every encoder target must name an enabled compiled neuron");
        assert!(target_lobe.enabled);
        assert!(
            active_input_lobes.contains(&target_lobe.kind),
            "encoder target must lie in an active sensor input lobe",
        );
    }
    for gene in active_genes {
        assert_eq!(
            assignment_count_for_gene(&phenotype, gene),
            usize::from(gene.receptor_count),
            "active sensor {:?} must compile every requested receptor",
            gene.kind,
        );
    }
}

#[test]
fn sensor_gene_mutation_and_maturation_gate_change_the_real_encoder_deterministically() {
    let capacity = BrainCapacityClass::n512();
    let baseline_genome = BrainGenome::scaffold(0x005E_450A, capacity.id());
    let at_gate = DevelopmentState::new(
        baseline_genome.id,
        Tick::ZERO,
        NormalizedScalar::new(0.50).unwrap(),
    );
    let baseline = compile_genome(
        &baseline_genome,
        &at_gate,
        SensorProfile::PrivilegedAffordanceV1,
    );

    let mut mutated_genome = baseline_genome.clone();
    let hearing_gene = SensorChannelGene {
        kind: SensorChannelKind::Hearing,
        receptor_count: 8,
        target_lobe: LobeKind::AuditorySpeech,
        enabled_at_maturation: 50,
    };
    mutated_genome.sensor_layout.channels.push(hearing_gene);

    let before_gate = DevelopmentState::new(
        mutated_genome.id,
        Tick::ZERO,
        NormalizedScalar::new(0.49).unwrap(),
    );
    let at_gate = DevelopmentState::new(
        mutated_genome.id,
        Tick::ZERO,
        NormalizedScalar::new(0.50).unwrap(),
    );
    let before = compile_genome(
        &mutated_genome,
        &before_gate,
        SensorProfile::PrivilegedAffordanceV1,
    );
    let enabled = compile_genome(
        &mutated_genome,
        &at_gate,
        SensorProfile::PrivilegedAffordanceV1,
    );
    let enabled_again = compile_genome(
        &mutated_genome,
        &at_gate,
        SensorProfile::PrivilegedAffordanceV1,
    );

    assert_eq!(assignment_count_for_gene(&before, &hearing_gene), 0);
    assert_eq!(assignment_count_for_gene(&enabled, &hearing_gene), 8);
    assert_eq!(
        enabled.sensor_encoder().assignments().len(),
        baseline.sensor_encoder().assignments().len() + 8,
    );
    assert_ne!(
        baseline.sensor_encoder().canonical_digest(),
        enabled.sensor_encoder().canonical_digest(),
    );
    assert_ne!(
        before.sensor_encoder().canonical_digest(),
        enabled.sensor_encoder().canonical_digest(),
    );
    assert_eq!(
        serde_json::to_vec(enabled.sensor_encoder()).unwrap(),
        serde_json::to_vec(enabled_again.sensor_encoder()).unwrap(),
    );
}

#[test]
fn decoder_covers_all_families_in_raw_order_with_exact_spans_and_coordinates() {
    let phenotype = compile(BrainCapacityClass::N512_ID, 0xDEC0_DE01);
    let decoder = phenotype.candidate_decoder();
    decoder.validate_against(&phenotype).unwrap();

    let expected_families = [
        CandidateActionFamily::Idle,
        CandidateActionFamily::Rest,
        CandidateActionFamily::Inspect,
        CandidateActionFamily::Approach,
        CandidateActionFamily::Avoid,
        CandidateActionFamily::Contact,
        CandidateActionFamily::Ingest,
        CandidateActionFamily::Other,
    ];
    assert_eq!(decoder.feature_count(), 24);
    assert_eq!(decoder.flattened_input_lane_count(), 24);
    assert_eq!(decoder.families().len(), expected_families.len());
    assert_eq!(
        decoder
            .families()
            .iter()
            .map(|family| family.family())
            .collect::<Vec<_>>(),
        expected_families,
    );
    for (raw, family) in expected_families.into_iter().enumerate() {
        let raw = u8::try_from(raw).unwrap();
        assert_eq!(family.raw(), raw);
        assert_eq!(CandidateActionFamily::try_from_raw(raw).unwrap(), family);
    }

    let recurrent_end = phenotype.budgets().global.recurrent_synapses;
    let action_end = recurrent_end
        .checked_add(phenotype.budgets().global.action_decoder_synapses)
        .unwrap();
    let decoder_projection = phenotype.projections().last().unwrap();
    assert_eq!(decoder_projection.source_lobe(), LobeKind::MotorArbitration);
    assert_eq!(decoder_projection.target_lobe(), LobeKind::MotorArbitration);
    assert_eq!(
        decoder_projection.projection_type(),
        alife_core::ProjectionType::MotorProposal,
    );
    assert_eq!(
        decoder_projection.synapse_range(),
        (recurrent_end, action_end - recurrent_end)
    );

    let mut cursor = recurrent_end;
    let mut seen_coordinates = BTreeSet::new();
    for family_plan in decoder.families() {
        assert_eq!(family_plan.bias().to_bits(), 0.0_f32.to_bits());
        assert_eq!(family_plan.decoder_synapse_start(), cursor);
        let end = cursor
            .checked_add(family_plan.decoder_synapse_count())
            .unwrap();
        for (global_index, synapse) in phenotype.synapses()[cursor as usize..end as usize]
            .iter()
            .enumerate()
        {
            let global_index = cursor + u32::try_from(global_index).unwrap();
            let coordinate = match synapse.kind() {
                CompiledSynapseKind::Recurrent => {
                    panic!("decoder family span {global_index} contains a recurrent synapse")
                }
                CompiledSynapseKind::Decoder(coordinate) => coordinate,
            };
            assert_eq!(coordinate.head(), DecoderHeadKind::ActionCandidate);
            assert_eq!(coordinate.family(), family_plan.family());
            assert!(coordinate.input_lane() < decoder.flattened_input_lane_count());
            assert!(coordinate.motor_index() < decoder.motor_width());
            assert_eq!(
                synapse.source(),
                decoder.motor_start() + u32::from(coordinate.motor_index()),
                "decoder coordinate and compiled synapse source must agree",
            );
            assert_eq!(
                synapse.target(),
                decoder.motor_start() + u32::from(coordinate.motor_index()),
                "decoder coordinate and compiled synapse target must agree",
            );
            assert!(seen_coordinates.insert((
                coordinate.head().raw(),
                coordinate.family().raw(),
                coordinate.input_lane(),
                coordinate.motor_index(),
            )));
        }
        cursor = end;
    }

    assert_eq!(cursor, action_end);
    assert_eq!(action_end, phenotype.budgets().global.total_synapses);
    let action_decoder_synapses = phenotype
        .synapses()
        .iter()
        .filter(|synapse| {
            matches!(
                synapse.kind(),
                CompiledSynapseKind::Decoder(coordinate)
                    if coordinate.head() == DecoderHeadKind::ActionCandidate
            )
        })
        .count();
    assert_eq!(
        action_decoder_synapses,
        phenotype.budgets().global.action_decoder_synapses as usize,
    );
    assert_eq!(
        decoder.decoder_synapse_count(),
        phenotype.budgets().global.action_decoder_synapses,
    );
    assert_eq!(phenotype.budgets().global.memory_decoder_synapses, 0);
    assert!(phenotype.synapses().iter().all(|synapse| !matches!(
        synapse.kind(),
        CompiledSynapseKind::Decoder(coordinate)
            if coordinate.head() == DecoderHeadKind::MemoryContext
    )));
}

#[test]
fn decoder_serialized_identity_contains_no_raw_entity_id_lane() {
    let phenotype = compile(BrainCapacityClass::N512_ID, 0xE171_7A55);
    let decoder = serde_json::to_value(phenotype.candidate_decoder()).unwrap();
    assert_eq!(
        decoder_wire_keys(&decoder),
        BTreeSet::from([
            "schema_version",
            "motor_start",
            "motor_width",
            "feature_count",
            "flattened_input_lane_count",
            "families",
            "canonical_digest",
        ]),
    );
    let family_rows = decoder["families"].as_array().unwrap();
    assert_eq!(family_rows.len(), 8);
    for row in family_rows {
        assert_eq!(
            decoder_wire_keys(row),
            BTreeSet::from([
                "family",
                "bias",
                "decoder_synapse_start",
                "decoder_synapse_count",
            ]),
        );
    }

    let mut decoder_coordinate_count = 0_usize;
    for synapse in phenotype.synapses() {
        if let CompiledSynapseKind::Decoder(_) = synapse.kind() {
            let serialized_kind = serde_json::to_value(synapse.kind()).unwrap();
            let coordinate = serialized_kind
                .get("Decoder")
                .expect("decoder synapse kind must serialize its exact coordinate");
            assert_eq!(
                decoder_wire_keys(coordinate),
                BTreeSet::from(["head", "family", "input_lane", "motor_index"]),
            );
            decoder_coordinate_count += 1;
        }
    }
    assert_eq!(
        decoder_coordinate_count,
        phenotype.budgets().global.action_decoder_synapses as usize,
    );
}

#[test]
fn phenotype_rejects_scalar_and_nested_collection_tamper_with_the_old_hash() {
    let phenotype = compile(BrainCapacityClass::N512_ID, 0x7A6E_E001);
    let serialized = serde_json::to_value(&phenotype).unwrap();
    serde_json::from_value::<BrainPhenotype>(serialized.clone()).unwrap();

    for (label, pointer) in [
        ("top-level scalar", "/microstep_count"),
        ("lobe region", "/lobe_layout/regions/0/len"),
        ("projection span", "/projections/0/synapse_len"),
        ("synapse payload", "/synapses/0/genetic_weight"),
        ("neuron dynamics", "/neuron_dynamics/0/leak"),
        ("route receipt", "/budgets/routes/0/recurrent_synapses"),
        ("global receipt", "/budgets/global/total_synapses"),
        ("encoder assignment", "/sensor_encoder/assignments/0/scale"),
        ("decoder family", "/decoder/families/0/bias"),
    ] {
        let tampered = tamper_at(serialized.clone(), pointer);
        assert!(
            serde_json::from_value::<BrainPhenotype>(tampered).is_err(),
            "phenotype accepted {label} tamper at {pointer} with a stale hash",
        );
    }
}

#[test]
fn encoder_and_decoder_reject_stale_or_tampered_nested_digests() {
    let phenotype = compile(BrainCapacityClass::N512_ID, 0x7A6E_E002);

    let encoder = serde_json::to_value(phenotype.sensor_encoder()).unwrap();
    serde_json::from_value::<SensorEncoderPlan>(encoder.clone()).unwrap();
    for pointer in ["/canonical_digest/0", "/assignments/0/scale"] {
        assert!(
            serde_json::from_value::<SensorEncoderPlan>(tamper_at(encoder.clone(), pointer))
                .is_err(),
            "encoder accepted tamper at {pointer} with its old digest",
        );
    }

    let decoder = serde_json::to_value(phenotype.candidate_decoder()).unwrap();
    serde_json::from_value::<CandidateDecoderPlan>(decoder.clone()).unwrap();
    for pointer in ["/canonical_digest/0", "/families/0/bias"] {
        assert!(
            serde_json::from_value::<CandidateDecoderPlan>(tamper_at(decoder.clone(), pointer))
                .is_err(),
            "decoder accepted tamper at {pointer} with its old digest",
        );
    }
}

#[test]
fn compiler_inputs_reject_genome_development_and_capacity_digest_tamper() {
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(0x7A6E_E003, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let inputs = PhenotypeCompilerInputs::try_new(
        genome,
        &capacity,
        development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    let serialized = serde_json::to_value(&inputs).unwrap();
    serde_json::from_value::<PhenotypeCompilerInputs>(serialized.clone()).unwrap();

    for (label, pointer) in [
        (
            "genome sensor manifest",
            "/genome/sensor_layout/channels/0/receptor_count",
        ),
        ("development maturation", "/development/maturation"),
        ("capacity ABI digest", "/capacity_digest/0"),
        ("compiler-input digest", "/canonical_digest/0"),
    ] {
        assert!(
            serde_json::from_value::<PhenotypeCompilerInputs>(tamper_at(
                serialized.clone(),
                pointer,
            ))
            .is_err(),
            "compiler inputs accepted {label} tamper at {pointer}",
        );
    }
}

#[test]
fn persistence_rejects_unknown_schema_versions_and_unknown_hashes() {
    let phenotype = compile(BrainCapacityClass::N512_ID, 0x7A6E_E004);
    let serialized_phenotype = serde_json::to_value(&phenotype).unwrap();
    for (label, pointer) in [
        ("phenotype schema", "/schema_version"),
        ("encoder schema", "/sensor_encoder/schema_version"),
        ("decoder schema", "/decoder/schema_version"),
        ("phenotype hash", "/phenotype_hash/0"),
    ] {
        assert!(
            serde_json::from_value::<BrainPhenotype>(tamper_at(
                serialized_phenotype.clone(),
                pointer,
            ))
            .is_err(),
            "persistence accepted unknown {label}",
        );
    }

    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(0x7A6E_E004, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let inputs = PhenotypeCompilerInputs::try_new(
        genome,
        &capacity,
        development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    let serialized_inputs = serde_json::to_value(&inputs).unwrap();
    for (label, pointer) in [
        ("compiler-input schema", "/schema_version"),
        ("compiler-input hash", "/canonical_digest/0"),
    ] {
        assert!(
            serde_json::from_value::<PhenotypeCompilerInputs>(tamper_at(
                serialized_inputs.clone(),
                pointer,
            ))
            .is_err(),
            "persistence accepted unknown {label}",
        );
    }
}

#[test]
fn recurrent_tile_receipts_equal_independently_recomputed_touched_tiles() {
    let phenotype = compile(BrainCapacityClass::N512_ID, 0x711E_0001);
    let mut global_total = 0_u32;
    for (projection, receipt) in phenotype
        .projections()
        .iter()
        .zip(&phenotype.budgets().routes)
    {
        let (start, len) = projection.synapse_range();
        let touched = phenotype.synapses()[start as usize..(start + len) as usize]
            .iter()
            .filter(|synapse| matches!(synapse.kind(), CompiledSynapseKind::Recurrent))
            .map(|synapse| (synapse.source() / 16, synapse.target() / 16))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            u32::try_from(touched.len()).unwrap(),
            projection.active_tile_count()
        );
        assert_eq!(projection.active_tile_count(), receipt.active_tiles);
        global_total += u32::try_from(touched.len()).unwrap();
    }
    assert_eq!(global_total, phenotype.budgets().global.active_tiles);
    assert!(global_total <= BrainCapacityClass::n512().execution().max_active_tiles());
}

#[test]
fn shared_sensor_lane_ranges_allocate_unique_vision_and_glyph_coordinates() {
    let capacity = BrainCapacityClass::n512();
    let mut genome = BrainGenome::scaffold(0x5E45_1001, capacity.id());
    genome.sensor_layout.channels.push(SensorChannelGene {
        kind: SensorChannelKind::GlyphVision,
        receptor_count: 16,
        target_lobe: LobeKind::SensoryGrounding,
        enabled_at_maturation: 0,
    });
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let phenotype = compile_genome(&genome, &development, SensorProfile::PrivilegedAffordanceV1);
    assert_eq!(phenotype.sensor_encoder().assignments().len(), 120);
    let keys = phenotype
        .sensor_encoder()
        .assignments()
        .iter()
        .map(|assignment| {
            (
                assignment.target_neuron(),
                assignment.source_group().raw(),
                assignment.source_index(),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(keys.len(), phenotype.sensor_encoder().assignments().len());
}

#[test]
fn shared_sensor_lane_ranges_allocate_unique_smell_and_taste_coordinates() {
    let capacity = BrainCapacityClass::n512();
    let mut genome = BrainGenome::scaffold(0x5E45_1002, capacity.id());
    for kind in [SensorChannelKind::Smell, SensorChannelKind::Taste] {
        genome.sensor_layout.channels.push(SensorChannelGene {
            kind,
            receptor_count: 8,
            target_lobe: LobeKind::SensoryGrounding,
            enabled_at_maturation: 0,
        });
    }
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let phenotype = compile_genome(&genome, &development, SensorProfile::PrivilegedAffordanceV1);
    let chemistry = phenotype
        .sensor_encoder()
        .assignments()
        .iter()
        .filter(|assignment| {
            assignment.source_group() == SensorEncoderSourceGroup::SensoryChannel
                && (24..32).contains(&assignment.source_index())
        })
        .map(|assignment| (assignment.target_neuron(), assignment.source_index()))
        .collect::<BTreeSet<_>>();
    assert_eq!(chemistry.len(), 16);
}
