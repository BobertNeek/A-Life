//! Sleep scheduling, durable GPU consolidation, and bounded memory, predictor,
//! and topology replay. Legacy report types remain readable for checkpoints.

use std::collections::BTreeSet;

use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

mod gpu_contracts;
pub use gpu_contracts::*;
mod replay;
pub use replay::*;

use crate::{
    predictive::GroundedSuccessorPredictor, require_current_version, validate_finite,
    CanonicalDigestBuilder, ChemistryModulation, Confidence, DurationTicks, HomeostaticParameters,
    HomeostaticSnapshot, MemoryBank, MemoryId, NormalizedScalar, ProjectionRoutingRef,
    RecoveryTrigger, ScaffoldContractError, SchemaKind, SchemaVersions, Tick, TopologicalMap,
    TopologySidecar, Validate,
};

pub const SLEEP_CONSOLIDATION_SCHEMA_VERSION: u16 = SchemaVersions::CURRENT.sleep_consolidation.0;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SleepPhase {
    Awake = 1,
    EnteringSleep = 2,
    Consolidating = 3,
    Waking = 4,
    ForcedRecoverySleep = 5,
}

impl SleepPhase {
    pub const fn raw(self) -> u16 {
        self as u16
    }

    pub fn try_from_raw(raw: u16) -> Result<Self, ScaffoldContractError> {
        match raw {
            1 => Ok(Self::Awake),
            2 => Ok(Self::EnteringSleep),
            3 => Ok(Self::Consolidating),
            4 => Ok(Self::Waking),
            5 => Ok(Self::ForcedRecoverySleep),
            _ => Err(ScaffoldContractError::ScalarOutOfRange),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SleepTrigger {
    FatigueThreshold,
    ForcedRequest,
    RecoveryProtocol,
    SeizureHyperactivity,
    CatatoniaEnergyHypoplasia,
    ExtremeFatigue,
    UnsafeActiveState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleepTransition {
    pub from: SleepPhase,
    pub to: SleepPhase,
    pub tick: Tick,
    pub trigger: SleepTrigger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SleepState {
    pub schema_version: u16,
    pub phase: SleepPhase,
    pub phase_started_tick: Tick,
    pub entered_sleep_tick: Option<Tick>,
    pub cycles_completed: u32,
    pub last_trigger: Option<SleepTrigger>,
    pub active_cycle_id: u64,
    pub last_consolidated_cycle_id: u64,
    pub consolidation: ConsolidationState,
}

impl SleepState {
    pub const fn awake_at(tick: Tick) -> Self {
        Self {
            schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
            phase: SleepPhase::Awake,
            phase_started_tick: tick,
            entered_sleep_tick: None,
            cycles_completed: 0,
            last_trigger: None,
            active_cycle_id: 0,
            last_consolidated_cycle_id: 0,
            consolidation: ConsolidationState::None,
        }
    }
}

impl<'de> Deserialize<'de> for SleepState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            phase: SleepPhase,
            phase_started_tick: Tick,
            entered_sleep_tick: Option<Tick>,
            cycles_completed: u32,
            last_trigger: Option<SleepTrigger>,
            #[serde(default)]
            active_cycle_id: u64,
            #[serde(default)]
            last_consolidated_cycle_id: u64,
            #[serde(default)]
            consolidation: ConsolidationState,
        }

        let wire = Wire::deserialize(deserializer)?;
        let migrated_last = if wire.last_consolidated_cycle_id == 0 && wire.cycles_completed != 0 {
            u64::from(wire.cycles_completed)
        } else {
            wire.last_consolidated_cycle_id
        };
        let migrated_active = if wire.active_cycle_id == 0 && wire.phase != SleepPhase::Awake {
            migrated_last.saturating_add(1)
        } else {
            wire.active_cycle_id
        };
        let state = Self {
            schema_version: wire.schema_version,
            phase: wire.phase,
            phase_started_tick: wire.phase_started_tick,
            entered_sleep_tick: wire.entered_sleep_tick,
            cycles_completed: wire.cycles_completed,
            last_trigger: wire.last_trigger,
            active_cycle_id: migrated_active,
            last_consolidated_cycle_id: migrated_last,
            consolidation: wire.consolidation,
        };
        state.validate_contract().map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl Validate for SleepState {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        require_current_version(SchemaKind::SleepConsolidation, self.schema_version)?;
        if let Some(entered) = self.entered_sleep_tick {
            Tick::validate_monotonic(entered, self.phase_started_tick)?;
        }
        if self.last_consolidated_cycle_id > u64::from(self.cycles_completed)
            || (self.phase == SleepPhase::Awake
                && (self.active_cycle_id != 0 || self.consolidation != ConsolidationState::None))
            || (self.phase != SleepPhase::Awake && self.active_cycle_id == 0)
            || (!matches!(self.phase, SleepPhase::Consolidating | SleepPhase::Waking)
                && self.consolidation != ConsolidationState::None)
            || (self.phase == SleepPhase::Waking
                && !matches!(self.consolidation, ConsolidationState::Committed { .. }))
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        if matches!(self.phase, SleepPhase::Consolidating | SleepPhase::Waking) {
            self.consolidation
                .validate_for_cycle(self.active_cycle_id)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SleepConsolidationConfig {
    pub schema_version: u16,
    pub fatigue_threshold: NormalizedScalar,
    pub sleep_pressure_threshold: NormalizedScalar,
    pub entering_duration: DurationTicks,
    pub waking_duration: DurationTicks,
    pub forced_recovery_min_duration: DurationTicks,
    pub memory_max_records_after: usize,
    pub concept_simplex_consolidation_limit: usize,
}

impl SleepConsolidationConfig {
    pub const fn reference() -> Self {
        Self {
            schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
            fatigue_threshold: NormalizedScalar(0.9),
            sleep_pressure_threshold: NormalizedScalar(0.85),
            entering_duration: DurationTicks(3),
            waking_duration: DurationTicks(3),
            forced_recovery_min_duration: DurationTicks(8),
            memory_max_records_after: 64,
            concept_simplex_consolidation_limit: 64,
        }
    }
}

impl Validate for SleepConsolidationConfig {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        require_current_version(SchemaKind::SleepConsolidation, self.schema_version)?;
        NormalizedScalar::new(self.fatigue_threshold.raw())?;
        NormalizedScalar::new(self.sleep_pressure_threshold.raw())?;
        if self.entering_duration.raw() == 0
            || self.waking_duration.raw() == 0
            || self.forced_recovery_min_duration.raw() == 0
            || self.memory_max_records_after == 0
            || self.concept_simplex_consolidation_limit == 0
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SleepController {
    config: SleepConsolidationConfig,
    state: SleepState,
}

impl SleepController {
    pub fn new(config: SleepConsolidationConfig) -> Result<Self, ScaffoldContractError> {
        config.validate_contract()?;
        Ok(Self {
            config,
            state: SleepState::awake_at(Tick::ZERO),
        })
    }

    /// Restores an already validated durable sleep transaction without
    /// replaying phase transitions or manufacturing consolidation progress.
    pub fn restore(
        config: SleepConsolidationConfig,
        state: SleepState,
    ) -> Result<Self, ScaffoldContractError> {
        config.validate_contract()?;
        state.validate_contract()?;
        Ok(Self { config, state })
    }

    pub const fn config(&self) -> SleepConsolidationConfig {
        self.config
    }

    pub const fn state(&self) -> SleepState {
        self.state
    }

    pub fn evaluate_homeostasis(
        &mut self,
        homeostasis: &HomeostaticSnapshot,
        parameters: HomeostaticParameters,
        tick: Tick,
    ) -> Result<Option<SleepTransition>, ScaffoldContractError> {
        homeostasis.validate_contract()?;
        parameters.validate_contract()?;
        Tick::validate_monotonic(self.state.phase_started_tick, tick)?;
        if self.state.phase != SleepPhase::Awake {
            return Ok(None);
        }

        let recovery = ChemistryModulation::recovery_triggers(homeostasis, parameters)?;
        let (phase, trigger) = if recovery.contains(RecoveryTrigger::SeizureHyperactivity) {
            (
                SleepPhase::ForcedRecoverySleep,
                SleepTrigger::SeizureHyperactivity,
            )
        } else if recovery.contains(RecoveryTrigger::CatatoniaEnergyHypoplasia) {
            (
                SleepPhase::ForcedRecoverySleep,
                SleepTrigger::CatatoniaEnergyHypoplasia,
            )
        } else if homeostasis.drives.fatigue >= self.config.fatigue_threshold.raw()
            || homeostasis.hormones.sleep_pressure >= self.config.sleep_pressure_threshold.raw()
        {
            (SleepPhase::EnteringSleep, SleepTrigger::FatigueThreshold)
        } else {
            return Ok(None);
        };

        Ok(Some(self.transition_to(phase, tick, trigger)?))
    }

    pub fn force_sleep(
        &mut self,
        tick: Tick,
        trigger: SleepTrigger,
    ) -> Result<SleepTransition, ScaffoldContractError> {
        Tick::validate_monotonic(self.state.phase_started_tick, tick)?;
        let trigger = match trigger {
            SleepTrigger::ForcedRequest
            | SleepTrigger::RecoveryProtocol
            | SleepTrigger::UnsafeActiveState
            | SleepTrigger::SeizureHyperactivity
            | SleepTrigger::CatatoniaEnergyHypoplasia
            | SleepTrigger::ExtremeFatigue => trigger,
            SleepTrigger::FatigueThreshold => SleepTrigger::ForcedRequest,
        };
        self.transition_to(SleepPhase::ForcedRecoverySleep, tick, trigger)
    }

    pub fn advance(
        &mut self,
        tick: Tick,
    ) -> Result<Option<SleepTransition>, ScaffoldContractError> {
        Tick::validate_monotonic(self.state.phase_started_tick, tick)?;
        let elapsed = tick
            .raw()
            .saturating_sub(self.state.phase_started_tick.raw())
            .min(u64::from(u32::MAX)) as u32;
        let next_phase = match self.state.phase {
            SleepPhase::Awake => None,
            SleepPhase::EnteringSleep if elapsed >= self.config.entering_duration.raw() => {
                Some(SleepPhase::Consolidating)
            }
            SleepPhase::Consolidating
                if matches!(
                    self.state.consolidation,
                    ConsolidationState::Committed { .. }
                ) =>
            {
                Some(SleepPhase::Waking)
            }
            SleepPhase::Waking if elapsed >= self.config.waking_duration.raw() => {
                Some(SleepPhase::Awake)
            }
            SleepPhase::ForcedRecoverySleep
                if elapsed >= self.config.forced_recovery_min_duration.raw() =>
            {
                Some(SleepPhase::Consolidating)
            }
            _ => None,
        };

        let Some(next_phase) = next_phase else {
            return Ok(None);
        };
        let trigger = self
            .state
            .last_trigger
            .unwrap_or(SleepTrigger::RecoveryProtocol);
        Ok(Some(self.transition_to(next_phase, tick, trigger)?))
    }

    pub fn apply_consolidation_driver_event(
        &mut self,
        event: ConsolidationDriverEvent,
    ) -> Result<(), ScaffoldContractError> {
        if self.state.phase != SleepPhase::Consolidating || self.state.active_cycle_id == 0 {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let active_cycle_id = self.state.active_cycle_id;
        let next = match (self.state.consolidation, event) {
            (
                ConsolidationState::None,
                ConsolidationDriverEvent::ReplayAssetPersisted {
                    intent,
                    replay_digest,
                    replay_event_count,
                    replay_eligibility_sample_count,
                },
            ) if intent.cycle_id == active_cycle_id => ConsolidationState::Pending {
                intent,
                replay_digest,
                replay_event_count,
                replay_eligibility_sample_count,
            },
            (
                ConsolidationState::Pending {
                    intent,
                    replay_digest,
                    replay_event_count,
                    replay_eligibility_sample_count,
                },
                ConsolidationDriverEvent::Prepared { request },
            ) if request.cycle_id == intent.cycle_id
                && request.replay_digest == replay_digest
                && replay_event_count <= request.max_replay_events
                && replay_eligibility_sample_count <= request.max_replay_eligibility_samples =>
            {
                request.validate_contract()?;
                ConsolidationState::Prepared { request }
            }
            (
                ConsolidationState::Prepared { request: expected },
                ConsolidationDriverEvent::Submitted { request, job_id },
            ) if request == expected => ConsolidationState::Submitted { request, job_id },
            (
                ConsolidationState::Submitted {
                    request: expected,
                    job_id: expected_lost_job,
                },
                ConsolidationDriverEvent::RecoveredSubmitted {
                    request,
                    lost_job_id,
                    recovered_job_id,
                },
            ) if request == expected
                && lost_job_id == expected_lost_job
                && recovered_job_id != lost_job_id =>
            {
                ConsolidationState::Submitted {
                    request,
                    job_id: recovered_job_id,
                }
            }
            (
                ConsolidationState::Submitted {
                    request: expected,
                    job_id,
                },
                ConsolidationDriverEvent::Completed { request, staged },
            ) if request == expected && staged.job_id == job_id => {
                staged.validate_against(&request, 1, 1)?;
                ConsolidationState::Completed { request, staged }
            }
            (
                ConsolidationState::Completed { request, staged },
                ConsolidationDriverEvent::Committed {
                    cycle_id,
                    output_generation,
                    output_digest,
                },
            ) if cycle_id == request.cycle_id
                && output_generation == staged.output_generation
                && output_digest == staged.output_digest =>
            {
                ConsolidationState::Committed {
                    cycle_id,
                    output_generation,
                    output_digest,
                }
            }
            _ => return Err(ScaffoldContractError::ConsolidationGenerationMismatch),
        };
        next.validate_for_cycle(active_cycle_id)?;
        self.state.consolidation = next;
        self.state.validate_contract()
    }

    fn transition_to(
        &mut self,
        phase: SleepPhase,
        tick: Tick,
        trigger: SleepTrigger,
    ) -> Result<SleepTransition, ScaffoldContractError> {
        let from = self.state.phase;
        let entered_sleep_tick = match (from, phase, self.state.entered_sleep_tick) {
            (SleepPhase::Awake, SleepPhase::Awake, _) => None,
            (SleepPhase::Awake, _, _) => Some(tick),
            (_, SleepPhase::Awake, _) => None,
            (_, _, existing) => existing,
        };
        let cycles_completed = if phase == SleepPhase::Awake && from != SleepPhase::Awake {
            self.state.cycles_completed.saturating_add(1)
        } else {
            self.state.cycles_completed
        };
        let (active_cycle_id, last_consolidated_cycle_id, consolidation) = match (from, phase) {
            (SleepPhase::Awake, SleepPhase::Awake) => (
                0,
                self.state.last_consolidated_cycle_id,
                ConsolidationState::None,
            ),
            (SleepPhase::Awake, _) => (
                self.state
                    .last_consolidated_cycle_id
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::InvalidId)?,
                self.state.last_consolidated_cycle_id,
                ConsolidationState::None,
            ),
            (_, SleepPhase::Awake) => {
                if !matches!(
                    self.state.consolidation,
                    ConsolidationState::Committed { .. }
                ) {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                }
                (0, self.state.active_cycle_id, ConsolidationState::None)
            }
            (_, SleepPhase::Consolidating) => (
                self.state.active_cycle_id,
                self.state.last_consolidated_cycle_id,
                self.state.consolidation,
            ),
            (_, SleepPhase::Waking) => (
                self.state.active_cycle_id,
                self.state.last_consolidated_cycle_id,
                self.state.consolidation,
            ),
            _ => (
                self.state.active_cycle_id,
                self.state.last_consolidated_cycle_id,
                ConsolidationState::None,
            ),
        };
        self.state = SleepState {
            schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
            phase,
            phase_started_tick: tick,
            entered_sleep_tick,
            cycles_completed,
            last_trigger: Some(trigger),
            active_cycle_id,
            last_consolidated_cycle_id,
            consolidation,
        };
        self.state.validate_contract()?;
        Ok(SleepTransition {
            from,
            to: phase,
            tick,
            trigger,
        })
    }
}

impl Validate for SleepController {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.config.validate_contract()?;
        self.state.validate_contract()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitPromotionReport {
    pub promoted_count: u32,
    pub insufficient_evidence_count: u32,
    pub rejected_variance_count: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct HTraceDrainReport {
    pub schema_version: u16,
    pub active_tiles: u32,
    pub active_synapses: u32,
    pub h_operational_delta_l1: f32,
    pub h_shadow_decay_l1: f32,
    pub lifetime_delta_l1: f32,
    pub alpha_reset_count: u32,
    pub promoted_trait_count: u32,
    pub genetic_layer_unchanged: bool,
}

impl Validate for HTraceDrainReport {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        require_current_version(SchemaKind::SleepConsolidation, self.schema_version)?;
        validate_finite(self.h_operational_delta_l1)?;
        validate_finite(self.h_shadow_decay_l1)?;
        validate_finite(self.lifetime_delta_l1)?;
        if self.h_operational_delta_l1 < 0.0
            || self.h_shadow_decay_l1 < 0.0
            || self.lifetime_delta_l1 < 0.0
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCompressionReport {
    pub input_records: usize,
    pub output_records: usize,
    pub retained_source_memory_ids: Vec<MemoryId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConceptConsolidationReport {
    pub concepts_considered: u32,
    pub simplexes_considered: u32,
    pub preserved_gap_count: u32,
    pub decayed_gap_count: u32,
    pub curiosity_bias_count: u32,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SleepWorkStatus {
    SkippedLowPressure = 1,
    Consolidated = 2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SleepWorkReceipt {
    pub schema_version: u16,
    pub tick: Tick,
    pub status: SleepWorkStatus,
    pub fatigue: NormalizedScalar,
    pub sleep_pressure: NormalizedScalar,
    pub replay_digest: [u64; 4],
    pub replay_event_count: u32,
    pub replay_eligibility_sample_count: u32,
    pub promoted_memory_ids: Vec<MemoryId>,
    pub predictor_update_count: u32,
    pub concept: Option<ConceptConsolidationReport>,
    pub work_units: u64,
    pub canonical_digest: [u64; 4],
}

impl SleepWorkReceipt {
    #[expect(
        clippy::too_many_arguments,
        reason = "sleep work receipt validates each independently measured counter"
    )]
    fn new(
        tick: Tick,
        status: SleepWorkStatus,
        fatigue: NormalizedScalar,
        sleep_pressure: NormalizedScalar,
        replay_digest: [u64; 4],
        replay_event_count: u32,
        replay_eligibility_sample_count: u32,
        promoted_memory_ids: Vec<MemoryId>,
        predictor_update_count: u32,
        concept: Option<ConceptConsolidationReport>,
        work_units: u64,
    ) -> Result<Self, ScaffoldContractError> {
        let mut receipt = Self {
            schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
            tick,
            status,
            fatigue,
            sleep_pressure,
            replay_digest,
            replay_event_count,
            replay_eligibility_sample_count,
            promoted_memory_ids,
            predictor_update_count,
            concept,
            work_units,
            canonical_digest: [0; 4],
        };
        receipt.canonical_digest = receipt.recompute_canonical_digest()?;
        receipt.validate_contract()?;
        Ok(receipt)
    }

    pub fn recompute_canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(b"ALIFE-SLEEP-WORK-RECEIPT-V1");
        digest.write_u16(self.schema_version);
        digest.write_u64(self.tick.raw());
        digest.write_u8(self.status as u8);
        digest.write_f32(self.fatigue.raw())?;
        digest.write_f32(self.sleep_pressure.raw())?;
        digest.write_sequence_len(self.replay_digest.len());
        for word in self.replay_digest {
            digest.write_u64(word);
        }
        digest.write_u32(self.replay_event_count);
        digest.write_u32(self.replay_eligibility_sample_count);
        digest.write_sequence_len(self.promoted_memory_ids.len());
        for memory_id in &self.promoted_memory_ids {
            digest.write_u64(memory_id.raw());
        }
        digest.write_u32(self.predictor_update_count);
        if let Some(concept) = self.concept {
            digest.write_u8(1);
            digest.write_u32(concept.concepts_considered);
            digest.write_u32(concept.simplexes_considered);
            digest.write_u32(concept.preserved_gap_count);
            digest.write_u32(concept.decayed_gap_count);
            digest.write_u32(concept.curiosity_bias_count);
        } else {
            digest.write_u8(0);
        }
        digest.write_u64(self.work_units);
        Ok(digest.finish256())
    }
}

impl Validate for SleepWorkReceipt {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        require_current_version(SchemaKind::SleepConsolidation, self.schema_version)?;
        NormalizedScalar::new(self.fatigue.raw())?;
        NormalizedScalar::new(self.sleep_pressure.raw())?;
        let mut unique_promoted_memory_ids = BTreeSet::new();
        for memory_id in &self.promoted_memory_ids {
            memory_id.validate()?;
            if !unique_promoted_memory_ids.insert(memory_id.raw()) {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
            }
        }
        match self.status {
            SleepWorkStatus::SkippedLowPressure => {
                if self.replay_digest != [0; 4]
                    || self.replay_event_count != 0
                    || self.replay_eligibility_sample_count != 0
                    || !self.promoted_memory_ids.is_empty()
                    || self.predictor_update_count != 0
                    || self.concept.is_some()
                    || self.work_units != 0
                {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                }
            }
            SleepWorkStatus::Consolidated => {
                if self.replay_digest == [0; 4]
                    || self.replay_event_count == 0
                    || self.replay_eligibility_sample_count == 0
                    || self.promoted_memory_ids.is_empty()
                    || self.promoted_memory_ids.len() > self.replay_event_count as usize
                    || self.predictor_update_count != self.replay_event_count
                    || self.concept.is_none()
                    || self.work_units == 0
                {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                }
            }
        }
        if self.canonical_digest != self.recompute_canonical_digest()? {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StructuralEditKind {
    PruneMarker,
    Strengthen,
    Weaken,
    SynaptogenesisCandidate,
    Consolidate,
    RecompactionHint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StructuralEditReason {
    MemoryCorrelation,
    TopologyCorrelation,
    LowSalience,
    Recovery,
    Fatigue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructuralEditApplicationStatus {
    DeferredForSleepCompilation,
    DeferredForOfflineCompilation,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StructuralEditCandidate {
    pub candidate_id: u64,
    pub projection: ProjectionRoutingRef,
    pub kind: StructuralEditKind,
    pub reason: StructuralEditReason,
    pub salience: NormalizedScalar,
    pub confidence: Confidence,
    pub estimated_synapses: u32,
}

impl StructuralEditCandidate {
    pub fn new(
        candidate_id: u64,
        projection: ProjectionRoutingRef,
        kind: StructuralEditKind,
        reason: StructuralEditReason,
        salience: NormalizedScalar,
        confidence: Confidence,
        estimated_synapses: u32,
    ) -> Result<Self, ScaffoldContractError> {
        let candidate = Self {
            candidate_id,
            projection,
            kind,
            reason,
            salience,
            confidence,
            estimated_synapses,
        };
        candidate.validate_contract()?;
        Ok(candidate)
    }
}

impl Validate for StructuralEditCandidate {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.candidate_id == 0 || self.estimated_synapses == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        NormalizedScalar::new(self.salience.raw())?;
        Confidence::new(self.confidence.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuralEditBatch {
    pub schema_version: u16,
    pub tick: Tick,
    candidates: Vec<StructuralEditCandidate>,
    pub application_status: StructuralEditApplicationStatus,
}

impl StructuralEditBatch {
    pub fn new(
        tick: Tick,
        mut candidates: Vec<StructuralEditCandidate>,
        max_candidates: usize,
    ) -> Result<Self, ScaffoldContractError> {
        if max_candidates == 0 || candidates.len() > max_candidates {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        candidates.sort_by_key(|candidate| {
            (
                candidate.candidate_id,
                candidate.kind,
                candidate.reason,
                candidate.projection.source_lobe.raw(),
                candidate.projection.target_lobe.raw(),
            )
        });
        let batch = Self {
            schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
            tick,
            candidates,
            application_status: StructuralEditApplicationStatus::DeferredForSleepCompilation,
        };
        batch.validate_contract()?;
        Ok(batch)
    }

    pub fn candidates(&self) -> &[StructuralEditCandidate] {
        &self.candidates
    }
}

impl Validate for StructuralEditBatch {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        require_current_version(SchemaKind::SleepConsolidation, self.schema_version)?;
        let mut seen = BTreeSet::new();
        for candidate in &self.candidates {
            candidate.validate_contract()?;
            if !seen.insert(candidate.candidate_id) {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SleepConsolidationReport {
    pub schema_version: u16,
    pub tick: Tick,
    pub sleep_phase: SleepPhase,
    pub neural: HTraceDrainReport,
    pub memory: MemoryCompressionReport,
    pub topology: ConceptConsolidationReport,
    pub structural_edits: StructuralEditBatch,
    pub traits: TraitPromotionReport,
}

impl Validate for SleepConsolidationReport {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        require_current_version(SchemaKind::SleepConsolidation, self.schema_version)?;
        self.neural.validate_contract()?;
        self.structural_edits.validate_contract()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SleepConsolidator {
    config: SleepConsolidationConfig,
}

impl SleepConsolidator {
    pub fn new(config: SleepConsolidationConfig) -> Result<Self, ScaffoldContractError> {
        config.validate_contract()?;
        Ok(Self { config })
    }

    pub const fn config(&self) -> SleepConsolidationConfig {
        self.config
    }

    pub fn run_bounded_transaction(
        &self,
        homeostasis: &HomeostaticSnapshot,
        tick: Tick,
        evidence: &SleepReplayEvidence,
        memory: &mut MemoryBank,
        predictor: &mut GroundedSuccessorPredictor,
        topology: &mut TopologySidecar,
    ) -> Result<SleepWorkReceipt, ScaffoldContractError> {
        homeostasis.validate_contract()?;
        let fatigue = NormalizedScalar::new(homeostasis.drives.fatigue)?;
        let sleep_pressure = NormalizedScalar::new(homeostasis.hormones.sleep_pressure)?;
        let sleep_due = fatigue.raw() >= self.config.fatigue_threshold.raw()
            || sleep_pressure.raw() >= self.config.sleep_pressure_threshold.raw();
        if !sleep_due {
            return SleepWorkReceipt::new(
                tick,
                SleepWorkStatus::SkippedLowPressure,
                fatigue,
                sleep_pressure,
                [0; 4],
                0,
                0,
                Vec::new(),
                0,
                None,
                0,
            );
        }

        evidence.validate_contract()?;
        if evidence.batch.events.is_empty() {
            return Err(ScaffoldContractError::MissingPhaseData);
        }

        // GPU replay can outlive bounded memory. Keep validated event/target
        // pairs whose sealed experience is still retained in either memory
        // representation, while stale replay entries remain discardable.
        let mut eligible_events = Vec::with_capacity(evidence.batch.events.len());
        let mut eligible_event_indices = BTreeSet::new();
        let mut eligible_memory_ids = Vec::with_capacity(evidence.batch.events.len());
        for (event_index, (event, target)) in evidence
            .batch
            .events
            .iter()
            .zip(&evidence.prediction_targets)
            .enumerate()
        {
            let Some(memory_id) = memory.memory_id_for_sleep_sequence(
                event.sequence_id,
                target.organism_id,
                event.action_id,
                event.family,
                event.originating_tick,
            )?
            else {
                continue;
            };
            eligible_event_indices.insert(event_index);
            eligible_events.push((event, target));
            eligible_memory_ids.push(memory_id);
        }
        if eligible_events.is_empty() {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        let source_sequences = eligible_events
            .iter()
            .map(|(event, _)| event.sequence_id)
            .collect::<Vec<_>>();

        let mut next_memory = memory.clone();
        let mut next_predictor = predictor.clone();
        let mut next_topology = topology.clone();
        let promoted_memory_ids = next_memory.memory_ids_for_sleep(
            &source_sequences,
            &eligible_memory_ids,
            self.config.memory_max_records_after,
        )?;
        let mut predictor_update_count = 0_u32;
        for (_, target) in eligible_events.iter().copied() {
            next_predictor.observe(target)?;
            predictor_update_count = predictor_update_count.saturating_add(1);
        }
        let concept = self.consolidate_topology_sidecar(&mut next_topology, 1)?;
        let replay_event_count = u32::try_from(eligible_events.len())
            .map_err(|_| ScaffoldContractError::ScalarOutOfRange)?;
        let replay_eligibility_sample_count = u32::try_from(
            evidence
                .batch
                .eligibility_samples
                .iter()
                .filter(|sample| eligible_event_indices.contains(&usize::from(sample.event_index)))
                .count(),
        )
        .map_err(|_| ScaffoldContractError::ScalarOutOfRange)?;
        let work_units = u64::from(replay_event_count)
            .saturating_add(u64::from(replay_eligibility_sample_count))
            .saturating_add(u64::try_from(promoted_memory_ids.len()).unwrap_or(u64::MAX))
            .saturating_add(u64::from(predictor_update_count))
            .saturating_add(u64::from(concept.concepts_considered))
            .saturating_add(u64::from(concept.simplexes_considered))
            .saturating_add(u64::from(concept.preserved_gap_count))
            .saturating_add(u64::from(concept.decayed_gap_count));
        let receipt = SleepWorkReceipt::new(
            tick,
            SleepWorkStatus::Consolidated,
            fatigue,
            sleep_pressure,
            evidence.canonical_digest,
            replay_event_count,
            replay_eligibility_sample_count,
            promoted_memory_ids,
            predictor_update_count,
            Some(concept),
            work_units,
        )?;
        *memory = next_memory;
        *predictor = next_predictor;
        *topology = next_topology;
        Ok(receipt)
    }

    pub fn consolidate_topology(
        &self,
        topology: &mut TopologicalMap,
        elapsed_ticks: u64,
    ) -> Result<ConceptConsolidationReport, ScaffoldContractError> {
        topology.validate_contract()?;
        topology.decay_edges(elapsed_ticks)?;
        let limit = self.config.concept_simplex_consolidation_limit;
        Ok(ConceptConsolidationReport {
            concepts_considered: topology.concepts().len().min(limit) as u32,
            simplexes_considered: topology.simplexes().len().min(limit) as u32,
            preserved_gap_count: topology.unresolved_gaps().len() as u32,
            decayed_gap_count: 0,
            curiosity_bias_count: topology.curiosity_biases().len() as u32,
        })
    }

    pub fn consolidate_topology_sidecar(
        &self,
        topology: &mut TopologySidecar,
        elapsed_ticks: u64,
    ) -> Result<ConceptConsolidationReport, ScaffoldContractError> {
        let report = self.consolidate_topology(topology.map_mut(), elapsed_ticks)?;
        topology.refresh_diagnostics_after_map_mutation()?;
        Ok(report)
    }
}

impl Validate for SleepConsolidator {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.config.validate_contract()
    }
}
