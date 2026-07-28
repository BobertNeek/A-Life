//! Engine-neutral sleep scheduling, durable GPU consolidation transactions,
//! and legacy offline consolidation diagnostics.
//!
//! Sleep consolidation is an explicit offline/sleep-phase path. It may drain
//! plastic traces and stage structural edit candidates, but it does not resize
//! active tick neural structures or mutate inherited genetic weights by default.

use std::collections::{BTreeMap, BTreeSet};

use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

mod gpu_contracts;
pub use gpu_contracts::*;
mod replay;
pub use replay::*;

use crate::{
    require_current_version, validate_finite, ChemistryModulation, Confidence, DurationTicks,
    HomeostaticParameters, HomeostaticSnapshot, LobeKind, MemoryBank, MemoryId,
    NeuralProjectionSchema, NormalizedScalar, ProjectionRoutingRef, RecoveryTrigger,
    ScaffoldContractError, SchemaKind, SchemaVersions, SparseTilePayload, SynapseWeightSplit, Tick,
    TopologicalMap, TopologySidecar, Validate,
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
    pub consolidation_duration: DurationTicks,
    pub waking_duration: DurationTicks,
    pub forced_recovery_min_duration: DurationTicks,
    pub h_shadow_drain_rate: NormalizedScalar,
    pub h_shadow_decay_rate: NormalizedScalar,
    pub lifetime_staging_rate: NormalizedScalar,
    pub memory_max_records_after: usize,
    pub concept_simplex_consolidation_limit: usize,
    pub structural_edit_candidate_limit: usize,
    pub stable_trait_promotion_threshold: u32,
    pub stable_trait_strength_threshold: NormalizedScalar,
    pub stable_trait_variance_threshold: NormalizedScalar,
    pub allow_lamarckian_inheritance: bool,
    pub reset_alpha_after_lifetime_staging: bool,
    pub weight_abs_limit: f32,
}

impl SleepConsolidationConfig {
    pub const fn reference() -> Self {
        Self {
            schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
            fatigue_threshold: NormalizedScalar(0.9),
            sleep_pressure_threshold: NormalizedScalar(0.85),
            entering_duration: DurationTicks(3),
            consolidation_duration: DurationTicks(16),
            waking_duration: DurationTicks(3),
            forced_recovery_min_duration: DurationTicks(8),
            h_shadow_drain_rate: NormalizedScalar(0.25),
            h_shadow_decay_rate: NormalizedScalar(0.1),
            lifetime_staging_rate: NormalizedScalar(0.1),
            memory_max_records_after: 64,
            concept_simplex_consolidation_limit: 64,
            structural_edit_candidate_limit: 32,
            stable_trait_promotion_threshold: 3,
            stable_trait_strength_threshold: NormalizedScalar(0.6),
            stable_trait_variance_threshold: NormalizedScalar(0.05),
            allow_lamarckian_inheritance: false,
            reset_alpha_after_lifetime_staging: false,
            weight_abs_limit: 4.0,
        }
    }
}

impl Validate for SleepConsolidationConfig {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        require_current_version(SchemaKind::SleepConsolidation, self.schema_version)?;
        NormalizedScalar::new(self.fatigue_threshold.raw())?;
        NormalizedScalar::new(self.sleep_pressure_threshold.raw())?;
        NormalizedScalar::new(self.h_shadow_drain_rate.raw())?;
        NormalizedScalar::new(self.h_shadow_decay_rate.raw())?;
        NormalizedScalar::new(self.lifetime_staging_rate.raw())?;
        NormalizedScalar::new(self.stable_trait_strength_threshold.raw())?;
        NormalizedScalar::new(self.stable_trait_variance_threshold.raw())?;
        validate_finite(self.weight_abs_limit)?;
        if self.entering_duration.raw() == 0
            || self.consolidation_duration.raw() == 0
            || self.waking_duration.raw() == 0
            || self.forced_recovery_min_duration.raw() == 0
            || self.memory_max_records_after == 0
            || self.concept_simplex_consolidation_limit == 0
            || self.structural_edit_candidate_limit == 0
            || self.stable_trait_promotion_threshold == 0
            || self.weight_abs_limit <= 0.0
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StableLifetimeTraitKind {
    MotorHabit,
    HomeostaticRecovery,
    MemoryCorrelation,
    TopologyCorrelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LifetimeTraitEvidence {
    pub trait_id: u64,
    pub kind: StableLifetimeTraitKind,
    pub strength: NormalizedScalar,
    pub variance: NormalizedScalar,
    pub cycle_index: u32,
}

impl LifetimeTraitEvidence {
    pub fn new(
        trait_id: u64,
        kind: StableLifetimeTraitKind,
        strength: NormalizedScalar,
        variance: NormalizedScalar,
        cycle_index: u32,
    ) -> Result<Self, ScaffoldContractError> {
        let evidence = Self {
            trait_id,
            kind,
            strength,
            variance,
            cycle_index,
        };
        evidence.validate_contract()?;
        Ok(evidence)
    }
}

impl Validate for LifetimeTraitEvidence {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.trait_id == 0 || self.cycle_index == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        NormalizedScalar::new(self.strength.raw())?;
        NormalizedScalar::new(self.variance.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StableLifetimeTrait {
    pub trait_id: u64,
    pub kind: StableLifetimeTraitKind,
    pub strength: NormalizedScalar,
    pub confidence: Confidence,
    pub source_cycle_count: u32,
    pub promoted_at_tick: Tick,
}

impl Validate for StableLifetimeTrait {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.trait_id == 0 || self.source_cycle_count == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        NormalizedScalar::new(self.strength.raw())?;
        Confidence::new(self.confidence.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifetimeTraitLedger {
    max_evidence: usize,
    evidence: Vec<LifetimeTraitEvidence>,
    promoted_traits: Vec<StableLifetimeTrait>,
}

impl LifetimeTraitLedger {
    pub fn new(max_evidence: usize) -> Result<Self, ScaffoldContractError> {
        if max_evidence == 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(Self {
            max_evidence,
            evidence: Vec::new(),
            promoted_traits: Vec::new(),
        })
    }

    pub fn observe(
        &mut self,
        evidence: LifetimeTraitEvidence,
    ) -> Result<(), ScaffoldContractError> {
        evidence.validate_contract()?;
        if self.evidence.len() == self.max_evidence {
            self.evidence.remove(0);
        }
        self.evidence.push(evidence);
        Ok(())
    }

    pub fn promoted_traits(&self) -> &[StableLifetimeTrait] {
        &self.promoted_traits
    }

    pub fn has_promoted_traits(&self) -> bool {
        !self.promoted_traits.is_empty()
    }
}

impl Validate for LifetimeTraitLedger {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.max_evidence == 0 || self.evidence.len() > self.max_evidence {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for evidence in &self.evidence {
            evidence.validate_contract()?;
        }
        for stable_trait in &self.promoted_traits {
            stable_trait.validate_contract()?;
        }
        Ok(())
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
        candidates.sort_by_key(candidate_sort_key);
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

    pub fn promote_stable_traits(
        &self,
        ledger: &mut LifetimeTraitLedger,
        tick: Tick,
    ) -> Result<TraitPromotionReport, ScaffoldContractError> {
        ledger.validate_contract()?;
        let mut groups: BTreeMap<(u64, StableLifetimeTraitKind), Vec<LifetimeTraitEvidence>> =
            BTreeMap::new();
        for evidence in &ledger.evidence {
            groups
                .entry((evidence.trait_id, evidence.kind))
                .or_default()
                .push(*evidence);
        }

        let already_promoted: BTreeSet<(u64, StableLifetimeTraitKind)> = ledger
            .promoted_traits
            .iter()
            .map(|stable_trait| (stable_trait.trait_id, stable_trait.kind))
            .collect();

        let mut report = TraitPromotionReport::default();
        for ((trait_id, kind), evidence) in groups {
            if already_promoted.contains(&(trait_id, kind)) {
                continue;
            }
            if evidence.len() < self.config.stable_trait_promotion_threshold as usize {
                report.insufficient_evidence_count =
                    report.insufficient_evidence_count.saturating_add(1);
                continue;
            }
            let max_variance = evidence
                .iter()
                .map(|sample| sample.variance.raw())
                .fold(0.0_f32, f32::max);
            if max_variance > self.config.stable_trait_variance_threshold.raw() {
                report.rejected_variance_count = report.rejected_variance_count.saturating_add(1);
                continue;
            }
            let strength_sum: f32 = evidence.iter().map(|sample| sample.strength.raw()).sum();
            let strength = strength_sum / evidence.len() as f32;
            if strength < self.config.stable_trait_strength_threshold.raw() {
                report.insufficient_evidence_count =
                    report.insufficient_evidence_count.saturating_add(1);
                continue;
            }
            let confidence = (strength
                * (evidence.len() as f32 / self.config.stable_trait_promotion_threshold as f32))
                .clamp(0.0, 1.0);
            let stable_trait = StableLifetimeTrait {
                trait_id,
                kind,
                strength: NormalizedScalar::new(strength.clamp(0.0, 1.0))?,
                confidence: Confidence::new(confidence)?,
                source_cycle_count: evidence.len() as u32,
                promoted_at_tick: tick,
            };
            stable_trait.validate_contract()?;
            ledger.promoted_traits.push(stable_trait);
            report.promoted_count = report.promoted_count.saturating_add(1);
        }
        ledger.promoted_traits.sort_by_key(|stable_trait| {
            (
                stable_trait.trait_id,
                stable_trait.kind,
                stable_trait.promoted_at_tick.raw(),
            )
        });
        Ok(report)
    }

    pub fn consolidate_neural_schema(
        &self,
        schema: &mut NeuralProjectionSchema,
        ledger: &mut LifetimeTraitLedger,
        tick: Tick,
    ) -> Result<HTraceDrainReport, ScaffoldContractError> {
        schema.validate()?;
        let promotion = self.promote_stable_traits(ledger, tick)?;
        let lifetime_enabled = ledger.has_promoted_traits();
        let mut report = HTraceDrainReport {
            schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
            genetic_layer_unchanged: true,
            promoted_trait_count: promotion.promoted_count,
            ..HTraceDrainReport::default()
        };

        for projection in &mut schema.projections {
            let mut tile_touched = false;
            for tile in &mut projection.tiles {
                let before_tile_synapses = report.active_synapses;
                match &mut tile.payload {
                    SparseTilePayload::Dense(dense) => {
                        for weights in &mut dense.weights {
                            update_sleep_weight(
                                weights,
                                self.config,
                                lifetime_enabled,
                                &mut report,
                            )?;
                        }
                    }
                    SparseTilePayload::Coo(coo) => {
                        for entry in &mut coo.entries {
                            update_sleep_weight(
                                &mut entry.weights,
                                self.config,
                                lifetime_enabled,
                                &mut report,
                            )?;
                        }
                    }
                    SparseTilePayload::RowRunUnsupported
                    | SparseTilePayload::ColumnRunUnsupported => {
                        return Err(ScaffoldContractError::UnsupportedSparseTileFormat);
                    }
                }
                if report.active_synapses > before_tile_synapses {
                    tile_touched = true;
                }
            }
            if tile_touched {
                report.active_tiles = report.active_tiles.saturating_add(1);
            }
        }
        report.validate_contract()?;
        Ok(report)
    }

    pub fn compress_memory_bank(
        &self,
        bank: &mut MemoryBank,
    ) -> Result<MemoryCompressionReport, ScaffoldContractError> {
        let input_records = bank.len();
        let mut records = bank.records_chronological();
        let output_records = input_records.min(self.config.memory_max_records_after);
        if input_records > output_records {
            let start = input_records - output_records;
            records = records.split_off(start);
        }
        let retained: Vec<_> = records.into_iter().cloned().collect();
        let retained_source_memory_ids = retained.iter().map(|record| record.memory_id).collect();
        bank.replace_with_consolidated_records(retained)?;
        Ok(MemoryCompressionReport {
            input_records,
            output_records,
            retained_source_memory_ids,
        })
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

    pub fn generate_structural_edit_batch(
        &self,
        schema: &NeuralProjectionSchema,
        topology: &TopologicalMap,
        tick: Tick,
    ) -> Result<StructuralEditBatch, ScaffoldContractError> {
        schema.validate()?;
        topology.validate_contract()?;
        let mut candidates = Vec::new();
        for (index, bias) in topology.curiosity_biases().into_iter().enumerate() {
            if candidates.len() == self.config.structural_edit_candidate_limit {
                break;
            }
            let projection = schema.projections[index % schema.projections.len()].routing_ref;
            candidates.push(StructuralEditCandidate::new(
                (index + 1) as u64,
                projection,
                StructuralEditKind::SynaptogenesisCandidate,
                StructuralEditReason::TopologyCorrelation,
                bias.salience,
                bias.confidence,
                16,
            )?);
        }
        if candidates.is_empty() {
            let projection = schema.projections[0].routing_ref;
            candidates.push(StructuralEditCandidate::new(
                1,
                projection,
                StructuralEditKind::PruneMarker,
                StructuralEditReason::LowSalience,
                NormalizedScalar::new(0.1)?,
                Confidence::new(0.25)?,
                16,
            )?);
        }
        StructuralEditBatch::new(
            tick,
            candidates,
            self.config.structural_edit_candidate_limit,
        )
    }

    pub fn reject_active_tick_structural_application(
        &self,
        batch: &StructuralEditBatch,
    ) -> Result<(), ScaffoldContractError> {
        batch.validate_contract()?;
        Err(ScaffoldContractError::InvalidSparseProjectionSchema)
    }
}

impl Validate for SleepConsolidator {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.config.validate_contract()
    }
}

fn update_sleep_weight(
    weights: &mut SynapseWeightSplit,
    config: SleepConsolidationConfig,
    lifetime_enabled: bool,
    report: &mut HTraceDrainReport,
) -> Result<(), ScaffoldContractError> {
    let before = *weights;
    validate_weight_split(before)?;
    if before.h_shadow == 0.0
        && before.h_operational == 0.0
        && before.lifetime_consolidated == 0.0
        && before.genetic_fixed == 0.0
    {
        return Ok(());
    }

    let drain = validate_finite(before.h_shadow * config.h_shadow_drain_rate.raw())?;
    weights.h_operational = clamp_weight(before.h_operational + drain, config.weight_abs_limit)?;
    let after_drain_shadow = validate_finite(before.h_shadow - drain)?;
    weights.h_shadow = clamp_weight(
        after_drain_shadow * (1.0 - config.h_shadow_decay_rate.raw()),
        config.weight_abs_limit,
    )?;
    report.h_operational_delta_l1 += drain.abs();
    report.h_shadow_decay_l1 += (before.h_shadow - weights.h_shadow).abs();

    if lifetime_enabled {
        let lifetime_delta = validate_finite(drain * config.lifetime_staging_rate.raw())?;
        weights.lifetime_consolidated = clamp_weight(
            before.lifetime_consolidated + lifetime_delta,
            config.weight_abs_limit,
        )?;
        report.lifetime_delta_l1 += lifetime_delta.abs();
        if config.reset_alpha_after_lifetime_staging && lifetime_delta != 0.0 {
            weights.alpha = 0.0;
            report.alpha_reset_count = report.alpha_reset_count.saturating_add(1);
        }
    }

    if weights.genetic_fixed != before.genetic_fixed {
        report.genetic_layer_unchanged = false;
        return Err(ScaffoldContractError::LamarckianInheritanceRequiresOptIn);
    }
    validate_weight_split(*weights)?;
    report.active_synapses = report.active_synapses.saturating_add(1);
    Ok(())
}

fn validate_weight_split(weights: SynapseWeightSplit) -> Result<(), ScaffoldContractError> {
    validate_finite(weights.genetic_fixed)?;
    validate_finite(weights.lifetime_consolidated)?;
    NormalizedScalar::new(weights.alpha)?;
    validate_finite(weights.h_operational)?;
    validate_finite(weights.h_shadow)?;
    Ok(())
}

fn clamp_weight(value: f32, limit: f32) -> Result<f32, ScaffoldContractError> {
    validate_finite(value).map(|value| value.clamp(-limit, limit))
}

fn candidate_sort_key(
    candidate: &StructuralEditCandidate,
) -> (u64, StructuralEditKind, StructuralEditReason, u16, u16) {
    (
        candidate.candidate_id,
        candidate.kind,
        candidate.reason,
        lobe_sort_key(candidate.projection.source_lobe),
        lobe_sort_key(candidate.projection.target_lobe),
    )
}

fn lobe_sort_key(lobe: LobeKind) -> u16 {
    lobe.stable_id().raw()
}
