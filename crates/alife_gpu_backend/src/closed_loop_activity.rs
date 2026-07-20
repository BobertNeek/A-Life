//! Backend derivation of executed GPU neural work from a validated route schedule.

use std::collections::BTreeSet;

use alife_core::{
    BrainExecutionBudget, BrainPhenotype, BrainWorkCounters, CompiledSynapseKind,
    NeuralThrottleDecision, ScaffoldContractError, UpdateCadence,
};
use bytemuck::{Pod, Zeroable};

use crate::GpuBrainSlot;

pub const GPU_ACTIVITY_DISPATCH_HEADER_WORDS: usize = 24;
const GPU_ACTIVITY_ROUTE_MASK_WORDS: usize = 8;

/// Exact per-row neural schedule accepted by the recurrent WGSL pass.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct GpuActivityDispatchHeader {
    schema_version: u32,
    policy_version: u32,
    class_id: u32,
    slot: u32,
    slot_generation: u32,
    brain_slot_index: u32,
    microsteps: u32,
    enabled_route_count: u32,
    enabled_route_mask: [u32; GPU_ACTIVITY_ROUTE_MASK_WORDS],
    route_schedule_digest: [u32; 8],
}

impl GpuActivityDispatchHeader {
    pub fn try_from_decision(
        decision: &NeuralThrottleDecision,
        phenotype: &BrainPhenotype,
        execution: &BrainExecutionBudget,
        slot: &GpuBrainSlot,
    ) -> Result<Self, ScaffoldContractError> {
        decision.validate_for(phenotype, execution)?;
        decision.validate_runtime_binding(slot.record().slot, slot.record().slot_generation)?;
        if u32::from(decision.class_id_raw) != slot.record().class_id
            || slot.brain_slot_index() != slot.record().slot
            || decision.microsteps == 0
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        let mut enabled_route_mask = [0_u32; GPU_ACTIVITY_ROUTE_MASK_WORDS];
        for route_id in &decision.enabled_route_ids {
            let route = usize::from(*route_id);
            if route >= GPU_ACTIVITY_ROUTE_MASK_WORDS * u32::BITS as usize {
                return Err(ScaffoldContractError::BrainActivityPolicyMismatch);
            }
            enabled_route_mask[route / u32::BITS as usize] |= 1_u32 << (route % u32::BITS as usize);
        }
        let value = Self {
            schema_version: u32::from(decision.schema_version),
            policy_version: u32::from(decision.policy_version),
            class_id: u32::from(decision.class_id_raw),
            slot: decision.handle_slot,
            slot_generation: decision.handle_generation,
            brain_slot_index: slot.brain_slot_index(),
            microsteps: u32::from(decision.microsteps),
            enabled_route_count: u32::try_from(decision.enabled_route_ids.len())
                .map_err(|_| ScaffoldContractError::BrainActivityPolicyMismatch)?,
            enabled_route_mask,
            route_schedule_digest: decision.route_schedule_digest_words(),
        };
        value.validate_for(decision, phenotype, execution, slot)?;
        Ok(value)
    }

    pub fn validate_for(
        &self,
        decision: &NeuralThrottleDecision,
        phenotype: &BrainPhenotype,
        execution: &BrainExecutionBudget,
        slot: &GpuBrainSlot,
    ) -> Result<(), ScaffoldContractError> {
        decision.validate_for(phenotype, execution)?;
        decision.validate_runtime_binding(slot.record().slot, slot.record().slot_generation)?;
        let expected = Self::try_from_valid_decision(decision, slot)?;
        if *self != expected {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        Ok(())
    }

    fn try_from_valid_decision(
        decision: &NeuralThrottleDecision,
        slot: &GpuBrainSlot,
    ) -> Result<Self, ScaffoldContractError> {
        let mut enabled_route_mask = [0_u32; GPU_ACTIVITY_ROUTE_MASK_WORDS];
        for route_id in &decision.enabled_route_ids {
            let route = usize::from(*route_id);
            if route >= GPU_ACTIVITY_ROUTE_MASK_WORDS * u32::BITS as usize {
                return Err(ScaffoldContractError::BrainActivityPolicyMismatch);
            }
            enabled_route_mask[route / u32::BITS as usize] |= 1_u32 << (route % u32::BITS as usize);
        }
        Ok(Self {
            schema_version: u32::from(decision.schema_version),
            policy_version: u32::from(decision.policy_version),
            class_id: u32::from(decision.class_id_raw),
            slot: decision.handle_slot,
            slot_generation: decision.handle_generation,
            brain_slot_index: slot.brain_slot_index(),
            microsteps: u32::from(decision.microsteps),
            enabled_route_count: u32::try_from(decision.enabled_route_ids.len())
                .map_err(|_| ScaffoldContractError::BrainActivityPolicyMismatch)?,
            enabled_route_mask,
            route_schedule_digest: decision.route_schedule_digest_words(),
        })
    }

    pub fn words(&self) -> &[u32; GPU_ACTIVITY_DISPATCH_HEADER_WORDS] {
        bytemuck::cast_ref(self)
    }

    pub const fn microsteps(&self) -> u8 {
        self.microsteps as u8
    }

    pub const fn class_id(&self) -> u32 {
        self.class_id
    }

    pub const fn slot(&self) -> u32 {
        self.slot
    }

    pub const fn slot_generation(&self) -> u32 {
        self.slot_generation
    }

    pub const fn brain_slot_index(&self) -> u32 {
        self.brain_slot_index
    }

    pub const fn enabled_route_count(&self) -> usize {
        self.enabled_route_count as usize
    }

    pub const fn route_schedule_digest_words(&self) -> [u32; 8] {
        self.route_schedule_digest
    }

    pub fn route_is_enabled(&self, route_id: u16) -> bool {
        let route = usize::from(route_id);
        route < GPU_ACTIVITY_ROUTE_MASK_WORDS * u32::BITS as usize
            && self.enabled_route_mask[route / u32::BITS as usize]
                & (1_u32 << (route % u32::BITS as usize))
                != 0
    }

    #[cfg(feature = "gpu-tests")]
    pub fn tamper_route_schedule_digest_for_hardware_diagnostic(&mut self) {
        self.route_schedule_digest[0] ^= 1;
    }
}

/// Derives the exact operations encoded for one completed neural row. The
/// compact GPU diagnostic counters independently validate tile and synapse
/// totals after execution; this function never computes neural values.
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
