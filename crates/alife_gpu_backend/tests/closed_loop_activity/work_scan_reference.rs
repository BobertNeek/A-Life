//! Frozen structural-accounting reference from 69f00c05ec6774c3655ae87c5d4baa5df3861929.
//! Test/benchmark only. Counts work; never computes neural values.

use alife_core::{
    BrainPhenotype, BrainWorkCounters, CompiledSynapseKind, ScaffoldContractError, UpdateCadence,
};
use std::collections::BTreeSet;

pub fn derive_executed_work(
    phenotype: &BrainPhenotype,
    microsteps: u8,
    enabled_route_ids: &[u16],
    candidate_count: u32,
    memory_context_count: u32,
) -> Result<BrainWorkCounters, ScaffoldContractError> {
    if microsteps == 0 || enabled_route_ids.is_empty() {
        return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
    }
    let enabled = enabled_route_ids.iter().copied().collect::<BTreeSet<_>>();
    if enabled.len() != enabled_route_ids.len()
        || !enabled_route_ids.windows(2).all(|pair| pair[0] < pair[1])
        || enabled.iter().any(|route_id| {
            !phenotype
                .projections()
                .iter()
                .any(|projection| projection.route_index() == *route_id)
        })
    {
        return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
    }

    let mut tile_visits = 0_u64;
    let mut synapse_ops = 0_u64;
    for step in 0..u32::from(microsteps) {
        for projection in phenotype
            .projections()
            .iter()
            .filter(|projection| enabled.contains(&projection.route_index()))
        {
            if !cadence_fires(projection.update_cadence(), step) {
                continue;
            }
            tile_visits = tile_visits
                .checked_add(u64::from(projection.active_tile_count()))
                .ok_or(ScaffoldContractError::BrainActivityPolicyMismatch)?;
            let (start, len) = projection.synapse_range();
            let end = start
                .checked_add(len)
                .ok_or(ScaffoldContractError::BrainActivityPolicyMismatch)?;
            let recurrent_count = phenotype.synapses()[start as usize..end as usize]
                .iter()
                .filter(|synapse| matches!(synapse.kind(), CompiledSynapseKind::Recurrent))
                .count();
            synapse_ops = synapse_ops
                .checked_add(
                    u64::try_from(recurrent_count)
                        .map_err(|_| ScaffoldContractError::BrainActivityPolicyMismatch)?,
                )
                .ok_or(ScaffoldContractError::BrainActivityPolicyMismatch)?;
        }
    }
    let neuron_updates = u64::from(phenotype.neuron_count())
        .checked_mul(u64::from(microsteps))
        .ok_or(ScaffoldContractError::BrainActivityPolicyMismatch)?;
    Ok(BrainWorkCounters {
        microsteps: u32::from(microsteps),
        neuron_updates,
        tile_visits,
        synapse_ops,
        decoder_candidate_ops: u64::from(candidate_count),
        memory_context_ops: u64::from(memory_context_count),
    })
}

const fn cadence_fires(cadence: UpdateCadence, microstep: u32) -> bool {
    match cadence {
        UpdateCadence::Hot60Hz => true,
        UpdateCadence::Hot15To60Hz | UpdateCadence::Hot10To30Hz => microstep.is_multiple_of(2),
        UpdateCadence::Hot5To15Hz | UpdateCadence::Hot1To5Hz => microstep == 0,
        UpdateCadence::SleepOrOffline | UpdateCadence::Disabled => false,
    }
}
