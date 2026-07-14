//! Contract-only compiler-owned projection, synapse, and neuron-dynamics records.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    ActivationFunction, ActiveTilePolicy, BiologicalPriority, CandidateActionFamily, LobeKind,
    ProjectionType, ScaffoldContractError, UpdateCadence,
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct NeuronDynamics {
    bias: f32,
    leak: f32,
    activation: ActivationFunction,
    activity_ema_decay: f32,
    metabolic_decay: f32,
    homeostatic_gain: f32,
}

impl NeuronDynamics {
    pub const fn bias(&self) -> f32 {
        self.bias
    }
    pub const fn leak(&self) -> f32 {
        self.leak
    }
    pub const fn activation(&self) -> ActivationFunction {
        self.activation
    }
    pub const fn activity_ema_decay(&self) -> f32 {
        self.activity_ema_decay
    }
    pub const fn metabolic_decay(&self) -> f32 {
        self.metabolic_decay
    }
    pub const fn homeostatic_gain(&self) -> f32 {
        self.homeostatic_gain
    }

    pub(super) fn validate(&self) -> Result<(), ScaffoldContractError> {
        if ![
            self.bias,
            self.leak,
            self.activity_ema_decay,
            self.metabolic_decay,
            self.homeostatic_gain,
        ]
        .into_iter()
        .all(f32::is_finite)
            || !(0.0..=1.0).contains(&self.leak)
            || !(0.0..=1.0).contains(&self.activity_ema_decay)
            || !(0.0..=1.0).contains(&self.metabolic_decay)
            || !(0.0..=2.0).contains(&self.homeostatic_gain)
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        ActivationFunction::try_from_raw(self.activation.raw())?;
        Ok(())
    }

    pub(super) const fn new(
        bias: f32,
        leak: f32,
        activation: ActivationFunction,
        activity_ema_decay: f32,
        metabolic_decay: f32,
        homeostatic_gain: f32,
    ) -> Self {
        Self {
            bias,
            leak,
            activation,
            activity_ema_decay,
            metabolic_decay,
            homeostatic_gain,
        }
    }
}

impl<'de> Deserialize<'de> for NeuronDynamics {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            bias: f32,
            leak: f32,
            activation: ActivationFunction,
            activity_ema_decay: f32,
            metabolic_decay: f32,
            homeostatic_gain: f32,
        }
        let wire = Wire::deserialize(deserializer)?;
        let value = Self::new(
            wire.bias,
            wire.leak,
            wire.activation,
            wire.activity_ema_decay,
            wire.metabolic_decay,
            wire.homeostatic_gain,
        );
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DecoderHeadKind {
    ActionCandidate = 1,
    MemoryContext = 2,
}

impl DecoderHeadKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }
    pub fn try_from_raw(raw: u8) -> Result<Self, ScaffoldContractError> {
        match raw {
            1 => Ok(Self::ActionCandidate),
            2 => Ok(Self::MemoryContext),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct DecoderSynapseCoordinate {
    head: DecoderHeadKind,
    family: CandidateActionFamily,
    input_lane: u16,
    motor_index: u16,
}

impl DecoderSynapseCoordinate {
    pub const fn head(&self) -> DecoderHeadKind {
        self.head
    }
    pub const fn family(&self) -> CandidateActionFamily {
        self.family
    }
    pub const fn input_lane(&self) -> u16 {
        self.input_lane
    }
    pub const fn motor_index(&self) -> u16 {
        self.motor_index
    }
    pub(super) const fn new(
        head: DecoderHeadKind,
        family: CandidateActionFamily,
        input_lane: u16,
        motor_index: u16,
    ) -> Self {
        Self {
            head,
            family,
            input_lane,
            motor_index,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum CompiledSynapseKind {
    Recurrent,
    Decoder(DecoderSynapseCoordinate),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CompiledSynapse {
    source: u32,
    target: u32,
    genetic_weight: f32,
    alpha: f32,
    route_index: u16,
    kind: CompiledSynapseKind,
}

impl CompiledSynapse {
    pub const fn source(&self) -> u32 {
        self.source
    }
    pub const fn target(&self) -> u32 {
        self.target
    }
    pub const fn genetic_weight(&self) -> f32 {
        self.genetic_weight
    }
    pub const fn alpha(&self) -> f32 {
        self.alpha
    }
    pub const fn route_index(&self) -> u16 {
        self.route_index
    }
    pub const fn kind(&self) -> CompiledSynapseKind {
        self.kind
    }
    pub(super) const fn new(
        source: u32,
        target: u32,
        genetic_weight: f32,
        alpha: f32,
        route_index: u16,
        kind: CompiledSynapseKind,
    ) -> Self {
        Self {
            source,
            target,
            genetic_weight,
            alpha,
            route_index,
            kind,
        }
    }
    pub(super) fn validate_local(&self) -> Result<(), ScaffoldContractError> {
        if !self.genetic_weight.is_finite()
            || !self.alpha.is_finite()
            || !(0.0..=1.0).contains(&self.alpha)
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        if let CompiledSynapseKind::Decoder(coordinate) = self.kind {
            DecoderHeadKind::try_from_raw(coordinate.head.raw())?;
            CandidateActionFamily::try_from_raw(coordinate.family.raw())?;
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for CompiledSynapse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CoordinateWire {
            head: DecoderHeadKind,
            family: CandidateActionFamily,
            input_lane: u16,
            motor_index: u16,
        }
        #[derive(Deserialize)]
        enum KindWire {
            Recurrent,
            Decoder(CoordinateWire),
        }
        #[derive(Deserialize)]
        struct Wire {
            source: u32,
            target: u32,
            genetic_weight: f32,
            alpha: f32,
            route_index: u16,
            kind: KindWire,
        }
        let w = Wire::deserialize(deserializer)?;
        let kind =
            match w.kind {
                KindWire::Recurrent => CompiledSynapseKind::Recurrent,
                KindWire::Decoder(c) => CompiledSynapseKind::Decoder(
                    DecoderSynapseCoordinate::new(c.head, c.family, c.input_lane, c.motor_index),
                ),
            };
        let value = Self::new(
            w.source,
            w.target,
            w.genetic_weight,
            w.alpha,
            w.route_index,
            kind,
        );
        value.validate_local().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompiledProjection {
    route_index: u16,
    source_lobe: LobeKind,
    target_lobe: LobeKind,
    projection_type: ProjectionType,
    active_tile_policy: ActiveTilePolicy,
    update_cadence: UpdateCadence,
    priority: BiologicalPriority,
    delay_microsteps: u8,
    synapse_start: u32,
    synapse_len: u32,
    active_tile_count: u32,
}

impl CompiledProjection {
    pub const fn route_index(&self) -> u16 {
        self.route_index
    }
    pub const fn source_lobe(&self) -> LobeKind {
        self.source_lobe
    }
    pub const fn target_lobe(&self) -> LobeKind {
        self.target_lobe
    }
    pub const fn projection_type(&self) -> ProjectionType {
        self.projection_type
    }
    pub const fn active_tile_policy(&self) -> ActiveTilePolicy {
        self.active_tile_policy
    }
    pub const fn update_cadence(&self) -> UpdateCadence {
        self.update_cadence
    }
    pub const fn priority(&self) -> BiologicalPriority {
        self.priority
    }
    pub const fn delay_microsteps(&self) -> u8 {
        self.delay_microsteps
    }
    pub const fn synapse_range(&self) -> (u32, u32) {
        (self.synapse_start, self.synapse_len)
    }
    pub const fn active_tile_count(&self) -> u32 {
        self.active_tile_count
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) const fn new(
        route_index: u16,
        source_lobe: LobeKind,
        target_lobe: LobeKind,
        projection_type: ProjectionType,
        active_tile_policy: ActiveTilePolicy,
        update_cadence: UpdateCadence,
        priority: BiologicalPriority,
        delay_microsteps: u8,
        synapse_start: u32,
        synapse_len: u32,
        active_tile_count: u32,
    ) -> Self {
        Self {
            route_index,
            source_lobe,
            target_lobe,
            projection_type,
            active_tile_policy,
            update_cadence,
            priority,
            delay_microsteps,
            synapse_start,
            synapse_len,
            active_tile_count,
        }
    }
    pub(super) fn validate_local(&self) -> Result<(), ScaffoldContractError> {
        LobeKind::try_from_raw(self.source_lobe.raw())?;
        LobeKind::try_from_raw(self.target_lobe.raw())?;
        ProjectionType::try_from_raw(self.projection_type.raw())?;
        ActiveTilePolicy::try_from_raw(self.active_tile_policy.raw())?;
        UpdateCadence::try_from_raw(self.update_cadence.raw())?;
        BiologicalPriority::try_from_raw(self.priority.raw())?;
        if self.delay_microsteps != 0 || self.synapse_start.checked_add(self.synapse_len).is_none()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for CompiledProjection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            route_index: u16,
            source_lobe: LobeKind,
            target_lobe: LobeKind,
            projection_type: ProjectionType,
            active_tile_policy: ActiveTilePolicy,
            update_cadence: UpdateCadence,
            priority: BiologicalPriority,
            delay_microsteps: u8,
            synapse_start: u32,
            synapse_len: u32,
            active_tile_count: u32,
        }
        let w = Wire::deserialize(deserializer)?;
        let value = Self::new(
            w.route_index,
            w.source_lobe,
            w.target_lobe,
            w.projection_type,
            w.active_tile_policy,
            w.update_cadence,
            w.priority,
            w.delay_microsteps,
            w.synapse_start,
            w.synapse_len,
            w.active_tile_count,
        );
        value.validate_local().map_err(D::Error::custom)?;
        Ok(value)
    }
}
