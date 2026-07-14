//! Contract-only checked per-route and global phenotype budget receipts.

use serde::{Deserialize, Serialize};

use super::BrainCapacityClass;
use crate::{BrainClassId, ScaffoldContractError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteBudgetReceipt {
    pub route_index: u16,
    pub active_tiles: u32,
    pub recurrent_synapses: u32,
    pub action_decoder_synapses: u32,
    pub memory_decoder_synapses: u32,
    pub immutable_payload_words: u32,
    pub tile_ceiling: u32,
    pub synapse_ceiling: u32,
    pub payload_word_ceiling: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalPhenotypeBudgetReceipt {
    pub neuron_count: u32,
    pub active_tiles: u32,
    pub recurrent_synapses: u32,
    pub action_decoder_synapses: u32,
    pub memory_decoder_synapses: u32,
    pub total_synapses: u32,
    pub immutable_payload_words: u32,
    pub candidate_capacity: u16,
    pub object_slot_capacity: u16,
    pub memory_context_capacity: u16,
    pub decoder_input_lanes: u16,
    pub replay_event_capacity: u32,
    pub replay_eligibility_sample_capacity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledBudgets {
    pub capacity_class_id: BrainClassId,
    pub execution_abi_digest: [u64; 4],
    pub routes: Vec<RouteBudgetReceipt>,
    pub global: GlobalPhenotypeBudgetReceipt,
}

impl CompiledBudgets {
    pub fn validate_against(
        &self,
        capacity: &BrainCapacityClass,
    ) -> Result<(), ScaffoldContractError> {
        capacity.validate_contract()?;
        if self.capacity_class_id != capacity.id()
            || self.execution_abi_digest != capacity.canonical_digest()
            || self.routes.is_empty()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }

        let mut active_tiles = 0_u32;
        let mut recurrent_synapses = 0_u32;
        let mut action_decoder_synapses = 0_u32;
        let mut memory_decoder_synapses = 0_u32;
        let mut immutable_payload_words = 0_u32;
        let last_route = self.routes.len() - 1;

        for (index, route) in self.routes.iter().enumerate() {
            if route.route_index != u16::try_from(index).map_err(|_| compile_error())?
                || route.active_tiles > route.tile_ceiling
                || route.immutable_payload_words > route.payload_word_ceiling
            {
                return Err(compile_error());
            }
            let route_synapses = checked_synapse_sum(
                route.recurrent_synapses,
                route.action_decoder_synapses,
                route.memory_decoder_synapses,
            )?;
            if route_synapses > route.synapse_ceiling {
                return Err(compile_error());
            }
            if index == last_route {
                if route.recurrent_synapses != 0 || route.memory_decoder_synapses != 0 {
                    return Err(compile_error());
                }
            } else if route.action_decoder_synapses != 0 || route.memory_decoder_synapses != 0 {
                return Err(compile_error());
            }

            active_tiles = active_tiles
                .checked_add(route.active_tiles)
                .ok_or_else(compile_error)?;
            recurrent_synapses = recurrent_synapses
                .checked_add(route.recurrent_synapses)
                .ok_or_else(compile_error)?;
            action_decoder_synapses = action_decoder_synapses
                .checked_add(route.action_decoder_synapses)
                .ok_or_else(compile_error)?;
            memory_decoder_synapses = memory_decoder_synapses
                .checked_add(route.memory_decoder_synapses)
                .ok_or_else(compile_error)?;
            immutable_payload_words = immutable_payload_words
                .checked_add(route.immutable_payload_words)
                .ok_or_else(compile_error)?;
        }

        let global = &self.global;
        let total_synapses = checked_synapse_sum(
            global.recurrent_synapses,
            global.action_decoder_synapses,
            global.memory_decoder_synapses,
        )?;
        if global.neuron_count != capacity.execution().max_neurons()
            || global.active_tiles != active_tiles
            || global.recurrent_synapses != recurrent_synapses
            || global.action_decoder_synapses != action_decoder_synapses
            || global.memory_decoder_synapses != memory_decoder_synapses
            || global.immutable_payload_words != immutable_payload_words
            || global.total_synapses != total_synapses
            || global.recurrent_synapses > capacity.execution().max_recurrent_synapses()
            || global.action_decoder_synapses > capacity.execution().max_action_decoder_synapses()
            || global.memory_decoder_synapses != 0
            || global.total_synapses > capacity.execution().max_total_synapses()
            || global.active_tiles > capacity.execution().max_active_tiles()
            || global.candidate_capacity > capacity.execution().max_candidates()
            || global.object_slot_capacity > capacity.execution().max_object_slots()
            || global.memory_context_capacity > capacity.execution().max_memory_context_records()
            || global.decoder_input_lanes > capacity.execution().max_decoder_input_lanes()
            || global.replay_event_capacity > capacity.execution().max_replay_events()
            || global.replay_eligibility_sample_capacity
                > capacity.execution().max_replay_eligibility_samples()
        {
            return Err(compile_error());
        }
        Ok(())
    }
}

fn checked_synapse_sum(
    recurrent: u32,
    action_decoder: u32,
    memory_decoder: u32,
) -> Result<u32, ScaffoldContractError> {
    recurrent
        .checked_add(action_decoder)
        .and_then(|sum| sum.checked_add(memory_decoder))
        .ok_or_else(compile_error)
}

const fn compile_error() -> ScaffoldContractError {
    ScaffoldContractError::PhenotypeCompile
}
