//! Deterministic orchestration for immutable GPU phenotype construction.

use crate::{ActivationFunction, AlphaStoragePolicy, BrainGenome, ScaffoldContractError};

use super::{
    BrainCapacityClass, BrainPhenotype, CompiledBudgets, GlobalPhenotypeBudgetReceipt,
    NeuronDynamics, PhenotypeCompilerInputs,
};

pub(super) fn compile(
    inputs: &PhenotypeCompilerInputs,
    capacity: &BrainCapacityClass,
) -> Result<BrainPhenotype, ScaffoldContractError> {
    compile_inner(inputs, capacity, None, None)
}

pub(super) fn compile_with_foundation_asset(
    inputs: &PhenotypeCompilerInputs,
    capacity: &BrainCapacityClass,
    foundation: &crate::FoundationWeightAsset,
) -> Result<BrainPhenotype, ScaffoldContractError> {
    compile_inner(inputs, capacity, Some(foundation), None)
}

pub(super) fn compile_with_foundation_asset_and_overlay_seed(
    inputs: &PhenotypeCompilerInputs,
    capacity: &BrainCapacityClass,
    foundation: &crate::FoundationWeightAsset,
    overlay_seed: u64,
) -> Result<BrainPhenotype, ScaffoldContractError> {
    if overlay_seed == 0 {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    compile_inner(inputs, capacity, Some(foundation), Some(overlay_seed))
}

fn compile_inner(
    inputs: &PhenotypeCompilerInputs,
    capacity: &BrainCapacityClass,
    foundation: Option<&crate::FoundationWeightAsset>,
    overlay_seed: Option<u64>,
) -> Result<BrainPhenotype, ScaffoldContractError> {
    inputs.validate_against(capacity)?;
    let genome = inputs.genome();
    let development = inputs.development();
    validate_supported_inputs(genome, capacity)?;
    let layout = if inputs.legacy_foundation_compatibility_abi().is_some() {
        crate::legacy_nano512_compatibility::legacy_nano512_runtime_layout()?
    } else if capacity.id() == BrainCapacityClass::N2048_ID {
        crate::N2048FoundationLayoutV1::lobe_layout()
    } else {
        super::layout_compile::compile_layout(
            genome,
            development,
            capacity.execution().max_neurons(),
        )?
    };
    let encoder =
        super::io_compile::compile_encoder(genome, development, &layout, inputs.sensor_profile())?;
    let (mut projections, mut synapses, mut receipts) =
        super::topology_compile::compile_recurrent(genome, &layout, capacity)?;
    let decoders = super::io_compile::compile_decoders(
        genome,
        development,
        &layout,
        capacity,
        u16::try_from(projections.len()).map_err(|_| compile_error())?,
        u32::try_from(synapses.len()).map_err(|_| compile_error())?,
    )?;
    projections.extend(decoders.projections);
    synapses.extend(decoders.synapses);
    receipts.extend(decoders.receipts);
    if inputs.cognitive_channel_extension().is_some() {
        let route_index = u16::try_from(projections.len()).map_err(|_| compile_error())?;
        let start = u32::try_from(synapses.len()).map_err(|_| compile_error())?;
        let episodic = layout
            .region(crate::LobeKind::MemoryInterface)
            .filter(|region| {
                region.enabled && region.len >= u32::from(crate::COGNITIVE_CHANNEL_LANE_COUNT)
            })
            .ok_or_else(compile_error)?;
        let core = layout
            .region(crate::LobeKind::TemporalPredictive)
            .filter(|region| {
                region.enabled && region.len >= u32::from(crate::COGNITIVE_CHANNEL_FAMILY_COUNT)
            })
            .ok_or_else(compile_error)?;
        let mut cognitive_synapses =
            Vec::with_capacity(crate::COGNITIVE_CHANNEL_TOTAL_SYNAPSES as usize);
        for raw in 0_u8..crate::COGNITIVE_CHANNEL_FAMILY_COUNT {
            let family = crate::CandidateActionFamily::try_from_raw(raw)?;
            for lane in crate::COGNITIVE_CHANNEL_LANE_START..crate::COGNITIVE_CHANNEL_LANE_END {
                let motor_index = u16::from(raw);
                let source = episodic.start
                    + u32::from(lane - crate::COGNITIVE_CHANNEL_LANE_START) % episodic.len;
                let target = core.start + u32::from(motor_index);
                cognitive_synapses.push(super::CompiledSynapse::new(
                    source,
                    target,
                    super::io_compile::genetic_weight(
                        genome.genetic_prior_seed,
                        route_index,
                        source,
                        target,
                    ),
                    super::topology_compile::inherited_memory_decoder_alpha(genome),
                    route_index,
                    super::CompiledSynapseKind::Decoder(super::DecoderSynapseCoordinate::new(
                        super::DecoderHeadKind::CognitiveContext,
                        family,
                        lane,
                        motor_index,
                    )),
                ));
            }
        }
        synapses.extend(cognitive_synapses);
        let count = crate::COGNITIVE_CHANNEL_TOTAL_SYNAPSES;
        projections.push(super::CompiledProjection::new(
            route_index,
            crate::LobeKind::MemoryInterface,
            crate::LobeKind::TemporalPredictive,
            crate::ProjectionType::Feedback,
            crate::ActiveTilePolicy::EssentialReservation,
            crate::UpdateCadence::Hot60Hz,
            crate::BiologicalPriority::Essential,
            0,
            start,
            count,
            0,
        ));
        receipts.push(super::RouteBudgetReceipt {
            route_index,
            active_tiles: 0,
            recurrent_synapses: 0,
            action_decoder_synapses: 0,
            memory_decoder_synapses: count,
            immutable_payload_words: count,
            tile_ceiling: 0,
            synapse_ceiling: count,
            payload_word_ceiling: count,
        });
    }
    super::topology_compile::validate_alpha_matches(genome, &projections, &synapses, &layout)?;
    let learning = super::learning::compile_learning_plans(
        genome,
        development,
        capacity,
        &projections,
        &mut synapses,
    )?;

    let recurrent = receipts
        .iter()
        .map(|receipt| receipt.recurrent_synapses)
        .sum::<u32>();
    let action = receipts
        .iter()
        .map(|receipt| receipt.action_decoder_synapses)
        .sum::<u32>();
    let memory = receipts
        .iter()
        .map(|receipt| receipt.memory_decoder_synapses)
        .sum::<u32>();
    let active_tiles = receipts
        .iter()
        .map(|receipt| receipt.active_tiles)
        .sum::<u32>();
    let total = recurrent
        .checked_add(action)
        .and_then(|value| value.checked_add(memory))
        .ok_or_else(compile_error)?;
    let execution = capacity.execution();
    let budgets = CompiledBudgets {
        capacity_class_id: capacity.id(),
        execution_abi_digest: capacity.canonical_digest(),
        routes: receipts,
        global: GlobalPhenotypeBudgetReceipt {
            neuron_count: execution.max_neurons(),
            active_tiles,
            recurrent_synapses: recurrent,
            action_decoder_synapses: action,
            memory_decoder_synapses: memory,
            total_synapses: total,
            immutable_payload_words: total,
            candidate_capacity: execution.max_candidates(),
            object_slot_capacity: execution.max_object_slots(),
            memory_context_capacity: execution.max_memory_context_records(),
            decoder_input_lanes: if inputs.cognitive_channel_extension().is_some() {
                crate::COGNITIVE_CHANNEL_LANE_END
            } else {
                decoders.candidate.flattened_input_lane_count()
            },
            replay_event_capacity: execution.max_replay_events(),
            replay_eligibility_sample_capacity: execution.max_replay_eligibility_samples(),
            replay_capture_synapse_count: u32::try_from(learning.replay.global_synapse_ids().len())
                .map_err(|_| compile_error())?,
        },
    };
    budgets.validate_against(capacity)?;
    let motor = layout
        .region(crate::LobeKind::ActionPlanning)
        .filter(|region| region.enabled)
        .ok_or_else(compile_error)?;
    let dynamics: Vec<NeuronDynamics> = (0..execution.max_neurons())
        .map(|neuron| {
            let bias = if motor.contains_neuron(neuron) {
                0.05
            } else {
                0.0
            };
            NeuronDynamics::new(bias, 0.25, ActivationFunction::Tanh, 0.95, 0.01, 1.0)
        })
        .collect();
    let microstep_count = match development.maturation.raw() {
        value if value < 1.0 / 3.0 => 2,
        value if value < 2.0 / 3.0 => 3,
        _ => 4,
    };
    if let Some(foundation) = foundation {
        if let Some(descriptor) = inputs.legacy_foundation_compatibility_abi() {
            if descriptor.source_weight_asset() != foundation.asset_ref()
                || foundation.weights().len() != synapses.len()
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            for (global_index, (synapse, weight)) in
                synapses.iter_mut().zip(foundation.weights()).enumerate()
            {
                let Some(overlay_seed) = overlay_seed else {
                    synapse.set_genetic_weight(*weight);
                    continue;
                };
                let projection = projections
                    .get(usize::from(synapse.route_index()))
                    .ok_or(ScaffoldContractError::PhenotypeCompile)?;
                let delta = genome_weight_delta(overlay_seed, global_index as u32);
                let mut composed = *weight + delta;
                match projection.projection_type() {
                    crate::ProjectionType::LateralInhibition if composed >= 0.0 => {
                        composed = -0.000_1;
                    }
                    crate::ProjectionType::Homeostatic | crate::ProjectionType::MotorProposal
                        if composed < 0.0 =>
                    {
                        composed = 0.000_1;
                    }
                    _ => {}
                }
                synapse.set_genetic_weight(composed);
            }
        } else {
            let coordinate_plan = BrainPhenotype::try_new(
                inputs,
                capacity,
                execution.max_neurons(),
                microstep_count,
                layout.clone(),
                projections.clone(),
                synapses.clone(),
                dynamics.clone(),
                encoder.clone(),
                decoders.candidate.clone(),
                decoders.speech.clone(),
                decoders.memory.clone(),
                learning.receptors.clone(),
                learning.replay.clone(),
                learning.sleep,
                learning.digest,
                budgets.clone(),
            )?;
            if let Some(descriptor) = inputs.migrated_n2048_foundation_v1() {
                descriptor.validate_source_asset(foundation)?;
            } else {
                foundation.validate_against(&coordinate_plan)?;
            }
            for (global_index, (synapse, weight)) in
                synapses.iter_mut().zip(foundation.weights()).enumerate()
            {
                let projection = projections
                    .get(usize::from(synapse.route_index()))
                    .ok_or(ScaffoldContractError::PhenotypeCompile)?;
                let delta = genome_weight_delta(
                    overlay_seed.unwrap_or(genome.genetic_prior_seed),
                    global_index as u32,
                );
                let mut composed = *weight + delta;
                match projection.projection_type() {
                    crate::ProjectionType::LateralInhibition if composed >= 0.0 => {
                        composed = -0.000_1;
                    }
                    crate::ProjectionType::Homeostatic | crate::ProjectionType::MotorProposal
                        if composed < 0.0 =>
                    {
                        composed = 0.000_1;
                    }
                    _ => {}
                }
                synapse.set_genetic_weight(composed);
            }
        }
    } else if inputs.legacy_foundation_compatibility_abi().is_some()
        || inputs.migrated_n2048_foundation_v1().is_some()
    {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }

    BrainPhenotype::try_new(
        inputs,
        capacity,
        execution.max_neurons(),
        microstep_count,
        layout,
        projections,
        synapses,
        dynamics,
        encoder,
        decoders.candidate,
        decoders.speech,
        decoders.memory,
        learning.receptors,
        learning.replay,
        learning.sleep,
        learning.digest,
        budgets,
    )
}

fn validate_supported_inputs(
    genome: &BrainGenome,
    capacity: &BrainCapacityClass,
) -> Result<(), ScaffoldContractError> {
    if genome.brain_class_id != capacity.id()
        || genome.alpha_mask.storage_policy != AlphaStoragePolicy::HierarchicalSparse
        || genome.alpha_mask.dense_reference_opt_in
    {
        return Err(compile_error());
    }
    Ok(())
}

const fn compile_error() -> ScaffoldContractError {
    ScaffoldContractError::PhenotypeCompile
}

fn genome_weight_delta(seed: u64, global_index: u32) -> f32 {
    let bits = splitmix64(seed ^ (u64::from(global_index) << 21) ^ 0x9D39_2EA1_A903_347B);
    if bits & 0x0F != 0 {
        return 0.0;
    }
    let centered = ((bits >> 40) as f32 / ((1_u32 << 24) - 1) as f32) - 0.5;
    centered * 0.01
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}
