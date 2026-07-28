//! Research-gated function-preserving N2048 to N4096 phenotype growth.

use std::collections::BTreeSet;

use crate::{
    ActivationFunction, BrainGenome, CandidateActionFamily, LobeKind, LobeLayout,
    ScaffoldContractError, CANDIDATE_FEATURE_COUNT,
};

use super::learning::compute_plasticity_plan_digest;
use super::{
    AuxiliaryDecoderPlan, BrainCapacityClass, BrainPhenotype, CandidateDecoderFamilyPlan,
    CandidateDecoderPlan, CompiledBudgets, CompiledProjection, CompiledSynapse,
    CompiledSynapseKind, DecoderHeadKind, DecoderSynapseCoordinate, GlobalPhenotypeBudgetReceipt,
    MemoryChannelPlan, NeuronDynamics, PhenotypeCompilerInputs, ReplayCapturePlan,
    RouteBudgetReceipt, SensorEncoderAssignment, SensorEncoderPlan,
};

/// Frozen research-only size contract. It is deliberately absent from the
/// promoted production-class registry.
pub struct N4096ResearchLayoutV1;

impl N4096ResearchLayoutV1 {
    pub const NEURON_COUNT: u32 = 4_096;
    pub const RECURRENT_SYNAPSE_COUNT: u32 = 49_152;
    pub const ACTION_DECODER_SYNAPSE_COUNT: u32 = 8_192;
    pub const CANDIDATE_DECODER_SYNAPSE_COUNT: u32 = 7_168;
    pub const SPEECH_DECODER_SYNAPSE_COUNT: u32 = 1_024;
    pub const MEMORY_DECODER_SYNAPSE_COUNT: u32 = 8_192;

    pub fn lobe_layout() -> Result<LobeLayout, ScaffoldContractError> {
        LobeLayout::reference_for_neuron_count(Self::NEURON_COUNT)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PhenotypeGrowthReceipt {
    pub source_hash: super::PhenotypeHash,
    pub target_hash: super::PhenotypeHash,
    pub source_address_map_digest: crate::Blake3Digest,
    pub target_address_map_digest: crate::Blake3Digest,
    pub source_to_target_neurons: Vec<u32>,
    pub source_to_target_synapses: Vec<u32>,
    pub expansion_neurons_dormant: u32,
    pub expansion_synapses_dormant: u32,
    pub language_codebook_preserved: bool,
    pub promoted: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PhenotypeGrowthMigration {
    pub compiler_inputs: PhenotypeCompilerInputs,
    pub phenotype: BrainPhenotype,
    pub receipt: PhenotypeGrowthReceipt,
}

impl PhenotypeGrowthMigration {
    pub fn compile_n2048_to_n4096(
        source: &BrainPhenotype,
        source_inputs: &PhenotypeCompilerInputs,
    ) -> Result<Self, ScaffoldContractError> {
        let source_capacity = BrainCapacityClass::n2048();
        source_inputs.validate_against(&source_capacity)?;
        source.validate_against(&source_capacity)?;
        if source.compiler_inputs_digest() != source_inputs.canonical_digest()
            || source.lobe_layout() != &crate::N2048FoundationLayoutV1::lobe_layout()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }

        let target_capacity = BrainCapacityClass::n4096_research();
        let target_layout = N4096ResearchLayoutV1::lobe_layout()?;
        let mut target_genome = BrainGenome::scaffold(
            source_inputs.genome().species_seed,
            BrainCapacityClass::N4096_RESEARCH_ID,
        );
        target_genome.parent_genome_ids = vec![source_inputs.genome().id];
        target_genome.lineage_id = source_inputs.genome().lineage_id;
        let mut target_development = source_inputs.development().clone();
        target_development.genome_id = target_genome.id;
        let target_inputs = PhenotypeCompilerInputs::try_new(
            target_genome,
            &target_capacity,
            target_development,
            source.sensor_profile(),
        )?;

        let source_to_target_neurons = source
            .persistent_address_map()
            .neurons()
            .iter()
            .map(|entry| {
                packed_neuron_for_address(
                    &target_layout,
                    entry.address().lobe,
                    entry.address().ordinal,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut source_to_target_synapses = vec![u32::MAX; source.synapses().len()];
        let mut projections = Vec::with_capacity(source.projections().len());
        let mut synapses = Vec::with_capacity(65_536);
        let mut receipts = Vec::with_capacity(source.projections().len());

        for projection in source.projections().iter().take(16) {
            let route = projection.route_index();
            let start = u32::try_from(synapses.len()).map_err(|_| compile_error())?;
            let (source_start, source_len) = projection.synapse_range();
            let source_slice =
                &source.synapses()[source_start as usize..(source_start + source_len) as usize];
            let source_lobe_len = source
                .lobe_layout()
                .region(projection.source_lobe())
                .ok_or_else(compile_error)?
                .len;
            let target_lobe_len = source
                .lobe_layout()
                .region(projection.target_lobe())
                .ok_or_else(compile_error)?
                .len;
            let mut rows = Vec::with_capacity(source_slice.len() * 2);
            for (local, old) in source_slice.iter().enumerate() {
                let mapped = remap_synapse(old, source.lobe_layout(), &target_layout, route)?;
                rows.push((Some(source_start + local as u32), mapped));
                let mut dormant = CompiledSynapse::new(
                    offset_packed_neuron(
                        old.source(),
                        source.lobe_layout(),
                        &target_layout,
                        source_lobe_len,
                    )?,
                    offset_packed_neuron(
                        old.target(),
                        source.lobe_layout(),
                        &target_layout,
                        target_lobe_len,
                    )?,
                    if projection.projection_type() == crate::ProjectionType::LateralInhibition {
                        -0.000_1
                    } else {
                        0.0
                    },
                    0.0,
                    route,
                    CompiledSynapseKind::Recurrent,
                );
                dormant.set_receptor_index(0);
                rows.push((None, dormant));
            }
            rows.sort_by_key(|(_, row)| (row.source(), row.target()));
            for (old_index, row) in rows {
                let new_index = u32::try_from(synapses.len()).map_err(|_| compile_error())?;
                if let Some(old_index) = old_index {
                    source_to_target_synapses[old_index as usize] = new_index;
                }
                synapses.push(row);
            }
            let len = u32::try_from(synapses.len()).map_err(|_| compile_error())? - start;
            let active_tiles = active_tile_count(&synapses[start as usize..(start + len) as usize]);
            projections.push(CompiledProjection::new(
                route,
                projection.source_lobe(),
                projection.target_lobe(),
                projection.projection_type(),
                projection.active_tile_policy(),
                projection.update_cadence(),
                projection.priority(),
                projection.delay_microsteps(),
                start,
                len,
                active_tiles,
            ));
            receipts.push(route_receipt(route, active_tiles, len, 0, 0));
        }

        let action_projection = source.projections().get(16).ok_or_else(compile_error)?;
        let action_route = action_projection.route_index();
        let action_start = u32::try_from(synapses.len()).map_err(|_| compile_error())?;
        let target_motor = target_layout
            .region(LobeKind::MotorArbitration)
            .ok_or_else(compile_error)?;
        let old_motor_len = source
            .lobe_layout()
            .region(LobeKind::MotorArbitration)
            .ok_or_else(compile_error)?
            .len;
        let mut families = Vec::with_capacity(8);
        for raw in 0_u8..8 {
            let family = CandidateActionFamily::try_from_raw(raw)?;
            let family_start = u32::try_from(synapses.len()).map_err(|_| compile_error())?;
            let mut rows = Vec::with_capacity(896);
            for (old_index, old) in source.synapses().iter().enumerate() {
                let CompiledSynapseKind::Decoder(coordinate) = old.kind() else {
                    continue;
                };
                if coordinate.head() == DecoderHeadKind::ActionCandidate
                    && coordinate.family() == family
                {
                    rows.push((
                        Some(u32::try_from(old_index).map_err(|_| compile_error())?),
                        remap_synapse(old, source.lobe_layout(), &target_layout, action_route)?,
                    ));
                }
            }
            let mut added = 0_u32;
            'lanes: for input_lane in 0_u16..CANDIDATE_FEATURE_COUNT as u16 {
                for motor_index in old_motor_len as u16..target_motor.len as u16 {
                    let neuron = target_motor.start + u32::from(motor_index);
                    let mut row = CompiledSynapse::new(
                        neuron,
                        neuron,
                        0.0,
                        0.0,
                        action_route,
                        CompiledSynapseKind::Decoder(DecoderSynapseCoordinate::new(
                            DecoderHeadKind::ActionCandidate,
                            family,
                            input_lane,
                            motor_index,
                        )),
                    );
                    row.set_receptor_index(0);
                    rows.push((None, row));
                    added += 1;
                    if added == 512 {
                        break 'lanes;
                    }
                }
            }
            rows.sort_by_key(|(_, row)| match row.kind() {
                CompiledSynapseKind::Decoder(coordinate) => (
                    coordinate.input_lane(),
                    coordinate.motor_index(),
                    row.source(),
                    row.target(),
                ),
                CompiledSynapseKind::Recurrent => unreachable!(),
            });
            for (old_index, row) in rows {
                let new_index = u32::try_from(synapses.len()).map_err(|_| compile_error())?;
                if let Some(old_index) = old_index {
                    source_to_target_synapses[old_index as usize] = new_index;
                }
                synapses.push(row);
            }
            families.push(CandidateDecoderFamilyPlan::new(
                family,
                0.0,
                family_start,
                u32::try_from(synapses.len()).map_err(|_| compile_error())? - family_start,
            ));
        }
        let memory_channel = MemoryChannelPlan::try_new_v1(8_192)?;
        let candidate = CandidateDecoderPlan::try_new(
            target_motor.start,
            u16::try_from(target_motor.len).map_err(|_| compile_error())?,
            CANDIDATE_FEATURE_COUNT as u16,
            u16::try_from(memory_channel.decoder_input_stride()).map_err(|_| compile_error())?,
            Some(memory_channel),
            families,
        )?;
        let speech_start = u32::try_from(synapses.len()).map_err(|_| compile_error())?;
        for (old_index, old) in source.synapses().iter().enumerate() {
            if matches!(
                old.kind(),
                CompiledSynapseKind::Decoder(coordinate)
                    if coordinate.head() == DecoderHeadKind::SpeechPayload
            ) {
                let new_index = u32::try_from(synapses.len()).map_err(|_| compile_error())?;
                source_to_target_synapses[old_index] = new_index;
                synapses.push(remap_synapse(
                    old,
                    source.lobe_layout(),
                    &target_layout,
                    action_route,
                )?);
            }
        }
        let speech = AuxiliaryDecoderPlan::try_new(
            DecoderHeadKind::SpeechPayload,
            crate::SpeechDecoderLayoutV1::INPUT_WIDTH,
            crate::SpeechDecoderLayoutV1::OUTPUT_WIDTH,
            speech_start,
            N4096ResearchLayoutV1::SPEECH_DECODER_SYNAPSE_COUNT,
        )?;
        let action_len = u32::try_from(synapses.len()).map_err(|_| compile_error())? - action_start;
        projections.push(CompiledProjection::new(
            action_route,
            action_projection.source_lobe(),
            action_projection.target_lobe(),
            action_projection.projection_type(),
            action_projection.active_tile_policy(),
            action_projection.update_cadence(),
            action_projection.priority(),
            0,
            action_start,
            action_len,
            0,
        ));
        receipts.push(route_receipt(action_route, 0, 0, action_len, 0));

        let memory_projection = source.projections().get(17).ok_or_else(compile_error)?;
        let memory_route = memory_projection.route_index();
        let memory_start = u32::try_from(synapses.len()).map_err(|_| compile_error())?;
        let source_episodic_len = source
            .lobe_layout()
            .region(LobeKind::EpisodicMemory)
            .ok_or_else(compile_error)?
            .len;
        let mut memory_rows = Vec::with_capacity(8_192);
        for (old_index, old) in source.synapses().iter().enumerate() {
            let CompiledSynapseKind::Decoder(coordinate) = old.kind() else {
                continue;
            };
            if coordinate.head() != DecoderHeadKind::MemoryContext {
                continue;
            }
            memory_rows.push((
                Some(u32::try_from(old_index).map_err(|_| compile_error())?),
                remap_synapse(old, source.lobe_layout(), &target_layout, memory_route)?,
            ));
            let mut dormant = CompiledSynapse::new(
                offset_packed_neuron(
                    old.source(),
                    source.lobe_layout(),
                    &target_layout,
                    source_episodic_len,
                )?,
                remap_packed_neuron(old.target(), source.lobe_layout(), &target_layout)?,
                0.0,
                0.0,
                memory_route,
                old.kind(),
            );
            dormant.set_receptor_index(0);
            memory_rows.push((None, dormant));
        }
        memory_rows.sort_by_key(|(_, row)| match row.kind() {
            CompiledSynapseKind::Decoder(coordinate) => (
                coordinate.family().raw(),
                coordinate.input_lane(),
                coordinate.motor_index(),
                row.source(),
                row.target(),
            ),
            CompiledSynapseKind::Recurrent => unreachable!(),
        });
        for (old_index, row) in memory_rows {
            let new_index = u32::try_from(synapses.len()).map_err(|_| compile_error())?;
            if let Some(old_index) = old_index {
                source_to_target_synapses[old_index as usize] = new_index;
            }
            synapses.push(row);
        }
        let memory_len = u32::try_from(synapses.len()).map_err(|_| compile_error())? - memory_start;
        let memory = AuxiliaryDecoderPlan::try_new(
            DecoderHeadKind::MemoryContext,
            128,
            64,
            memory_start,
            memory_len,
        )?;
        projections.push(CompiledProjection::new(
            memory_route,
            memory_projection.source_lobe(),
            memory_projection.target_lobe(),
            memory_projection.projection_type(),
            memory_projection.active_tile_policy(),
            memory_projection.update_cadence(),
            memory_projection.priority(),
            0,
            memory_start,
            memory_len,
            0,
        ));
        receipts.push(route_receipt(memory_route, 0, 0, 0, memory_len));

        if source_to_target_synapses.contains(&u32::MAX) {
            return Err(compile_error());
        }
        let sensor_encoder = SensorEncoderPlan::try_new(
            source.sensor_profile(),
            source
                .sensor_encoder()
                .assignments()
                .iter()
                .map(|assignment| {
                    let (min, max) = assignment.clamp_range();
                    Ok(SensorEncoderAssignment::new(
                        assignment.source_group(),
                        assignment.source_index(),
                        remap_packed_neuron(
                            assignment.target_neuron(),
                            source.lobe_layout(),
                            &target_layout,
                        )?,
                        assignment.scale(),
                        assignment.bias(),
                        min,
                        max,
                    ))
                })
                .collect::<Result<Vec<_>, ScaffoldContractError>>()?,
        )?;
        let dynamics = compile_dynamics(source, &target_layout)?;
        let replay_ids = source
            .replay_capture_plan()
            .global_synapse_ids()
            .iter()
            .map(|id| source_to_target_synapses[*id as usize])
            .collect::<Vec<_>>();
        let replay = ReplayCapturePlan::try_new(
            replay_ids,
            source.replay_capture_plan().samples_per_event(),
            source.replay_capture_plan().event_capacity(),
            source.replay_capture_plan().sample_capacity(),
        )?;
        let plasticity_digest = compute_plasticity_plan_digest(
            source.plasticity_receptors(),
            &replay,
            source.sleep_consolidation_plan(),
        )?;
        let active_tiles = receipts.iter().map(|row| row.active_tiles).sum();
        let budgets = CompiledBudgets {
            capacity_class_id: target_capacity.id(),
            execution_abi_digest: target_capacity.canonical_digest(),
            routes: receipts,
            global: GlobalPhenotypeBudgetReceipt {
                neuron_count: N4096ResearchLayoutV1::NEURON_COUNT,
                active_tiles,
                recurrent_synapses: N4096ResearchLayoutV1::RECURRENT_SYNAPSE_COUNT,
                action_decoder_synapses: N4096ResearchLayoutV1::ACTION_DECODER_SYNAPSE_COUNT,
                memory_decoder_synapses: N4096ResearchLayoutV1::MEMORY_DECODER_SYNAPSE_COUNT,
                total_synapses: 65_536,
                immutable_payload_words: 65_536,
                candidate_capacity: target_capacity.execution().max_candidates(),
                object_slot_capacity: target_capacity.execution().max_object_slots(),
                memory_context_capacity: target_capacity.execution().max_memory_context_records(),
                decoder_input_lanes: candidate.flattened_input_lane_count(),
                replay_event_capacity: replay.event_capacity(),
                replay_eligibility_sample_capacity: replay.sample_capacity(),
                replay_capture_synapse_count: replay.global_synapse_ids().len() as u32,
            },
        };
        let phenotype = BrainPhenotype::try_new(
            &target_inputs,
            &target_capacity,
            N4096ResearchLayoutV1::NEURON_COUNT,
            source.microstep_count(),
            target_layout,
            projections,
            synapses,
            dynamics,
            sensor_encoder,
            candidate,
            Some(speech),
            Some(memory),
            source.plasticity_receptors().to_vec(),
            replay,
            *source.sleep_consolidation_plan(),
            plasticity_digest,
            budgets,
        )?;
        let receipt = PhenotypeGrowthReceipt {
            source_hash: source.phenotype_hash(),
            target_hash: phenotype.phenotype_hash(),
            source_address_map_digest: source.persistent_address_map().digest(),
            target_address_map_digest: phenotype.persistent_address_map().digest(),
            source_to_target_neurons,
            source_to_target_synapses,
            expansion_neurons_dormant: 2_048,
            expansion_synapses_dormant: 32_768,
            language_codebook_preserved: source.language_codebook()
                == phenotype.language_codebook(),
            promoted: false,
        };
        Ok(Self {
            compiler_inputs: target_inputs,
            phenotype,
            receipt,
        })
    }
}

fn remap_synapse(
    old: &CompiledSynapse,
    source_layout: &LobeLayout,
    target_layout: &LobeLayout,
    route: u16,
) -> Result<CompiledSynapse, ScaffoldContractError> {
    let mut row = CompiledSynapse::new(
        remap_packed_neuron(old.source(), source_layout, target_layout)?,
        remap_packed_neuron(old.target(), source_layout, target_layout)?,
        old.genetic_weight(),
        old.alpha(),
        route,
        old.kind(),
    );
    row.set_receptor_index(old.receptor_index());
    Ok(row)
}

fn remap_packed_neuron(
    packed: u32,
    source_layout: &LobeLayout,
    target_layout: &LobeLayout,
) -> Result<u32, ScaffoldContractError> {
    let source_region = source_layout
        .lobe_by_neuron_index(packed)
        .ok_or_else(compile_error)?;
    packed_neuron_for_address(
        target_layout,
        source_region.kind,
        packed - source_region.start,
    )
}

fn offset_packed_neuron(
    packed: u32,
    source_layout: &LobeLayout,
    target_layout: &LobeLayout,
    ordinal_offset: u32,
) -> Result<u32, ScaffoldContractError> {
    let source_region = source_layout
        .lobe_by_neuron_index(packed)
        .ok_or_else(compile_error)?;
    packed_neuron_for_address(
        target_layout,
        source_region.kind,
        packed - source_region.start + ordinal_offset,
    )
}

fn packed_neuron_for_address(
    layout: &LobeLayout,
    lobe: LobeKind,
    ordinal: u32,
) -> Result<u32, ScaffoldContractError> {
    let region = layout
        .region(lobe)
        .filter(|region| ordinal < region.len)
        .ok_or_else(compile_error)?;
    Ok(region.start + ordinal)
}

fn active_tile_count(rows: &[CompiledSynapse]) -> u32 {
    rows.iter()
        .map(|row| (row.source() / 16, row.target() / 16))
        .collect::<BTreeSet<_>>()
        .len() as u32
}

fn route_receipt(
    route_index: u16,
    active_tiles: u32,
    recurrent: u32,
    action: u32,
    memory: u32,
) -> RouteBudgetReceipt {
    let total = recurrent + action + memory;
    RouteBudgetReceipt {
        route_index,
        active_tiles,
        recurrent_synapses: recurrent,
        action_decoder_synapses: action,
        memory_decoder_synapses: memory,
        immutable_payload_words: total,
        tile_ceiling: active_tiles,
        synapse_ceiling: total,
        payload_word_ceiling: total,
    }
}

fn compile_dynamics(
    source: &BrainPhenotype,
    target_layout: &LobeLayout,
) -> Result<Vec<NeuronDynamics>, ScaffoldContractError> {
    let mut rows = Vec::with_capacity(N4096ResearchLayoutV1::NEURON_COUNT as usize);
    for target in 0..N4096ResearchLayoutV1::NEURON_COUNT {
        let region = target_layout
            .lobe_by_neuron_index(target)
            .ok_or_else(compile_error)?;
        let ordinal = target - region.start;
        let source_region = source
            .lobe_layout()
            .region(region.kind)
            .ok_or_else(compile_error)?;
        if ordinal < source_region.len {
            rows.push(source.neuron_dynamics()[(source_region.start + ordinal) as usize]);
        } else {
            rows.push(NeuronDynamics::new(
                0.0,
                1.0,
                ActivationFunction::Tanh,
                1.0,
                0.0,
                0.0,
            ));
        }
    }
    Ok(rows)
}

const fn compile_error() -> ScaffoldContractError {
    ScaffoldContractError::PhenotypeCompile
}
