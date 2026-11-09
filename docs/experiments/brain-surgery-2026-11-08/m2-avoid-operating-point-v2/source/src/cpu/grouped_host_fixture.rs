//! Fresh B1 plus explicitly inherited bounded memory reader and four motor heads.
//! No archived life, learned checkpoint, concentration patch or training input.
#[allow(dead_code)]
#[path = "../../examples/support/grounded_r0_dedicated_fatigue.rs"]
mod dedicated;
#[allow(dead_code)]
#[path = "../../examples/support/multidrive_frozen.rs"]
mod frozen;
#[allow(dead_code)]
#[path = "../../examples/support/grounded_r0_motor_operating_point.rs"]
mod readout;
#[allow(dead_code)]
#[path = "../../examples/support/grounded_care_reference_v1.rs"]
pub(super) mod reference;
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_world::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub struct Recipe {
    pub config: CanonicalNewGameConfig,
    pub genotype: CreatureGenome,
    pub phenotype: BrainPhenotype,
    pub asset: FoundationWeightAsset,
    pub inputs: PhenotypeCompilerInputs,
    pub metadata: Value,
}
pub fn build(seed: u64, timer: bool) -> Result<Recipe> {
    build_revision(seed, timer, false)
}
pub fn build_matched_reference(seed: u64, timer: bool) -> Result<Recipe> {
    build_revision(seed, timer, true)
}
fn build_revision(seed: u64, timer: bool, matched_reference: bool) -> Result<Recipe> {
    let mut config = CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    let builtin = FoundationWeightAsset::builtin_n2048_v1(config.sensor_profile)?;
    let original = create_canonical_new_game_with_n2048_candidate(&config, &builtin)?;
    let mut genotype = original
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("missing newborn")?
        .genome()
        .clone();
    // These are new inherited genes before this source's expression/compilation.
    let a = genotype.brain.construction.maternal_architecture;
    let heads = alife_core::genome::CognitiveArchitectureGenomeParameters::try_new_v1(
        a.attention_capacity(),
        a.active_concept_limit(),
        a.active_gap_limit(),
        a.predictor_capacity(),
        a.predictor_learning_rate(),
        4,
        a.motor_head_width(),
        a.dendritic_branch_capacity(),
        a.structural_candidate_budget(),
        a.structural_edit_budget(),
        a.sleep_trigger_threshold(),
        a.sleep_replay_rate(),
        a.sleep_consolidation_rate(),
        a.attention_learning_rate(),
        a.concept_learning_rate(),
        a.motor_learning_rate(),
        a.structural_learning_rate(),
    )?;
    genotype.brain.construction.maternal_architecture = heads;
    genotype.brain.construction.paternal_architecture = heads;
    genotype.chemistry.graph = genotype
        .chemistry
        .graph
        .with_awake_rest_recovery_v1(AlleleSide::Maternal)?
        .with_awake_rest_recovery_v1(AlleleSide::Paternal)?;
    if timer {
        genotype.chemistry.graph = genotype
            .chemistry
            .graph
            .with_recovery_timescale_v1(AlleleSide::Maternal)?
            .with_recovery_timescale_v1(AlleleSide::Paternal)?;
    }
    let expressed = genotype.express()?;
    let genome = expressed.brain_genome.clone();
    let development = foundation_construction_development(
        &genome,
        &BrainCapacityClass::n2048(),
        &expressed.development_state_at(Tick::ZERO)?,
    )?;
    let (bounded, _) = PhenotypeCompiler::compile_n2048_bounded_founder(
        genome.clone(),
        development.clone(),
        config.sensor_profile,
    )?;
    let (weights, circuits) = frozen::whole_care(&bounded)?;
    let (weights, differential) = readout::counterbalance(&bounded, &weights, &circuits, 0., true)?;
    let (weights, references) =
        dedicated::full_reference_calibration(&bounded, &weights, &circuits, true)?;
    let (mut weights, dedicated_b) =
        dedicated::build(&bounded, &weights, dedicated::Factor::Dedicated)?;
    let mut reader = Vec::new();
    for (index, synapse) in bounded.synapses().iter().enumerate() {
        if matches!(synapse.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::MemoryContext)
        {
            if weights[index] != 0. {
                return Err("archived B1 reader is no longer zero".into());
            }
            weights[index] = synapse.genetic_weight();
            reader.push(json!({"bounded_index":index,"address":bounded.persistent_address_map().synapses()[index].address(),"b1":0.,"composed":weights[index]}));
        }
    }
    if reader.len() != 4096
        || reader
            .iter()
            .filter(|r| r["composed"].as_f64() != Some(0.))
            .count()
            != 592
    {
        return Err("qualified bounded reader coordinates changed".into());
    }
    let reference_revision = if matched_reference {
        let (next, metadata) = reference::build(&bounded, &weights)?;
        weights = next;
        Some(metadata)
    } else {
        None
    };
    let source_asset = FoundationWeightAsset::from_trained_weights(
        &bounded,
        weights,
        TrainingStageManifest::bootstrap(),
    )?;
    let m = source_asset.manifest();
    genotype.foundation = FoundationGeneticIdentity::new(
        m.foundation_id().raw(),
        m.foundation_version().raw() as u16,
        m.compatibility_family_id().raw(),
        BrainCapacityClass::N2048_ID,
    )?;
    genotype.n2048_foundation_candidate = Some(source_asset.clone());
    genotype.validate_contract()?;
    if genotype.express()?.brain_genome != genome {
        return Err("asset binding changed inherited brain".into());
    }
    let (source, source_inputs) = PhenotypeCompiler::compile_n2048_bounded_founder_candidate(
        genome,
        development,
        source_asset.clone(),
    )?;
    let transfer = crate::transfer_bounded_genetic_founder_to_grounded_drive(
        &source,
        &source_inputs,
        &source_asset,
    )?;
    if transfer.added_zero_rows != 1280 {
        return Err("grounded transfer changed".into());
    }
    for row in &mut reader {
        let index = row["bounded_index"]
            .as_u64()
            .ok_or("reader index missing")? as u32;
        let (_, target) = transfer
            .source_to_target
            .iter()
            .find(|(source, _)| *source == index)
            .ok_or("reader transfer row missing")?;
        let weight = transfer.asset.weights()[*target as usize];
        if weight.to_bits() != source_asset.weights()[index as usize].to_bits() {
            return Err("reader weight changed during transfer".into());
        }
        row["grounded_index"] = json!(target);
        row["grounded_address"] = json!(transfer.phenotype.persistent_address_map().synapses()
            [*target as usize]
            .address());
    }
    let reader_max = reader
        .iter()
        .map(|r| r["composed"].as_f64().unwrap().abs())
        .fold(0_f64, f64::max) as f32;
    if reader_max.to_bits() != 0.01375_f32.to_bits() {
        return Err("bounded reader maximum differs".into());
    }
    let reader_sha256 = format!("{:x}", Sha256::digest(serde_json::to_vec(&reader)?));
    let genotype = CreatureGenome::from_n2048_grounded_drive_transfer(
        &genotype,
        &transfer.inputs,
        transfer.asset.clone(),
    )?;
    genotype.validate_n2048_grounded_drive_inputs(&transfer.inputs)?;
    if transfer
        .phenotype
        .cognitive_architecture()
        .motor_head_count()
        != 4
        || !transfer
            .phenotype
            .candidate_decoder()
            .factorized_motor_channels(&transfer.phenotype)?
            .contains(&MotorChannel::Posture)
        || genotype
            .express_n2048_grounded_drive_research()?
            .chemistry
            .biochemical
            .value_profile()
            .hormones[8]
            != 0.
    {
        return Err("composed birth capability or valuation mismatch".into());
    }
    let mut metadata = json!({"recipe":"B1+bounded-reader+four-head+awake-rest-v1", "seed":seed,"timer":timer,
        "b1_recipe":"whole-care6/counterbalance/full references/dedicated fatigue B/tonic-on",
        "reader_is_authored_not_learned":true,"reader_rows":reader,"reader_sha256":reader_sha256,"differential":differential,"references":references,"dedicated_b":dedicated_b,
        "source_asset":source_asset.digest(),"target_asset":transfer.asset.digest(),"inputs":transfer.inputs.canonical_digest(),
        "phenotype":transfer.phenotype.phenotype_hash(),"added_zero_rows":transfer.added_zero_rows,
        "reader_max_abs":reader_max,"genotype_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(&genotype)?)),"expressed_graph":genotype.express_n2048_grounded_drive_research()?.chemistry.biochemical,"pressure_value":0.,"world_steps":0,"neural_steps":0,"optimizer_steps":0});
    if let Some(revision) = reference_revision {
        metadata["recipe"] =
            json!("B1+bounded-reader+four-head+awake-rest+care-matched-reference-v1");
        metadata["source_revision"] = revision;
    }
    Ok(Recipe {
        config,
        genotype,
        phenotype: transfer.phenotype,
        asset: transfer.asset,
        inputs: transfer.inputs,
        metadata,
    })
}
