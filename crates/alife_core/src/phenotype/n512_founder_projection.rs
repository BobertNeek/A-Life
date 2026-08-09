//! Explicit Nano512 V1 founder projection over a frozen coordinate contract.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    AlleleSide, Blake3Digest, BrainCapacityClass, BrainClassId, BrainGenome, BrainPhenotype,
    CanonicalDigestBuilder, ChromosomeKind, CreaturePhenotype, DevelopmentState,
    FoundationAbiBinding, FoundationCompatibilityFamilyId, FoundationGeneticIdentity, FoundationId,
    FoundationVersion, FoundationWeightAsset, GeneticLineageProvenance, GenomeId, LineageId,
    MutationRecord, PhenotypeCompiler, PhenotypeCompilerInputs, PhenotypeHash,
    ScaffoldContractError, SensorProfile, Tick, Validate,
};

const N512_PROJECTION_SCHEMA_VERSION: u16 = 1;
const N512_COORDINATE_RECIPE_SEED: u64 = 0x4E35_3132_5F00_0001;
const OVERLAY_DOMAIN: &[u8] = b"alife.phenotype.n512-founder-overlay.v1";
const RUNTIME_DEVELOPMENT_DOMAIN: &[u8] = b"alife.phenotype.n512-runtime-development.v1";
const PROVENANCE_DOMAIN: &[u8] = b"alife.phenotype.n512-genetic-provenance.v1";
const RECEIPT_DOMAIN: &[u8] = b"alife.phenotype.n512-founder-receipt.v1";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct N512FrozenAbiRecipe {
    schema_version: u16,
    coordinate_genome: BrainGenome,
    coordinate_development_state: DevelopmentState,
    foundation_abi: FoundationAbiBinding,
    layout_digest: Blake3Digest,
    address_map_digest: Blake3Digest,
    decoder_digest: [u64; 4],
    route_abi_digest: Blake3Digest,
    plasticity_abi_digest: Blake3Digest,
}

impl N512FrozenAbiRecipe {
    fn from_compiled(
        coordinate_genome: BrainGenome,
        coordinate_development_state: DevelopmentState,
        compiled: &BrainPhenotype,
    ) -> Self {
        Self {
            schema_version: N512_PROJECTION_SCHEMA_VERSION,
            coordinate_genome,
            coordinate_development_state,
            foundation_abi: compiled.foundation_abi().clone(),
            layout_digest: compiled.foundation_abi().layout_digest(),
            address_map_digest: compiled.persistent_address_map().digest(),
            decoder_digest: compiled.candidate_decoder().canonical_digest(),
            route_abi_digest: compiled.route_abi_digest(),
            plasticity_abi_digest: compiled.plasticity_abi_digest(),
        }
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn foundation_abi(&self) -> &FoundationAbiBinding {
        &self.foundation_abi
    }

    pub const fn layout_digest(&self) -> Blake3Digest {
        self.layout_digest
    }

    pub const fn address_map_digest(&self) -> Blake3Digest {
        self.address_map_digest
    }

    pub const fn decoder_digest(&self) -> [u64; 4] {
        self.decoder_digest
    }

    pub const fn route_abi_digest(&self) -> Blake3Digest {
        self.route_abi_digest
    }

    pub const fn plasticity_abi_digest(&self) -> Blake3Digest {
        self.plasticity_abi_digest
    }

    pub const fn coordinate_genome(&self) -> &BrainGenome {
        &self.coordinate_genome
    }

    pub const fn coordinate_development_state(&self) -> &DevelopmentState {
        &self.coordinate_development_state
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct N512FounderProjectionReceipt {
    schema_version: u16,
    source_genome_id: GenomeId,
    lineage_id: LineageId,
    source_inputs_digest: [u64; 4],
    foundation_id: FoundationId,
    foundation_version: FoundationVersion,
    compatibility_family_id: FoundationCompatibilityFamilyId,
    capacity_class_id: BrainClassId,
    sensor_profile: SensorProfile,
    foundation_asset_digest: Blake3Digest,
    coordinate_layout_digest: Blake3Digest,
    coordinate_address_map_digest: Blake3Digest,
    coordinate_decoder_digest: [u64; 4],
    coordinate_route_abi_digest: Blake3Digest,
    coordinate_plasticity_abi_digest: Blake3Digest,
    runtime_development_digest: [u64; 4],
    genetic_provenance_digest: [u64; 4],
    overlay_seed: u64,
    phenotype_hash: PhenotypeHash,
    digest: [u64; 4],
}

impl N512FounderProjectionReceipt {
    fn new(
        source_genome_id: GenomeId,
        lineage_id: LineageId,
        source_inputs_digest: [u64; 4],
        foundation: FoundationGeneticIdentity,
        sensor_profile: SensorProfile,
        foundation_asset_digest: Blake3Digest,
        frozen_abi: &N512FrozenAbiRecipe,
        runtime_development_digest: [u64; 4],
        genetic_provenance_digest: [u64; 4],
        overlay_seed: u64,
        phenotype_hash: PhenotypeHash,
    ) -> Result<Self, ScaffoldContractError> {
        if foundation != expected_foundation() {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let mut value = Self {
            schema_version: N512_PROJECTION_SCHEMA_VERSION,
            source_genome_id,
            lineage_id,
            source_inputs_digest,
            foundation_id: FoundationId::N512_V1,
            foundation_version: FoundationVersion::V1,
            compatibility_family_id: FoundationCompatibilityFamilyId::N512_FOUNDATION,
            capacity_class_id: foundation.brain_class_id,
            sensor_profile,
            foundation_asset_digest,
            coordinate_layout_digest: frozen_abi.layout_digest,
            coordinate_address_map_digest: frozen_abi.address_map_digest,
            coordinate_decoder_digest: frozen_abi.decoder_digest,
            coordinate_route_abi_digest: frozen_abi.route_abi_digest,
            coordinate_plasticity_abi_digest: frozen_abi.plasticity_abi_digest,
            runtime_development_digest,
            genetic_provenance_digest,
            overlay_seed,
            phenotype_hash,
            digest: [0; 4],
        };
        value.digest = value.recompute_digest();
        value.validate()?;
        Ok(value)
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn source_genome_id(&self) -> GenomeId {
        self.source_genome_id
    }

    pub const fn lineage_id(&self) -> LineageId {
        self.lineage_id
    }

    pub const fn source_inputs_digest(&self) -> [u64; 4] {
        self.source_inputs_digest
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

    pub const fn capacity_class_id(&self) -> BrainClassId {
        self.capacity_class_id
    }

    pub const fn sensor_profile(&self) -> SensorProfile {
        self.sensor_profile
    }

    pub const fn foundation_asset_digest(&self) -> Blake3Digest {
        self.foundation_asset_digest
    }

    pub const fn overlay_seed(&self) -> u64 {
        self.overlay_seed
    }

    pub const fn runtime_development_digest(&self) -> [u64; 4] {
        self.runtime_development_digest
    }

    pub const fn genetic_provenance_digest(&self) -> [u64; 4] {
        self.genetic_provenance_digest
    }

    pub const fn phenotype_hash(&self) -> PhenotypeHash {
        self.phenotype_hash
    }

    pub const fn digest(&self) -> [u64; 4] {
        self.digest
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        let foundation = FoundationWeightAsset::builtin_nano512_v1(self.sensor_profile)
            .map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
        let frozen_abi = canonical_frozen_abi(self.sensor_profile)?;
        if self.schema_version != N512_PROJECTION_SCHEMA_VERSION
            || self.source_genome_id.0 == 0
            || self.lineage_id.0 == 0
            || self.foundation_id != FoundationId::N512_V1
            || self.foundation_version != FoundationVersion::V1
            || self.compatibility_family_id != FoundationCompatibilityFamilyId::N512_FOUNDATION
            || self.capacity_class_id != BrainCapacityClass::N512_ID
            || self.overlay_seed == 0
            || self.foundation_asset_digest.bytes() == &[0; 32]
            || self.foundation_asset_digest != foundation.digest()
            || self.coordinate_layout_digest != frozen_abi.layout_digest
            || self.coordinate_address_map_digest != frozen_abi.address_map_digest
            || self.coordinate_decoder_digest != frozen_abi.decoder_digest
            || self.coordinate_route_abi_digest != frozen_abi.route_abi_digest
            || self.coordinate_plasticity_abi_digest != frozen_abi.plasticity_abi_digest
            || self.digest != self.recompute_digest()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    pub fn validate_against_projection(
        &self,
        projection: &N512FounderFoundationProjection,
    ) -> Result<(), ScaffoldContractError> {
        self.validate()?;
        projection.validate()?;
        if self != &projection.receipt {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(RECEIPT_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u64(self.source_genome_id.0);
        digest.write_u64(self.lineage_id.0);
        write_digest4(&mut digest, self.source_inputs_digest);
        digest.write_u64(self.foundation_id.raw());
        digest.write_u32(self.foundation_version.raw());
        digest.write_u64(self.compatibility_family_id.raw());
        digest.write_u16(self.capacity_class_id.raw());
        digest.write_u16(self.sensor_profile.raw());
        write_blake3(&mut digest, self.foundation_asset_digest);
        write_blake3(&mut digest, self.coordinate_layout_digest);
        write_blake3(&mut digest, self.coordinate_address_map_digest);
        write_digest4(&mut digest, self.coordinate_decoder_digest);
        write_blake3(&mut digest, self.coordinate_route_abi_digest);
        write_blake3(&mut digest, self.coordinate_plasticity_abi_digest);
        write_digest4(&mut digest, self.runtime_development_digest);
        write_digest4(&mut digest, self.genetic_provenance_digest);
        digest.write_u64(self.overlay_seed);
        write_digest4(&mut digest, self.phenotype_hash.0);
        digest.finish256()
    }
}

impl<'de> Deserialize<'de> for N512FounderProjectionReceipt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            source_genome_id: GenomeId,
            lineage_id: LineageId,
            source_inputs_digest: [u64; 4],
            foundation_id: FoundationId,
            foundation_version: FoundationVersion,
            compatibility_family_id: FoundationCompatibilityFamilyId,
            capacity_class_id: BrainClassId,
            sensor_profile: SensorProfile,
            foundation_asset_digest: Blake3Digest,
            coordinate_layout_digest: Blake3Digest,
            coordinate_address_map_digest: Blake3Digest,
            coordinate_decoder_digest: [u64; 4],
            coordinate_route_abi_digest: Blake3Digest,
            coordinate_plasticity_abi_digest: Blake3Digest,
            runtime_development_digest: [u64; 4],
            genetic_provenance_digest: [u64; 4],
            overlay_seed: u64,
            phenotype_hash: PhenotypeHash,
            digest: [u64; 4],
        }
        let wire = Wire::deserialize(deserializer)?;
        let value = Self {
            schema_version: wire.schema_version,
            source_genome_id: wire.source_genome_id,
            lineage_id: wire.lineage_id,
            source_inputs_digest: wire.source_inputs_digest,
            foundation_id: wire.foundation_id,
            foundation_version: wire.foundation_version,
            compatibility_family_id: wire.compatibility_family_id,
            capacity_class_id: wire.capacity_class_id,
            sensor_profile: wire.sensor_profile,
            foundation_asset_digest: wire.foundation_asset_digest,
            coordinate_layout_digest: wire.coordinate_layout_digest,
            coordinate_address_map_digest: wire.coordinate_address_map_digest,
            coordinate_decoder_digest: wire.coordinate_decoder_digest,
            coordinate_route_abi_digest: wire.coordinate_route_abi_digest,
            coordinate_plasticity_abi_digest: wire.coordinate_plasticity_abi_digest,
            runtime_development_digest: wire.runtime_development_digest,
            genetic_provenance_digest: wire.genetic_provenance_digest,
            overlay_seed: wire.overlay_seed,
            phenotype_hash: wire.phenotype_hash,
            digest: wire.digest,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct N512FounderFoundationProjection {
    source_brain_genome: BrainGenome,
    source_genome_id: GenomeId,
    lineage_id: LineageId,
    foundation: FoundationGeneticIdentity,
    genetic_provenance: GeneticLineageProvenance,
    runtime_development_state: DevelopmentState,
    frozen_abi: N512FrozenAbiRecipe,
    overlay_seed: u64,
    receipt: N512FounderProjectionReceipt,
    compiled_phenotype: BrainPhenotype,
}

impl N512FounderFoundationProjection {
    pub fn compile(
        phenotype: &CreaturePhenotype,
        sensor_profile: SensorProfile,
        foundation: &FoundationWeightAsset,
    ) -> Result<Self, ScaffoldContractError> {
        let capacity = BrainCapacityClass::n512();
        validate_source(phenotype, &capacity)?;
        let foundation = canonical_builtin_foundation(sensor_profile, foundation)?;
        let foundation_abi =
            FoundationAbiBinding::canonical_for_foundation_asset(&capacity, &foundation)?;
        let runtime_development_state = phenotype.development_state_at(Tick::ZERO)?;
        let material = projection_material(
            &phenotype.brain_genome,
            phenotype.source_genome_id,
            phenotype.lineage_id,
            phenotype.foundation,
            &phenotype.genetic_provenance,
            &runtime_development_state,
            &capacity,
            sensor_profile,
            &foundation_abi,
        )?;
        let coordinate_genome = BrainGenome::scaffold(N512_COORDINATE_RECIPE_SEED, capacity.id());
        let coordinate_development_state = DevelopmentState::new(
            coordinate_genome.id,
            Tick::ZERO,
            crate::NormalizedScalar::new(1.0)?,
        );
        let compiled_phenotype =
            PhenotypeCompiler::compile_from_foundation_asset_with_overlay_seed(
                &coordinate_genome,
                &capacity,
                &coordinate_development_state,
                sensor_profile,
                &foundation,
                material.overlay_seed,
            )?;
        foundation.validate_against(&compiled_phenotype)?;
        let frozen_abi = N512FrozenAbiRecipe::from_compiled(
            coordinate_genome,
            coordinate_development_state,
            &compiled_phenotype,
        );
        let receipt = N512FounderProjectionReceipt::new(
            phenotype.source_genome_id,
            phenotype.lineage_id,
            material.source_inputs_digest,
            phenotype.foundation,
            sensor_profile,
            foundation.digest(),
            &frozen_abi,
            material.runtime_development_digest,
            material.genetic_provenance_digest,
            material.overlay_seed,
            compiled_phenotype.phenotype_hash(),
        )?;
        let projection = Self {
            source_brain_genome: phenotype.brain_genome.clone(),
            source_genome_id: phenotype.source_genome_id,
            lineage_id: phenotype.lineage_id,
            foundation: phenotype.foundation,
            genetic_provenance: phenotype.genetic_provenance.clone(),
            runtime_development_state,
            frozen_abi,
            overlay_seed: material.overlay_seed,
            receipt,
            compiled_phenotype,
        };
        projection.validate()?;
        Ok(projection)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        let capacity = BrainCapacityClass::n512();
        self.source_brain_genome.validate_contract()?;
        if self.source_genome_id != self.source_brain_genome.id
            || self.source_brain_genome.brain_class_id != capacity.id()
            || self.source_brain_genome.lineage_id != Some(self.lineage_id)
            || self.foundation != expected_foundation()
            || self.runtime_development_state.genome_id != self.source_brain_genome.id
            || self.compiled_phenotype.brain_class_id() != capacity.id()
            || self.compiled_phenotype.sensor_profile() != self.receipt.sensor_profile()
            || self.overlay_seed != self.receipt.overlay_seed()
            || self.compiled_phenotype.phenotype_hash() != self.receipt.phenotype_hash()
            || self.compiled_phenotype.foundation_abi() != self.frozen_abi.foundation_abi()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        self.foundation.validate_contract()?;
        self.runtime_development_state.validate_contract()?;
        self.frozen_abi
            .validate_against(&capacity, &self.compiled_phenotype)?;
        self.receipt.validate()?;
        if self.receipt.source_genome_id() != self.source_genome_id
            || self.receipt.lineage_id() != self.lineage_id
            || self.receipt.capacity_class_id() != capacity.id()
            || self.receipt.foundation_id() != FoundationId::N512_V1
            || self.receipt.foundation_version() != FoundationVersion::V1
            || self.receipt.compatibility_family_id()
                != FoundationCompatibilityFamilyId::N512_FOUNDATION
            || self.receipt.foundation_asset_digest()
                != self
                    .frozen_abi
                    .foundation_abi()
                    .foundation_payload_digest()
                    .ok_or(ScaffoldContractError::PhenotypeCompile)?
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let material = projection_material(
            &self.source_brain_genome,
            self.source_genome_id,
            self.lineage_id,
            self.foundation,
            &self.genetic_provenance,
            &self.runtime_development_state,
            &capacity,
            self.receipt.sensor_profile(),
            self.frozen_abi.foundation_abi(),
        )?;
        if material.source_inputs_digest != self.receipt.source_inputs_digest()
            || material.overlay_seed != self.overlay_seed
            || material.runtime_development_digest != self.receipt.runtime_development_digest()
            || material.genetic_provenance_digest != self.receipt.genetic_provenance_digest()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    pub const fn source_genome_id(&self) -> GenomeId {
        self.source_genome_id
    }

    pub const fn lineage_id(&self) -> LineageId {
        self.lineage_id
    }

    pub const fn foundation(&self) -> &FoundationGeneticIdentity {
        &self.foundation
    }

    pub const fn source_brain_genome(&self) -> &BrainGenome {
        &self.source_brain_genome
    }

    pub const fn genetic_provenance(&self) -> &GeneticLineageProvenance {
        &self.genetic_provenance
    }

    pub const fn runtime_development_state(&self) -> &DevelopmentState {
        &self.runtime_development_state
    }

    pub const fn frozen_abi(&self) -> &N512FrozenAbiRecipe {
        &self.frozen_abi
    }

    pub const fn sensor_profile(&self) -> SensorProfile {
        self.receipt.sensor_profile()
    }

    pub const fn foundation_asset_digest(&self) -> Blake3Digest {
        self.receipt.foundation_asset_digest()
    }

    pub const fn overlay_seed(&self) -> u64 {
        self.overlay_seed
    }

    pub const fn receipt(&self) -> &N512FounderProjectionReceipt {
        &self.receipt
    }

    pub const fn compiled_phenotype(&self) -> &BrainPhenotype {
        &self.compiled_phenotype
    }
}

impl N512FrozenAbiRecipe {
    fn validate_against(
        &self,
        capacity: &BrainCapacityClass,
        compiled: &BrainPhenotype,
    ) -> Result<(), ScaffoldContractError> {
        if self.schema_version != N512_PROJECTION_SCHEMA_VERSION
            || self.coordinate_genome.brain_class_id != capacity.id()
            || self.coordinate_development_state.genome_id != self.coordinate_genome.id
            || self.coordinate_development_state.maturation.raw() != 1.0
            || self.foundation_abi != *compiled.foundation_abi()
            || self.layout_digest != compiled.foundation_abi().layout_digest()
            || self.address_map_digest != compiled.persistent_address_map().digest()
            || self.decoder_digest != compiled.candidate_decoder().canonical_digest()
            || self.route_abi_digest != compiled.route_abi_digest()
            || self.plasticity_abi_digest != compiled.plasticity_abi_digest()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        self.foundation_abi.validate_against(capacity)
    }
}

fn canonical_builtin_foundation(
    sensor_profile: SensorProfile,
    supplied: &FoundationWeightAsset,
) -> Result<FoundationWeightAsset, ScaffoldContractError> {
    let builtin = FoundationWeightAsset::builtin_nano512_v1(sensor_profile)?;
    if supplied.digest() != builtin.digest()
        || supplied.encode_canonical()? != builtin.encode_canonical()?
    {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    Ok(builtin)
}

fn canonical_frozen_abi(
    sensor_profile: SensorProfile,
) -> Result<N512FrozenAbiRecipe, ScaffoldContractError> {
    let capacity = BrainCapacityClass::n512();
    let foundation = FoundationWeightAsset::builtin_nano512_v1(sensor_profile)?;
    let coordinate_genome = BrainGenome::scaffold(N512_COORDINATE_RECIPE_SEED, capacity.id());
    let coordinate_development_state = DevelopmentState::new(
        coordinate_genome.id,
        Tick::ZERO,
        crate::NormalizedScalar::new(1.0)?,
    );
    let compiled = PhenotypeCompiler::compile_from_foundation_asset(
        &coordinate_genome,
        &capacity,
        &coordinate_development_state,
        sensor_profile,
        &foundation,
    )?;
    Ok(N512FrozenAbiRecipe::from_compiled(
        coordinate_genome,
        coordinate_development_state,
        &compiled,
    ))
}

struct ProjectionMaterial {
    source_inputs_digest: [u64; 4],
    runtime_development_digest: [u64; 4],
    genetic_provenance_digest: [u64; 4],
    overlay_seed: u64,
}

fn projection_material(
    source_brain_genome: &BrainGenome,
    source_genome_id: GenomeId,
    lineage_id: LineageId,
    foundation: FoundationGeneticIdentity,
    genetic_provenance: &GeneticLineageProvenance,
    runtime_development_state: &DevelopmentState,
    capacity: &BrainCapacityClass,
    sensor_profile: SensorProfile,
    foundation_abi: &FoundationAbiBinding,
) -> Result<ProjectionMaterial, ScaffoldContractError> {
    let source_inputs = PhenotypeCompilerInputs::try_new_with_foundation_abi(
        source_brain_genome.clone(),
        capacity,
        runtime_development_state.clone(),
        sensor_profile,
        foundation_abi.clone(),
    )?;
    let runtime_development_digest = digest_development(runtime_development_state)?;
    let genetic_provenance_digest = digest_provenance(genetic_provenance)?;
    let mut digest = CanonicalDigestBuilder::new(OVERLAY_DOMAIN);
    digest.write_u16(N512_PROJECTION_SCHEMA_VERSION);
    write_digest4(&mut digest, source_inputs.canonical_digest());
    digest.write_u64(source_genome_id.0);
    digest.write_u64(lineage_id.0);
    digest.write_u64(foundation.foundation_id);
    digest.write_u16(foundation.version);
    digest.write_u64(foundation.compatibility_family_id);
    digest.write_u16(foundation.brain_class_id.raw());
    write_digest4(&mut digest, genetic_provenance_digest);
    write_digest4(&mut digest, runtime_development_digest);
    digest.write_u16(sensor_profile.raw());
    digest.write_u64(foundation_abi.layout_id().0);
    write_blake3(&mut digest, foundation_abi.layout_digest());
    write_blake3(
        &mut digest,
        foundation_abi
            .foundation_payload_digest()
            .ok_or(ScaffoldContractError::PhenotypeCompile)?,
    );
    let projection_digest = digest.finish256();
    let overlay_seed = nonzero_seed(
        projection_digest[0]
            ^ projection_digest[1].rotate_left(17)
            ^ projection_digest[2].rotate_right(11)
            ^ projection_digest[3],
    );
    Ok(ProjectionMaterial {
        source_inputs_digest: source_inputs.canonical_digest(),
        runtime_development_digest,
        genetic_provenance_digest,
        overlay_seed,
    })
}

fn validate_source(
    phenotype: &CreaturePhenotype,
    capacity: &BrainCapacityClass,
) -> Result<(), ScaffoldContractError> {
    let expected = expected_foundation();
    phenotype.brain_genome.validate_contract()?;
    phenotype.foundation.validate_contract()?;
    if phenotype.source_genome_id.0 == 0
        || phenotype.lineage_id.0 == 0
        || phenotype.source_genome_id != phenotype.brain_genome.id
        || phenotype.brain_genome.brain_class_id != capacity.id()
        || phenotype.brain_genome.lineage_id != Some(phenotype.lineage_id)
        || phenotype.foundation != expected
    {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    Ok(())
}

fn expected_foundation() -> FoundationGeneticIdentity {
    FoundationGeneticIdentity {
        foundation_id: FoundationId::N512_V1.raw(),
        version: FoundationVersion::V1.raw() as u16,
        compatibility_family_id: FoundationCompatibilityFamilyId::N512_FOUNDATION.raw(),
        brain_class_id: BrainCapacityClass::N512_ID,
    }
}

fn digest_development(state: &DevelopmentState) -> Result<[u64; 4], ScaffoldContractError> {
    let mut digest = CanonicalDigestBuilder::new(RUNTIME_DEVELOPMENT_DOMAIN);
    digest.write_u64(state.genome_id.0);
    digest.write_u64(state.age_ticks.0);
    digest.write_f32(state.maturation.raw())?;
    digest.write_sequence_len(state.enabled_lobes.len());
    for lobe in &state.enabled_lobes {
        digest.write_u16(lobe.raw());
    }
    digest.write_sequence_len(state.active_sensor_channels.len());
    for sensor in &state.active_sensor_channels {
        digest.write_u8(sensor.raw());
    }
    digest.write_sequence_len(state.active_motor_affordances.len());
    for affordance in &state.active_motor_affordances {
        digest.write_u8(affordance.raw());
    }
    digest.write_sequence_len(state.open_critical_periods.len());
    for period in &state.open_critical_periods {
        digest.write_u16(period.lobe.raw());
        digest.write_u64(period.opens_at.0);
        digest.write_u64(period.closes_at.0);
        digest.write_f32(period.plasticity_bias.raw())?;
    }
    digest.write_u32(state.sleep_cycle_count);
    digest.write_u32(state.consolidation_cycle_count);
    match state.last_sleep_tick {
        Some(tick) => {
            digest.write_some();
            digest.write_u64(tick.0);
        }
        None => digest.write_none(),
    }
    Ok(digest.finish256())
}

fn digest_provenance(
    provenance: &GeneticLineageProvenance,
) -> Result<[u64; 4], ScaffoldContractError> {
    let mut digest = CanonicalDigestBuilder::new(PROVENANCE_DOMAIN);
    digest.write_u64(provenance.conception_seed);
    digest.write_bool(provenance.ordinary_birth);
    digest.write_sequence_len(provenance.recombination.len());
    for record in &provenance.recombination {
        digest.write_u8(chromosome_kind_raw(record.chromosome));
        digest.write_u8(record.maternal_segments);
        digest.write_u8(record.paternal_segments);
    }
    digest.write_sequence_len(provenance.mutations.len());
    for mutation in &provenance.mutations {
        match mutation {
            MutationRecord::Continuous {
                chromosome,
                locus_index,
                allele,
                before,
                after,
                lower,
                upper,
            } => {
                digest.write_u8(1);
                digest.write_u8(chromosome_kind_raw(*chromosome));
                digest.write_u8(*locus_index);
                digest.write_u8(allele_side_raw(*allele));
                digest.write_f32(*before)?;
                digest.write_f32(*after)?;
                digest.write_f32(*lower)?;
                digest.write_f32(*upper)?;
            }
            MutationRecord::Discrete {
                chromosome,
                locus_index,
                allele,
                before,
                after,
            } => {
                digest.write_u8(2);
                digest.write_u8(chromosome_kind_raw(*chromosome));
                digest.write_u8(*locus_index);
                digest.write_u8(allele_side_raw(*allele));
                digest.write_u16(*before);
                digest.write_u16(*after);
            }
        }
    }
    Ok(digest.finish256())
}

fn chromosome_kind_raw(value: ChromosomeKind) -> u8 {
    match value {
        ChromosomeKind::Body => 1,
        ChromosomeKind::Brain => 2,
        ChromosomeKind::Chemistry => 3,
        ChromosomeKind::Development => 4,
        ChromosomeKind::Reproduction => 5,
        ChromosomeKind::Predisposition => 6,
    }
}

fn allele_side_raw(value: AlleleSide) -> u8 {
    match value {
        AlleleSide::Maternal => 1,
        AlleleSide::Paternal => 2,
    }
}

fn write_digest4(digest: &mut CanonicalDigestBuilder, value: [u64; 4]) {
    for word in value {
        digest.write_u64(word);
    }
}

fn write_blake3(digest: &mut CanonicalDigestBuilder, value: Blake3Digest) {
    digest.write_bytes(value.bytes());
}

fn nonzero_seed(value: u64) -> u64 {
    if value == 0 {
        1
    } else {
        value
    }
}
