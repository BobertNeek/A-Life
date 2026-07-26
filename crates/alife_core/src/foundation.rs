//! Frozen foundation ABI and N2048 trainable layout contracts.

use std::mem::size_of;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::blake3_digest::{domain_hasher, Blake3Write};
use crate::{
    ActiveTilePolicy, BiologicalPriority, Blake3Digest, BrainCapacityClass, BrainClassId,
    BrainPhenotype, LobeKind, LobeLayout, ProjectionType, ScaffoldContractError, SensorProfile,
    UpdateCadence,
};
use crate::{LanguageCodebookV1, LobeRegion};

const LAYOUT_DOMAIN: &[u8] = b"alife.foundation.layout-abi.v1";
const ROUTE_DOMAIN: &[u8] = b"alife.foundation.route-abi.v1";
const PLASTICITY_DOMAIN: &[u8] = b"alife.foundation.plasticity-abi.v1";
const FOUNDATION_PAYLOAD_DOMAIN: &[u8] = b"alife.foundation.weight-payload.v1";
const TRAINING_STAGE_DOMAIN: &[u8] = b"alife.foundation.training-stage.v1";
const PROMOTION_RECEIPT_DOMAIN: &[u8] = b"alife.foundation.promotion-receipt.v1";
const FOUNDATION_BOOTSTRAP_PROVENANCE_DOMAIN: &[u8] = b"alife.foundation.bootstrap-provenance.v1";
const FOUNDATION_ASSET_MAGIC: [u8; 8] = *b"ALFN2048";
const FOUNDATION_ASSET_CODEC_VERSION: u16 = 1;
const FOUNDATION_ASSET_MAX_WEIGHTS: usize = 65_536;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifetimePlasticityBand {
    Fixed = 0,
    Slow = 1,
    Fast = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundationSectionPolicy {
    fixed_synapses: u32,
    slow_synapses: u32,
    fast_synapses: u32,
}

impl FoundationSectionPolicy {
    pub const fn new(fixed_synapses: u32, slow_synapses: u32, fast_synapses: u32) -> Self {
        Self {
            fixed_synapses,
            slow_synapses,
            fast_synapses,
        }
    }

    pub const fn count(self, band: LifetimePlasticityBand) -> u32 {
        match band {
            LifetimePlasticityBand::Fixed => self.fixed_synapses,
            LifetimePlasticityBand::Slow => self.slow_synapses,
            LifetimePlasticityBand::Fast => self.fast_synapses,
        }
    }

    pub const fn total_synapses(self) -> u32 {
        self.fixed_synapses + self.slow_synapses + self.fast_synapses
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct N2048FoundationRouteSpec {
    source_lobe: LobeKind,
    target_lobe: LobeKind,
    synapse_count: u32,
    section_policy: FoundationSectionPolicy,
    projection_type: ProjectionType,
    active_tile_policy: ActiveTilePolicy,
    update_cadence: UpdateCadence,
    priority: BiologicalPriority,
}

impl N2048FoundationRouteSpec {
    #[allow(clippy::too_many_arguments)]
    const fn new(
        source_lobe: LobeKind,
        target_lobe: LobeKind,
        synapse_count: u32,
        section_policy: FoundationSectionPolicy,
        projection_type: ProjectionType,
        update_cadence: UpdateCadence,
        priority: BiologicalPriority,
    ) -> Self {
        Self {
            source_lobe,
            target_lobe,
            synapse_count,
            section_policy,
            projection_type,
            active_tile_policy: ActiveTilePolicy::EssentialReservation,
            update_cadence,
            priority,
        }
    }

    pub const fn source_lobe(self) -> LobeKind {
        self.source_lobe
    }
    pub const fn target_lobe(self) -> LobeKind {
        self.target_lobe
    }
    pub const fn synapse_count(self) -> u32 {
        self.synapse_count
    }
    pub const fn section_policy(self) -> FoundationSectionPolicy {
        self.section_policy
    }
    pub const fn projection_type(self) -> ProjectionType {
        self.projection_type
    }
    pub const fn active_tile_policy(self) -> ActiveTilePolicy {
        self.active_tile_policy
    }
    pub const fn update_cadence(self) -> UpdateCadence {
        self.update_cadence
    }
    pub const fn priority(self) -> BiologicalPriority {
        self.priority
    }
}

const fn slow(count: u32) -> FoundationSectionPolicy {
    FoundationSectionPolicy::new(0, count, 0)
}

const fn fast(count: u32) -> FoundationSectionPolicy {
    FoundationSectionPolicy::new(0, 0, count)
}

const N2048_ROUTES: [N2048FoundationRouteSpec; 16] = [
    N2048FoundationRouteSpec::new(
        LobeKind::SensoryGrounding,
        LobeKind::CoreAssociation,
        3_584,
        slow(3_584),
        ProjectionType::FeedForward,
        UpdateCadence::Hot60Hz,
        BiologicalPriority::Essential,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::AuditorySpeech,
        LobeKind::CoreAssociation,
        1_536,
        slow(1_536),
        ProjectionType::FeedForward,
        UpdateCadence::Hot15To60Hz,
        BiologicalPriority::High,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::GlyphVision,
        LobeKind::CoreAssociation,
        1_536,
        slow(1_536),
        ProjectionType::FeedForward,
        UpdateCadence::Hot15To60Hz,
        BiologicalPriority::High,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::MetabolicDrive,
        LobeKind::HomeostaticRegulation,
        1_024,
        slow(1_024),
        ProjectionType::Homeostatic,
        UpdateCadence::Hot10To30Hz,
        BiologicalPriority::Essential,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::HomeostaticRegulation,
        LobeKind::CoreAssociation,
        1_024,
        slow(1_024),
        ProjectionType::Modulatory,
        UpdateCadence::Hot10To30Hz,
        BiologicalPriority::Essential,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::HomeostaticRegulation,
        LobeKind::MotorArbitration,
        768,
        slow(768),
        ProjectionType::Homeostatic,
        UpdateCadence::Hot10To30Hz,
        BiologicalPriority::Essential,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::CoreAssociation,
        LobeKind::MotorArbitration,
        3_072,
        FoundationSectionPolicy::new(0, 2_048, 1_024),
        ProjectionType::MotorProposal,
        UpdateCadence::Hot60Hz,
        BiologicalPriority::Essential,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::MotorArbitration,
        LobeKind::MotorArbitration,
        1_536,
        slow(1_536),
        ProjectionType::LateralInhibition,
        UpdateCadence::Hot60Hz,
        BiologicalPriority::Essential,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::CoreAssociation,
        LobeKind::WorkingMemory,
        1_536,
        fast(1_536),
        ProjectionType::FeedForward,
        UpdateCadence::Hot15To60Hz,
        BiologicalPriority::High,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::WorkingMemory,
        LobeKind::CoreAssociation,
        1_536,
        fast(1_536),
        ProjectionType::Feedback,
        UpdateCadence::Hot15To60Hz,
        BiologicalPriority::High,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::CoreAssociation,
        LobeKind::EpisodicMemory,
        1_536,
        fast(1_536),
        ProjectionType::FeedForward,
        UpdateCadence::Hot5To15Hz,
        BiologicalPriority::Normal,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::EpisodicMemory,
        LobeKind::CoreAssociation,
        1_536,
        fast(1_536),
        ProjectionType::Feedback,
        UpdateCadence::Hot5To15Hz,
        BiologicalPriority::Normal,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::CoreAssociation,
        LobeKind::LexiconConcept,
        1_536,
        fast(1_536),
        ProjectionType::FeedForward,
        UpdateCadence::Hot5To15Hz,
        BiologicalPriority::Normal,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::LexiconConcept,
        LobeKind::CoreAssociation,
        1_536,
        fast(1_536),
        ProjectionType::Feedback,
        UpdateCadence::Hot5To15Hz,
        BiologicalPriority::Normal,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::LexiconConcept,
        LobeKind::WorkingMemory,
        768,
        fast(768),
        ProjectionType::FeedForward,
        UpdateCadence::Hot15To60Hz,
        BiologicalPriority::High,
    ),
    N2048FoundationRouteSpec::new(
        LobeKind::WorkingMemory,
        LobeKind::LexiconConcept,
        512,
        fast(512),
        ProjectionType::Feedback,
        UpdateCadence::Hot15To60Hz,
        BiologicalPriority::High,
    ),
];

pub struct N2048FoundationLayoutV1;

impl N2048FoundationLayoutV1 {
    pub const NEURON_COUNT: u32 = 2_048;
    pub const RECURRENT_SYNAPSE_COUNT: u32 = 24_576;
    pub const ACTION_DECODER_SYNAPSE_COUNT: u32 = 4_096;
    pub const CANDIDATE_DECODER_SYNAPSE_COUNT: u32 = 3_072;
    pub const CANDIDATE_FAMILY_COUNT: u16 = 8;
    pub const CANDIDATE_MOTOR_UNITS_PER_FAMILY: u16 = 16;
    pub const SPEECH_DECODER_SYNAPSE_COUNT: u32 = 1_024;
    pub const MEMORY_DECODER_SYNAPSE_COUNT: u32 = 4_096;
    pub const MEMORY_DECODER_INPUT_WIDTH: u16 = 64;
    pub const MEMORY_DECODER_OUTPUT_WIDTH: u16 = 64;

    pub fn lobe_layout() -> LobeLayout {
        let lengths = [256, 128, 128, 128, 256, 448, 256, 128, 224, 96];
        let mut cursor = 0_u32;
        let mut regions = Vec::with_capacity(LobeKind::ALL.len());
        for (kind, len) in LobeKind::CORE.into_iter().zip(lengths) {
            regions.push(LobeRegion::enabled(kind, cursor, len));
            cursor += len;
        }
        for kind in LobeKind::ALL.into_iter().skip(LobeKind::CORE.len()) {
            regions.push(LobeRegion::disabled(kind, cursor));
        }
        let layout = LobeLayout { regions };
        debug_assert!(layout.validate_for_neuron_count(Self::NEURON_COUNT).is_ok());
        layout
    }

    pub const fn route_specs() -> &'static [N2048FoundationRouteSpec] {
        &N2048_ROUTES
    }

    pub fn route_abi_digest() -> Blake3Digest {
        route_digest(Self::route_specs())
    }

    pub fn plasticity_abi_digest() -> Blake3Digest {
        plasticity_digest(Self::route_specs())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FoundationLayoutId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FoundationId(u64);

impl FoundationId {
    pub const N2048_V1: Self = Self(0x4E32_3034_385F_5631);

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FoundationVersion(u32);

impl FoundationVersion {
    pub const V1: Self = Self(1);

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FoundationCompatibilityFamilyId(u64);

impl FoundationCompatibilityFamilyId {
    pub const N2048_FOUNDATION: Self = Self(0x4E32_3034_385F_FA11);

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FoundationWeightAssetRef {
    digest: Blake3Digest,
    weight_count: u32,
}

/// Training-curriculum identity bound into an immutable foundation payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrainingStageManifest {
    schema_version: u16,
    curriculum_version: u32,
    evaluation_version: u32,
    completed_stage_count: u16,
    manifest_digest: Blake3Digest,
}

impl TrainingStageManifest {
    pub fn bootstrap() -> Self {
        Self::new(0, 0, 0)
    }

    pub fn new(
        curriculum_version: u32,
        evaluation_version: u32,
        completed_stage_count: u16,
    ) -> Self {
        let mut value = Self {
            schema_version: 1,
            curriculum_version,
            evaluation_version,
            completed_stage_count,
            manifest_digest: Blake3Digest::default(),
        };
        value.manifest_digest = value.recompute_digest();
        value
    }

    pub const fn curriculum_version(self) -> u32 {
        self.curriculum_version
    }

    pub const fn evaluation_version(self) -> u32 {
        self.evaluation_version
    }

    pub const fn completed_stage_count(self) -> u16 {
        self.completed_stage_count
    }

    pub const fn digest(self) -> Blake3Digest {
        self.manifest_digest
    }

    fn validate(self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != 1 || self.manifest_digest != self.recompute_digest() {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn recompute_digest(self) -> Blake3Digest {
        let mut digest = domain_hasher(TRAINING_STAGE_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u32(self.curriculum_version);
        digest.write_u32(self.evaluation_version);
        digest.write_u16(self.completed_stage_count);
        Blake3Digest::from_hasher(digest)
    }
}

/// Auditable promotion state. Bootstrap assets are explicit and never masquerade as trained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FoundationPromotionReceipt {
    schema_version: u16,
    training_manifest_digest: Blake3Digest,
    evaluation_evidence_digest: Option<Blake3Digest>,
    provenance_digest: Blake3Digest,
    receipt_digest: Blake3Digest,
}

impl FoundationPromotionReceipt {
    pub fn bootstrap(training: TrainingStageManifest) -> Self {
        let mut provenance = domain_hasher(FOUNDATION_BOOTSTRAP_PROVENANCE_DOMAIN);
        for byte in training.digest().bytes() {
            provenance.write_u8(*byte);
        }
        Self::new(
            training.digest(),
            None,
            Blake3Digest::from_hasher(provenance),
        )
    }

    pub fn promoted(
        training_manifest_digest: Blake3Digest,
        evaluation_evidence_digest: Blake3Digest,
        provenance_digest: Blake3Digest,
    ) -> Result<Self, ScaffoldContractError> {
        if evaluation_evidence_digest == Blake3Digest::default()
            || provenance_digest == Blake3Digest::default()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(Self::new(
            training_manifest_digest,
            Some(evaluation_evidence_digest),
            provenance_digest,
        ))
    }

    fn new(
        training_manifest_digest: Blake3Digest,
        evaluation_evidence_digest: Option<Blake3Digest>,
        provenance_digest: Blake3Digest,
    ) -> Self {
        let mut value = Self {
            schema_version: 1,
            training_manifest_digest,
            evaluation_evidence_digest,
            provenance_digest,
            receipt_digest: Blake3Digest::default(),
        };
        value.receipt_digest = value.recompute_digest();
        value
    }

    pub const fn is_promoted(self) -> bool {
        self.evaluation_evidence_digest.is_some()
    }

    pub const fn digest(self) -> Blake3Digest {
        self.receipt_digest
    }

    fn validate(self, training: TrainingStageManifest) -> Result<(), ScaffoldContractError> {
        if self.schema_version != 1
            || self.training_manifest_digest != training.digest()
            || self.provenance_digest == Blake3Digest::default()
            || self
                .evaluation_evidence_digest
                .is_some_and(|digest| digest == Blake3Digest::default())
            || self.receipt_digest != self.recompute_digest()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn recompute_digest(self) -> Blake3Digest {
        let mut digest = domain_hasher(PROMOTION_RECEIPT_DOMAIN);
        digest.write_u16(self.schema_version);
        for byte in self.training_manifest_digest.bytes() {
            digest.write_u8(*byte);
        }
        write_optional_blake3_digest(&mut digest, self.evaluation_evidence_digest);
        for byte in self.provenance_digest.bytes() {
            digest.write_u8(*byte);
        }
        Blake3Digest::from_hasher(digest)
    }
}

impl FoundationWeightAssetRef {
    pub const fn digest(self) -> Blake3Digest {
        self.digest
    }

    pub const fn weight_count(self) -> u32 {
        self.weight_count
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundationManifest {
    schema_version: u16,
    foundation_id: FoundationId,
    foundation_version: FoundationVersion,
    compatibility_family_id: FoundationCompatibilityFamilyId,
    capacity_class_id: BrainClassId,
    sensor_profile: SensorProfile,
    layout_digest: Blake3Digest,
    language_codebook_digest: Blake3Digest,
    action_decoder_digest: [u64; 4],
    speech_decoder_digest: Option<[u64; 4]>,
    memory_decoder_digest: Option<[u64; 4]>,
    route_abi_digest: Blake3Digest,
    plasticity_abi_digest: Blake3Digest,
    address_map_digest: Blake3Digest,
    training_stage: TrainingStageManifest,
    promotion_receipt: FoundationPromotionReceipt,
    weight_asset: FoundationWeightAssetRef,
}

impl FoundationManifest {
    pub fn from_phenotype(
        phenotype: &BrainPhenotype,
        weight_asset: FoundationWeightAssetRef,
    ) -> Result<Self, ScaffoldContractError> {
        let training_stage = TrainingStageManifest::bootstrap();
        Ok(Self {
            schema_version: 1,
            foundation_id: FoundationId::N2048_V1,
            foundation_version: FoundationVersion::V1,
            compatibility_family_id: FoundationCompatibilityFamilyId::N2048_FOUNDATION,
            capacity_class_id: phenotype.brain_class_id(),
            sensor_profile: phenotype.sensor_profile(),
            layout_digest: phenotype.foundation_abi().layout_digest(),
            language_codebook_digest: phenotype.language_codebook().canonical_digest(),
            action_decoder_digest: phenotype.candidate_decoder().canonical_digest(),
            speech_decoder_digest: phenotype
                .speech_decoder()
                .map(|decoder| decoder.canonical_digest()),
            memory_decoder_digest: phenotype
                .memory_decoder()
                .map(|decoder| decoder.canonical_digest()),
            route_abi_digest: phenotype.route_abi_digest(),
            plasticity_abi_digest: phenotype.plasticity_abi_digest(),
            address_map_digest: phenotype.persistent_address_map().digest(),
            training_stage,
            promotion_receipt: FoundationPromotionReceipt::bootstrap(training_stage),
            weight_asset,
        })
    }

    pub const fn foundation_id(&self) -> FoundationId {
        self.foundation_id
    }

    pub const fn foundation_version(&self) -> FoundationVersion {
        self.foundation_version
    }

    pub const fn compatibility_family_id(&self) -> FoundationCompatibilityFamilyId {
        self.compatibility_family_id
    }

    pub const fn weight_asset(&self) -> FoundationWeightAssetRef {
        self.weight_asset
    }

    pub const fn training_stage(&self) -> TrainingStageManifest {
        self.training_stage
    }

    pub const fn promotion_receipt(&self) -> FoundationPromotionReceipt {
        self.promotion_receipt
    }

    pub fn validate_against(
        &self,
        phenotype: &BrainPhenotype,
    ) -> Result<(), ScaffoldContractError> {
        self.training_stage.validate()?;
        self.promotion_receipt.validate(self.training_stage)?;
        if self.schema_version != 1
            || self.foundation_id != FoundationId::N2048_V1
            || self.foundation_version != FoundationVersion::V1
            || self.compatibility_family_id != FoundationCompatibilityFamilyId::N2048_FOUNDATION
            || self.capacity_class_id != phenotype.brain_class_id()
            || self.sensor_profile != phenotype.sensor_profile()
            || self.layout_digest != phenotype.foundation_abi().layout_digest()
            || phenotype.foundation_abi().foundation_id() != Some(self.foundation_id)
            || phenotype.foundation_abi().foundation_version() != Some(self.foundation_version)
            || phenotype.foundation_abi().compatibility_family_id()
                != Some(self.compatibility_family_id)
            || phenotype.foundation_abi().foundation_weight_asset() != Some(self.weight_asset)
            || self.language_codebook_digest != phenotype.language_codebook().canonical_digest()
            || self.action_decoder_digest != phenotype.candidate_decoder().canonical_digest()
            || self.speech_decoder_digest
                != phenotype
                    .speech_decoder()
                    .map(|decoder| decoder.canonical_digest())
            || self.memory_decoder_digest
                != phenotype
                    .memory_decoder()
                    .map(|decoder| decoder.canonical_digest())
            || self.route_abi_digest != phenotype.route_abi_digest()
            || self.plasticity_abi_digest != phenotype.plasticity_abi_digest()
            || self.address_map_digest != phenotype.persistent_address_map().digest()
            || self.weight_asset.weight_count as usize != phenotype.synapses().len()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundationWeightAsset {
    manifest: FoundationManifest,
    weights: Vec<f32>,
    digest: Blake3Digest,
}

impl FoundationWeightAsset {
    pub fn builtin_n2048_v1(sensor_profile: SensorProfile) -> Result<Self, ScaffoldContractError> {
        let bytes: &[u8] = match sensor_profile {
            SensorProfile::PrivilegedAffordanceV1 => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/brain_foundations/n2048-v1-privileged.alife-foundation"
            )),
            SensorProfile::GroundedObjectSlotsV1 => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/brain_foundations/n2048-v1-grounded.alife-foundation"
            )),
        };
        Self::decode_canonical(bytes)
    }

    pub fn from_phenotype_for_genetic_birth(
        phenotype: &BrainPhenotype,
    ) -> Result<Self, ScaffoldContractError> {
        let training_stage = TrainingStageManifest::bootstrap();
        Self::from_phenotype_with_provenance(
            phenotype,
            training_stage,
            FoundationPromotionReceipt::bootstrap(training_stage),
        )
    }

    pub fn from_phenotype_with_provenance(
        phenotype: &BrainPhenotype,
        training_stage: TrainingStageManifest,
        promotion_receipt: FoundationPromotionReceipt,
    ) -> Result<Self, ScaffoldContractError> {
        let weights = phenotype
            .synapses()
            .iter()
            .map(|synapse| synapse.genetic_weight())
            .collect::<Vec<_>>();
        Self::from_weights_with_provenance(phenotype, weights, training_stage, promotion_receipt)
    }

    /// Builds an unpromoted training candidate from canonical phenotype-order
    /// weights. Optimizer moments, gradients, targets, and auxiliary heads are
    /// intentionally absent from the production foundation asset.
    pub fn from_trained_weights(
        phenotype: &BrainPhenotype,
        weights: Vec<f32>,
        training_stage: TrainingStageManifest,
    ) -> Result<Self, ScaffoldContractError> {
        Self::from_weights_with_provenance(
            phenotype,
            weights,
            training_stage,
            FoundationPromotionReceipt::bootstrap(training_stage),
        )
    }

    fn from_weights_with_provenance(
        phenotype: &BrainPhenotype,
        weights: Vec<f32>,
        training_stage: TrainingStageManifest,
        promotion_receipt: FoundationPromotionReceipt,
    ) -> Result<Self, ScaffoldContractError> {
        if phenotype.brain_class_id() != BrainCapacityClass::N2048_ID
            || weights.len() != phenotype.synapses().len()
            || weights.iter().any(|weight| !weight.is_finite())
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        training_stage.validate()?;
        promotion_receipt.validate(training_stage)?;
        let mut manifest = FoundationManifest::from_phenotype(
            phenotype,
            FoundationWeightAssetRef {
                digest: Blake3Digest::default(),
                weight_count: u32::try_from(weights.len())
                    .map_err(|_| ScaffoldContractError::PhenotypeCompile)?,
            },
        )?;
        manifest.training_stage = training_stage;
        manifest.promotion_receipt = promotion_receipt;
        let digest = compute_weight_asset_digest(&manifest, &weights)?;
        manifest.weight_asset = FoundationWeightAssetRef {
            digest,
            weight_count: u32::try_from(weights.len())
                .map_err(|_| ScaffoldContractError::PhenotypeCompile)?,
        };
        Ok(Self {
            manifest,
            weights,
            digest,
        })
    }

    pub const fn digest(&self) -> Blake3Digest {
        self.digest
    }

    pub const fn manifest(&self) -> &FoundationManifest {
        &self.manifest
    }

    pub fn weights(&self) -> &[f32] {
        &self.weights
    }

    pub const fn asset_ref(&self) -> FoundationWeightAssetRef {
        self.manifest.weight_asset()
    }

    pub fn validate_against(
        &self,
        phenotype: &BrainPhenotype,
    ) -> Result<(), ScaffoldContractError> {
        self.manifest.validate_against(phenotype)?;
        if self.weights.len() != phenotype.synapses().len()
            || self.weights.iter().any(|weight| !weight.is_finite())
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let digest = compute_weight_asset_digest(&self.manifest, &self.weights)?;
        if digest != self.digest || digest != self.manifest.weight_asset.digest {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    /// Stable little-endian payload used by checked-in foundation assets and trainer exports.
    pub fn encode_canonical(&self) -> Result<Vec<u8>, ScaffoldContractError> {
        self.validate_self_contained()?;
        let mut out = Vec::with_capacity(512 + self.weights.len() * size_of::<f32>());
        out.extend_from_slice(&FOUNDATION_ASSET_MAGIC);
        push_u16(&mut out, FOUNDATION_ASSET_CODEC_VERSION);
        encode_manifest(&mut out, &self.manifest);
        push_u32(
            &mut out,
            u32::try_from(self.weights.len())
                .map_err(|_| ScaffoldContractError::PhenotypeCompile)?,
        );
        for weight in &self.weights {
            push_u32(&mut out, weight.to_bits());
        }
        push_blake3(&mut out, self.digest);
        Ok(out)
    }

    pub fn decode_canonical(bytes: &[u8]) -> Result<Self, ScaffoldContractError> {
        let mut cursor = FoundationAssetCursor::new(bytes);
        if cursor.take(FOUNDATION_ASSET_MAGIC.len())? != FOUNDATION_ASSET_MAGIC {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        if cursor.u16()? != FOUNDATION_ASSET_CODEC_VERSION {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let manifest = decode_manifest(&mut cursor)?;
        let weight_count =
            usize::try_from(cursor.u32()?).map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
        if weight_count == 0 || weight_count > FOUNDATION_ASSET_MAX_WEIGHTS {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let weight_bytes = weight_count
            .checked_mul(size_of::<f32>())
            .and_then(|value| value.checked_add(32))
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        if cursor.remaining() != weight_bytes {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let mut weights = Vec::with_capacity(weight_count);
        for _ in 0..weight_count {
            weights.push(f32::from_bits(cursor.u32()?));
        }
        let digest = cursor.blake3()?;
        if cursor.remaining() != 0 {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let asset = Self {
            manifest,
            weights,
            digest,
        };
        asset.validate_self_contained()?;
        Ok(asset)
    }

    fn validate_self_contained(&self) -> Result<(), ScaffoldContractError> {
        self.manifest.training_stage.validate()?;
        self.manifest
            .promotion_receipt
            .validate(self.manifest.training_stage)?;
        if self.manifest.schema_version != 1
            || self.manifest.capacity_class_id != BrainCapacityClass::N2048_ID
            || self.weights.is_empty()
            || self.weights.len() > FOUNDATION_ASSET_MAX_WEIGHTS
            || self.weights.len() != self.manifest.weight_asset.weight_count as usize
            || self.weights.iter().any(|weight| !weight.is_finite())
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let digest = compute_weight_asset_digest(&self.manifest, &self.weights)?;
        if digest != self.digest || digest != self.manifest.weight_asset.digest {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FoundationAbiBinding {
    schema_version: u16,
    capacity_class_id: BrainClassId,
    layout_id: FoundationLayoutId,
    layout_digest: Blake3Digest,
    foundation_id: Option<FoundationId>,
    foundation_version: Option<FoundationVersion>,
    compatibility_family_id: Option<FoundationCompatibilityFamilyId>,
    weight_asset: Option<FoundationWeightAssetRef>,
    language_codebook: LanguageCodebookV1,
}

impl FoundationAbiBinding {
    pub fn canonical_for_capacity(
        capacity: &BrainCapacityClass,
    ) -> Result<Self, ScaffoldContractError> {
        capacity.validate_contract()?;
        let layout = if capacity.execution().max_neurons() == N2048FoundationLayoutV1::NEURON_COUNT
        {
            N2048FoundationLayoutV1::lobe_layout()
        } else {
            LobeLayout::reference_for_neuron_count(capacity.execution().max_neurons())?
        };
        Ok(Self {
            schema_version: 1,
            capacity_class_id: capacity.id(),
            layout_id: FoundationLayoutId(0xA11F_0000_0000_0000 | u64::from(capacity.id().raw())),
            layout_digest: layout_digest(&layout),
            foundation_id: None,
            foundation_version: None,
            compatibility_family_id: None,
            weight_asset: None,
            language_codebook: LanguageCodebookV1::canonical(),
        })
    }

    pub fn canonical_for_foundation_asset(
        capacity: &BrainCapacityClass,
        asset: &FoundationWeightAsset,
    ) -> Result<Self, ScaffoldContractError> {
        asset.validate_self_contained()?;
        let mut value = Self::canonical_for_capacity(capacity)?;
        value.foundation_id = Some(asset.manifest.foundation_id());
        value.foundation_version = Some(asset.manifest.foundation_version());
        value.compatibility_family_id = Some(asset.manifest.compatibility_family_id());
        value.weight_asset = Some(asset.asset_ref());
        value.validate_against(capacity)?;
        Ok(value)
    }

    pub const fn layout_id(&self) -> FoundationLayoutId {
        self.layout_id
    }
    pub const fn capacity_class_id(&self) -> BrainClassId {
        self.capacity_class_id
    }
    pub const fn layout_digest(&self) -> Blake3Digest {
        self.layout_digest
    }
    pub const fn language_codebook(&self) -> &LanguageCodebookV1 {
        &self.language_codebook
    }
    pub const fn foundation_id(&self) -> Option<FoundationId> {
        self.foundation_id
    }
    pub const fn foundation_version(&self) -> Option<FoundationVersion> {
        self.foundation_version
    }
    pub const fn compatibility_family_id(&self) -> Option<FoundationCompatibilityFamilyId> {
        self.compatibility_family_id
    }
    pub const fn foundation_payload_digest(&self) -> Option<Blake3Digest> {
        match self.weight_asset {
            Some(asset) => Some(asset.digest),
            None => None,
        }
    }
    pub const fn foundation_weight_asset(&self) -> Option<FoundationWeightAssetRef> {
        self.weight_asset
    }

    pub fn validate_against(
        &self,
        capacity: &BrainCapacityClass,
    ) -> Result<(), ScaffoldContractError> {
        self.language_codebook.validate_contract()?;
        let canonical = Self::canonical_for_capacity(capacity)?;
        if self.schema_version != 1
            || self.capacity_class_id != canonical.capacity_class_id
            || self.layout_id != canonical.layout_id
            || self.layout_digest != canonical.layout_digest
            || self.language_codebook != canonical.language_codebook
            || !matches!(
                (
                    self.foundation_id,
                    self.foundation_version,
                    self.compatibility_family_id,
                    self.weight_asset,
                ),
                (Some(_), Some(_), Some(_), Some(_)) | (None, None, None, None)
            )
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        if let (Some(id), Some(version), Some(family), Some(asset)) = (
            self.foundation_id,
            self.foundation_version,
            self.compatibility_family_id,
            self.weight_asset,
        ) {
            if capacity.id() != BrainCapacityClass::N2048_ID
                || id != FoundationId::N2048_V1
                || version != FoundationVersion::V1
                || family != FoundationCompatibilityFamilyId::N2048_FOUNDATION
                || asset.weight_count == 0
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
        }
        if self.schema_version != 1 {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for FoundationAbiBinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            capacity_class_id: BrainClassId,
            layout_id: FoundationLayoutId,
            layout_digest: Blake3Digest,
            foundation_id: Option<FoundationId>,
            foundation_version: Option<FoundationVersion>,
            compatibility_family_id: Option<FoundationCompatibilityFamilyId>,
            weight_asset: Option<FoundationWeightAssetRef>,
            language_codebook: LanguageCodebookV1,
        }
        let w = Wire::deserialize(deserializer)?;
        let value = Self {
            schema_version: w.schema_version,
            capacity_class_id: w.capacity_class_id,
            layout_id: w.layout_id,
            layout_digest: w.layout_digest,
            foundation_id: w.foundation_id,
            foundation_version: w.foundation_version,
            compatibility_family_id: w.compatibility_family_id,
            weight_asset: w.weight_asset,
            language_codebook: w.language_codebook,
        };
        let capacity = BrainCapacityClass::production_for_id(value.capacity_class_id)
            .map_err(D::Error::custom)?;
        value
            .validate_against(&capacity)
            .map_err(D::Error::custom)?;
        Ok(value)
    }
}

fn compute_weight_asset_digest(
    manifest: &FoundationManifest,
    weights: &[f32],
) -> Result<Blake3Digest, ScaffoldContractError> {
    if weights.is_empty() || weights.iter().any(|weight| !weight.is_finite()) {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    let mut digest = domain_hasher(FOUNDATION_PAYLOAD_DOMAIN);
    digest.write_u64(manifest.foundation_id.raw());
    digest.write_u32(manifest.foundation_version.raw());
    digest.write_u16(manifest.capacity_class_id.raw());
    digest.write_u16(manifest.sensor_profile.raw());
    for source in [
        manifest.layout_digest,
        manifest.language_codebook_digest,
        manifest.route_abi_digest,
        manifest.plasticity_abi_digest,
        manifest.address_map_digest,
    ] {
        for byte in source.bytes() {
            digest.write_u8(*byte);
        }
    }
    for word in manifest.action_decoder_digest {
        digest.write_u64(word);
    }
    write_optional_digest(&mut digest, manifest.speech_decoder_digest);
    write_optional_digest(&mut digest, manifest.memory_decoder_digest);
    for source in [
        manifest.training_stage.digest(),
        manifest.promotion_receipt.digest(),
    ] {
        for byte in source.bytes() {
            digest.write_u8(*byte);
        }
    }
    digest.write_len(weights.len());
    for weight in weights {
        digest.write_u32(weight.to_bits());
    }
    Ok(Blake3Digest::from_hasher(digest))
}

fn write_optional_blake3_digest(digest: &mut blake3::Hasher, value: Option<Blake3Digest>) {
    match value {
        Some(value) => {
            digest.write_u8(1);
            for byte in value.bytes() {
                digest.write_u8(*byte);
            }
        }
        None => digest.write_u8(0),
    }
}

fn write_optional_digest(digest: &mut blake3::Hasher, value: Option<[u64; 4]>) {
    match value {
        Some(words) => {
            digest.write_u8(1);
            for word in words {
                digest.write_u64(word);
            }
        }
        None => digest.write_u8(0),
    }
}

fn encode_manifest(out: &mut Vec<u8>, manifest: &FoundationManifest) {
    push_u16(out, manifest.schema_version);
    push_u64(out, manifest.foundation_id.raw());
    push_u32(out, manifest.foundation_version.raw());
    push_u64(out, manifest.compatibility_family_id.raw());
    push_u16(out, manifest.capacity_class_id.raw());
    push_u16(out, manifest.sensor_profile.raw());
    push_blake3(out, manifest.layout_digest);
    push_blake3(out, manifest.language_codebook_digest);
    push_digest4(out, manifest.action_decoder_digest);
    push_optional_digest4(out, manifest.speech_decoder_digest);
    push_optional_digest4(out, manifest.memory_decoder_digest);
    push_blake3(out, manifest.route_abi_digest);
    push_blake3(out, manifest.plasticity_abi_digest);
    push_blake3(out, manifest.address_map_digest);
    encode_training_stage(out, manifest.training_stage);
    encode_promotion_receipt(out, manifest.promotion_receipt);
    push_blake3(out, manifest.weight_asset.digest);
    push_u32(out, manifest.weight_asset.weight_count);
}

fn decode_manifest(
    cursor: &mut FoundationAssetCursor<'_>,
) -> Result<FoundationManifest, ScaffoldContractError> {
    let schema_version = cursor.u16()?;
    let foundation_id = FoundationId(cursor.u64()?);
    let foundation_version = FoundationVersion(cursor.u32()?);
    let compatibility_family_id = FoundationCompatibilityFamilyId(cursor.u64()?);
    let capacity_class_id = BrainClassId(cursor.u16()?);
    let sensor_profile = SensorProfile::try_from_raw(cursor.u16()?)?;
    let layout_digest = cursor.blake3()?;
    let language_codebook_digest = cursor.blake3()?;
    let action_decoder_digest = cursor.digest4()?;
    let speech_decoder_digest = cursor.optional_digest4()?;
    let memory_decoder_digest = cursor.optional_digest4()?;
    let route_abi_digest = cursor.blake3()?;
    let plasticity_abi_digest = cursor.blake3()?;
    let address_map_digest = cursor.blake3()?;
    let training_stage = decode_training_stage(cursor)?;
    let promotion_receipt = decode_promotion_receipt(cursor)?;
    let weight_asset = FoundationWeightAssetRef {
        digest: cursor.blake3()?,
        weight_count: cursor.u32()?,
    };
    Ok(FoundationManifest {
        schema_version,
        foundation_id,
        foundation_version,
        compatibility_family_id,
        capacity_class_id,
        sensor_profile,
        layout_digest,
        language_codebook_digest,
        action_decoder_digest,
        speech_decoder_digest,
        memory_decoder_digest,
        route_abi_digest,
        plasticity_abi_digest,
        address_map_digest,
        training_stage,
        promotion_receipt,
        weight_asset,
    })
}

fn encode_training_stage(out: &mut Vec<u8>, training: TrainingStageManifest) {
    push_u16(out, training.schema_version);
    push_u32(out, training.curriculum_version);
    push_u32(out, training.evaluation_version);
    push_u16(out, training.completed_stage_count);
    push_blake3(out, training.manifest_digest);
}

fn decode_training_stage(
    cursor: &mut FoundationAssetCursor<'_>,
) -> Result<TrainingStageManifest, ScaffoldContractError> {
    Ok(TrainingStageManifest {
        schema_version: cursor.u16()?,
        curriculum_version: cursor.u32()?,
        evaluation_version: cursor.u32()?,
        completed_stage_count: cursor.u16()?,
        manifest_digest: cursor.blake3()?,
    })
}

fn encode_promotion_receipt(out: &mut Vec<u8>, receipt: FoundationPromotionReceipt) {
    push_u16(out, receipt.schema_version);
    push_blake3(out, receipt.training_manifest_digest);
    push_optional_blake3(out, receipt.evaluation_evidence_digest);
    push_blake3(out, receipt.provenance_digest);
    push_blake3(out, receipt.receipt_digest);
}

fn decode_promotion_receipt(
    cursor: &mut FoundationAssetCursor<'_>,
) -> Result<FoundationPromotionReceipt, ScaffoldContractError> {
    Ok(FoundationPromotionReceipt {
        schema_version: cursor.u16()?,
        training_manifest_digest: cursor.blake3()?,
        evaluation_evidence_digest: cursor.optional_blake3()?,
        provenance_digest: cursor.blake3()?,
        receipt_digest: cursor.blake3()?,
    })
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_blake3(out: &mut Vec<u8>, value: Blake3Digest) {
    out.extend_from_slice(value.bytes());
}

fn push_digest4(out: &mut Vec<u8>, value: [u64; 4]) {
    for word in value {
        push_u64(out, word);
    }
}

fn push_optional_digest4(out: &mut Vec<u8>, value: Option<[u64; 4]>) {
    match value {
        Some(value) => {
            out.push(1);
            push_digest4(out, value);
        }
        None => out.push(0),
    }
}

fn push_optional_blake3(out: &mut Vec<u8>, value: Option<Blake3Digest>) {
    match value {
        Some(value) => {
            out.push(1);
            push_blake3(out, value);
        }
        None => out.push(0),
    }
}

struct FoundationAssetCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> FoundationAssetCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], ScaffoldContractError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, ScaffoldContractError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ScaffoldContractError> {
        let bytes = self
            .take(2)?
            .try_into()
            .map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, ScaffoldContractError> {
        let bytes = self
            .take(4)?
            .try_into()
            .map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, ScaffoldContractError> {
        let bytes = self
            .take(8)?
            .try_into()
            .map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn blake3(&mut self) -> Result<Blake3Digest, ScaffoldContractError> {
        let bytes = self
            .take(32)?
            .try_into()
            .map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
        Ok(Blake3Digest::from_bytes(bytes))
    }

    fn digest4(&mut self) -> Result<[u64; 4], ScaffoldContractError> {
        Ok([self.u64()?, self.u64()?, self.u64()?, self.u64()?])
    }

    fn optional_digest4(&mut self) -> Result<Option<[u64; 4]>, ScaffoldContractError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.digest4()?)),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }

    fn optional_blake3(&mut self) -> Result<Option<Blake3Digest>, ScaffoldContractError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.blake3()?)),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }
}

pub(crate) fn layout_digest(layout: &LobeLayout) -> Blake3Digest {
    let mut h = domain_hasher(LAYOUT_DOMAIN);
    h.write_len(layout.regions.len());
    for region in &layout.regions {
        h.write_u16(region.kind.raw());
        h.write_u32(region.start);
        h.write_u32(region.len);
        h.write_u8(region.enabled as u8);
    }
    Blake3Digest::from_hasher(h)
}

pub(crate) fn route_digest(routes: &[N2048FoundationRouteSpec]) -> Blake3Digest {
    let mut h = domain_hasher(ROUTE_DOMAIN);
    h.write_len(routes.len());
    for route in routes {
        h.write_u16(route.source_lobe.raw());
        h.write_u16(route.target_lobe.raw());
        h.write_u32(route.synapse_count);
        h.write_u8(route.projection_type.raw());
        h.write_u8(route.active_tile_policy.raw());
        h.write_u8(route.update_cadence.raw());
        h.write_u8(route.priority.raw());
    }
    Blake3Digest::from_hasher(h)
}

pub(crate) fn plasticity_digest(routes: &[N2048FoundationRouteSpec]) -> Blake3Digest {
    let mut h = domain_hasher(PLASTICITY_DOMAIN);
    h.write_len(routes.len());
    for route in routes {
        h.write_u16(route.source_lobe.raw());
        h.write_u16(route.target_lobe.raw());
        for band in [
            LifetimePlasticityBand::Fixed,
            LifetimePlasticityBand::Slow,
            LifetimePlasticityBand::Fast,
        ] {
            h.write_u8(band as u8);
            h.write_u32(route.section_policy.count(band));
        }
    }
    Blake3Digest::from_hasher(h)
}

pub(crate) fn procedural_route_digest(
    routes: impl IntoIterator<Item = (LobeKind, LobeKind, u32)>,
) -> Blake3Digest {
    let rows = routes.into_iter().collect::<Vec<_>>();
    let mut h = domain_hasher(ROUTE_DOMAIN);
    h.write_len(rows.len());
    for (source, target, count) in rows {
        h.write_u16(source.raw());
        h.write_u16(target.raw());
        h.write_u32(count);
    }
    Blake3Digest::from_hasher(h)
}

pub(crate) fn procedural_plasticity_digest(
    routes: impl IntoIterator<Item = (u16, u32, u32)>,
) -> Blake3Digest {
    let rows = routes.into_iter().collect::<Vec<_>>();
    let mut h = domain_hasher(PLASTICITY_DOMAIN);
    h.write_len(rows.len());
    for (route, count, alpha_bits) in rows {
        h.write_u16(route);
        h.write_u32(count);
        h.write_u32(alpha_bits);
    }
    Blake3Digest::from_hasher(h)
}
