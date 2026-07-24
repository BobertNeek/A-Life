//! Contract-only canonical phenotype evidence identities shared by runtime evidence producers.

use serde::{Deserialize, Serialize};

use crate::{
    BrainCapacityClass, BrainPhenotype, CanonicalDigestBuilder, CompiledSynapseKind, PhenotypeHash,
    ScaffoldContractError, SensorProfile,
};

pub const GPU_PHENOTYPE_EVIDENCE_MANIFEST_SCHEMA: u16 = 1;
pub const GPU_CLOSED_LOOP_BENCHMARK_SCHEMA: u16 = 1;
pub const GPU_CLOSED_LOOP_BENCHMARK_PROTOCOL_VERSION: u16 = 1;
pub const GPU_CLOSED_LOOP_BENCHMARK_BASE_SEED: u64 = 4_404;
pub const GPU_CLOSED_LOOP_BENCHMARK_WARMUP_TICKS: u32 = 256;
pub const GPU_CLOSED_LOOP_BENCHMARK_MEASURED_TICKS: u32 = 1_024;
pub const GPU_CLOSED_LOOP_BENCHMARK_TIMESTAMP_SCOPE_SPLIT: u16 = 2;

const MANIFEST_DOMAIN: &[u8] = b"alife.gpu.evidence.phenotype-manifest.v1";
const BENCHMARK_PROTOCOL_DOMAIN: &[u8] = b"alife.gpu.closed-loop-benchmark.protocol.v1";
const LOBE_LAYOUT_DOMAIN: &[u8] = b"alife.gpu.evidence.lobe-layout.v1";
const PROJECTION_PLAN_DOMAIN: &[u8] = b"alife.gpu.evidence.projection-plan.v1";
const SYNAPSE_PAYLOAD_DOMAIN: &[u8] = b"alife.gpu.evidence.synapse-payload.v1";
const PLASTICITY_NONE_DOMAIN: &[u8] = b"alife.gpu.evidence.plasticity-plan.none.v1";
const REPLAY_CAPTURE_NONE_DOMAIN: &[u8] = b"alife.gpu.evidence.replay-capture.none.v1";

/// Versioned benchmark protocol identity shared by evidence producers and promotion ingestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GpuClosedLoopBenchmarkProtocolV1 {
    pub schema_version: u16,
    pub protocol_version: u16,
    pub warmup_ticks: u32,
    pub measured_ticks: u32,
    pub samples_per_tick: u16,
    pub nearest_rank_percentile: u16,
    pub timestamp_scope_raw: u16,
    pub base_seed: u64,
    pub protocol_digest: [u64; 4],
}

impl GpuClosedLoopBenchmarkProtocolV1 {
    pub fn canonical() -> Self {
        let mut protocol = Self {
            schema_version: GPU_CLOSED_LOOP_BENCHMARK_SCHEMA,
            protocol_version: GPU_CLOSED_LOOP_BENCHMARK_PROTOCOL_VERSION,
            warmup_ticks: GPU_CLOSED_LOOP_BENCHMARK_WARMUP_TICKS,
            measured_ticks: GPU_CLOSED_LOOP_BENCHMARK_MEASURED_TICKS,
            samples_per_tick: 1,
            nearest_rank_percentile: 95,
            timestamp_scope_raw: GPU_CLOSED_LOOP_BENCHMARK_TIMESTAMP_SCOPE_SPLIT,
            base_seed: GPU_CLOSED_LOOP_BENCHMARK_BASE_SEED,
            protocol_digest: [0; 4],
        };
        protocol.protocol_digest = protocol.recompute_digest();
        protocol
    }

    pub fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(BENCHMARK_PROTOCOL_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u16(self.protocol_version);
        digest.write_u32(self.warmup_ticks);
        digest.write_u32(self.measured_ticks);
        digest.write_u16(self.samples_per_tick);
        digest.write_u16(self.nearest_rank_percentile);
        digest.write_u16(self.timestamp_scope_raw);
        digest.write_u64(self.base_seed);
        digest.finish256()
    }

    pub fn is_canonical(&self) -> bool {
        self == &Self::canonical()
    }

    pub const fn row_seed(self, class_id_raw: u16, profile_id_raw: u16, population: u32) -> u64 {
        self.base_seed
            ^ ((class_id_raw as u64) << 48)
            ^ ((profile_id_raw as u64) << 32)
            ^ (population as u64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhenotypeEvidenceManifest {
    pub schema_version: u16,
    pub class_id_raw: u16,
    pub phenotype_sensor_profile_raw: u16,
    pub phenotype_hash: PhenotypeHash,
    pub compile_inputs_digest: [u64; 4],
    pub capacity_digest: [u64; 4],
    pub lobe_layout_digest: [u64; 4],
    pub projection_plan_digest: [u64; 4],
    pub synapse_payload_digest: [u64; 4],
    pub encoder_plan_digest: [u64; 4],
    pub decoder_plan_digest: [u64; 4],
    pub plasticity_plan_digest: [u64; 4],
    pub replay_capture_plan_digest: [u64; 4],
    pub manifest_digest: [u64; 4],
}

impl PhenotypeEvidenceManifest {
    pub fn from_phenotype(
        phenotype: &BrainPhenotype,
        capacity: &BrainCapacityClass,
    ) -> Result<Self, ScaffoldContractError> {
        Self::build(phenotype, capacity, false)
    }

    pub fn from_learning_phenotype(
        phenotype: &BrainPhenotype,
        capacity: &BrainCapacityClass,
    ) -> Result<Self, ScaffoldContractError> {
        Self::build(phenotype, capacity, true)
    }

    fn build(
        phenotype: &BrainPhenotype,
        capacity: &BrainCapacityClass,
        learning: bool,
    ) -> Result<Self, ScaffoldContractError> {
        phenotype.validate_against(capacity)?;
        let (plasticity_plan_digest, replay_capture_plan_digest) = if learning {
            (
                phenotype.plasticity_plan_digest(),
                phenotype.replay_capture_plan().canonical_digest(),
            )
        } else {
            (
                explicit_none_digest(PLASTICITY_NONE_DOMAIN),
                explicit_none_digest(REPLAY_CAPTURE_NONE_DOMAIN),
            )
        };
        let mut manifest = Self {
            schema_version: GPU_PHENOTYPE_EVIDENCE_MANIFEST_SCHEMA,
            class_id_raw: phenotype.brain_class_id().raw(),
            phenotype_sensor_profile_raw: phenotype.sensor_profile().raw(),
            phenotype_hash: phenotype.phenotype_hash(),
            compile_inputs_digest: phenotype.compiler_inputs_digest(),
            capacity_digest: capacity.canonical_digest(),
            lobe_layout_digest: lobe_layout_evidence_digest(phenotype),
            projection_plan_digest: projection_plan_evidence_digest(phenotype),
            synapse_payload_digest: synapse_payload_evidence_digest(phenotype)?,
            encoder_plan_digest: phenotype.sensor_encoder().canonical_digest(),
            decoder_plan_digest: phenotype.candidate_decoder().canonical_digest(),
            plasticity_plan_digest,
            replay_capture_plan_digest,
            manifest_digest: [0; 4],
        };
        manifest.manifest_digest = manifest.recompute_manifest_digest();
        manifest.validate_for_capacity(capacity)?;
        Ok(manifest)
    }

    pub fn recompute_manifest_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(MANIFEST_DOMAIN);
        self.encode_without_digest(&mut digest);
        digest.finish256()
    }

    pub fn validate_for_capacity(
        &self,
        capacity: &BrainCapacityClass,
    ) -> Result<(), ScaffoldContractError> {
        capacity.validate_contract()?;
        SensorProfile::try_from_raw(self.phenotype_sensor_profile_raw)?;
        if self.schema_version != GPU_PHENOTYPE_EVIDENCE_MANIFEST_SCHEMA
            || self.class_id_raw != capacity.id().raw()
            || self.capacity_digest != capacity.canonical_digest()
            || self.phenotype_hash == PhenotypeHash([0; 4])
            || self.manifest_digest != self.recompute_manifest_digest()
            || self
                .component_digests()
                .into_iter()
                .any(|digest| digest == [0; 4])
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    pub fn encode_without_digest(&self, digest: &mut CanonicalDigestBuilder) {
        digest.write_u16(self.schema_version);
        digest.write_u16(self.class_id_raw);
        digest.write_u16(self.phenotype_sensor_profile_raw);
        write_digest4(digest, self.phenotype_hash.0);
        for component in self.component_digests() {
            write_digest4(digest, component);
        }
    }

    pub fn encode_with_digest(&self, digest: &mut CanonicalDigestBuilder) {
        self.encode_without_digest(digest);
        write_digest4(digest, self.manifest_digest);
    }

    pub const fn component_digests(&self) -> [[u64; 4]; 9] {
        [
            self.compile_inputs_digest,
            self.capacity_digest,
            self.lobe_layout_digest,
            self.projection_plan_digest,
            self.synapse_payload_digest,
            self.encoder_plan_digest,
            self.decoder_plan_digest,
            self.plasticity_plan_digest,
            self.replay_capture_plan_digest,
        ]
    }
}

pub fn lobe_layout_evidence_digest(phenotype: &BrainPhenotype) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(LOBE_LAYOUT_DOMAIN);
    digest.write_sequence_len(phenotype.lobe_layout().regions.len());
    for region in &phenotype.lobe_layout().regions {
        digest.write_u16(region.id.0);
        digest.write_u16(region.kind.raw());
        digest.write_u32(region.start);
        digest.write_u32(region.len);
        digest.write_bool(region.enabled);
        digest.write_u8(region.update_cadence.raw());
        digest.write_u8(region.plasticity_policy as u8);
        digest.write_u8(region.activation_policy as u8);
        digest.write_u8(region.essentiality as u8);
        digest.write_u8(region.throttle_priority as u8);
    }
    digest.finish256()
}

pub fn projection_plan_evidence_digest(phenotype: &BrainPhenotype) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(PROJECTION_PLAN_DOMAIN);
    digest.write_sequence_len(phenotype.projections().len());
    for projection in phenotype.projections() {
        digest.write_u16(projection.route_index());
        digest.write_u16(projection.source_lobe().raw());
        digest.write_u16(projection.target_lobe().raw());
        digest.write_u8(projection.projection_type().raw());
        digest.write_u8(projection.active_tile_policy().raw());
        digest.write_u8(projection.update_cadence().raw());
        digest.write_u8(projection.priority().raw());
        digest.write_u8(projection.delay_microsteps());
        let (start, len) = projection.synapse_range();
        digest.write_u32(start);
        digest.write_u32(len);
        digest.write_u32(projection.active_tile_count());
    }
    digest.finish256()
}

pub fn synapse_payload_evidence_digest(
    phenotype: &BrainPhenotype,
) -> Result<[u64; 4], ScaffoldContractError> {
    let mut digest = CanonicalDigestBuilder::new(SYNAPSE_PAYLOAD_DOMAIN);
    digest.write_sequence_len(phenotype.synapses().len());
    for synapse in phenotype.synapses() {
        digest.write_u32(synapse.source());
        digest.write_u32(synapse.target());
        digest.write_f32(synapse.genetic_weight())?;
        digest.write_f32(synapse.alpha())?;
        digest.write_u16(synapse.route_index());
        match synapse.kind() {
            CompiledSynapseKind::Recurrent => digest.write_u8(0),
            CompiledSynapseKind::Decoder(coordinate) => {
                digest.write_u8(1);
                digest.write_u32(coordinate.head().raw());
                digest.write_u8(coordinate.family().raw());
                digest.write_u16(coordinate.input_lane());
                digest.write_u16(coordinate.motor_index());
            }
        }
    }
    Ok(digest.finish256())
}

fn explicit_none_digest(domain: &[u8]) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(domain);
    digest.write_none();
    digest.finish256()
}

fn write_digest4(digest: &mut CanonicalDigestBuilder, words: [u64; 4]) {
    for word in words {
        digest.write_u64(word);
    }
}
