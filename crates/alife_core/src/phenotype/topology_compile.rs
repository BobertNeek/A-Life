//! Canonical recurrent route admission, sparse coordinates, weights, and alpha policy.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ActiveTilePolicy, BiologicalPriority, BrainGenome, LobeKind, LobeLayout, ProjectionKey,
    ProjectionType, RoutingMask, ScaffoldContractError, UpdateCadence,
};

use super::{
    BrainCapacityClass, CompiledProjection, CompiledSynapse, CompiledSynapseKind,
    RouteBudgetReceipt,
};

type RecurrentCompilation = (
    Vec<CompiledProjection>,
    Vec<CompiledSynapse>,
    Vec<RouteBudgetReceipt>,
);

pub(super) fn compile_recurrent(
    genome: &BrainGenome,
    layout: &LobeLayout,
    capacity: &BrainCapacityClass,
) -> Result<RecurrentCompilation, ScaffoldContractError> {
    let mut masks = BTreeMap::new();
    for mask in &genome.macro_connectome_masks {
        let key = (
            mask.projection.source_lobe.raw(),
            mask.projection.target_lobe.raw(),
        );
        if masks.insert(key, mask).is_some()
            || canonical_route(mask.projection).is_none()
            || mask.structural_growth_allowed
        {
            return Err(compile_error());
        }
    }
    let mut densities = BTreeMap::new();
    for density in &genome.sparse_density_priors {
        let key = (
            density.projection.source_lobe.raw(),
            density.projection.target_lobe.raw(),
        );
        if density.density.raw() <= 0.0
            || density.max_active_synapse_share.raw() <= 0.0
            || densities.insert(key, density).is_some()
            || masks.get(&key).is_none_or(|mask| !mask.enabled)
        {
            return Err(compile_error());
        }
    }
    for (key, mask) in &masks {
        if mask.enabled != densities.contains_key(key) {
            return Err(compile_error());
        }
    }
    validate_alpha_keys(genome)?;
    if capacity.id() == BrainCapacityClass::N2048_ID {
        return compile_n2048_foundation(genome, layout, capacity, &masks, &densities);
    }
    let mut enabled = masks
        .values()
        .filter(|mask| mask.enabled)
        .map(|mask| {
            (
                canonical_route(mask.projection).expect("validated canonical route"),
                densities[&(
                    mask.projection.source_lobe.raw(),
                    mask.projection.target_lobe.raw(),
                )],
            )
        })
        .collect::<Vec<_>>();
    enabled.sort_by_key(|(route, _)| {
        (
            route.priority.raw(),
            route.source_lobe.raw(),
            route.target_lobe.raw(),
        )
    });

    let mut projections = Vec::new();
    let mut synapses = Vec::new();
    let mut receipts = Vec::new();
    let execution = capacity.execution();
    let mut tile_total = 0_u32;
    for (route_index, (route, density)) in enabled.into_iter().enumerate() {
        let source = layout
            .region(route.source_lobe)
            .filter(|region| region.enabled)
            .ok_or_else(compile_error)?;
        let target = layout
            .region(route.target_lobe)
            .filter(|region| region.enabled)
            .ok_or_else(compile_error)?;
        let possible = u64::from(source.len) * u64::from(target.len);
        let density_count = (possible as f64 * f64::from(density.density.raw())).ceil() as u32;
        let share_count = (f64::from(execution.max_recurrent_synapses())
            * f64::from(density.max_active_synapse_share.raw()))
        .floor() as u32;
        let remaining = execution
            .max_recurrent_synapses()
            .saturating_sub(synapses.len() as u32);
        let count = density_count.max(1).min(share_count.max(1)).min(remaining);
        if count == 0 {
            return Err(compile_error());
        }
        let active_tiles = count.div_ceil(256).max(1);
        if tile_total + active_tiles > execution.max_active_tiles() {
            return Err(compile_error());
        }
        tile_total += active_tiles;
        let route_index = u16::try_from(route_index).map_err(|_| compile_error())?;
        let start = u32::try_from(synapses.len()).map_err(|_| compile_error())?;
        let mut rows = deterministic_tile_pairs(
            source.start,
            source.len,
            target.start,
            target.len,
            count,
            genome.genetic_prior_seed ^ u64::from(route_index),
            route.active_tile_policy,
        );
        rows.sort_unstable();
        for (source_neuron, target_neuron) in rows {
            let weight = genetic_weight(
                genome.genetic_prior_seed,
                route_index,
                source_neuron,
                target_neuron,
                route.projection_type,
            );
            let alpha = alpha_for(genome, route, source_neuron, target_neuron, target.start);
            synapses.push(CompiledSynapse::new(
                source_neuron,
                target_neuron,
                weight,
                alpha,
                route_index,
                CompiledSynapseKind::Recurrent,
            ));
        }
        let len = u32::try_from(synapses.len()).map_err(|_| compile_error())? - start;
        projections.push(CompiledProjection::new(
            route_index,
            route.source_lobe,
            route.target_lobe,
            route.projection_type,
            route.active_tile_policy,
            route.update_cadence,
            route.priority,
            0,
            start,
            len,
            active_tiles,
        ));
        receipts.push(RouteBudgetReceipt {
            route_index,
            active_tiles,
            recurrent_synapses: len,
            action_decoder_synapses: 0,
            memory_decoder_synapses: 0,
            immutable_payload_words: len,
            tile_ceiling: active_tiles,
            synapse_ceiling: len,
            payload_word_ceiling: len,
        });
    }
    Ok((projections, synapses, receipts))
}

fn compile_n2048_foundation(
    genome: &BrainGenome,
    layout: &LobeLayout,
    capacity: &BrainCapacityClass,
    masks: &BTreeMap<(u16, u16), &crate::MacroConnectomeMask>,
    densities: &BTreeMap<(u16, u16), &crate::SparseDensityPrior>,
) -> Result<RecurrentCompilation, ScaffoldContractError> {
    let specs = crate::N2048FoundationLayoutV1::route_specs();
    if layout != &crate::N2048FoundationLayoutV1::lobe_layout()
        || masks.len() != specs.len()
        || densities.len() != specs.len()
        || capacity.execution().max_recurrent_synapses()
            != crate::N2048FoundationLayoutV1::RECURRENT_SYNAPSE_COUNT
    {
        return Err(compile_error());
    }

    let mut projections = Vec::with_capacity(specs.len());
    let mut synapses =
        Vec::with_capacity(crate::N2048FoundationLayoutV1::RECURRENT_SYNAPSE_COUNT as usize);
    let mut receipts = Vec::with_capacity(specs.len());
    let mut tile_total = 0_u32;
    for (index, spec) in specs.iter().copied().enumerate() {
        let key = (spec.source_lobe().raw(), spec.target_lobe().raw());
        let mask = masks.get(&key).ok_or_else(compile_error)?;
        let density = densities.get(&key).ok_or_else(compile_error)?;
        if !mask.enabled || mask.structural_growth_allowed {
            return Err(compile_error());
        }
        let source = layout
            .region(spec.source_lobe())
            .filter(|region| region.enabled)
            .ok_or_else(compile_error)?;
        let target = layout
            .region(spec.target_lobe())
            .filter(|region| region.enabled)
            .ok_or_else(compile_error)?;
        let route_index = u16::try_from(index).map_err(|_| compile_error())?;
        let start = u32::try_from(synapses.len()).map_err(|_| compile_error())?;
        let active_tiles = spec.synapse_count().div_ceil(256).max(1);
        tile_total = tile_total
            .checked_add(active_tiles)
            .ok_or_else(compile_error)?;
        if tile_total > capacity.execution().max_active_tiles() {
            return Err(compile_error());
        }
        let route = RoutingMask::new(
            spec.source_lobe(),
            spec.target_lobe(),
            spec.projection_type(),
            spec.active_tile_policy(),
            spec.update_cadence(),
            spec.priority(),
        );
        let mut rows = deterministic_tile_pairs(
            source.start,
            source.len,
            target.start,
            target.len,
            spec.synapse_count(),
            genome.genetic_prior_seed
                ^ u64::from(route_index)
                ^ u64::from(density.density.raw().to_bits()).rotate_left(19),
            spec.active_tile_policy(),
        );
        rows.sort_unstable();
        for (source_neuron, target_neuron) in rows {
            let weight = genetic_weight(
                genome.genetic_prior_seed,
                route_index,
                source_neuron,
                target_neuron,
                spec.projection_type(),
            );
            let alpha = alpha_for(genome, route, source_neuron, target_neuron, target.start);
            synapses.push(CompiledSynapse::new(
                source_neuron,
                target_neuron,
                weight,
                alpha,
                route_index,
                CompiledSynapseKind::Recurrent,
            ));
        }
        let len = u32::try_from(synapses.len()).map_err(|_| compile_error())? - start;
        if len != spec.synapse_count() {
            return Err(compile_error());
        }
        projections.push(CompiledProjection::new(
            route_index,
            spec.source_lobe(),
            spec.target_lobe(),
            spec.projection_type(),
            spec.active_tile_policy(),
            spec.update_cadence(),
            spec.priority(),
            0,
            start,
            len,
            active_tiles,
        ));
        receipts.push(RouteBudgetReceipt {
            route_index,
            active_tiles,
            recurrent_synapses: len,
            action_decoder_synapses: 0,
            memory_decoder_synapses: 0,
            immutable_payload_words: len,
            tile_ceiling: active_tiles,
            synapse_ceiling: len,
            payload_word_ceiling: len,
        });
    }
    if synapses.len() != crate::N2048FoundationLayoutV1::RECURRENT_SYNAPSE_COUNT as usize {
        return Err(compile_error());
    }
    Ok((projections, synapses, receipts))
}

pub(super) fn decoder_alpha(genome: &BrainGenome, motor_start: u32, neuron: u32) -> f32 {
    let route = RoutingMask::new(
        LobeKind::MotorArbitration,
        LobeKind::MotorArbitration,
        ProjectionType::MotorProposal,
        ActiveTilePolicy::EssentialReservation,
        UpdateCadence::Hot60Hz,
        BiologicalPriority::Essential,
    );
    alpha_for(genome, route, neuron, neuron, motor_start)
}

fn canonical_route(key: ProjectionKey) -> Option<RoutingMask> {
    if let Some(spec) = crate::N2048FoundationLayoutV1::route_specs()
        .iter()
        .copied()
        .find(|spec| spec.source_lobe() == key.source_lobe && spec.target_lobe() == key.target_lobe)
    {
        return Some(RoutingMask::new(
            spec.source_lobe(),
            spec.target_lobe(),
            spec.projection_type(),
            spec.active_tile_policy(),
            spec.update_cadence(),
            spec.priority(),
        ));
    }
    let (projection_type, cadence) = match (key.source_lobe, key.target_lobe) {
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
        _ => return None,
    };
    Some(RoutingMask::new(
        key.source_lobe,
        key.target_lobe,
        projection_type,
        ActiveTilePolicy::EssentialReservation,
        cadence,
        BiologicalPriority::Essential,
    ))
}

fn deterministic_tile_pairs(
    source_start: u32,
    source_len: u32,
    target_start: u32,
    target_len: u32,
    count: u32,
    seed: u64,
    policy: ActiveTilePolicy,
) -> Vec<(u32, u32)> {
    let source_tiles = source_len / 16;
    let target_tiles = target_len / 16;
    let available_tiles = u64::from(source_tiles) * u64::from(target_tiles);
    let tile_count = u64::from(count.div_ceil(256).max(1));
    debug_assert!(tile_count <= available_tiles);
    let policy_seed = seed ^ (u64::from(policy.raw()) << 56);
    let mut tile_step = (splitmix64(policy_seed) % available_tiles) | 1;
    while gcd(tile_step, available_tiles) != 1 {
        tile_step = (tile_step + 2) % available_tiles;
        if tile_step == 0 {
            tile_step = 1;
        }
    }
    let tile_start = splitmix64(policy_seed ^ 0xD1B5_4A32_D192_ED03) % available_tiles;
    let tiles = (0..tile_count)
        .map(|index| (tile_start + index * tile_step) % available_tiles)
        .collect::<Vec<_>>();
    (0..count)
        .map(|index| {
            let tile = tiles[index as usize % tiles.len()];
            let within = u64::from(index) / tile_count;
            let source_tile = tile / u64::from(target_tiles);
            let target_tile = tile % u64::from(target_tiles);
            (
                source_start + source_tile as u32 * 16 + (within / 16) as u32,
                target_start + target_tile as u32 * 16 + (within % 16) as u32,
            )
        })
        .collect()
}

fn genetic_weight(
    seed: u64,
    route: u16,
    source: u32,
    target: u32,
    projection: ProjectionType,
) -> f32 {
    let bits =
        splitmix64(seed ^ (u64::from(route) << 48) ^ (u64::from(source) << 16) ^ u64::from(target));
    let magnitude = 0.02 + ((bits >> 40) as f32 / ((1_u32 << 24) - 1) as f32) * 0.23;
    match projection {
        ProjectionType::LateralInhibition => -magnitude,
        ProjectionType::Homeostatic | ProjectionType::MotorProposal => magnitude,
        _ if bits & 1 == 0 => magnitude,
        _ => -magnitude,
    }
}

fn validate_alpha_keys(genome: &BrainGenome) -> Result<(), ScaffoldContractError> {
    let mut projections = BTreeSet::new();
    for row in &genome.alpha_mask.projection_overrides {
        if !projections.insert((
            row.projection.source_lobe.raw(),
            row.projection.target_lobe.raw(),
        )) {
            return Err(compile_error());
        }
    }
    let mut lobes = BTreeSet::new();
    for row in &genome.alpha_mask.lobe_overrides {
        if !lobes.insert(row.lobe.raw()) {
            return Err(compile_error());
        }
    }
    let mut tiles = BTreeSet::new();
    for row in &genome.alpha_mask.tile_overrides {
        if !tiles.insert((row.tile.lobe.raw(), row.tile.tile_index)) {
            return Err(compile_error());
        }
    }
    let mut synapses = BTreeSet::new();
    for row in &genome.alpha_mask.per_synapse_overrides {
        if !synapses.insert((row.synapse.source.0, row.synapse.target.0)) {
            return Err(compile_error());
        }
    }
    Ok(())
}

fn alpha_for(
    genome: &BrainGenome,
    route: RoutingMask,
    source: u32,
    target: u32,
    target_start: u32,
) -> f32 {
    if let Some(row) = genome
        .alpha_mask
        .per_synapse_overrides
        .iter()
        .find(|row| row.synapse.source.0 == source && row.synapse.target.0 == target)
    {
        return row.alpha.raw();
    }
    let tile = (target - target_start) / 16;
    if let Some(row) = genome
        .alpha_mask
        .tile_overrides
        .iter()
        .find(|row| row.tile.lobe == route.target_lobe && row.tile.tile_index == tile)
    {
        return row.alpha.raw();
    }
    if let Some(row) = genome
        .alpha_mask
        .projection_overrides
        .iter()
        .find(|row| row.projection == ProjectionKey::new(route.source_lobe, route.target_lobe))
    {
        return row.alpha.raw();
    }
    if let Some(row) = genome
        .alpha_mask
        .lobe_overrides
        .iter()
        .find(|row| row.lobe == route.target_lobe)
    {
        return row.alpha.raw();
    }
    if let Some(row) = genome
        .alpha_mask
        .lobe_overrides
        .iter()
        .find(|row| row.lobe == route.source_lobe)
    {
        return row.alpha.raw();
    }
    genome.alpha_mask.default_alpha.raw()
}

pub(super) fn validate_alpha_matches(
    genome: &BrainGenome,
    projections: &[CompiledProjection],
    synapses: &[CompiledSynapse],
    layout: &LobeLayout,
) -> Result<(), ScaffoldContractError> {
    for row in &genome.alpha_mask.projection_overrides {
        if !projections.iter().any(|projection| {
            projection.source_lobe() == row.projection.source_lobe
                && projection.target_lobe() == row.projection.target_lobe
        }) {
            return Err(compile_error());
        }
    }
    for row in &genome.alpha_mask.lobe_overrides {
        if !synapses.iter().any(|synapse| {
            layout
                .lobe_by_neuron_index(synapse.source())
                .is_some_and(|lobe| lobe.kind == row.lobe)
                || layout
                    .lobe_by_neuron_index(synapse.target())
                    .is_some_and(|lobe| lobe.kind == row.lobe)
        }) {
            return Err(compile_error());
        }
    }
    for row in &genome.alpha_mask.tile_overrides {
        let region = layout
            .region(row.tile.lobe)
            .filter(|region| region.enabled)
            .ok_or_else(compile_error)?;
        if row.tile.tile_index >= region.len / 16
            || !synapses.iter().any(|synapse| {
                region.contains_neuron(synapse.target())
                    && (synapse.target() - region.start) / 16 == row.tile.tile_index
            })
        {
            return Err(compile_error());
        }
    }
    for row in &genome.alpha_mask.per_synapse_overrides {
        if !synapses.iter().any(|synapse| {
            synapse.source() == row.synapse.source.0 && synapse.target() == row.synapse.target.0
        }) {
            return Err(compile_error());
        }
    }
    Ok(())
}

const fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

const fn compile_error() -> ScaffoldContractError {
    ScaffoldContractError::PhenotypeCompile
}
