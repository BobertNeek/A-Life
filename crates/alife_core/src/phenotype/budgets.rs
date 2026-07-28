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

impl RouteBudgetReceipt {
    pub fn total_synapses(&self) -> Option<u32> {
        self.recurrent_synapses
            .checked_add(self.action_decoder_synapses)?
            .checked_add(self.memory_decoder_synapses)
    }

    pub fn within_ceiling(&self) -> bool {
        self.active_tiles <= self.tile_ceiling
            && self
                .total_synapses()
                .is_some_and(|total| total <= self.synapse_ceiling)
            && self.immutable_payload_words <= self.payload_word_ceiling
            && self.total_synapses() == Some(self.immutable_payload_words)
    }
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
    pub replay_capture_synapse_count: u32,
}

impl GlobalPhenotypeBudgetReceipt {
    pub fn within(&self, execution: &super::BrainExecutionBudget) -> bool {
        let total_synapses = checked_synapse_sum(
            self.recurrent_synapses,
            self.action_decoder_synapses,
            self.memory_decoder_synapses,
        )
        .ok();
        let replay_capture_limit = self
            .replay_eligibility_sample_capacity
            .checked_div(self.replay_event_capacity)
            .unwrap_or(0);

        self.neuron_count != 0
            && self.neuron_count <= execution.max_neurons()
            && self.active_tiles <= execution.max_active_tiles()
            && self.recurrent_synapses <= execution.max_recurrent_synapses()
            && self.action_decoder_synapses <= execution.max_action_decoder_synapses()
            && self.memory_decoder_synapses <= execution.max_memory_decoder_synapses()
            && total_synapses == Some(self.total_synapses)
            && self.total_synapses <= execution.max_total_synapses()
            && self.immutable_payload_words == self.total_synapses
            && self.candidate_capacity <= execution.max_candidates()
            && self.object_slot_capacity <= execution.max_object_slots()
            && self.memory_context_capacity <= execution.max_memory_context_records()
            && self.decoder_input_lanes <= execution.max_decoder_input_lanes()
            && self.replay_event_capacity != 0
            && self.replay_event_capacity <= execution.max_replay_events()
            && self.replay_eligibility_sample_capacity <= execution.max_replay_eligibility_samples()
            && self.replay_capture_synapse_count != 0
            && self.replay_capture_synapse_count <= self.total_synapses
            && self.replay_capture_synapse_count <= replay_capture_limit
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledBudgets {
    pub capacity_class_id: BrainClassId,
    pub execution_abi_digest: [u64; 4],
    pub routes: Vec<RouteBudgetReceipt>,
    pub global: GlobalPhenotypeBudgetReceipt,
}

impl CompiledBudgets {
    pub fn sum_route_synapses(&self) -> Result<u32, ScaffoldContractError> {
        self.routes.iter().try_fold(0_u32, |sum, route| {
            sum.checked_add(route.total_synapses().ok_or_else(compile_error)?)
                .ok_or_else(compile_error)
        })
    }

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
        for (index, route) in self.routes.iter().enumerate() {
            if route.route_index != u16::try_from(index).map_err(|_| compile_error())?
                || !route.within_ceiling()
            {
                return Err(compile_error());
            }
            let populated_categories = [
                route.recurrent_synapses != 0,
                route.action_decoder_synapses != 0,
                route.memory_decoder_synapses != 0,
            ]
            .into_iter()
            .filter(|populated| *populated)
            .count();
            if populated_categories != 1 {
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
        if global.neuron_count != capacity.execution().max_neurons()
            || global.active_tiles != active_tiles
            || global.recurrent_synapses != recurrent_synapses
            || global.action_decoder_synapses != action_decoder_synapses
            || global.memory_decoder_synapses != memory_decoder_synapses
            || global.immutable_payload_words != immutable_payload_words
            || !global.within(capacity.execution())
            || self.sum_route_synapses()? != global.total_synapses
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
