//! Validation helpers for frozen route and decoder ABI partitions.

use crate::{Blake3Digest, BrainCapacityClass, ScaffoldContractError};

use super::{
    BrainPhenotype, CompiledProjection, CompiledSynapse, CompiledSynapseKind, DecoderHeadKind,
    DecoderSynapseCoordinate,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectionKind {
    Recurrent,
    ActionAndSpeechDecoder,
    MemoryDecoder,
}

pub(super) fn classify_projection(
    synapses: &[CompiledSynapse],
) -> Result<ProjectionKind, ScaffoldContractError> {
    let mut kind = None;
    for synapse in synapses {
        let current = match synapse.kind() {
            CompiledSynapseKind::Recurrent => ProjectionKind::Recurrent,
            CompiledSynapseKind::Decoder(coordinate)
                if matches!(
                    coordinate.head(),
                    DecoderHeadKind::ActionCandidate | DecoderHeadKind::SpeechPayload
                ) =>
            {
                ProjectionKind::ActionAndSpeechDecoder
            }
            CompiledSynapseKind::Decoder(coordinate)
                if coordinate.head() == DecoderHeadKind::MemoryContext =>
            {
                ProjectionKind::MemoryDecoder
            }
            _ => return Err(ScaffoldContractError::PhenotypeCompile),
        };
        if kind.is_some_and(|prior| prior != current) {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        kind = Some(current);
    }
    kind.ok_or(ScaffoldContractError::PhenotypeCompile)
}

pub(super) fn validate_decoder_synapse(
    phenotype: &BrainPhenotype,
    projection_kind: ProjectionKind,
    synapse: &CompiledSynapse,
    coordinate: DecoderSynapseCoordinate,
) -> Result<(), ScaffoldContractError> {
    match coordinate.head() {
        DecoderHeadKind::ActionCandidate => {
            let decoder = phenotype.candidate_decoder();
            let motor_neuron = decoder
                .motor_start()
                .checked_add(u32::from(coordinate.motor_index()))
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            if projection_kind != ProjectionKind::ActionAndSpeechDecoder
                || coordinate.input_lane() >= decoder.flattened_input_lane_count()
                || coordinate.motor_index() >= decoder.motor_width()
                || synapse.source() != motor_neuron
                || synapse.target() != motor_neuron
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
        }
        DecoderHeadKind::SpeechPayload => {
            let plan = phenotype
                .speech_decoder()
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            let motor_start = phenotype.candidate_decoder().motor_start();
            if projection_kind != ProjectionKind::ActionAndSpeechDecoder
                || plan.head() != DecoderHeadKind::SpeechPayload
                || coordinate.family() != crate::CandidateActionFamily::Other
                || coordinate.input_lane() >= plan.input_width()
                || coordinate.motor_index() >= plan.output_width()
                || synapse.source()
                    != motor_start
                        + crate::SpeechDecoderLayoutV1::MOTOR_SOURCE_OFFSET
                        + u32::from(coordinate.input_lane())
                || synapse.target()
                    != motor_start
                        + crate::SpeechDecoderLayoutV1::MOTOR_TARGET_OFFSET
                        + u32::from(coordinate.motor_index())
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
        }
        DecoderHeadKind::MemoryContext => {
            let plan = phenotype
                .memory_decoder()
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            let episodic = phenotype
                .lobe_layout()
                .region(crate::LobeKind::EpisodicMemory)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            let core = phenotype
                .lobe_layout()
                .region(crate::LobeKind::CoreAssociation)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            if projection_kind != ProjectionKind::MemoryDecoder
                || plan.head() != DecoderHeadKind::MemoryContext
                || coordinate.family() != crate::CandidateActionFamily::Other
                || coordinate.input_lane() >= plan.input_width()
                || coordinate.motor_index() >= plan.output_width()
                || synapse.source() != episodic.start + u32::from(coordinate.input_lane())
                || synapse.target() != core.start + u32::from(coordinate.motor_index())
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
        }
    }
    Ok(())
}

pub(super) fn compute_abi_digests(
    capacity: &BrainCapacityClass,
    projections: &[CompiledProjection],
    synapses: &[CompiledSynapse],
) -> (Blake3Digest, Blake3Digest) {
    if capacity.id() == BrainCapacityClass::N2048_ID {
        return (
            crate::N2048FoundationLayoutV1::route_abi_digest(),
            crate::N2048FoundationLayoutV1::plasticity_abi_digest(),
        );
    }
    let recurrent = projections.iter().filter(|projection| {
        let (start, len) = projection.synapse_range();
        synapses[start as usize..(start + len) as usize]
            .iter()
            .all(|synapse| matches!(synapse.kind(), CompiledSynapseKind::Recurrent))
    });
    let route = crate::foundation::procedural_route_digest(recurrent.clone().map(|projection| {
        (
            projection.source_lobe(),
            projection.target_lobe(),
            projection.synapse_range().1,
        )
    }));
    let plasticity = crate::foundation::procedural_plasticity_digest(recurrent.map(|projection| {
        let (start, len) = projection.synapse_range();
        let alpha_fold = synapses[start as usize..(start + len) as usize]
            .iter()
            .enumerate()
            .fold(0_u32, |fold, (index, synapse)| {
                fold ^ synapse.alpha().to_bits().rotate_left((index % 31) as u32)
            });
        (projection.route_index(), len, alpha_fold)
    }));
    (route, plasticity)
}

pub(super) fn canonical_recurrent_projection(projection: &CompiledProjection) -> bool {
    use crate::{ActiveTilePolicy, BiologicalPriority, LobeKind, ProjectionType, UpdateCadence};
    if let Some(spec) = crate::N2048FoundationLayoutV1::route_specs()
        .iter()
        .copied()
        .find(|spec| {
            spec.source_lobe() == projection.source_lobe()
                && spec.target_lobe() == projection.target_lobe()
        })
    {
        return projection.projection_type() == spec.projection_type()
            && projection.update_cadence() == spec.update_cadence()
            && projection.active_tile_policy() == spec.active_tile_policy()
            && projection.priority() == spec.priority();
    }
    let expected = match (projection.source_lobe(), projection.target_lobe()) {
        (LobeKind::SensoryGrounding, LobeKind::CoreAssociation) => {
            (ProjectionType::FeedForward, UpdateCadence::Hot60Hz)
        }
        (LobeKind::CoreAssociation, LobeKind::MotorArbitration) => {
            (ProjectionType::MotorProposal, UpdateCadence::Hot60Hz)
        }
        (LobeKind::MetabolicDrive, LobeKind::HomeostaticRegulation) => {
            (ProjectionType::Homeostatic, UpdateCadence::Hot10To30Hz)
        }
        (LobeKind::MotorArbitration, LobeKind::MotorArbitration) => {
            (ProjectionType::LateralInhibition, UpdateCadence::Hot60Hz)
        }
        _ => return false,
    };
    projection.projection_type() == expected.0
        && projection.update_cadence() == expected.1
        && projection.active_tile_policy() == ActiveTilePolicy::EssentialReservation
        && projection.priority() == BiologicalPriority::Essential
}
