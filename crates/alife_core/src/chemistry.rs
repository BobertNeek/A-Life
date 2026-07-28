//! v0 scaffold: bounded endocrine and drive modulation contracts.

use serde::{Deserialize, Serialize};

use crate::{
    math::{validate_finite, validate_finite_slice},
    require_current_version, Confidence, ScaffoldContractError, SchemaKind, SchemaVersions, Tick,
    Validate,
};

pub const DRIVE_EXTENSION_SLOTS: usize = 2;
pub const ENDOCRINE_EXTENSION_SLOTS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DriveSnapshot {
    pub hunger: f32,
    pub fatigue: f32,
    pub fear: f32,
    pub pain: f32,
    pub loneliness: f32,
    pub curiosity: f32,
    pub brain_atp: f32,
    pub temperature_stress: f32,
    pub reproductive_drive: f32,
    pub extension: [f32; DRIVE_EXTENSION_SLOTS],
}

impl DriveSnapshot {
    pub const CHANNEL_COUNT: usize = 9 + DRIVE_EXTENSION_SLOTS;

    pub const fn baseline() -> Self {
        Self {
            hunger: 0.25,
            fatigue: 0.2,
            fear: 0.1,
            pain: 0.0,
            loneliness: 0.15,
            curiosity: 0.45,
            brain_atp: 0.75,
            temperature_stress: 0.0,
            reproductive_drive: 0.1,
            extension: [0.0; DRIVE_EXTENSION_SLOTS],
        }
    }

    pub fn to_array(self) -> [f32; Self::CHANNEL_COUNT] {
        [
            self.hunger,
            self.fatigue,
            self.fear,
            self.pain,
            self.loneliness,
            self.curiosity,
            self.brain_atp,
            self.temperature_stress,
            self.reproductive_drive,
            self.extension[0],
            self.extension[1],
        ]
    }

    fn apply_drift(self, parameters: HomeostaticParameters, steps: f32) -> Self {
        let decay = parameters.drive_decay_per_update * steps;
        let mut next = Self {
            hunger: self.hunger + parameters.hunger_drift_per_update * steps,
            fatigue: self.fatigue + parameters.fatigue_drift_per_update * steps,
            fear: decay_toward_zero(self.fear, decay),
            pain: decay_toward_zero(self.pain, decay),
            loneliness: self.loneliness + parameters.loneliness_drift_per_update * steps,
            curiosity: self.curiosity + parameters.curiosity_drift_per_update * steps,
            brain_atp: self.brain_atp - parameters.brain_atp_drain_per_update * steps,
            temperature_stress: decay_toward_zero(self.temperature_stress, decay),
            reproductive_drive: self.reproductive_drive
                + parameters.reproductive_drift_per_update * steps,
            extension: self.extension.map(|value| decay_toward_zero(value, decay)),
        };
        next.clamp_in_place();
        next
    }

    fn apply_delta(self, delta: DriveDelta) -> Self {
        let mut next = Self {
            hunger: self.hunger + delta.hunger,
            fatigue: self.fatigue + delta.fatigue,
            fear: self.fear + delta.fear,
            pain: self.pain + delta.pain,
            loneliness: self.loneliness + delta.loneliness,
            curiosity: self.curiosity + delta.curiosity,
            brain_atp: self.brain_atp + delta.brain_atp,
            temperature_stress: self.temperature_stress + delta.temperature_stress,
            reproductive_drive: self.reproductive_drive + delta.reproductive_drive,
            extension: [
                self.extension[0] + delta.extension[0],
                self.extension[1] + delta.extension[1],
            ],
        };
        next.clamp_in_place();
        next
    }

    fn clamp_in_place(&mut self) {
        self.hunger = clamp01(self.hunger);
        self.fatigue = clamp01(self.fatigue);
        self.fear = clamp01(self.fear);
        self.pain = clamp01(self.pain);
        self.loneliness = clamp01(self.loneliness);
        self.curiosity = clamp01(self.curiosity);
        self.brain_atp = clamp01(self.brain_atp);
        self.temperature_stress = clamp01(self.temperature_stress);
        self.reproductive_drive = clamp01(self.reproductive_drive);
        self.extension = self.extension.map(clamp01);
    }
}

impl Validate for DriveSnapshot {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_unit_values(&self.to_array())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DriveDelta {
    pub hunger: f32,
    pub fatigue: f32,
    pub fear: f32,
    pub pain: f32,
    pub loneliness: f32,
    pub curiosity: f32,
    pub brain_atp: f32,
    pub temperature_stress: f32,
    pub reproductive_drive: f32,
    pub extension: [f32; DRIVE_EXTENSION_SLOTS],
}

impl DriveDelta {
    pub const CHANNEL_COUNT: usize = DriveSnapshot::CHANNEL_COUNT;

    pub const fn zero() -> Self {
        Self {
            hunger: 0.0,
            fatigue: 0.0,
            fear: 0.0,
            pain: 0.0,
            loneliness: 0.0,
            curiosity: 0.0,
            brain_atp: 0.0,
            temperature_stress: 0.0,
            reproductive_drive: 0.0,
            extension: [0.0; DRIVE_EXTENSION_SLOTS],
        }
    }

    pub fn to_array(self) -> [f32; Self::CHANNEL_COUNT] {
        [
            self.hunger,
            self.fatigue,
            self.fear,
            self.pain,
            self.loneliness,
            self.curiosity,
            self.brain_atp,
            self.temperature_stress,
            self.reproductive_drive,
            self.extension[0],
            self.extension[1],
        ]
    }
}

impl Validate for DriveDelta {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_signed_unit_values(&self.to_array())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EndocrineSnapshot {
    pub adrenaline: f32,
    pub cortisol: f32,
    pub dopamine: f32,
    pub oxytocin: f32,
    pub serotonin: f32,
    pub acetylcholine: f32,
    pub learning_modulator: f32,
    pub developmental_hormone: f32,
    pub sleep_pressure: f32,
    pub extension: [f32; ENDOCRINE_EXTENSION_SLOTS],
}

impl EndocrineSnapshot {
    pub const CHANNEL_COUNT: usize = 9 + ENDOCRINE_EXTENSION_SLOTS;

    pub const fn baseline() -> Self {
        Self {
            adrenaline: 0.2,
            cortisol: 0.2,
            dopamine: 0.5,
            oxytocin: 0.5,
            serotonin: 0.6,
            acetylcholine: 0.5,
            learning_modulator: 0.5,
            developmental_hormone: 0.5,
            sleep_pressure: 0.2,
            extension: [0.0; ENDOCRINE_EXTENSION_SLOTS],
        }
    }

    pub fn to_array(self) -> [f32; Self::CHANNEL_COUNT] {
        [
            self.adrenaline,
            self.cortisol,
            self.dopamine,
            self.oxytocin,
            self.serotonin,
            self.acetylcholine,
            self.learning_modulator,
            self.developmental_hormone,
            self.sleep_pressure,
            self.extension[0],
            self.extension[1],
        ]
    }

    fn apply_decay(self, parameters: HomeostaticParameters, steps: f32) -> Self {
        let baseline = Self::baseline();
        let decay = parameters.hormone_decay_per_update * steps;
        let sleep_pressure = if self.sleep_pressure > baseline.sleep_pressure {
            decay_toward(self.sleep_pressure, baseline.sleep_pressure, decay)
        } else {
            self.sleep_pressure + parameters.sleep_pressure_drift_per_update * steps
        };
        let mut next = Self {
            adrenaline: decay_toward(self.adrenaline, baseline.adrenaline, decay),
            cortisol: decay_toward(self.cortisol, baseline.cortisol, decay),
            dopamine: decay_toward(self.dopamine, baseline.dopamine, decay),
            oxytocin: decay_toward(self.oxytocin, baseline.oxytocin, decay),
            serotonin: decay_toward(self.serotonin, baseline.serotonin, decay),
            acetylcholine: decay_toward(self.acetylcholine, baseline.acetylcholine, decay),
            learning_modulator: decay_toward(
                self.learning_modulator,
                baseline.learning_modulator,
                decay,
            ),
            developmental_hormone: decay_toward(
                self.developmental_hormone,
                baseline.developmental_hormone,
                decay,
            ),
            sleep_pressure,
            extension: self.extension.map(|value| decay_toward_zero(value, decay)),
        };
        next.clamp_in_place();
        next
    }

    fn apply_delta(self, delta: EndocrineDelta) -> Self {
        let mut next = Self {
            adrenaline: self.adrenaline + delta.adrenaline,
            cortisol: self.cortisol + delta.cortisol,
            dopamine: self.dopamine + delta.dopamine,
            oxytocin: self.oxytocin + delta.oxytocin,
            serotonin: self.serotonin + delta.serotonin,
            acetylcholine: self.acetylcholine + delta.acetylcholine,
            learning_modulator: self.learning_modulator + delta.learning_modulator,
            developmental_hormone: self.developmental_hormone + delta.developmental_hormone,
            sleep_pressure: self.sleep_pressure + delta.sleep_pressure,
            extension: [
                self.extension[0] + delta.extension[0],
                self.extension[1] + delta.extension[1],
            ],
        };
        next.clamp_in_place();
        next
    }

    fn clamp_in_place(&mut self) {
        self.adrenaline = clamp01(self.adrenaline);
        self.cortisol = clamp01(self.cortisol);
        self.dopamine = clamp01(self.dopamine);
        self.oxytocin = clamp01(self.oxytocin);
        self.serotonin = clamp01(self.serotonin);
        self.acetylcholine = clamp01(self.acetylcholine);
        self.learning_modulator = clamp01(self.learning_modulator);
        self.developmental_hormone = clamp01(self.developmental_hormone);
        self.sleep_pressure = clamp01(self.sleep_pressure);
        self.extension = self.extension.map(clamp01);
    }
}

impl Validate for EndocrineSnapshot {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_unit_values(&self.to_array())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EndocrineDelta {
    pub adrenaline: f32,
    pub cortisol: f32,
    pub dopamine: f32,
    pub oxytocin: f32,
    pub serotonin: f32,
    pub acetylcholine: f32,
    pub learning_modulator: f32,
    pub developmental_hormone: f32,
    pub sleep_pressure: f32,
    pub extension: [f32; ENDOCRINE_EXTENSION_SLOTS],
}

impl EndocrineDelta {
    pub const CHANNEL_COUNT: usize = EndocrineSnapshot::CHANNEL_COUNT;

    pub const fn zero() -> Self {
        Self {
            adrenaline: 0.0,
            cortisol: 0.0,
            dopamine: 0.0,
            oxytocin: 0.0,
            serotonin: 0.0,
            acetylcholine: 0.0,
            learning_modulator: 0.0,
            developmental_hormone: 0.0,
            sleep_pressure: 0.0,
            extension: [0.0; ENDOCRINE_EXTENSION_SLOTS],
        }
    }

    pub fn to_array(self) -> [f32; Self::CHANNEL_COUNT] {
        [
            self.adrenaline,
            self.cortisol,
            self.dopamine,
            self.oxytocin,
            self.serotonin,
            self.acetylcholine,
            self.learning_modulator,
            self.developmental_hormone,
            self.sleep_pressure,
            self.extension[0],
            self.extension[1],
        ]
    }
}

impl Validate for EndocrineDelta {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_signed_unit_values(&self.to_array())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HomeostaticSnapshot {
    pub schema_version: u16,
    pub tick: Tick,
    pub drives: DriveSnapshot,
    pub hormones: EndocrineSnapshot,
}

impl HomeostaticSnapshot {
    pub const SCHEMA_VERSION: u16 = SchemaVersions::CURRENT.chemistry.0;

    pub fn new(
        tick: Tick,
        drives: DriveSnapshot,
        hormones: EndocrineSnapshot,
    ) -> Result<Self, ScaffoldContractError> {
        let snapshot = Self {
            schema_version: Self::SCHEMA_VERSION,
            tick,
            drives,
            hormones,
        };
        snapshot.validate_contract()?;
        Ok(snapshot)
    }

    pub fn baseline(tick: Tick) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            tick,
            drives: DriveSnapshot::baseline(),
            hormones: EndocrineSnapshot::baseline(),
        }
    }

    pub fn advance(
        self,
        next_tick: Tick,
        delta: HomeostaticDelta,
        parameters: HomeostaticParameters,
    ) -> Result<Self, ScaffoldContractError> {
        self.validate_contract()?;
        delta.validate_contract()?;
        parameters.validate_contract()?;
        Tick::validate_monotonic(self.tick, next_tick)?;

        let elapsed_ticks = next_tick.raw().saturating_sub(self.tick.raw());
        let steps = elapsed_ticks.min(u64::from(u32::MAX)) as f32;
        let drives = self
            .drives
            .apply_drift(parameters, steps)
            .apply_delta(delta.drives);
        let hormones = self
            .hormones
            .apply_decay(parameters, steps)
            .apply_delta(delta.hormones);

        Self::new(next_tick, drives, hormones)
    }
}

impl Validate for HomeostaticSnapshot {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        require_current_version(SchemaKind::Chemistry, self.schema_version)?;
        self.drives.validate_contract()?;
        self.hormones.validate_contract()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HomeostaticDelta {
    pub drives: DriveDelta,
    pub hormones: EndocrineDelta,
}

impl HomeostaticDelta {
    pub const fn zero() -> Self {
        Self {
            drives: DriveDelta::zero(),
            hormones: EndocrineDelta::zero(),
        }
    }

    /// Bounded recovery applied once per non-awake world tick.
    ///
    /// Ordinary drift still advances hunger and other drives. These lanes only
    /// model the restorative effects required for a completed sleep cycle to
    /// return below its own fatigue and sleep-pressure entry thresholds.
    pub const fn sleep_recovery_per_tick() -> Self {
        Self {
            drives: DriveDelta {
                fatigue: -0.05,
                brain_atp: 0.04,
                ..DriveDelta::zero()
            },
            hormones: EndocrineDelta {
                sleep_pressure: -0.06,
                ..EndocrineDelta::zero()
            },
        }
    }

    pub fn pain_frustration_spike(
        pain: f32,
        cortisol: f32,
        adrenaline: f32,
    ) -> Result<Self, ScaffoldContractError> {
        validate_unit_values(&[pain, cortisol, adrenaline])?;
        Ok(Self {
            drives: DriveDelta {
                pain,
                ..DriveDelta::zero()
            },
            hormones: EndocrineDelta {
                cortisol,
                adrenaline,
                ..EndocrineDelta::zero()
            },
        })
    }
}

impl Validate for HomeostaticDelta {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.drives.validate_contract()?;
        self.hormones.validate_contract()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HomeostaticParameters {
    pub hunger_drift_per_update: f32,
    pub fatigue_drift_per_update: f32,
    pub loneliness_drift_per_update: f32,
    pub curiosity_drift_per_update: f32,
    pub reproductive_drift_per_update: f32,
    pub brain_atp_drain_per_update: f32,
    pub drive_decay_per_update: f32,
    pub hormone_decay_per_update: f32,
    pub sleep_pressure_drift_per_update: f32,
    pub seizure_adrenaline_threshold: f32,
    pub seizure_cortisol_threshold: f32,
    pub catatonia_brain_atp_threshold: f32,
    pub fatigue_sleep_threshold: f32,
    pub sleep_pressure_threshold: f32,
    pub pain_frustration_threshold: f32,
    pub safe_idle_brain_atp_threshold: f32,
    pub safe_idle_pain_threshold: f32,
}

impl HomeostaticParameters {
    pub const fn reference() -> Self {
        Self {
            hunger_drift_per_update: 0.01,
            fatigue_drift_per_update: 0.01,
            loneliness_drift_per_update: 0.003,
            curiosity_drift_per_update: 0.002,
            reproductive_drift_per_update: 0.001,
            brain_atp_drain_per_update: 0.01,
            drive_decay_per_update: 0.04,
            hormone_decay_per_update: 0.06,
            sleep_pressure_drift_per_update: 0.01,
            seizure_adrenaline_threshold: 0.95,
            seizure_cortisol_threshold: 0.9,
            catatonia_brain_atp_threshold: 0.05,
            fatigue_sleep_threshold: 0.9,
            sleep_pressure_threshold: 0.85,
            pain_frustration_threshold: 0.85,
            safe_idle_brain_atp_threshold: 0.08,
            safe_idle_pain_threshold: 0.95,
        }
    }

    fn to_array(self) -> [f32; 17] {
        [
            self.hunger_drift_per_update,
            self.fatigue_drift_per_update,
            self.loneliness_drift_per_update,
            self.curiosity_drift_per_update,
            self.reproductive_drift_per_update,
            self.brain_atp_drain_per_update,
            self.drive_decay_per_update,
            self.hormone_decay_per_update,
            self.sleep_pressure_drift_per_update,
            self.seizure_adrenaline_threshold,
            self.seizure_cortisol_threshold,
            self.catatonia_brain_atp_threshold,
            self.fatigue_sleep_threshold,
            self.sleep_pressure_threshold,
            self.pain_frustration_threshold,
            self.safe_idle_brain_atp_threshold,
            self.safe_idle_pain_threshold,
        ]
    }
}

impl Validate for HomeostaticParameters {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_unit_values(&self.to_array())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HomeostaticCadenceBand {
    Hot,
    Warm,
    Cold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeostaticCadence {
    pub min_hz: u8,
    pub max_hz: u8,
}

impl HomeostaticCadence {
    pub const fn for_band(band: HomeostaticCadenceBand) -> Self {
        match band {
            HomeostaticCadenceBand::Hot => Self {
                min_hz: 10,
                max_hz: 30,
            },
            HomeostaticCadenceBand::Warm => Self {
                min_hz: 2,
                max_hz: 10,
            },
            HomeostaticCadenceBand::Cold => Self {
                min_hz: 0,
                max_hz: 1,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EndocrineProfile {
    pub baseline: EndocrineSnapshot,
    pub parameters: HomeostaticParameters,
}

impl EndocrineProfile {
    pub const fn baseline() -> Self {
        Self {
            baseline: EndocrineSnapshot::baseline(),
            parameters: HomeostaticParameters::reference(),
        }
    }

    pub const fn modulator_count(&self) -> usize {
        EndocrineSnapshot::CHANNEL_COUNT
    }
}

impl Validate for EndocrineProfile {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.baseline.validate_contract()?;
        self.parameters.validate_contract()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryTrigger {
    SeizureHyperactivity,
    CatatoniaEnergyHypoplasia,
    FatigueSleepEntry,
    PainFrustrationSpike,
    SafeIdleFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryAssessment {
    pub seizure_hyperactivity: bool,
    pub catatonia_energy_hypoplasia: bool,
    pub fatigue_sleep_entry: bool,
    pub pain_frustration_spike: bool,
    pub safe_idle_fallback: bool,
}

impl RecoveryAssessment {
    pub const fn none() -> Self {
        Self {
            seizure_hyperactivity: false,
            catatonia_energy_hypoplasia: false,
            fatigue_sleep_entry: false,
            pain_frustration_spike: false,
            safe_idle_fallback: false,
        }
    }

    pub const fn contains(self, trigger: RecoveryTrigger) -> bool {
        match trigger {
            RecoveryTrigger::SeizureHyperactivity => self.seizure_hyperactivity,
            RecoveryTrigger::CatatoniaEnergyHypoplasia => self.catatonia_energy_hypoplasia,
            RecoveryTrigger::FatigueSleepEntry => self.fatigue_sleep_entry,
            RecoveryTrigger::PainFrustrationSpike => self.pain_frustration_spike,
            RecoveryTrigger::SafeIdleFallback => self.safe_idle_fallback,
        }
    }

    pub const fn any(self) -> bool {
        self.seizure_hyperactivity
            || self.catatonia_energy_hypoplasia
            || self.fatigue_sleep_entry
            || self.pain_frustration_spike
            || self.safe_idle_fallback
    }
}

pub struct ChemistryModulation;

impl ChemistryModulation {
    pub fn threshold_scale(
        state: &HomeostaticSnapshot,
        parameters: HomeostaticParameters,
    ) -> Result<f32, ScaffoldContractError> {
        state.validate_contract()?;
        parameters.validate_contract()?;
        validate_finite(clamp01(
            0.45 + 0.18 * state.hormones.cortisol
                + 0.12 * state.drives.fear
                + 0.12 * state.drives.pain
                + 0.08 * state.drives.fatigue
                - 0.05 * state.hormones.dopamine,
        ))
    }

    pub fn learning_rate_scale(
        state: &HomeostaticSnapshot,
        parameters: HomeostaticParameters,
    ) -> Result<f32, ScaffoldContractError> {
        state.validate_contract()?;
        parameters.validate_contract()?;
        let stress_penalty = 1.0 - 0.55 * state.hormones.cortisol;
        let reward_gain = 0.5 + 0.5 * state.hormones.dopamine;
        let energy_gate = 0.25 + 0.75 * state.drives.brain_atp;
        validate_finite(clamp01(
            state.hormones.learning_modulator * stress_penalty * reward_gain * energy_gate,
        ))
    }

    pub fn salience_weight(
        state: &HomeostaticSnapshot,
        parameters: HomeostaticParameters,
    ) -> Result<f32, ScaffoldContractError> {
        state.validate_contract()?;
        parameters.validate_contract()?;
        validate_finite(clamp01(
            0.15 + 0.18 * state.drives.curiosity
                + 0.18 * state.drives.fear
                + 0.2 * state.drives.pain
                + 0.12 * state.drives.hunger
                + 0.1 * state.hormones.adrenaline
                + 0.07 * state.hormones.dopamine,
        ))
    }

    pub fn motor_confidence(
        base_confidence: Confidence,
        state: &HomeostaticSnapshot,
        parameters: HomeostaticParameters,
    ) -> Result<Confidence, ScaffoldContractError> {
        state.validate_contract()?;
        parameters.validate_contract()?;
        let fatigue_penalty = 1.0 - 0.35 * state.drives.fatigue;
        let pain_penalty = 1.0 - 0.25 * state.drives.pain;
        let fear_penalty = 1.0 - 0.2 * state.drives.fear;
        let energy_gate = 0.35 + 0.65 * state.drives.brain_atp;
        Confidence::new(clamp01(
            base_confidence.raw() * fatigue_penalty * pain_penalty * fear_penalty * energy_gate,
        ))
    }

    pub fn recovery_triggers(
        state: &HomeostaticSnapshot,
        parameters: HomeostaticParameters,
    ) -> Result<RecoveryAssessment, ScaffoldContractError> {
        state.validate_contract()?;
        parameters.validate_contract()?;
        let seizure_hyperactivity = state.hormones.adrenaline
            >= parameters.seizure_adrenaline_threshold
            && state.hormones.cortisol >= parameters.seizure_cortisol_threshold;
        let catatonia_energy_hypoplasia =
            state.drives.brain_atp <= parameters.catatonia_brain_atp_threshold;
        let fatigue_sleep_entry = state.drives.fatigue >= parameters.fatigue_sleep_threshold
            || state.hormones.sleep_pressure >= parameters.sleep_pressure_threshold;
        let pain_frustration_spike = state.drives.pain >= parameters.pain_frustration_threshold;
        let safe_idle_fallback = seizure_hyperactivity
            || catatonia_energy_hypoplasia
            || state.drives.brain_atp <= parameters.safe_idle_brain_atp_threshold
            || state.drives.pain >= parameters.safe_idle_pain_threshold;

        Ok(RecoveryAssessment {
            seizure_hyperactivity,
            catatonia_energy_hypoplasia,
            fatigue_sleep_entry,
            pain_frustration_spike,
            safe_idle_fallback,
        })
    }

    pub fn should_enter_sleep(
        state: &HomeostaticSnapshot,
        parameters: HomeostaticParameters,
    ) -> Result<bool, ScaffoldContractError> {
        Ok(Self::recovery_triggers(state, parameters)?.fatigue_sleep_entry)
    }
}

fn validate_unit_values(values: &[f32]) -> Result<(), ScaffoldContractError> {
    validate_finite_slice(values)?;
    if values.iter().all(|value| (0.0..=1.0).contains(value)) {
        Ok(())
    } else {
        Err(ScaffoldContractError::OutOfRangeDriveHormone)
    }
}

fn validate_signed_unit_values(values: &[f32]) -> Result<(), ScaffoldContractError> {
    validate_finite_slice(values)?;
    if values.iter().all(|value| (-1.0..=1.0).contains(value)) {
        Ok(())
    } else {
        Err(ScaffoldContractError::OutOfRangeDriveHormone)
    }
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn decay_toward_zero(value: f32, amount: f32) -> f32 {
    decay_toward(value, 0.0, amount)
}

fn decay_toward(value: f32, target: f32, amount: f32) -> f32 {
    if value > target {
        (value - amount).max(target)
    } else if value < target {
        (value + amount).min(target)
    } else {
        value
    }
}
