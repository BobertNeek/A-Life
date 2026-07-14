//! Deterministic sensor encoder and candidate decoder plan compilation.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ActiveTilePolicy, BiologicalPriority, BrainGenome, CandidateActionFamily, DevelopmentState,
    LobeKind, LobeLayout, MotorAffordanceKind, ProjectionType, ScaffoldContractError,
    SensorChannelKind, SensorProfile, UpdateCadence, CANDIDATE_FEATURE_COUNT,
};

use super::{
    BrainCapacityClass, CandidateDecoderFamilyPlan, CandidateDecoderPlan, CompiledProjection,
    CompiledSynapse, CompiledSynapseKind, DecoderHeadKind, DecoderSynapseCoordinate,
    RouteBudgetReceipt, SensorEncoderAssignment, SensorEncoderPlan, SensorEncoderSourceGroup,
};

pub(super) fn compile_encoder(
    genome: &BrainGenome,
    development: &DevelopmentState,
    layout: &LobeLayout,
    profile: SensorProfile,
) -> Result<SensorEncoderPlan, ScaffoldContractError> {
    let mut assignments = Vec::new();
    let mut gene_keys = BTreeSet::new();
    let mut active_genes = genome
        .sensor_layout
        .channels
        .iter()
        .filter(|gene| {
            gene.enabled_at_maturation as f32 <= development.maturation.raw() * 100.0
                && (development.active_sensor_channels.is_empty()
                    || development.active_sensor_channels.contains(&gene.kind))
        })
        .collect::<Vec<_>>();
    active_genes.sort_by_key(|gene| {
        (
            splitmix64(
                genome.seeds.sensor_layout_seed
                    ^ u64::from(gene.kind.raw())
                    ^ (u64::from(gene.target_lobe.raw()) << 16),
            ),
            gene.kind.raw(),
            gene.target_lobe.raw(),
        )
    });
    let mut occupied = BTreeSet::new();
    for gene in active_genes {
        if !gene_keys.insert((gene.kind.raw(), gene.target_lobe.raw())) {
            return Err(compile_error());
        }
        let region = layout
            .region(gene.target_lobe)
            .filter(|region| region.enabled)
            .ok_or_else(compile_error)?;
        let (group, lane_start, lane_end) = sensor_lanes(gene.kind);
        let lane_width = lane_end - lane_start;
        let available = region.len * u32::from(lane_width);
        if u32::from(gene.receptor_count) > available {
            return Err(compile_error());
        }
        let seed = genome.seeds.sensor_layout_seed
            ^ u64::from(gene.kind.raw())
            ^ (u64::from(gene.target_lobe.raw()) << 16);
        let mut cursor = (splitmix64(seed) % u64::from(available)) as u32;
        let mut step =
            ((splitmix64(seed ^ 0xA076_1D64_78BD_642F) % u64::from(available)) as u32) | 1;
        while gcd_u32(step, available) != 1 {
            step = (step + 2) % available;
            if step == 0 {
                step = 1;
            }
        }
        for _ in 0..gene.receptor_count {
            let mut selected = None;
            for _ in 0..available {
                let source_index = lane_start + (cursor % u32::from(lane_width)) as u16;
                let target_neuron = region.start + cursor / u32::from(lane_width);
                let key = (target_neuron, group.raw(), source_index);
                cursor = (cursor + step) % available;
                if occupied.insert(key) {
                    selected = Some((source_index, target_neuron));
                    break;
                }
            }
            let (source_index, target_neuron) = selected.ok_or_else(compile_error)?;
            assignments.push(SensorEncoderAssignment::new(
                group,
                source_index,
                target_neuron,
                1.0,
                0.0,
                -1.0,
                1.0,
            ));
        }
    }
    assignments.sort_by_key(|assignment| {
        (
            assignment.target_neuron(),
            assignment.source_group().raw(),
            assignment.source_index(),
        )
    });
    if assignments.windows(2).any(|rows| {
        (
            rows[0].target_neuron(),
            rows[0].source_group().raw(),
            rows[0].source_index(),
        ) == (
            rows[1].target_neuron(),
            rows[1].source_group().raw(),
            rows[1].source_index(),
        )
    }) {
        return Err(compile_error());
    }
    SensorEncoderPlan::try_new(profile, assignments)
}

#[allow(clippy::type_complexity)]
pub(super) fn compile_decoder(
    genome: &BrainGenome,
    development: &DevelopmentState,
    layout: &LobeLayout,
    capacity: &BrainCapacityClass,
    route_index: u16,
    start: u32,
) -> Result<
    (
        CandidateDecoderPlan,
        CompiledProjection,
        Vec<CompiledSynapse>,
        RouteBudgetReceipt,
    ),
    ScaffoldContractError,
> {
    let motor = layout
        .region(LobeKind::MotorArbitration)
        .filter(|region| region.enabled)
        .ok_or_else(compile_error)?;
    let motor_width = u16::try_from(motor.len).map_err(|_| compile_error())?;
    let mut family_units: BTreeMap<u8, Vec<u16>> = (0_u8..8).map(|raw| (raw, Vec::new())).collect();
    family_units
        .get_mut(&CandidateActionFamily::Idle.raw())
        .expect("all action families initialized")
        .push(0);
    let mut cursor = 1_u16;
    let mut seen_genes = BTreeSet::new();
    for gene in &genome.motor_affordances {
        if !gene.enabled
            || gene.enabled_at_maturation as f32 > development.maturation.raw() * 100.0
            || (!development.active_motor_affordances.is_empty()
                && !development.active_motor_affordances.contains(&gene.kind))
        {
            continue;
        }
        if !seen_genes.insert(gene.kind.raw()) {
            return Err(compile_error());
        }
        let families: &[CandidateActionFamily] = match gene.kind {
            MotorAffordanceKind::Move | MotorAffordanceKind::Turn => &[
                CandidateActionFamily::Approach,
                CandidateActionFamily::Avoid,
            ],
            MotorAffordanceKind::Eat => &[CandidateActionFamily::Ingest],
            MotorAffordanceKind::Rest => &[CandidateActionFamily::Rest],
            MotorAffordanceKind::Interact => &[
                CandidateActionFamily::Inspect,
                CandidateActionFamily::Contact,
            ],
            MotorAffordanceKind::Vocalize
            | MotorAffordanceKind::Write
            | MotorAffordanceKind::Gesture
            | MotorAffordanceKind::Reproduce => &[CandidateActionFamily::Other],
        };
        for unit in 0..gene.motor_lobe_units {
            if cursor >= motor_width {
                return Err(compile_error());
            }
            let family = families[usize::from(unit) % families.len()];
            family_units
                .get_mut(&family.raw())
                .expect("all action families initialized")
                .push(cursor);
            cursor += 1;
        }
    }
    let mut synapses = Vec::new();
    let mut family_plans = Vec::new();
    for raw in 0_u8..8 {
        let family = CandidateActionFamily::try_from_raw(raw)?;
        let family_start = start + u32::try_from(synapses.len()).map_err(|_| compile_error())?;
        for input_lane in 0..CANDIDATE_FEATURE_COUNT as u16 {
            for &motor_index in &family_units[&raw] {
                let neuron = motor.start + u32::from(motor_index);
                let coordinate = DecoderSynapseCoordinate::new(
                    DecoderHeadKind::ActionCandidate,
                    family,
                    input_lane,
                    motor_index,
                );
                let weight = genetic_weight(
                    genome.genetic_prior_seed,
                    route_index,
                    neuron,
                    u32::from(input_lane),
                );
                let alpha = super::topology_compile::decoder_alpha(genome, motor.start, neuron);
                synapses.push(CompiledSynapse::new(
                    neuron,
                    neuron,
                    weight,
                    alpha,
                    route_index,
                    CompiledSynapseKind::Decoder(coordinate),
                ));
            }
        }
        let count =
            start + u32::try_from(synapses.len()).map_err(|_| compile_error())? - family_start;
        family_plans.push(CandidateDecoderFamilyPlan::new(
            family,
            0.0,
            family_start,
            count,
        ));
    }
    let len = u32::try_from(synapses.len()).map_err(|_| compile_error())?;
    if len == 0 || len > capacity.execution().max_action_decoder_synapses() {
        return Err(compile_error());
    }
    let decoder = CandidateDecoderPlan::try_new(
        motor.start,
        motor_width,
        CANDIDATE_FEATURE_COUNT as u16,
        CANDIDATE_FEATURE_COUNT as u16,
        family_plans,
    )?;
    let projection = CompiledProjection::new(
        route_index,
        LobeKind::MotorArbitration,
        LobeKind::MotorArbitration,
        ProjectionType::MotorProposal,
        ActiveTilePolicy::EssentialReservation,
        UpdateCadence::Hot60Hz,
        BiologicalPriority::Essential,
        0,
        start,
        len,
        0,
    );
    let receipt = RouteBudgetReceipt {
        route_index,
        active_tiles: 0,
        recurrent_synapses: 0,
        action_decoder_synapses: len,
        memory_decoder_synapses: 0,
        immutable_payload_words: len,
        tile_ceiling: 0,
        synapse_ceiling: len,
        payload_word_ceiling: len,
    };
    Ok((decoder, projection, synapses, receipt))
}

fn sensor_lanes(kind: SensorChannelKind) -> (SensorEncoderSourceGroup, u16, u16) {
    match kind {
        SensorChannelKind::Vision | SensorChannelKind::GlyphVision => {
            (SensorEncoderSourceGroup::SensoryChannel, 0, 16)
        }
        SensorChannelKind::Hearing => (SensorEncoderSourceGroup::SensoryChannel, 16, 24),
        SensorChannelKind::Smell | SensorChannelKind::Taste => {
            (SensorEncoderSourceGroup::SensoryChannel, 24, 32)
        }
        SensorChannelKind::Touch => (SensorEncoderSourceGroup::SensoryChannel, 32, 40),
        SensorChannelKind::Proprioception => (SensorEncoderSourceGroup::Body, 0, 13),
        SensorChannelKind::Interoception => (SensorEncoderSourceGroup::Homeostasis, 0, 22),
    }
}

fn genetic_weight(seed: u64, route: u16, source: u32, target: u32) -> f32 {
    let bits =
        splitmix64(seed ^ (u64::from(route) << 48) ^ (u64::from(source) << 16) ^ u64::from(target));
    0.02 + ((bits >> 40) as f32 / ((1_u32 << 24) - 1) as f32) * 0.23
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

const fn gcd_u32(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

const fn compile_error() -> ScaffoldContractError {
    ScaffoldContractError::PhenotypeCompile
}
