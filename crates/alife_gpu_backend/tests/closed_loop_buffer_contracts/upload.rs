use std::collections::BTreeSet;

use alife_core::{BrainCapacityClass, CandidateActionFamily, CompiledSynapseKind, PhenotypeHash};
use alife_gpu_backend::{GpuPhenotypeUpload, GpuProjectionSpanDomain, GPU_NO_EXTENSION_SENTINEL};

use super::support::{production_phenotypes, recurrent_count};

#[test]
fn promoted_classes_have_exact_phenotype_owned_count_and_byte_plans() {
    for phenotype in production_phenotypes() {
        let upload = GpuPhenotypeUpload::try_from(&phenotype).unwrap();
        let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id()).unwrap();
        let recurrent = recurrent_count(&phenotype);
        let candidate_decoder = phenotype.candidate_decoder().decoder_synapse_count() as usize;

        assert_eq!(upload.class_id, phenotype.brain_class_id().raw() as u32);
        assert_eq!(upload.neuron_count, phenotype.neuron_count());
        assert_eq!(
            upload.microstep_count,
            u32::from(phenotype.microstep_count())
        );
        assert_eq!(
            upload.gpu_layout_version,
            capacity.execution().gpu_layout_version()
        );
        assert_eq!(upload.encoder_plans.len(), 1);
        assert_eq!(
            upload.encoder_assignments.len(),
            phenotype.sensor_encoder().assignments().len()
        );
        assert_eq!(
            upload.encoder_target_offsets.len(),
            phenotype.neuron_count() as usize + 1
        );
        assert_eq!(
            upload.neuron_dynamics.len(),
            phenotype.neuron_count() as usize
        );
        assert_eq!(upload.projections.len(), phenotype.projections().len());
        assert_eq!(upload.route_metadata.len(), phenotype.projections().len());
        assert_eq!(
            upload.target_offsets.len(),
            phenotype.neuron_count() as usize + 1
        );
        assert_eq!(upload.source_indices.len(), recurrent);
        assert_eq!(upload.route_indices.len(), recurrent);
        assert_eq!(upload.decoder_plans.len(), 1);
        assert_eq!(
            upload.decoder_families.len(),
            phenotype.candidate_decoder().families().len()
        );
        assert_eq!(upload.decoder_weight_indices.len(), candidate_decoder);
        assert_eq!(upload.genetic_weights.len(), phenotype.synapses().len());
        assert_eq!(upload.alpha.len(), phenotype.synapses().len());

        let counts = upload.exact_count_plan();
        let bytes = upload.exact_byte_plan();
        assert_eq!(counts.neurons, phenotype.neuron_count() as usize);
        assert_eq!(counts.synapses, phenotype.synapses().len());
        assert_eq!(counts.recurrent_synapses, recurrent);
        assert_eq!(counts.candidate_decoder_synapses, candidate_decoder);
        assert_eq!(bytes.immutable_weight_bytes, phenotype.synapses().len() * 8);
        assert_eq!(bytes.mutable_weight_bytes, phenotype.synapses().len() * 24);
        assert_eq!(
            bytes.activation_bytes,
            phenotype.neuron_count() as usize * 3 * 4
        );
        assert_eq!(
            bytes.homeostasis_bytes,
            phenotype.neuron_count() as usize * 2 * 4
        );

        match phenotype.brain_class_id() {
            BrainCapacityClass::N512_ID => assert_eq!(counts.neurons, 512),
            BrainCapacityClass::N1024_ID => assert_eq!(counts.neurons, 1024),
            BrainCapacityClass::N2048_ID => {
                assert_eq!(counts.neurons, 2_048);
                assert_eq!(counts.synapses, 32_768);
                assert_eq!(counts.recurrent_synapses, 24_576);
                assert_eq!(counts.candidate_decoder_synapses, 3_072);
                assert_eq!(upload.decoder_weight_indices.len(), 3_072);
            }
            other => panic!("unexpected production class {other:?}"),
        }
    }
}

#[test]
fn replay_capture_gpu_local_ids_preserve_the_bounded_batch_ordering_contract() {
    for phenotype in production_phenotypes() {
        let upload = GpuPhenotypeUpload::try_from(&phenotype).unwrap();
        assert!(
            upload
                .replay_capture_local_synapse_ids
                .windows(2)
                .all(|pair| pair[0] < pair[1]),
            "class {:?} translated replay capture IDs out of GPU-local order: {:?}",
            phenotype.brain_class_id(),
            upload.replay_capture_local_synapse_ids,
        );
    }
}

#[test]
fn upload_is_an_exact_projection_encoder_dynamics_and_decoder_translation() {
    let phenotype = production_phenotypes().into_iter().next().unwrap();
    let upload = GpuPhenotypeUpload::try_from(&phenotype).unwrap();
    assert_eq!(
        upload.projection_span_domain(),
        GpuProjectionSpanDomain::A3ProvenanceOnly
    );

    let encoder = &upload.encoder_plans[0];
    assert_eq!(
        encoder.schema_version,
        phenotype.sensor_encoder().schema_version() as u32
    );
    assert_eq!(
        encoder.sensor_profile_raw,
        phenotype.sensor_profile().raw() as u32
    );
    assert_eq!(
        encoder.assignment_count as usize,
        upload.encoder_assignments.len()
    );
    assert_eq!(
        encoder.sensory_lane_count,
        u32::from(phenotype.sensor_encoder().sensory_lane_count())
    );
    assert_eq!(
        encoder.body_lane_count,
        u32::from(phenotype.sensor_encoder().body_lane_count())
    );
    assert_eq!(
        encoder.homeostasis_lane_count,
        u32::from(phenotype.sensor_encoder().homeostasis_lane_count())
    );
    assert_eq!(
        *upload.encoder_target_offsets.last().unwrap(),
        upload.encoder_assignments.len() as u32
    );
    let mut expected_encoder_offsets = vec![0_u32; phenotype.neuron_count() as usize + 1];
    for assignment in phenotype.sensor_encoder().assignments() {
        expected_encoder_offsets[assignment.target_neuron() as usize + 1] += 1;
    }
    for index in 1..expected_encoder_offsets.len() {
        expected_encoder_offsets[index] += expected_encoder_offsets[index - 1];
    }
    assert_eq!(upload.encoder_target_offsets, expected_encoder_offsets);

    for (gpu, compiled) in upload
        .encoder_assignments
        .iter()
        .zip(phenotype.sensor_encoder().assignments())
    {
        let (clamp_min, clamp_max) = compiled.clamp_range();
        assert_eq!(gpu.source_group_raw, compiled.source_group().raw() as u32);
        assert_eq!(gpu.source_index, u32::from(compiled.source_index()));
        assert_eq!(gpu.target_neuron, compiled.target_neuron());
        assert_eq!(gpu.reserved0, 0);
        assert_eq!(gpu.scale_bits, compiled.scale().to_bits());
        assert_eq!(gpu.bias_bits, compiled.bias().to_bits());
        assert_eq!(gpu.clamp_min_bits, clamp_min.to_bits());
        assert_eq!(gpu.clamp_max_bits, clamp_max.to_bits());
    }

    for (gpu, compiled) in upload
        .neuron_dynamics
        .iter()
        .zip(phenotype.neuron_dynamics())
    {
        assert_eq!(gpu.bias_bits, compiled.bias().to_bits());
        assert_eq!(gpu.leak_bits, compiled.leak().to_bits());
        assert_eq!(gpu.activation_raw, compiled.activation().raw() as u32);
        assert_eq!(
            gpu.homeostatic_gain_bits,
            compiled.homeostatic_gain().to_bits()
        );
        assert_eq!(
            gpu.activity_ema_decay_bits,
            compiled.activity_ema_decay().to_bits()
        );
        assert_eq!(
            gpu.metabolic_decay_bits,
            compiled.metabolic_decay().to_bits()
        );
        assert_eq!((gpu.reserved0, gpu.reserved1), (0, 0));
    }

    for ((projection, route), compiled) in upload
        .projections
        .iter()
        .zip(&upload.route_metadata)
        .zip(phenotype.projections())
    {
        let (synapse_start, synapse_count) = compiled.synapse_range();
        let source = phenotype
            .lobe_layout()
            .region(compiled.source_lobe())
            .unwrap();
        let target = phenotype
            .lobe_layout()
            .region(compiled.target_lobe())
            .unwrap();
        assert_eq!(projection.route_index, u32::from(compiled.route_index()));
        assert_eq!(
            projection.source_lobe_raw,
            compiled.source_lobe().raw() as u32
        );
        assert_eq!(
            projection.target_lobe_raw,
            compiled.target_lobe().raw() as u32
        );
        assert_eq!(
            (projection.synapse_start, projection.synapse_count),
            (synapse_start, synapse_count)
        );
        assert_eq!(projection.active_tile_count, compiled.active_tile_count());
        assert_eq!((projection.reserved0, projection.reserved1), (0, 0));
        assert_eq!(route.route_index, u32::from(compiled.route_index()));
        assert_eq!(
            route.projection_type_raw,
            compiled.projection_type().raw() as u32
        );
        assert_eq!(
            route.active_tile_policy_raw,
            compiled.active_tile_policy().raw() as u32
        );
        assert_eq!(
            route.update_cadence_raw,
            compiled.update_cadence().raw() as u32
        );
        assert_eq!(
            route.biological_priority_raw,
            compiled.priority().raw() as u32
        );
        assert_eq!(
            route.delay_microsteps,
            u32::from(compiled.delay_microsteps())
        );
        assert_eq!(
            (route.source_start, route.source_count),
            (source.start, source.len)
        );
        assert_eq!(
            (route.target_start, route.target_count),
            (target.start, target.len)
        );
        assert_eq!((route.reserved0, route.reserved1), (0, 0));
    }

    let mut provenance_span_differs_from_executable_ids = false;
    for projection in &upload.projections {
        let executable_ids = upload
            .route_indices
            .iter()
            .enumerate()
            .filter_map(|(global_id, route)| {
                (*route == projection.route_index).then_some(global_id as u32)
            })
            .collect::<Vec<_>>();
        if !executable_ids.is_empty() {
            let provenance =
                projection.synapse_start..projection.synapse_start + projection.synapse_count;
            provenance_span_differs_from_executable_ids |=
                executable_ids != provenance.collect::<Vec<_>>();
        }
    }
    assert!(provenance_span_differs_from_executable_ids);

    let decoder = &upload.decoder_plans[0];
    let compiled = phenotype.candidate_decoder();
    assert_eq!(decoder.schema_version, compiled.schema_version() as u32);
    assert_eq!(decoder.motor_start, compiled.motor_start());
    assert_eq!(decoder.motor_width, u32::from(compiled.motor_width()));
    assert_eq!(decoder.feature_count, u32::from(compiled.feature_count()));
    assert_eq!(
        decoder.flattened_input_lane_count,
        u32::from(compiled.flattened_input_lane_count())
    );
    assert_eq!(decoder.family_count as usize, compiled.families().len());
    assert_eq!(
        decoder.decoder_synapse_count,
        compiled.decoder_synapse_count()
    );
    assert_eq!(upload.decoder_families.len(), 8);
    let recurrent = recurrent_count(&phenotype) as u32;
    let mut decoder_local_cursor = 0_u32;
    for raw in 0_u8..8 {
        let gpu = &upload.decoder_families[raw as usize];
        let family = CandidateActionFamily::try_from_raw(raw).unwrap();
        let source = &compiled.families()[raw as usize];
        assert_eq!(gpu.family_raw, u32::from(family.raw()));
        assert_eq!(gpu.bias_bits, source.bias().to_bits());
        assert_eq!(gpu.decoder_synapse_start, recurrent + decoder_local_cursor);
        assert_eq!(gpu.decoder_synapse_count, source.decoder_synapse_count());
        assert_eq!(
            gpu.weight_index_start,
            upload.decoder_weight_index_word_base + decoder_local_cursor * 4
        );
        assert_eq!(gpu.weight_index_count, source.decoder_synapse_count());
        assert_eq!((gpu.reserved0, gpu.reserved1), (0, 0));
        decoder_local_cursor += source.decoder_synapse_count();
    }
    assert_eq!(decoder_local_cursor, compiled.decoder_synapse_count());
}

#[test]
fn recurrent_and_decoder_views_are_a_disjoint_complete_global_partition() {
    for phenotype in production_phenotypes() {
        let upload = GpuPhenotypeUpload::try_from(&phenotype).unwrap();
        let recurrent = recurrent_count(&phenotype) as u32;
        let candidate_decoder = phenotype.candidate_decoder().decoder_synapse_count();
        let total = phenotype.synapses().len() as u32;

        assert_eq!(
            upload.target_offsets.len(),
            phenotype.neuron_count() as usize + 1
        );
        assert_eq!(upload.target_offsets[0], 0);
        assert_eq!(*upload.target_offsets.last().unwrap(), recurrent);
        assert!(upload
            .target_offsets
            .windows(2)
            .all(|span| span[0] <= span[1]));
        assert_eq!(upload.source_indices.len(), recurrent as usize);
        assert_eq!(upload.route_indices.len(), recurrent as usize);

        let mut expected_recurrent = phenotype
            .synapses()
            .iter()
            .filter(|row| matches!(row.kind(), CompiledSynapseKind::Recurrent))
            .collect::<Vec<_>>();
        expected_recurrent.sort_by_key(|row| (row.target(), row.source(), row.route_index()));
        let mut expected_target_offsets = vec![0_u32; phenotype.neuron_count() as usize + 1];
        for row in &expected_recurrent {
            expected_target_offsets[row.target() as usize + 1] += 1;
        }
        for index in 1..expected_target_offsets.len() {
            expected_target_offsets[index] += expected_target_offsets[index - 1];
        }
        assert_eq!(upload.target_offsets, expected_target_offsets);
        assert_eq!(
            upload.source_indices,
            expected_recurrent
                .iter()
                .map(|row| row.source())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            upload.route_indices,
            expected_recurrent
                .iter()
                .map(|row| u32::from(row.route_index()))
                .collect::<Vec<_>>()
        );

        let recurrent_ids: BTreeSet<u32> = (0..recurrent).collect();
        let candidate_decoder_ids: BTreeSet<u32> = upload
            .decoder_weight_indices
            .iter()
            .map(|row| row.global_synapse_id)
            .collect();
        assert!(recurrent_ids.is_disjoint(&candidate_decoder_ids));
        assert_eq!(candidate_decoder_ids.first().copied(), Some(recurrent));
        assert_eq!(candidate_decoder_ids.len(), candidate_decoder as usize);
        assert_eq!(
            candidate_decoder_ids,
            (recurrent..recurrent + candidate_decoder).collect(),
        );

        let candidate_execution_ids: BTreeSet<u32> = recurrent_ids
            .union(&candidate_decoder_ids)
            .copied()
            .collect();
        assert!(candidate_execution_ids.iter().all(|id| *id < total));
        let auxiliary_ids: BTreeSet<u32> =
            (recurrent + candidate_decoder..total).collect::<BTreeSet<_>>();
        assert!(candidate_execution_ids.is_disjoint(&auxiliary_ids));
        assert_eq!(
            candidate_execution_ids
                .union(&auxiliary_ids)
                .copied()
                .collect::<BTreeSet<_>>(),
            (0..total).collect(),
        );
        assert_eq!(total as usize, upload.genetic_weights.len());
        assert_eq!(total as usize, upload.alpha.len());
        assert_eq!(
            upload.recurrent_global_ids(),
            (0..recurrent).collect::<Vec<_>>()
        );

        let mut expected_all_decoders = phenotype
            .synapses()
            .iter()
            .filter_map(|row| match row.kind() {
                CompiledSynapseKind::Decoder(coordinate) => Some((row, coordinate)),
                CompiledSynapseKind::Recurrent => None,
            })
            .collect::<Vec<_>>();
        expected_all_decoders.sort_by_key(|(row, coordinate)| {
            (
                coordinate.head().raw(),
                coordinate.family().raw(),
                coordinate.input_lane(),
                coordinate.motor_index(),
                row.source(),
                row.target(),
            )
        });
        let expected = expected_recurrent
            .iter()
            .copied()
            .chain(expected_all_decoders.iter().map(|(row, _)| *row))
            .collect::<Vec<_>>();
        for (global_id, compiled) in expected.iter().enumerate() {
            assert_eq!(
                upload.genetic_weights[global_id].to_bits(),
                compiled.genetic_weight().to_bits()
            );
            assert_eq!(
                upload.alpha[global_id].to_bits(),
                compiled.alpha().to_bits()
            );
        }
        let expected_candidate_decoders = expected_all_decoders
            .iter()
            .copied()
            .filter(|(_, coordinate)| {
                coordinate.head() == alife_core::DecoderHeadKind::ActionCandidate
            })
            .collect::<Vec<_>>();
        for (global_id, (row, coordinate)) in expected_candidate_decoders.iter().enumerate() {
            let gpu = &upload.decoder_weight_indices[global_id];
            assert_eq!(gpu.global_synapse_id, recurrent + global_id as u32);
            assert_eq!(gpu.input_lane, u32::from(coordinate.input_lane()));
            assert_eq!(gpu.motor_index, u32::from(coordinate.motor_index()));
            assert_eq!(gpu.reserved0, 0);
            assert_eq!(
                upload.genetic_weights[gpu.global_synapse_id as usize].to_bits(),
                row.genetic_weight().to_bits()
            );
        }
    }
}

#[test]
fn upload_has_one_immutable_owner_full_identity_and_zeroed_reserved_lanes() {
    let phenotype = production_phenotypes().into_iter().next().unwrap();
    let upload = GpuPhenotypeUpload::try_from(&phenotype).unwrap();
    upload.validate_against(&phenotype).unwrap();

    let PhenotypeHash(words) = phenotype.phenotype_hash();
    let expected = words
        .into_iter()
        .flat_map(u64::to_le_bytes)
        .collect::<Vec<_>>();
    let actual = upload
        .identity
        .phenotype_hash
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);

    assert_eq!(upload.genetic_weights.len(), phenotype.synapses().len());
    assert_eq!(upload.alpha.len(), phenotype.synapses().len());
    assert_eq!(upload.immutable_genetic_owner_count(), 1);
    assert!(!upload.has_mutable_projection_copy());
    assert!(upload
        .encoder_assignments
        .iter()
        .all(|row| row.reserved0 == 0));
    assert!(upload
        .neuron_dynamics
        .iter()
        .all(|row| row.reserved0 == 0 && row.reserved1 == 0));
    assert!(upload
        .projections
        .iter()
        .all(|row| row.reserved0 == 0 && row.reserved1 == 0));
    assert!(upload
        .route_metadata
        .iter()
        .all(|row| row.reserved0 == 0 && row.reserved1 == 0));
    assert!(upload
        .decoder_families
        .iter()
        .all(|row| row.reserved0 == 0 && row.reserved1 == 0));
    assert!(upload
        .decoder_weight_indices
        .iter()
        .all(|row| row.reserved0 == 0));
    assert_eq!(upload.extension_record_offset, GPU_NO_EXTENSION_SENTINEL);
}

#[test]
fn phenotype_upload_validation_rejects_every_task4_corruption_category() {
    let phenotype = production_phenotypes().into_iter().next().unwrap();
    let valid = GpuPhenotypeUpload::try_from(&phenotype).unwrap();
    valid.validate_against(&phenotype).unwrap();

    let mut count = valid.clone();
    count.genetic_weights.pop();
    assert!(count.validate_against(&phenotype).is_err());

    let mut order = valid.clone();
    order.source_indices.reverse();
    assert!(order.validate_against(&phenotype).is_err());

    let mut range = valid.clone();
    range.source_indices[0] = phenotype.neuron_count();
    assert!(range.validate_against(&phenotype).is_err());

    let mut identity = valid.clone();
    identity.identity.phenotype_hash[0] ^= 1;
    assert!(identity.validate_against(&phenotype).is_err());

    let mut layout_version = valid.clone();
    layout_version.gpu_layout_version += 1;
    assert!(layout_version.validate_against(&phenotype).is_err());

    let mut enum_raw = valid.clone();
    enum_raw.route_metadata[0].projection_type_raw = u32::MAX;
    assert!(enum_raw.validate_against(&phenotype).is_err());

    let mut nonfinite = valid.clone();
    nonfinite.neuron_dynamics[0].bias_bits = f32::NAN.to_bits();
    assert!(nonfinite.validate_against(&phenotype).is_err());

    let mut target_offsets = valid.clone();
    *target_offsets.target_offsets.last_mut().unwrap() -= 1;
    assert!(target_offsets.validate_against(&phenotype).is_err());

    let mut provenance_range = valid.clone();
    provenance_range.projections[0].synapse_count += 1;
    assert!(provenance_range.validate_against(&phenotype).is_err());

    let mut reserved = valid.clone();
    reserved.decoder_weight_indices[0].reserved0 = 1;
    assert!(reserved.validate_against(&phenotype).is_err());

    let mut sentinel = valid;
    sentinel.extension_record_offset = 0;
    assert!(sentinel.validate_against(&phenotype).is_err());
}
