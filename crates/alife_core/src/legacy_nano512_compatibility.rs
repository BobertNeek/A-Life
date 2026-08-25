//! Explicit admission contract for the immutable pre-v2 Nano512 foundation.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    foundation::layout_digest, Blake3Digest, BrainCapacityClass, BrainPhenotype,
    CanonicalDigestBuilder, CompiledSynapseKind, FoundationAbiBinding,
    FoundationCompatibilityFamilyId, FoundationId, FoundationVersion, FoundationWeightAsset,
    FoundationWeightAssetRef, LanguageCodebookV1, LobeKind, LobeLayout, LobeRegion, PhenotypeHash,
    ScaffoldContractError, SensorProfile,
};

const DESCRIPTOR_DOMAIN: &[u8] = b"alife.foundation.compatibility.nano512.v1";
const SELECTION_DOMAIN: &[u8] = b"alife.foundation.abi-selection.v1";
const RECEIPT_DOMAIN: &[u8] = b"alife.foundation.compatibility.receipt.nano512.v1";
const ENDPOINT_AUDIT_DOMAIN: &[u8] = b"alife.audit.nano512.endpoints.v1";
const GRAPH_WEIGHT_AUDIT_DOMAIN: &[u8] = b"alife.audit.nano512.graph-and-weights.v1";

pub const LEGACY_NANO512_V1_COORDINATE_SEED: u64 = 0x4E35_3132_5F00_0001;

const EXPECTED_ENDPOINT_DIGEST: Blake3Digest = Blake3Digest::from_bytes([
    0xd5, 0x65, 0xd5, 0xb4, 0xb4, 0x1f, 0x6f, 0xbc, 0x90, 0x6f, 0x58, 0xdd, 0xab, 0x6e, 0xd6, 0xe3,
    0xe6, 0x3d, 0x48, 0xdb, 0x9f, 0x26, 0xb8, 0x2c, 0x3e, 0xae, 0xa8, 0xca, 0xdf, 0xef, 0xc0, 0xe9,
]);
const EXPECTED_GRAPH_WEIGHT_DIGEST: Blake3Digest = Blake3Digest::from_bytes([
    0x25, 0xde, 0x49, 0xf6, 0xa7, 0xc8, 0xe5, 0xa1, 0xac, 0x02, 0x21, 0xe2, 0x26, 0x69, 0xb0, 0xb5,
    0x2f, 0x5a, 0xf5, 0x67, 0xf6, 0xf1, 0x86, 0x78, 0x38, 0x3a, 0x1e, 0x10, 0xfc, 0x63, 0x54, 0x7c,
]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LegacyFoundationAbiId(u64);

impl LegacyFoundationAbiId {
    pub const NANO512_V1: Self = Self(0x4C4E_3531_325F_5631);

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProductionRuntimeAbiId(u32);

impl ProductionRuntimeAbiId {
    pub const V2: Self = Self(2);

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProductionRuntimePath(u32);

impl ProductionRuntimePath {
    pub const ORDINARY_GPU_ORGANISM_V2: Self = Self(1);

    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Wire-visible, mutually exclusive foundation ABI dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "abi_selection", content = "contract")]
pub enum FoundationAbiSelection {
    CanonicalV2(FoundationAbiBinding),
    LegacyNano512CompatibilityV1(LegacyNano512CompatibilityAbiDescriptor),
}

impl FoundationAbiSelection {
    pub(crate) fn legacy_nano512(descriptor: LegacyNano512CompatibilityAbiDescriptor) -> Self {
        Self::LegacyNano512CompatibilityV1(descriptor)
    }

    pub const fn canonical_v2(&self) -> Option<&FoundationAbiBinding> {
        match self {
            Self::CanonicalV2(binding) => Some(binding),
            Self::LegacyNano512CompatibilityV1(_) => None,
        }
    }

    pub const fn legacy_nano512_compatibility_v1(
        &self,
    ) -> Option<&LegacyNano512CompatibilityAbiDescriptor> {
        match self {
            Self::CanonicalV2(_) => None,
            Self::LegacyNano512CompatibilityV1(descriptor) => Some(descriptor),
        }
    }

    pub(crate) fn validate_against(
        &self,
        capacity: &BrainCapacityClass,
        sensor_profile: SensorProfile,
    ) -> Result<(), ScaffoldContractError> {
        match self {
            Self::CanonicalV2(binding) => binding.validate_against(capacity),
            Self::LegacyNano512CompatibilityV1(descriptor) => {
                descriptor.validate_contract()?;
                if capacity.id() != BrainCapacityClass::N512_ID
                    || descriptor.sensor_profile() != sensor_profile
                {
                    return Err(ScaffoldContractError::PhenotypeCompile);
                }
                Ok(())
            }
        }
    }

    pub(crate) fn language_codebook(&self) -> LanguageCodebookV1 {
        match self {
            Self::CanonicalV2(binding) => binding.language_codebook().clone(),
            Self::LegacyNano512CompatibilityV1(_) => LanguageCodebookV1::canonical(),
        }
    }

    pub const fn capacity_class_id(&self) -> crate::BrainClassId {
        match self {
            Self::CanonicalV2(binding) => binding.capacity_class_id(),
            Self::LegacyNano512CompatibilityV1(_) => BrainCapacityClass::N512_ID,
        }
    }

    pub const fn foundation_id(&self) -> Option<FoundationId> {
        match self {
            Self::CanonicalV2(binding) => binding.foundation_id(),
            Self::LegacyNano512CompatibilityV1(descriptor) => Some(descriptor.source_foundation_id),
        }
    }

    pub const fn foundation_version(&self) -> Option<FoundationVersion> {
        match self {
            Self::CanonicalV2(binding) => binding.foundation_version(),
            Self::LegacyNano512CompatibilityV1(descriptor) => {
                Some(descriptor.source_foundation_version)
            }
        }
    }

    pub const fn compatibility_family_id(&self) -> Option<FoundationCompatibilityFamilyId> {
        match self {
            Self::CanonicalV2(binding) => binding.compatibility_family_id(),
            Self::LegacyNano512CompatibilityV1(descriptor) => {
                Some(descriptor.source_compatibility_family_id)
            }
        }
    }

    pub const fn foundation_weight_asset(&self) -> Option<FoundationWeightAssetRef> {
        match self {
            Self::CanonicalV2(binding) => binding.foundation_weight_asset(),
            Self::LegacyNano512CompatibilityV1(descriptor) => Some(descriptor.source_weight_asset),
        }
    }

    pub const fn foundation_payload_digest(&self) -> Option<Blake3Digest> {
        match self.foundation_weight_asset() {
            Some(asset) => Some(asset.digest()),
            None => None,
        }
    }

    pub fn selector_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(SELECTION_DOMAIN);
        self.write_canonical(&mut digest);
        digest.finish256()
    }

    pub(crate) fn write_canonical(&self, digest: &mut CanonicalDigestBuilder) {
        match self {
            Self::CanonicalV2(binding) => {
                digest.write_u8(1);
                digest.write_u16(binding.capacity_class_id().raw());
                digest.write_u64(binding.layout_id().0);
                write_blake3(digest, binding.layout_digest());
                if let (Some(id), Some(version), Some(family), Some(asset)) = (
                    binding.foundation_id(),
                    binding.foundation_version(),
                    binding.compatibility_family_id(),
                    binding.foundation_weight_asset(),
                ) {
                    digest.write_some();
                    digest.write_u64(id.raw());
                    digest.write_u32(version.raw());
                    digest.write_u64(family.raw());
                    write_blake3(digest, asset.digest());
                    digest.write_u32(asset.weight_count());
                } else {
                    digest.write_none();
                }
            }
            Self::LegacyNano512CompatibilityV1(descriptor) => {
                digest.write_u8(2);
                for word in descriptor.canonical_digest() {
                    digest.write_u64(word);
                }
            }
        }
    }
}

/// Separately typed source ABI selected only from exact asset metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyNano512CompatibilityAbiDescriptor {
    schema_version: u16,
    source_abi_id: LegacyFoundationAbiId,
    source_foundation_id: FoundationId,
    source_foundation_version: FoundationVersion,
    source_compatibility_family_id: FoundationCompatibilityFamilyId,
    source_sensor_profile: SensorProfile,
    source_weight_asset: FoundationWeightAssetRef,
    source_layout_digest: Blake3Digest,
    source_route_abi_digest: Blake3Digest,
    source_address_map_digest: Blake3Digest,
    runtime_abi_id: ProductionRuntimeAbiId,
    runtime_path: ProductionRuntimePath,
    runtime_layout_digest: Blake3Digest,
    descriptor_digest: [u64; 4],
}

impl LegacyNano512CompatibilityAbiDescriptor {
    pub(crate) fn for_asset(
        capacity: &BrainCapacityClass,
        sensor_profile: SensorProfile,
        asset: &FoundationWeightAsset,
    ) -> Result<Self, ScaffoldContractError> {
        capacity.validate_contract()?;
        asset.validate_self_contained()?;
        let expected = FoundationWeightAsset::builtin_nano512_v1(sensor_profile)?;
        if capacity.id() != BrainCapacityClass::N512_ID || asset != &expected {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let manifest = asset.manifest();
        let mut value = Self {
            schema_version: 1,
            source_abi_id: LegacyFoundationAbiId::NANO512_V1,
            source_foundation_id: manifest.foundation_id(),
            source_foundation_version: manifest.foundation_version(),
            source_compatibility_family_id: manifest.compatibility_family_id(),
            source_sensor_profile: sensor_profile,
            source_weight_asset: asset.asset_ref(),
            source_layout_digest: manifest.layout_digest(),
            source_route_abi_digest: manifest.route_abi_digest(),
            source_address_map_digest: manifest.address_map_digest(),
            runtime_abi_id: ProductionRuntimeAbiId::V2,
            runtime_path: ProductionRuntimePath::ORDINARY_GPU_ORGANISM_V2,
            runtime_layout_digest: layout_digest(&legacy_nano512_runtime_layout()?),
            descriptor_digest: [0; 4],
        };
        value.descriptor_digest = value.recompute_digest();
        value.validate_contract()?;
        Ok(value)
    }

    pub const fn source_abi_id(&self) -> LegacyFoundationAbiId {
        self.source_abi_id
    }
    pub const fn source_weight_asset(&self) -> FoundationWeightAssetRef {
        self.source_weight_asset
    }
    pub const fn runtime_abi_id(&self) -> ProductionRuntimeAbiId {
        self.runtime_abi_id
    }
    pub const fn runtime_path(&self) -> ProductionRuntimePath {
        self.runtime_path
    }
    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.descriptor_digest
    }
    pub const fn sensor_profile(&self) -> SensorProfile {
        self.source_sensor_profile
    }

    pub(crate) fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        let expected_asset = FoundationWeightAsset::builtin_nano512_v1(self.source_sensor_profile)?;
        let manifest = expected_asset.manifest();
        if self.schema_version != 1
            || self.source_abi_id != LegacyFoundationAbiId::NANO512_V1
            || self.source_foundation_id != FoundationId::N512_V1
            || self.source_foundation_version != FoundationVersion::V1
            || self.source_compatibility_family_id
                != FoundationCompatibilityFamilyId::N512_FOUNDATION
            || self.source_foundation_id != manifest.foundation_id()
            || self.source_foundation_version != manifest.foundation_version()
            || self.source_compatibility_family_id != manifest.compatibility_family_id()
            || self.source_weight_asset != expected_asset.asset_ref()
            || self.source_layout_digest != manifest.layout_digest()
            || self.source_route_abi_digest != manifest.route_abi_digest()
            || self.source_address_map_digest != manifest.address_map_digest()
            || self.runtime_abi_id != ProductionRuntimeAbiId::V2
            || self.runtime_path != ProductionRuntimePath::ORDINARY_GPU_ORGANISM_V2
            || self.runtime_layout_digest != layout_digest(&legacy_nano512_runtime_layout()?)
            || self.descriptor_digest != self.recompute_digest()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    pub(crate) fn validate_for_phenotype(
        &self,
        phenotype: &BrainPhenotype,
    ) -> Result<(), ScaffoldContractError> {
        self.validate_contract()?;
        let expected_asset = FoundationWeightAsset::builtin_nano512_v1(self.source_sensor_profile)?;
        if phenotype.brain_class_id() != BrainCapacityClass::N512_ID
            || phenotype.sensor_profile() != self.source_sensor_profile
            || phenotype.lobe_layout() != &legacy_nano512_runtime_layout()?
            || phenotype.synapses().len() != self.source_weight_asset.weight_count() as usize
            || phenotype.foundation_abi().canonical_v2().is_some()
            || phenotype
                .synapses()
                .iter()
                .zip(expected_asset.weights())
                .any(|(synapse, weight)| synapse.genetic_weight().to_bits() != weight.to_bits())
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let (endpoint_digest, graph_weight_digest) = compatibility_graph_digests(phenotype)?;
        if endpoint_digest != EXPECTED_ENDPOINT_DIGEST
            || graph_weight_digest != EXPECTED_GRAPH_WEIGHT_DIGEST
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(DESCRIPTOR_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u64(self.source_abi_id.raw());
        digest.write_u64(self.source_foundation_id.raw());
        digest.write_u32(self.source_foundation_version.raw());
        digest.write_u64(self.source_compatibility_family_id.raw());
        digest.write_u16(self.source_sensor_profile.raw());
        write_blake3(&mut digest, self.source_weight_asset.digest());
        digest.write_u32(self.source_weight_asset.weight_count());
        write_blake3(&mut digest, self.source_layout_digest);
        write_blake3(&mut digest, self.source_route_abi_digest);
        write_blake3(&mut digest, self.source_address_map_digest);
        digest.write_u32(self.runtime_abi_id.raw());
        digest.write_u32(self.runtime_path.raw());
        write_blake3(&mut digest, self.runtime_layout_digest);
        digest.finish256()
    }
}

impl<'de> Deserialize<'de> for LegacyNano512CompatibilityAbiDescriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            source_abi_id: LegacyFoundationAbiId,
            source_foundation_id: FoundationId,
            source_foundation_version: FoundationVersion,
            source_compatibility_family_id: FoundationCompatibilityFamilyId,
            source_sensor_profile: SensorProfile,
            source_weight_asset: FoundationWeightAssetRef,
            source_layout_digest: Blake3Digest,
            source_route_abi_digest: Blake3Digest,
            source_address_map_digest: Blake3Digest,
            runtime_abi_id: ProductionRuntimeAbiId,
            runtime_path: ProductionRuntimePath,
            runtime_layout_digest: Blake3Digest,
            descriptor_digest: [u64; 4],
        }
        let wire = Wire::deserialize(deserializer)?;
        let value = Self {
            schema_version: wire.schema_version,
            source_abi_id: wire.source_abi_id,
            source_foundation_id: wire.source_foundation_id,
            source_foundation_version: wire.source_foundation_version,
            source_compatibility_family_id: wire.source_compatibility_family_id,
            source_sensor_profile: wire.source_sensor_profile,
            source_weight_asset: wire.source_weight_asset,
            source_layout_digest: wire.source_layout_digest,
            source_route_abi_digest: wire.source_route_abi_digest,
            source_address_map_digest: wire.source_address_map_digest,
            runtime_abi_id: wire.runtime_abi_id,
            runtime_path: wire.runtime_path,
            runtime_layout_digest: wire.runtime_layout_digest,
            descriptor_digest: wire.descriptor_digest,
        };
        value.validate_contract().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyNano512CompatibilityReceipt {
    schema_version: u16,
    descriptor: LegacyNano512CompatibilityAbiDescriptor,
    runtime_phenotype_hash: PhenotypeHash,
    runtime_route_abi_digest: Blake3Digest,
    runtime_address_map_digest: Blake3Digest,
    endpoint_digest: Blake3Digest,
    graph_weight_digest: Blake3Digest,
    receipt_digest: [u64; 4],
}

impl LegacyNano512CompatibilityReceipt {
    pub(crate) fn new(
        descriptor: &LegacyNano512CompatibilityAbiDescriptor,
        phenotype: &BrainPhenotype,
    ) -> Result<Self, ScaffoldContractError> {
        descriptor.validate_for_phenotype(phenotype)?;
        let (endpoint_digest, graph_weight_digest) = compatibility_graph_digests(phenotype)?;
        let mut value = Self {
            schema_version: 1,
            descriptor: descriptor.clone(),
            runtime_phenotype_hash: phenotype.phenotype_hash(),
            runtime_route_abi_digest: phenotype.route_abi_digest(),
            runtime_address_map_digest: phenotype.persistent_address_map().digest(),
            endpoint_digest,
            graph_weight_digest,
            receipt_digest: [0; 4],
        };
        value.receipt_digest = value.recompute_digest();
        value.validate_contract()?;
        Ok(value)
    }

    pub const fn source_abi_id(&self) -> LegacyFoundationAbiId {
        self.descriptor.source_abi_id()
    }
    pub const fn runtime_abi_id(&self) -> ProductionRuntimeAbiId {
        self.descriptor.runtime_abi_id()
    }
    pub const fn runtime_path(&self) -> ProductionRuntimePath {
        self.descriptor.runtime_path()
    }
    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.receipt_digest
    }

    pub fn validate_against(
        &self,
        phenotype: &BrainPhenotype,
        asset: &FoundationWeightAsset,
    ) -> Result<(), ScaffoldContractError> {
        asset.validate_self_contained()?;
        let expected_asset =
            FoundationWeightAsset::builtin_nano512_v1(self.descriptor.sensor_profile())?;
        let descriptor = phenotype
            .legacy_foundation_compatibility_abi()
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        descriptor.validate_for_phenotype(phenotype)?;
        let expected = Self::new(descriptor, phenotype)?;
        if asset != &expected_asset
            || asset.asset_ref() != self.descriptor.source_weight_asset()
            || self != &expected
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.descriptor.validate_contract()?;
        if self.schema_version != 1
            || self.runtime_phenotype_hash == PhenotypeHash::default()
            || self.runtime_route_abi_digest == Blake3Digest::default()
            || self.runtime_address_map_digest == Blake3Digest::default()
            || self.endpoint_digest != EXPECTED_ENDPOINT_DIGEST
            || self.graph_weight_digest != EXPECTED_GRAPH_WEIGHT_DIGEST
            || self.receipt_digest != self.recompute_digest()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(RECEIPT_DOMAIN);
        digest.write_u16(self.schema_version);
        for word in self.descriptor.canonical_digest() {
            digest.write_u64(word);
        }
        for word in self.runtime_phenotype_hash.0 {
            digest.write_u64(word);
        }
        write_blake3(&mut digest, self.runtime_route_abi_digest);
        write_blake3(&mut digest, self.runtime_address_map_digest);
        write_blake3(&mut digest, self.endpoint_digest);
        write_blake3(&mut digest, self.graph_weight_digest);
        digest.finish256()
    }
}

impl<'de> Deserialize<'de> for LegacyNano512CompatibilityReceipt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            descriptor: LegacyNano512CompatibilityAbiDescriptor,
            runtime_phenotype_hash: PhenotypeHash,
            runtime_route_abi_digest: Blake3Digest,
            runtime_address_map_digest: Blake3Digest,
            endpoint_digest: Blake3Digest,
            graph_weight_digest: Blake3Digest,
            receipt_digest: [u64; 4],
        }
        let wire = Wire::deserialize(deserializer)?;
        let value = Self {
            schema_version: wire.schema_version,
            descriptor: wire.descriptor,
            runtime_phenotype_hash: wire.runtime_phenotype_hash,
            runtime_route_abi_digest: wire.runtime_route_abi_digest,
            runtime_address_map_digest: wire.runtime_address_map_digest,
            endpoint_digest: wire.endpoint_digest,
            graph_weight_digest: wire.graph_weight_digest,
            receipt_digest: wire.receipt_digest,
        };
        value.validate_contract().map_err(D::Error::custom)?;
        Ok(value)
    }
}

pub struct LegacyNano512CompatibilityAdmission {
    phenotype: BrainPhenotype,
    compiler_inputs: crate::PhenotypeCompilerInputs,
    receipt: LegacyNano512CompatibilityReceipt,
}

impl LegacyNano512CompatibilityAdmission {
    pub(crate) fn new(
        phenotype: BrainPhenotype,
        compiler_inputs: crate::PhenotypeCompilerInputs,
        receipt: LegacyNano512CompatibilityReceipt,
    ) -> Self {
        Self {
            phenotype,
            compiler_inputs,
            receipt,
        }
    }

    pub fn into_parts(self) -> (BrainPhenotype, LegacyNano512CompatibilityReceipt) {
        (self.phenotype, self.receipt)
    }

    pub fn into_runtime_parts(
        self,
    ) -> (
        BrainPhenotype,
        crate::PhenotypeCompilerInputs,
        LegacyNano512CompatibilityReceipt,
    ) {
        (self.phenotype, self.compiler_inputs, self.receipt)
    }
}

pub(crate) fn legacy_nano512_runtime_layout() -> Result<LobeLayout, ScaffoldContractError> {
    let layout = LobeLayout {
        regions: vec![
            LobeRegion::enabled(LobeKind::PerceptualIntegration, 0, 64),
            LobeRegion::enabled(LobeKind::InteroceptiveMotivational, 64, 32),
            LobeRegion::enabled(LobeKind::SocialCommunication, 96, 64),
            LobeRegion::enabled(LobeKind::MultimodalAssociation, 160, 64),
            LobeRegion::enabled(LobeKind::TemporalPredictive, 224, 112),
            LobeRegion::enabled(LobeKind::MemoryInterface, 336, 64),
            LobeRegion::enabled(LobeKind::WorkingContextExecutive, 400, 32),
            LobeRegion::enabled(LobeKind::ActionPlanning, 432, 64),
            LobeRegion::enabled(LobeKind::FlexibleReserve, 496, 16),
        ],
    };
    layout.validate_for_neuron_count(512)?;
    Ok(layout)
}

fn compatibility_graph_digests(
    phenotype: &BrainPhenotype,
) -> Result<(Blake3Digest, Blake3Digest), ScaffoldContractError> {
    let mut endpoint = blake3::Hasher::new();
    endpoint.update(ENDPOINT_AUDIT_DOMAIN);
    let mut graph_weight = blake3::Hasher::new();
    graph_weight.update(GRAPH_WEIGHT_AUDIT_DOMAIN);
    for (index, synapse) in phenotype.synapses().iter().enumerate() {
        let index = u32::try_from(index).map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
        for digest in [&mut endpoint, &mut graph_weight] {
            digest.update(&index.to_le_bytes());
            digest.update(&synapse.source().to_le_bytes());
            digest.update(&synapse.target().to_le_bytes());
            digest.update(&synapse.route_index().to_le_bytes());
            digest.update(&synapse.kind().kind_raw().to_le_bytes());
            if let CompiledSynapseKind::Decoder(coordinate) = synapse.kind() {
                digest.update(&coordinate.head().raw().to_le_bytes());
                digest.update(&[coordinate.family().raw()]);
                digest.update(&coordinate.input_lane().to_le_bytes());
                digest.update(&coordinate.motor_index().to_le_bytes());
            }
        }
        graph_weight.update(&synapse.genetic_weight().to_bits().to_le_bytes());
    }
    Ok((
        Blake3Digest::from_hasher(endpoint),
        Blake3Digest::from_hasher(graph_weight),
    ))
}

fn write_blake3(digest: &mut CanonicalDigestBuilder, value: Blake3Digest) {
    for byte in value.bytes() {
        digest.write_u8(*byte);
    }
}
