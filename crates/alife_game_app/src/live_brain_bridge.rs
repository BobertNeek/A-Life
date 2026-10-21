//! Live runtime results projected into the frontend.

use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveBrainCausalStage {
    EvaluateSleep,
    AdvanceSleep,
    GatherSensory,
    RecallMemory,
    CpuBrainTick,
    GpuBrainTick,
    ExecuteAction,
    MeasureOutcome,
    SealPatch,
    ApplyLearning,
    ObserveMemory,
    ObserveTopology,
    UpdateLogs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveCognitivePresentationSnapshot {
    pub organism_id: OrganismId,
    pub brain_class_id: Option<u16>,
    pub brain_neuron_count: Option<u32>,
    pub fast_memory_count: Option<u32>,
    pub lifetime_memory_count: Option<u32>,
    pub concept_count: Option<u32>,
    pub unresolved_gap_count: Option<u32>,
    pub learning_active: Option<bool>,
    pub sleep_phase_raw: Option<u16>,
    pub consolidation_state_raw: Option<u16>,
    pub last_consolidated_tick: Option<u64>,
    pub topology_update_count: Option<u32>,
}

/// Passive projection of the contract-bounded motor bundle and world receipts.
/// Kept only in the current presentation tick; never saved or fed back to cognition.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveMotorExecutionTrace {
    pub requested_channels: Vec<alife_core::ChannelCommand>,
    pub channel_receipts: Vec<alife_world::HeadlessMotorChannelReceipt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveBrainTickSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub tick_before: Tick,
    pub tick_after: Tick,
    pub world_tick_before: Tick,
    pub world_tick_after: Tick,
    pub status: BrainTickStatus,
    pub selected_action_kind: Option<ActionKind>,
    pub selected_action_id: Option<ActionId>,
    pub target_entity: Option<WorldEntityId>,
    pub patch_sealed: bool,
    pub patch_sequence_id: Option<u64>,
    pub patch_success: Option<bool>,
    pub physical_contact: Option<PhysicalContactKind>,
    pub action_failure: Option<ReferenceActionFailure>,
    /// None means no joint motor receipt is available, not an empty motor command.
    pub motor_execution: Option<LiveMotorExecutionTrace>,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
    pub memory_updates: u32,
    pub topology_updates: u32,
    pub learning_updates: u32,
    pub invalid_or_rejected_action_count: u32,
    pub last_diagnostic: Option<ContractDiagnostic>,
    pub causal_stages: Vec<LiveBrainCausalStage>,
}
