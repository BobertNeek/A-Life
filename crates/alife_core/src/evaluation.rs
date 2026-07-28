//! Engine-neutral passive life statistics and bounded active-battery contracts.

use serde::{Deserialize, Serialize};

use crate::{
    ActionKind, ExperiencePatch, OrganismId, PhysicalContactKind, ScaffoldContractError, Tick,
    UtteranceSourceKind, Validate,
};

pub const PASSIVE_LIFE_STATISTICS_SCHEMA_VERSION: u16 = 1;
pub const ACTIVE_BATTERY_SCHEMA_VERSION: u16 = 1;
pub const ACTIVE_CHALLENGE_COUNT: usize = 15;
const Q16_ONE: u32 = 65_535;
const ENVIRONMENTAL_REGIME_COUNT: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricReading {
    Unknown,
    Measured { value_q16: u32, exposures: u64 },
}

impl MetricReading {
    pub const fn value_q16(self) -> Option<u32> {
        match self {
            Self::Unknown => None,
            Self::Measured { value_q16, .. } => Some(value_q16),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EnvironmentalRegime {
    Temperate = 0,
    Scarcity = 1,
    Abundance = 2,
    Hazardous = 3,
    Social = 4,
    Novel = 5,
}

impl EnvironmentalRegime {
    const fn index(self) -> usize {
        self as usize
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PassiveMetricKind {
    SurvivalTicks = 1,
    FoodSuccess = 2,
    PoisonAvoidance = 3,
    HazardAvoidance = 4,
    EnergyStability = 5,
    Movement = 6,
    Reproduction = 7,
    SleepRetention = 8,
    LearningSlope = 9,
    ReversalRecovery = 10,
    VocabularyGrounding = 11,
    UnaidedComprehension = 12,
    SlmAssistedComprehension = 13,
    NarrationFidelity = 14,
    PeerCommunication = 15,
    DialectTransfer = 16,
    DialectDivergence = 17,
    GpuThrottleAvoidance = 18,
}

impl PassiveMetricKind {
    pub const ALL: [Self; 18] = [
        Self::SurvivalTicks,
        Self::FoodSuccess,
        Self::PoisonAvoidance,
        Self::HazardAvoidance,
        Self::EnergyStability,
        Self::Movement,
        Self::Reproduction,
        Self::SleepRetention,
        Self::LearningSlope,
        Self::ReversalRecovery,
        Self::VocabularyGrounding,
        Self::UnaidedComprehension,
        Self::SlmAssistedComprehension,
        Self::NarrationFidelity,
        Self::PeerCommunication,
        Self::DialectTransfer,
        Self::DialectDivergence,
        Self::GpuThrottleAvoidance,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
struct BoundedMean {
    samples: u64,
    sum_q16: u128,
}

impl BoundedMean {
    fn observe(&mut self, value_q16: u32) -> Result<(), ScaffoldContractError> {
        if value_q16 > Q16_ONE {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.samples = self
            .samples
            .checked_add(1)
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
        self.sum_q16 = self
            .sum_q16
            .checked_add(u128::from(value_q16))
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
        Ok(())
    }

    fn reading(self) -> MetricReading {
        if self.samples == 0 {
            return MetricReading::Unknown;
        }
        let rounded = (self.sum_q16 + u128::from(self.samples / 2)) / u128::from(self.samples);
        MetricReading::Measured {
            value_q16: u32::try_from(rounded).unwrap_or(Q16_ONE).min(Q16_ONE),
            exposures: self.samples,
        }
    }

    fn validate(self) -> Result<(), ScaffoldContractError> {
        if self.sum_q16 > u128::from(self.samples).saturating_mul(u128::from(Q16_ONE)) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassiveLifeEvent {
    SurvivalTick {
        tick: Tick,
        regime: EnvironmentalRegime,
        energy_q16: u32,
        movement_distance_q16: u32,
        gpu_dispatched: bool,
        gpu_throttled: bool,
    },
    FoodOutcome {
        beneficial: bool,
    },
    PoisonEncounter {
        avoided: bool,
    },
    HazardEncounter {
        avoided: bool,
    },
    Reproduction {
        successful: bool,
    },
    SleepRetention {
        retained: bool,
    },
    LearningProbe {
        improvement_q16: u32,
    },
    ReversalRecovery {
        ticks_to_recover: u32,
    },
    VocabularyGrounding {
        correct: bool,
    },
    Comprehension {
        assisted: bool,
        correct: bool,
    },
    NarrationUtterance,
    Narration {
        faithful: bool,
    },
    PeerCommunication {
        successful: bool,
    },
    DialectTransfer {
        successful: bool,
    },
    DialectDivergence {
        distance_q16: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassiveLifeStatistics {
    schema_version: u16,
    organism_id: OrganismId,
    birth_tick: Tick,
    last_tick: Tick,
    death_tick: Option<Tick>,
    death_reason: Option<String>,
    survival_ticks: u64,
    environmental_regime_ticks: [u64; ENVIRONMENTAL_REGIME_COUNT],
    heard_token_exposures: u64,
    gpu_dispatches: u64,
    gpu_throttled_dispatches: u64,
    narration_utterances: u64,
    food_success: BoundedMean,
    poison_avoidance: BoundedMean,
    hazard_avoidance: BoundedMean,
    energy_stability: BoundedMean,
    movement: BoundedMean,
    reproduction: BoundedMean,
    sleep_retention: BoundedMean,
    learning_slope: BoundedMean,
    reversal_recovery: BoundedMean,
    vocabulary_grounding: BoundedMean,
    unaided_comprehension: BoundedMean,
    slm_assisted_comprehension: BoundedMean,
    narration_fidelity: BoundedMean,
    peer_communication: BoundedMean,
    dialect_transfer: BoundedMean,
    dialect_divergence: BoundedMean,
}

impl PassiveLifeStatistics {
    pub fn new(organism_id: OrganismId, birth_tick: Tick) -> Result<Self, ScaffoldContractError> {
        organism_id.validate()?;
        Ok(Self {
            schema_version: PASSIVE_LIFE_STATISTICS_SCHEMA_VERSION,
            organism_id,
            birth_tick,
            last_tick: birth_tick,
            death_tick: None,
            death_reason: None,
            survival_ticks: 0,
            environmental_regime_ticks: [0; ENVIRONMENTAL_REGIME_COUNT],
            heard_token_exposures: 0,
            gpu_dispatches: 0,
            gpu_throttled_dispatches: 0,
            narration_utterances: 0,
            food_success: BoundedMean::default(),
            poison_avoidance: BoundedMean::default(),
            hazard_avoidance: BoundedMean::default(),
            energy_stability: BoundedMean::default(),
            movement: BoundedMean::default(),
            reproduction: BoundedMean::default(),
            sleep_retention: BoundedMean::default(),
            learning_slope: BoundedMean::default(),
            reversal_recovery: BoundedMean::default(),
            vocabulary_grounding: BoundedMean::default(),
            unaided_comprehension: BoundedMean::default(),
            slm_assisted_comprehension: BoundedMean::default(),
            narration_fidelity: BoundedMean::default(),
            peer_communication: BoundedMean::default(),
            dialect_transfer: BoundedMean::default(),
            dialect_divergence: BoundedMean::default(),
        })
    }

    pub const fn organism_id(&self) -> OrganismId {
        self.organism_id
    }

    pub const fn survival_ticks(&self) -> u64 {
        self.survival_ticks
    }

    pub const fn environmental_regime_ticks(&self) -> &[u64; ENVIRONMENTAL_REGIME_COUNT] {
        &self.environmental_regime_ticks
    }

    pub const fn gpu_dispatches(&self) -> u64 {
        self.gpu_dispatches
    }

    pub const fn gpu_throttled_dispatches(&self) -> u64 {
        self.gpu_throttled_dispatches
    }

    pub const fn heard_token_exposures(&self) -> u64 {
        self.heard_token_exposures
    }

    pub const fn narration_utterances(&self) -> u64 {
        self.narration_utterances
    }

    pub const fn death_tick(&self) -> Option<Tick> {
        self.death_tick
    }

    pub fn observe(&mut self, event: PassiveLifeEvent) -> Result<(), ScaffoldContractError> {
        if self.death_tick.is_some() {
            return Err(ScaffoldContractError::InvalidId);
        }
        match event {
            PassiveLifeEvent::SurvivalTick {
                tick,
                regime,
                energy_q16,
                movement_distance_q16,
                gpu_dispatched,
                gpu_throttled,
            } => {
                if tick.raw() <= self.last_tick.raw()
                    || energy_q16 > Q16_ONE
                    || movement_distance_q16 > Q16_ONE
                    || gpu_throttled && !gpu_dispatched
                {
                    return Err(ScaffoldContractError::ScalarOutOfRange);
                }
                self.last_tick = tick;
                self.survival_ticks = self
                    .survival_ticks
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
                let regime_ticks = &mut self.environmental_regime_ticks[regime.index()];
                *regime_ticks = regime_ticks
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
                self.energy_stability.observe(energy_q16)?;
                self.movement.observe(movement_distance_q16)?;
                self.gpu_dispatches = self
                    .gpu_dispatches
                    .checked_add(u64::from(gpu_dispatched))
                    .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
                self.gpu_throttled_dispatches = self
                    .gpu_throttled_dispatches
                    .checked_add(u64::from(gpu_throttled))
                    .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
            }
            PassiveLifeEvent::FoodOutcome { beneficial } => {
                self.food_success.observe(bool_q16(beneficial))?
            }
            PassiveLifeEvent::PoisonEncounter { avoided } => {
                self.poison_avoidance.observe(bool_q16(avoided))?
            }
            PassiveLifeEvent::HazardEncounter { avoided } => {
                self.hazard_avoidance.observe(bool_q16(avoided))?
            }
            PassiveLifeEvent::Reproduction { successful } => {
                self.reproduction.observe(bool_q16(successful))?
            }
            PassiveLifeEvent::SleepRetention { retained } => {
                self.sleep_retention.observe(bool_q16(retained))?
            }
            PassiveLifeEvent::LearningProbe { improvement_q16 } => {
                self.learning_slope.observe(improvement_q16)?
            }
            PassiveLifeEvent::ReversalRecovery { ticks_to_recover } => self
                .reversal_recovery
                .observe(Q16_ONE / ticks_to_recover.saturating_add(1))?,
            PassiveLifeEvent::VocabularyGrounding { correct } => {
                self.vocabulary_grounding.observe(bool_q16(correct))?
            }
            PassiveLifeEvent::Comprehension { assisted, correct } => {
                if assisted {
                    self.slm_assisted_comprehension.observe(bool_q16(correct))?;
                } else {
                    self.unaided_comprehension.observe(bool_q16(correct))?;
                }
            }
            PassiveLifeEvent::NarrationUtterance => {
                self.narration_utterances = self
                    .narration_utterances
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
            }
            PassiveLifeEvent::Narration { faithful } => {
                self.narration_fidelity.observe(bool_q16(faithful))?
            }
            PassiveLifeEvent::PeerCommunication { successful } => {
                self.peer_communication.observe(bool_q16(successful))?
            }
            PassiveLifeEvent::DialectTransfer { successful } => {
                self.dialect_transfer.observe(bool_q16(successful))?
            }
            PassiveLifeEvent::DialectDivergence { distance_q16 } => {
                self.dialect_divergence.observe(distance_q16)?
            }
        }
        Ok(())
    }

    pub fn observe_sealed_patch(
        &mut self,
        patch: &ExperiencePatch,
    ) -> Result<(), ScaffoldContractError> {
        patch.validate_contract()?;
        if patch.header().organism_id != self.organism_id {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let heard = patch
            .pre_action()
            .perception()
            .sensory()
            .language_context
            .heard_tokens
            .iter()
            .flatten()
            .count() as u64;
        self.heard_token_exposures = self
            .heard_token_exposures
            .checked_add(heard)
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
        let outcome = patch.outcome();
        if outcome.physical.contact == PhysicalContactKind::Consumed {
            let harmful = outcome.pain_delta.raw() > 0.0 || outcome.reward_valence.raw() < 0.0;
            self.observe(PassiveLifeEvent::FoodOutcome {
                beneficial: !harmful,
            })?;
            if harmful {
                self.observe(PassiveLifeEvent::PoisonEncounter { avoided: false })?;
            }
        }
        if outcome.pain_delta.raw() > 0.0
            || outcome.physical.contact == PhysicalContactKind::Collision
        {
            self.observe(PassiveLifeEvent::HazardEncounter { avoided: false })?;
        }
        if patch.decision().selected_action.kind == ActionKind::Vocalize {
            self.observe(PassiveLifeEvent::NarrationUtterance)?;
        }
        if patch.decision().selected_action.kind == ActionKind::Vocalize
            && patch
                .pre_action()
                .perception()
                .sensory()
                .language_context
                .heard_tokens
                .iter()
                .flatten()
                .any(|token| token.source_kind == UtteranceSourceKind::Creature)
        {
            self.observe(PassiveLifeEvent::PeerCommunication {
                successful: outcome.success,
            })?;
        }
        Ok(())
    }

    pub fn finalize(
        &mut self,
        death_tick: Tick,
        death_reason: impl Into<String>,
    ) -> Result<(), ScaffoldContractError> {
        let death_reason = death_reason.into();
        if self.death_tick.is_some()
            || death_tick.raw() < self.last_tick.raw()
            || death_reason.trim().is_empty()
            || death_reason.chars().count() > 160
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.death_tick = Some(death_tick);
        self.death_reason = Some(death_reason);
        self.validate_contract()
    }

    pub fn metric(&self, kind: PassiveMetricKind) -> MetricReading {
        match kind {
            PassiveMetricKind::SurvivalTicks => {
                if self.survival_ticks == 0 {
                    MetricReading::Unknown
                } else {
                    MetricReading::Measured {
                        value_q16: u32::try_from(self.survival_ticks)
                            .unwrap_or(Q16_ONE)
                            .min(Q16_ONE),
                        exposures: self.survival_ticks,
                    }
                }
            }
            PassiveMetricKind::FoodSuccess => self.food_success.reading(),
            PassiveMetricKind::PoisonAvoidance => self.poison_avoidance.reading(),
            PassiveMetricKind::HazardAvoidance => self.hazard_avoidance.reading(),
            PassiveMetricKind::EnergyStability => self.energy_stability.reading(),
            PassiveMetricKind::Movement => self.movement.reading(),
            PassiveMetricKind::Reproduction => self.reproduction.reading(),
            PassiveMetricKind::SleepRetention => self.sleep_retention.reading(),
            PassiveMetricKind::LearningSlope => self.learning_slope.reading(),
            PassiveMetricKind::ReversalRecovery => self.reversal_recovery.reading(),
            PassiveMetricKind::VocabularyGrounding => self.vocabulary_grounding.reading(),
            PassiveMetricKind::UnaidedComprehension => self.unaided_comprehension.reading(),
            PassiveMetricKind::SlmAssistedComprehension => {
                self.slm_assisted_comprehension.reading()
            }
            PassiveMetricKind::NarrationFidelity => self.narration_fidelity.reading(),
            PassiveMetricKind::PeerCommunication => self.peer_communication.reading(),
            PassiveMetricKind::DialectTransfer => self.dialect_transfer.reading(),
            PassiveMetricKind::DialectDivergence => self.dialect_divergence.reading(),
            PassiveMetricKind::GpuThrottleAvoidance => {
                if self.gpu_dispatches == 0 {
                    MetricReading::Unknown
                } else {
                    let unthrottled = self
                        .gpu_dispatches
                        .saturating_sub(self.gpu_throttled_dispatches);
                    MetricReading::Measured {
                        value_q16: ratio_q16(unthrottled, self.gpu_dispatches),
                        exposures: self.gpu_dispatches,
                    }
                }
            }
        }
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        <Self as Validate>::validate_contract(self)
    }
}

impl Validate for PassiveLifeStatistics {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        let regime_ticks = self
            .environmental_regime_ticks
            .iter()
            .try_fold(0_u64, |sum, value| sum.checked_add(*value))
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
        if self.schema_version != PASSIVE_LIFE_STATISTICS_SCHEMA_VERSION
            || self.last_tick.raw() < self.birth_tick.raw()
            || regime_ticks != self.survival_ticks
            || self.gpu_throttled_dispatches > self.gpu_dispatches
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.organism_id.validate()?;
        if self.death_tick.is_some() != self.death_reason.is_some()
            || self
                .death_tick
                .is_some_and(|tick| tick.raw() < self.last_tick.raw())
            || self
                .death_reason
                .as_ref()
                .is_some_and(|reason| reason.trim().is_empty() || reason.chars().count() > 160)
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        for metric in [
            self.food_success,
            self.poison_avoidance,
            self.hazard_avoidance,
            self.energy_stability,
            self.movement,
            self.reproduction,
            self.sleep_retention,
            self.learning_slope,
            self.reversal_recovery,
            self.vocabulary_grounding,
            self.unaided_comprehension,
            self.slm_assisted_comprehension,
            self.narration_fidelity,
            self.peer_communication,
            self.dialect_transfer,
            self.dialect_divergence,
        ] {
            metric.validate()?;
        }
        Ok(())
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActiveChallengeKind {
    VisibleRewardNavigation = 1,
    BlockedRouteDetour = 2,
    DangerousShortVsSafeLong = 3,
    RewardHazardReversal = 4,
    DelayedChoice = 5,
    UnfamiliarEdibility = 6,
    PostSleepRetention = 7,
    LayoutAppearanceGeneralization = 8,
    InjuryFatigueRecovery = 9,
    NameAddressedInstruction = 10,
    WordObjectGrounding = 11,
    ActionWordGrounding = 12,
    WhatWhyNarration = 13,
    PeerTaughtAlias = 14,
    SlmDisabledDialectTransfer = 15,
}

impl ActiveChallengeKind {
    pub const ALL: [Self; ACTIVE_CHALLENGE_COUNT] = [
        Self::VisibleRewardNavigation,
        Self::BlockedRouteDetour,
        Self::DangerousShortVsSafeLong,
        Self::RewardHazardReversal,
        Self::DelayedChoice,
        Self::UnfamiliarEdibility,
        Self::PostSleepRetention,
        Self::LayoutAppearanceGeneralization,
        Self::InjuryFatigueRecovery,
        Self::NameAddressedInstruction,
        Self::WordObjectGrounding,
        Self::ActionWordGrounding,
        Self::WhatWhyNarration,
        Self::PeerTaughtAlias,
        Self::SlmDisabledDialectTransfer,
    ];

    pub const fn raw(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveChallengeResult {
    pub challenge: ActiveChallengeKind,
    pub score: MetricReading,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveBatteryReceipt {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub results: Vec<ActiveChallengeResult>,
}

impl ActiveBatteryReceipt {
    pub fn empty(organism_id: OrganismId) -> Self {
        Self {
            schema_version: ACTIVE_BATTERY_SCHEMA_VERSION,
            organism_id,
            results: ActiveChallengeKind::ALL
                .into_iter()
                .map(|challenge| ActiveChallengeResult {
                    challenge,
                    score: MetricReading::Unknown,
                })
                .collect(),
        }
    }

    pub fn record(
        &mut self,
        challenge: ActiveChallengeKind,
        score_q16: u32,
    ) -> Result<(), ScaffoldContractError> {
        if score_q16 > Q16_ONE {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let result = self
            .results
            .iter_mut()
            .find(|result| result.challenge == challenge)
            .ok_or(ScaffoldContractError::InvalidId)?;
        result.score = MetricReading::Measured {
            value_q16: score_q16,
            exposures: 1,
        };
        self.validate_contract()
    }

    pub fn completed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| matches!(result.score, MetricReading::Measured { .. }))
            .count()
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        <Self as Validate>::validate_contract(self)
    }
}

impl Validate for ActiveBatteryReceipt {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        if self.schema_version != ACTIVE_BATTERY_SCHEMA_VERSION
            || self.results.len() != ACTIVE_CHALLENGE_COUNT
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        for (expected, result) in ActiveChallengeKind::ALL.iter().zip(&self.results) {
            if expected != &result.challenge
                || matches!(result.score, MetricReading::Measured { value_q16, exposures } if value_q16 > Q16_ONE || exposures != 1)
            {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(())
    }
}

const fn bool_q16(value: bool) -> u32 {
    if value {
        Q16_ONE
    } else {
        0
    }
}

fn ratio_q16(numerator: u64, denominator: u64) -> u32 {
    if denominator == 0 {
        return 0;
    }
    let value = (u128::from(numerator) * u128::from(Q16_ONE) + u128::from(denominator / 2))
        / u128::from(denominator);
    u32::try_from(value).unwrap_or(Q16_ONE).min(Q16_ONE)
}
