//! Explicit genetic research initialization by complete persistent address.
//! No acquired state, optimizer checkpoint, or creature record enters this API.
use alife_core::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, thiserror::Error)]
pub enum GroundedDriveTransferError {
    #[error("source contract: {0}")]
    Contract(#[from] ScaffoldContractError),
    #[error("grounded-drive matching failed: {0}")]
    Mismatch(&'static str),
}

pub struct GroundedDriveGeneticTransfer {
    pub phenotype: BrainPhenotype,
    pub inputs: PhenotypeCompilerInputs,
    pub asset: FoundationWeightAsset,
    pub source_asset_digest: Blake3Digest,
    pub source_to_target: Vec<(u32, u32)>,
    pub added_zero_rows: usize,
}

/// Requires an asset-bound source from compile_n2048_bounded_founder_candidate.
/// Make a NEW unpromoted founder. This cannot restore or migrate an organism.
/// Retained weights are copied only after complete logical-address and causal
/// parameter checks. New derived rows remain zero as the matched control.
pub fn transfer_bounded_genetic_founder_to_grounded_drive(
    source: &BrainPhenotype,
    source_inputs: &PhenotypeCompilerInputs,
    source_asset: &FoundationWeightAsset,
) -> Result<GroundedDriveGeneticTransfer, GroundedDriveTransferError> {
    use GroundedDriveTransferError::Mismatch;
    if !source.foundation_abi().is_n2048_bounded_founder_v1() || source.synapses().len() != 32768 {
        return Err(Mismatch("requires exact bounded source graph"));
    }
    if source.foundation_abi().foundation_weight_asset() != Some(source_asset.asset_ref()) {
        return Err(Mismatch(
            "source must be explicitly asset-bound by the bounded candidate factory",
        ));
    }
    source_asset.validate_against(source)?;
    if PhenotypeCompiler::compile_validated(source_inputs, &BrainCapacityClass::n2048())? != *source
    {
        return Err(Mismatch("source compiler inputs do not reproduce source"));
    }
    // Ordinary public genome validation explicitly allows a retained genetic
    // prior. Preserve every causal gene, changing only the new class/genome ID.
    let identity = BrainGenome::scaffold(
        source_inputs.genome().species_seed,
        BrainCapacityClass::N2048_GROUNDED_DRIVE_RESEARCH_ID,
    );
    let mut genome = source_inputs.genome().clone();
    genome.brain_class_id = identity.brain_class_id;
    genome.id = identity.id;
    genome.seeds.genome_id_seed = identity.seeds.genome_id_seed;
    genome.validate_contract()?;
    let mut development = source_inputs.development().clone();
    development.genome_id = genome.id;
    development.validate_contract()?;
    let (fresh, _) = PhenotypeCompiler::compile_n2048_grounded_drive_basis(
        genome.clone(),
        development.clone(),
        source.sensor_profile(),
    )?;
    if source.neuron_dynamics() != fresh.neuron_dynamics()
        || source.sensor_encoder() != fresh.sensor_encoder()
        || source.language_codebook() != fresh.language_codebook()
        || source.microstep_count() != fresh.microstep_count()
        || source.cognitive_architecture() != fresh.cognitive_architecture()
        || source.cognitive_architecture_plan() != fresh.cognitive_architecture_plan()
        || source.plasticity_receptors() != fresh.plasticity_receptors()
        || source.sleep_consolidation_plan() != fresh.sleep_consolidation_plan()
        || source.candidate_decoder().memory_channel() != fresh.candidate_decoder().memory_channel()
        || source.foundation_abi().fresh_dendritic_parameters()
            != fresh.foundation_abi().fresh_dendritic_parameters()
    {
        return Err(Mismatch(
            "retained dynamics, encoder, learning or branch parameters differ",
        ));
    }
    for (a, b) in source
        .candidate_decoder()
        .families()
        .iter()
        .zip(fresh.candidate_decoder().families())
    {
        if a.family() != b.family()
            || a.bias().to_bits() != b.bias().to_bits()
            || a.innate_drive_mask() != b.innate_drive_mask()
            || a.innate_gain().to_bits() != b.innate_gain().to_bits()
            || a.innate_cue_lane() != b.innate_cue_lane()
            || a.innate_cue_inverted() != b.innate_cue_inverted()
            || a.innate_requires_reach() != b.innate_requires_reach()
        {
            return Err(Mismatch("innate candidate contribution differs"));
        }
    }
    if source.projections().len() != fresh.projections().len() {
        return Err(Mismatch("projection set differs"));
    }
    for (a, b) in source.projections().iter().zip(fresh.projections()) {
        if a.route_index() != b.route_index()
            || a.source_lobe() != b.source_lobe()
            || a.target_lobe() != b.target_lobe()
            || a.projection_type() != b.projection_type()
            || a.active_tile_policy() != b.active_tile_policy()
            || a.update_cadence() != b.update_cadence()
            || a.priority() != b.priority()
            || a.delay_microsteps() != b.delay_microsteps()
            || a.active_tile_count() != b.active_tile_count()
        {
            return Err(Mismatch("retained projection policy differs"));
        }
    }
    let mut destination = BTreeMap::new();
    for entry in fresh.persistent_address_map().synapses() {
        if destination
            .insert(entry.address(), entry.packed_index())
            .is_some()
        {
            return Err(Mismatch("duplicate destination logical address"));
        }
    }
    let mut weights = vec![0.; fresh.synapses().len()];
    let mut source_to_target = Vec::with_capacity(32768);
    let mut seen = BTreeSet::new();
    for entry in source.persistent_address_map().synapses() {
        let old = entry.packed_index();
        let new = destination
            .remove(&entry.address())
            .ok_or(Mismatch("missing retained logical address"))?;
        if !seen.insert(entry.address()) {
            return Err(Mismatch("duplicate source logical address"));
        }
        let a = &source.synapses()[old as usize];
        let b = &fresh.synapses()[new as usize];
        if a.kind() != b.kind()
            || a.source() != b.source()
            || a.target() != b.target()
            || a.alpha().to_bits() != b.alpha().to_bits()
            || a.receptor_index() != b.receptor_index()
            || a.route_index() != b.route_index()
        {
            return Err(Mismatch("retained coordinate alpha or receptor differs"));
        }
        weights[new as usize] = source_asset.weights()[old as usize];
        source_to_target.push((old, new));
    }
    if source_to_target.len() != 32768 || destination.len() != 1280 {
        return Err(Mismatch("unexpected retained/additional row count"));
    }
    for (address, id) in &destination {
        let c = address
            .decoder
            .ok_or(Mismatch("added non-decoder address"))?;
        if c.head != DecoderHeadKind::ActionCandidate
            || !(36..46).contains(&c.input_lane)
            || fresh.synapses()[*id as usize].genetic_weight().to_bits() != 0_f32.to_bits()
        {
            return Err(Mismatch("unexpected added coordinate or nonzero control"));
        }
    }
    let index_map: BTreeMap<u32, u32> = source_to_target.iter().copied().collect();
    let mut mapped_capture = source
        .replay_capture_plan()
        .global_synapse_ids()
        .iter()
        .map(|id| {
            index_map
                .get(id)
                .copied()
                .ok_or(Mismatch("capture address absent"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    mapped_capture.sort_unstable();
    if mapped_capture != fresh.replay_capture_plan().global_synapse_ids()
        || source.replay_capture_plan().samples_per_event()
            != fresh.replay_capture_plan().samples_per_event()
        || source.replay_capture_plan().event_capacity()
            != fresh.replay_capture_plan().event_capacity()
        || source.replay_capture_plan().sample_capacity()
            != fresh.replay_capture_plan().sample_capacity()
    {
        return Err(Mismatch("retained replay capture plan differs"));
    }
    let asset = FoundationWeightAsset::from_trained_weights(
        &fresh,
        weights,
        source_asset.manifest().training_stage(),
    )?;
    let (phenotype, inputs) = PhenotypeCompiler::compile_n2048_grounded_drive_basis_candidate(
        genome,
        development,
        asset.clone(),
    )?;
    for (old, new) in &source_to_target {
        if phenotype.synapses()[*new as usize]
            .genetic_weight()
            .to_bits()
            != source_asset.weights()[*old as usize].to_bits()
        {
            return Err(Mismatch("exact weight transfer changed during admission"));
        }
    }
    Ok(GroundedDriveGeneticTransfer {
        phenotype,
        inputs,
        asset,
        source_asset_digest: source_asset.digest(),
        source_to_target,
        added_zero_rows: destination.len(),
    })
}
