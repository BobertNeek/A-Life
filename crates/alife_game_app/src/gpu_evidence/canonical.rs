//! Typed canonical encoders for Slice A evidence identities.

use alife_core::{
    BrainGenome, BrainPhenotype, CanonicalDigestBuilder, CompiledSynapseKind, DevelopmentState,
    PhenotypeHash, PolicyBackend, Tick,
};
use alife_gpu_backend::GpuHardwareReceipt;

use super::{
    GpuClosedLoopAcceptanceOptions, GpuEvidenceError, GpuSameAdapterReplayEvidence,
    GpuSelectionEvidence, GpuSliceAAcceptanceReceipt, GpuSliceEvidenceHeader,
    PhenotypeEvidenceManifest, ADAPTER_IDENTITY_DOMAIN, ARTIFACT_DOMAIN, CANDIDATE_SEQUENCE_DOMAIN,
    FRAME_SEQUENCE_DOMAIN, GPU_EVIDENCE_ORGANISM_ID, GPU_SLICE_A_FIXTURE_SCHEMA,
    INITIAL_STATE_DOMAIN, LOBE_LAYOUT_DOMAIN, LOGIT_SEQUENCE_DOMAIN, MANIFEST_DOMAIN,
    PROJECTION_PLAN_DOMAIN, SYNAPSE_PAYLOAD_DOMAIN,
};

pub(super) fn new_artifact_digest() -> CanonicalDigestBuilder {
    CanonicalDigestBuilder::new(ARTIFACT_DOMAIN)
}

pub(super) fn new_manifest_digest() -> CanonicalDigestBuilder {
    CanonicalDigestBuilder::new(MANIFEST_DOMAIN)
}

pub(super) fn lobe_layout_digest(phenotype: &BrainPhenotype) -> [u64; 4] {
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

pub(super) fn projection_plan_digest(phenotype: &BrainPhenotype) -> [u64; 4] {
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

pub(super) fn synapse_payload_digest(
    phenotype: &BrainPhenotype,
) -> Result<[u64; 4], GpuEvidenceError> {
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

pub(super) fn explicit_none_digest(domain: &[u8]) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(domain);
    digest.write_none();
    digest.finish256()
}

pub(super) fn adapter_identity_digest(hardware: &GpuHardwareReceipt) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(ADAPTER_IDENTITY_DOMAIN);
    digest.write_utf8(&hardware.backend_api);
    digest.write_utf8(&hardware.adapter_name);
    digest.write_u32(hardware.vendor_id);
    digest.write_u32(hardware.device_id);
    write_digest4(&mut digest, hardware.driver_digest);
    write_digest4(&mut digest, hardware.feature_digest);
    write_digest4(&mut digest, hardware.limits_digest);
    digest.write_u16(hardware.gpu_layout_version);
    digest.write_utf8(&hardware.backend_version);
    digest.finish256()
}

pub(super) fn initial_state_digest(
    options: &GpuClosedLoopAcceptanceOptions,
    phenotype: &BrainPhenotype,
    genome: &BrainGenome,
    development: &DevelopmentState,
) -> Result<[u64; 4], GpuEvidenceError> {
    if development.genome_id != genome.id || development.age_ticks != Tick::ZERO {
        return Err(GpuEvidenceError::Contract(
            "initial development state is not a fresh genetic birth",
        ));
    }
    let mut digest = CanonicalDigestBuilder::new(INITIAL_STATE_DOMAIN);
    digest.write_u16(GPU_SLICE_A_FIXTURE_SCHEMA);
    digest.write_u16(options.capacity.id().raw());
    write_digest4(&mut digest, options.capacity.canonical_digest());
    write_digest4(&mut digest, phenotype.phenotype_hash().0);
    digest.write_u64(options.deterministic_seed);
    digest.write_u16(options.sensor_profile.raw());
    digest.write_u64(GPU_EVIDENCE_ORGANISM_ID.raw());
    digest.write_u64(genome.id.0);
    digest.write_u16(genome.schema_version);
    digest.write_u64(development.age_ticks.raw());
    digest.write_f32(development.maturation.raw())?;
    Ok(digest.finish256())
}

pub(super) fn initial_state_digest_from_receipt(
    phenotype_hash: PhenotypeHash,
    receipt: &GpuSliceAAcceptanceReceipt,
) -> [u64; 4] {
    let birth_seed = receipt.deterministic_seed ^ GPU_EVIDENCE_ORGANISM_ID.raw().rotate_left(17);
    let genome = BrainGenome::scaffold(birth_seed, receipt.capacity.id());
    let mut digest = CanonicalDigestBuilder::new(INITIAL_STATE_DOMAIN);
    digest.write_u16(receipt.fixture_schema_version);
    digest.write_u16(receipt.capacity.id().raw());
    write_digest4(&mut digest, receipt.capacity.canonical_digest());
    write_digest4(&mut digest, phenotype_hash.0);
    digest.write_u64(receipt.deterministic_seed);
    digest.write_u16(receipt.phenotype_manifest.phenotype_sensor_profile_raw);
    digest.write_u64(GPU_EVIDENCE_ORGANISM_ID.raw());
    digest.write_u64(genome.id.0);
    digest.write_u16(genome.schema_version);
    digest.write_u64(Tick::ZERO.raw());
    digest
        .write_f32(0.35)
        .expect("the canonical genetic-birth maturation is finite");
    digest.finish256()
}

pub(super) fn frame_sequence_digest(trace: &[GpuSelectionEvidence]) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(FRAME_SEQUENCE_DOMAIN);
    digest.write_sequence_len(trace.len());
    for entry in trace {
        digest.write_u64(entry.tick);
        write_digest4(&mut digest, entry.frame_digest);
    }
    digest.finish256()
}

pub(super) fn candidate_sequence_digest(trace: &[GpuSelectionEvidence]) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(CANDIDATE_SEQUENCE_DOMAIN);
    digest.write_sequence_len(trace.len());
    for entry in trace {
        digest.write_u64(entry.tick);
        digest.write_u16(entry.candidate_index);
        digest.write_u32(entry.action_id_raw);
        digest.write_u8(entry.action_family_raw);
        digest.write_u64(entry.candidate_feature_digest[0]);
        digest.write_u64(entry.candidate_feature_digest[1]);
    }
    digest.finish256()
}

pub(super) fn logit_sequence_digest(
    trace: &[GpuSelectionEvidence],
) -> Result<[u64; 4], GpuEvidenceError> {
    let mut digest = CanonicalDigestBuilder::new(LOGIT_SEQUENCE_DOMAIN);
    digest.write_sequence_len(trace.len());
    for entry in trace {
        digest.write_u64(entry.tick);
        digest.write_f32(entry.logit)?;
    }
    Ok(digest.finish256())
}

pub(super) fn max_logit_error(
    first: &[GpuSelectionEvidence],
    second: &[GpuSelectionEvidence],
) -> Result<f32, GpuEvidenceError> {
    if first.len() != second.len() {
        return Err(GpuEvidenceError::Contract(
            "replay traces have different lengths",
        ));
    }
    let mut max_error = 0.0_f32;
    for (left, right) in first.iter().zip(second) {
        if !left.logit.is_finite() || !right.logit.is_finite() {
            return Err(GpuEvidenceError::Contract(
                "replay trace contains a non-finite logit",
            ));
        }
        max_error = max_error.max((left.logit - right.logit).abs());
    }
    Ok(max_error)
}

pub(super) fn encode_manifest_without_digest(
    digest: &mut CanonicalDigestBuilder,
    manifest: &PhenotypeEvidenceManifest,
) {
    digest.write_u16(manifest.schema_version);
    digest.write_u16(manifest.class_id_raw);
    digest.write_u16(manifest.phenotype_sensor_profile_raw);
    write_digest4(digest, manifest.phenotype_hash.0);
    for component in manifest_component_digests(manifest) {
        write_digest4(digest, component);
    }
}

pub(super) fn encode_manifest_with_digest(
    digest: &mut CanonicalDigestBuilder,
    manifest: &PhenotypeEvidenceManifest,
) {
    encode_manifest_without_digest(digest, manifest);
    write_digest4(digest, manifest.manifest_digest);
}

pub(super) fn manifest_component_digests(manifest: &PhenotypeEvidenceManifest) -> [[u64; 4]; 9] {
    [
        manifest.compile_inputs_digest,
        manifest.capacity_digest,
        manifest.lobe_layout_digest,
        manifest.projection_plan_digest,
        manifest.synapse_payload_digest,
        manifest.encoder_plan_digest,
        manifest.decoder_plan_digest,
        manifest.plasticity_plan_digest,
        manifest.replay_capture_plan_digest,
    ]
}

pub(super) fn encode_header_without_artifact_digest(
    digest: &mut CanonicalDigestBuilder,
    header: &GpuSliceEvidenceHeader,
) {
    digest.write_u16(header.artifact_schema);
    digest.write_u16(header.slice_raw);
    digest.write_u16(header.class_id_raw);
    digest.write_u16(header.profile_id_raw);
    digest.write_u16(header.profile_schema);
    digest.write_u16(header.status_raw);
    digest.write_utf8(&header.git_commit);
    digest.write_utf8(&header.source_tree_digest);
    write_digest4(digest, header.phenotype_hash.0);
    write_digest4(digest, header.phenotype_manifest_digest);
    write_digest4(digest, header.capacity_digest);
}

pub(super) fn encode_hardware(digest: &mut CanonicalDigestBuilder, hardware: &GpuHardwareReceipt) {
    digest.write_u16(hardware.schema_version);
    digest.write_u64(hardware.generation);
    digest.write_utf8(&hardware.backend_api);
    digest.write_utf8(&hardware.adapter_name);
    digest.write_u32(hardware.vendor_id);
    digest.write_u32(hardware.device_id);
    write_digest4(digest, hardware.driver_digest);
    write_digest4(digest, hardware.feature_digest);
    write_digest4(digest, hardware.limits_digest);
    digest.write_u16(hardware.gpu_layout_version);
    digest.write_utf8(&hardware.backend_version);
}

pub(super) fn encode_selection(
    digest: &mut CanonicalDigestBuilder,
    selection: &GpuSelectionEvidence,
) -> Result<(), GpuEvidenceError> {
    digest.write_u64(selection.tick);
    write_digest4(digest, selection.frame_digest);
    digest.write_u16(selection.candidate_index);
    digest.write_u32(selection.action_id_raw);
    digest.write_u8(selection.action_family_raw);
    digest.write_u64(selection.candidate_feature_digest[0]);
    digest.write_u64(selection.candidate_feature_digest[1]);
    digest.write_f32(selection.logit)?;
    digest.write_u8(selection.active_activation_side);
    digest.write_u32(selection.active_tiles);
    digest.write_u32(selection.active_synapses);
    digest.write_u32(selection.compact_readback_bytes);
    digest.write_bool(selection.outcome_success);
    Ok(())
}

pub(super) fn encode_replay(
    digest: &mut CanonicalDigestBuilder,
    replay: &GpuSameAdapterReplayEvidence,
) -> Result<(), GpuEvidenceError> {
    write_digest4(digest, replay.adapter_identity_digest);
    write_digest4(digest, replay.initial_state_digest);
    write_digest4(digest, replay.frame_sequence_digest);
    write_digest4(digest, replay.selected_candidate_digest);
    write_digest4(digest, replay.first_logit_digest);
    write_digest4(digest, replay.second_logit_digest);
    digest.write_f32(replay.tolerance)?;
    digest.write_f32(replay.max_abs_error)?;
    digest.write_bool(replay.passed);
    Ok(())
}

pub(super) fn write_digest4(digest: &mut CanonicalDigestBuilder, words: [u64; 4]) {
    for word in words {
        digest.write_u64(word);
    }
}

pub(super) fn hardware_digests(hardware: &GpuHardwareReceipt) -> [[u64; 4]; 3] {
    [
        hardware.driver_digest,
        hardware.feature_digest,
        hardware.limits_digest,
    ]
}

pub(super) fn policy_raw(policy: PolicyBackend) -> u8 {
    match policy {
        PolicyBackend::NeuralClosedLoopGpu => 1,
        PolicyBackend::HeuristicBaseline => 2,
    }
}
