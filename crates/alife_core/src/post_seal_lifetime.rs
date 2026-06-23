//! Runtime milestone: post-seal lifetime/H_shadow delta contract.
//!
//! This module is engine-agnostic. It accepts validated, bounded lifetime
//! trace deltas after an `ExperiencePatch` has been sealed. It never carries
//! GPU handles, renderer state, adapter IDs, or raw buffers.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    validate_finite, BrainClassId, ExperiencePatch, ExperiencePatchPhase, ExperienceSequenceId,
    NeuralProjectionSchema, OrganismId, ScaffoldContractError, SparseTilePayload, Tick, Validate,
};

pub const POST_SEAL_LIFETIME_DELTA_SCHEMA_VERSION: u16 = 1;
pub const POST_SEAL_LIFETIME_DELTA_MAX_RECORDS: usize = 1024;
pub const POST_SEAL_HSHADOW_ABS_LIMIT: f32 = 4.0;
pub const POST_SEAL_HSHADOW_VALUE_EPSILON: f32 = 1.0e-3;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostSealLifetimeDeltaSchemaVersion(pub u16);

impl PostSealLifetimeDeltaSchemaVersion {
    pub const CURRENT: Self = Self(POST_SEAL_LIFETIME_DELTA_SCHEMA_VERSION);

    pub const fn raw(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PostSealLifetimeDeltaSourceKind {
    CpuReference,
    GpuShadow,
    GpuCpuShadowGuarded,
    SleepConsolidation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PostSealLifetimeLayer {
    HShadow,
    LifetimePlastic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PostSealHShadowDeltaTarget {
    pub projection_index: u32,
    pub tile_index: u32,
    pub synapse_index: u16,
}

impl PostSealHShadowDeltaTarget {
    pub const fn new(projection_index: u32, tile_index: u32, synapse_index: u16) -> Self {
        Self {
            projection_index,
            tile_index,
            synapse_index,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PostSealLifetimeDeltaRecord {
    pub layer: PostSealLifetimeLayer,
    pub target: PostSealHShadowDeltaTarget,
    pub before_value: f32,
    pub after_value: f32,
    pub min_value: f32,
    pub max_value: f32,
}

impl PostSealLifetimeDeltaRecord {
    pub fn h_shadow(
        target: PostSealHShadowDeltaTarget,
        before_value: f32,
        after_value: f32,
        min_value: f32,
        max_value: f32,
    ) -> Result<Self, ScaffoldContractError> {
        let record = Self {
            layer: PostSealLifetimeLayer::HShadow,
            target,
            before_value,
            after_value,
            min_value,
            max_value,
        };
        record.validate_contract()?;
        Ok(record)
    }

    pub fn abs_delta(self) -> Result<f32, ScaffoldContractError> {
        validate_finite((self.after_value - self.before_value).abs())
    }
}

impl Validate for PostSealLifetimeDeltaRecord {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.layer != PostSealLifetimeLayer::HShadow {
            return Err(ScaffoldContractError::BackendParity);
        }
        validate_finite(self.before_value)?;
        validate_finite(self.after_value)?;
        validate_finite(self.min_value)?;
        validate_finite(self.max_value)?;
        if self.min_value > self.max_value
            || self.min_value < -POST_SEAL_HSHADOW_ABS_LIMIT
            || self.max_value > POST_SEAL_HSHADOW_ABS_LIMIT
            || self.before_value < self.min_value
            || self.before_value > self.max_value
            || self.after_value < self.min_value
            || self.after_value > self.max_value
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostSealLifetimeDeltaBatch {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub brain_class_id: BrainClassId,
    pub neuron_count: u32,
    pub max_active_synapses: u32,
    pub originating_tick: Tick,
    pub sealed_sequence_id: ExperienceSequenceId,
    pub source_kind: PostSealLifetimeDeltaSourceKind,
    pub cpu_shadow_parity_passed: bool,
    pub genetic_fixed_unchanged: bool,
    pub lifetime_consolidated_unchanged: bool,
    pub h_operational_unchanged: bool,
    pub records: Vec<PostSealLifetimeDeltaRecord>,
}

impl PostSealLifetimeDeltaBatch {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        organism_id: OrganismId,
        brain_class_id: BrainClassId,
        neuron_count: u32,
        max_active_synapses: u32,
        originating_tick: Tick,
        sealed_sequence_id: ExperienceSequenceId,
        source_kind: PostSealLifetimeDeltaSourceKind,
        cpu_shadow_parity_passed: bool,
        genetic_fixed_unchanged: bool,
        lifetime_consolidated_unchanged: bool,
        h_operational_unchanged: bool,
        records: Vec<PostSealLifetimeDeltaRecord>,
    ) -> Result<Self, ScaffoldContractError> {
        let batch = Self {
            schema_version: POST_SEAL_LIFETIME_DELTA_SCHEMA_VERSION,
            organism_id,
            brain_class_id,
            neuron_count,
            max_active_synapses,
            originating_tick,
            sealed_sequence_id,
            source_kind,
            cpu_shadow_parity_passed,
            genetic_fixed_unchanged,
            lifetime_consolidated_unchanged,
            h_operational_unchanged,
            records,
        };
        batch.validate_contract()?;
        Ok(batch)
    }

    pub fn validate_against_token(
        &self,
        token: &PostSealLearningToken,
    ) -> Result<(), ScaffoldContractError> {
        self.validate_contract()?;
        if self.organism_id != token.organism_id()
            || self.brain_class_id != token.brain_class_id()
            || self.originating_tick != token.originating_tick()
            || self.sealed_sequence_id != token.sealed_sequence_id()
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

impl Validate for PostSealLifetimeDeltaBatch {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != POST_SEAL_LIFETIME_DELTA_SCHEMA_VERSION {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.organism_id.validate()?;
        self.brain_class_id.validate()?;
        self.sealed_sequence_id.validate()?;
        if self.neuron_count == 0
            || self.max_active_synapses == 0
            || self.records.is_empty()
            || self.records.len() > POST_SEAL_LIFETIME_DELTA_MAX_RECORDS
            || !self.cpu_shadow_parity_passed
            || !self.genetic_fixed_unchanged
            || !self.lifetime_consolidated_unchanged
            || !self.h_operational_unchanged
        {
            return Err(ScaffoldContractError::BackendParity);
        }
        let mut seen = BTreeSet::new();
        for record in &self.records {
            record.validate_contract()?;
            if !seen.insert(record.target) {
                return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostSealLearningToken {
    organism_id: OrganismId,
    brain_class_id: BrainClassId,
    originating_tick: Tick,
    outcome_tick: Tick,
    sealed_sequence_id: ExperienceSequenceId,
}

impl PostSealLearningToken {
    pub fn from_sealed_patch(patch: &ExperiencePatch) -> Result<Self, ScaffoldContractError> {
        patch.validate_contract()?;
        if patch.header().phase != ExperiencePatchPhase::Sealed {
            return Err(ScaffoldContractError::UnorderedExperiencePhase);
        }
        Ok(Self {
            organism_id: patch.header().organism_id,
            brain_class_id: patch.pre_action().brain_class_id,
            originating_tick: patch.header().world_tick,
            outcome_tick: patch.outcome().outcome_tick,
            sealed_sequence_id: patch.header().sequence_id,
        })
    }

    pub fn from_optional_sealed_patch(
        patch: Option<&ExperiencePatch>,
    ) -> Result<Self, ScaffoldContractError> {
        Self::from_sealed_patch(patch.ok_or(ScaffoldContractError::MissingPhaseData)?)
    }

    pub const fn organism_id(self) -> OrganismId {
        self.organism_id
    }

    pub const fn brain_class_id(self) -> BrainClassId {
        self.brain_class_id
    }

    pub const fn originating_tick(self) -> Tick {
        self.originating_tick
    }

    pub const fn outcome_tick(self) -> Tick {
        self.outcome_tick
    }

    pub const fn sealed_sequence_id(self) -> ExperienceSequenceId {
        self.sealed_sequence_id
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostSealLifetimeDeltaApplication {
    pub token: PostSealLearningToken,
    pub batch: PostSealLifetimeDeltaBatch,
}

impl PostSealLifetimeDeltaApplication {
    pub fn new(
        token: PostSealLearningToken,
        batch: PostSealLifetimeDeltaBatch,
    ) -> Result<Self, ScaffoldContractError> {
        batch.validate_against_token(&token)?;
        Ok(Self { token, batch })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PostSealLifetimeDeltaRejectionReason {
    MissingSealedPatch,
    MismatchedCreature,
    MismatchedTickOrSequence,
    ReplayOrStaleSequence,
    InvalidDeltaValue,
    InvalidLayer,
    ShapeMismatch,
    CpuShadowParityFailed,
    GeneticOrOperationalLayerChanged,
    BatchTooLargeOrEmpty,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostSealLifetimeDeltaReceipt {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub brain_class_id: BrainClassId,
    pub originating_tick: Tick,
    pub outcome_tick: Tick,
    pub sealed_sequence_id: ExperienceSequenceId,
    pub source_kind: PostSealLifetimeDeltaSourceKind,
    pub applied_records: u32,
    pub changed_records: u32,
    pub max_abs_delta: f32,
    pub h_shadow_changed: bool,
    pub genetic_fixed_unchanged: bool,
    pub lifetime_consolidated_unchanged: bool,
    pub h_operational_unchanged: bool,
    pub post_seal_only: bool,
    pub replay_protected: bool,
}

impl Validate for PostSealLifetimeDeltaReceipt {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != POST_SEAL_LIFETIME_DELTA_SCHEMA_VERSION
            || self.applied_records == 0
            || !self.genetic_fixed_unchanged
            || !self.lifetime_consolidated_unchanged
            || !self.h_operational_unchanged
            || !self.post_seal_only
            || !self.replay_protected
        {
            return Err(ScaffoldContractError::BackendParity);
        }
        self.organism_id.validate()?;
        self.brain_class_id.validate()?;
        self.sealed_sequence_id.validate()?;
        validate_finite(self.max_abs_delta)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AppliedPostSealDeltaStats {
    pub applied_records: u32,
    pub changed_records: u32,
    pub max_abs_delta: f32,
}

pub(crate) fn apply_hshadow_delta_records_to_schema(
    schema: &mut NeuralProjectionSchema,
    records: &[PostSealLifetimeDeltaRecord],
) -> Result<AppliedPostSealDeltaStats, ScaffoldContractError> {
    schema.validate()?;
    let mut applied_records = 0_u32;
    let mut changed_records = 0_u32;
    let mut max_abs_delta = 0.0_f32;
    for record in records {
        record.validate_contract()?;
        let weights = hshadow_target_mut(schema, record.target)?;
        if (weights.h_shadow - record.before_value).abs() > POST_SEAL_HSHADOW_VALUE_EPSILON {
            return Err(ScaffoldContractError::BackendParity);
        }
        let delta = record.abs_delta()?;
        weights.h_shadow = record.after_value;
        applied_records = applied_records.saturating_add(1);
        if delta > POST_SEAL_HSHADOW_VALUE_EPSILON {
            changed_records = changed_records.saturating_add(1);
        }
        max_abs_delta = max_abs_delta.max(delta);
    }
    schema.validate()?;
    Ok(AppliedPostSealDeltaStats {
        applied_records,
        changed_records,
        max_abs_delta,
    })
}

fn hshadow_target_mut(
    schema: &mut NeuralProjectionSchema,
    target: PostSealHShadowDeltaTarget,
) -> Result<&mut crate::SynapseWeightSplit, ScaffoldContractError> {
    let projection = schema
        .projections
        .get_mut(target.projection_index as usize)
        .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
    if projection.projection_index != target.projection_index {
        return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
    }
    let tile = projection
        .tiles
        .get_mut(target.tile_index as usize)
        .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
    match &mut tile.payload {
        SparseTilePayload::Dense(dense) => dense
            .weights
            .get_mut(target.synapse_index as usize)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema),
        SparseTilePayload::Coo(coo) => coo
            .entries
            .get_mut(target.synapse_index as usize)
            .map(|entry| &mut entry.weights)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema),
        SparseTilePayload::RowRunUnsupported | SparseTilePayload::ColumnRunUnsupported => {
            Err(ScaffoldContractError::UnsupportedSparseTileFormat)
        }
    }
}
