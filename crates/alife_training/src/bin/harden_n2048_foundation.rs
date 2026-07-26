use std::{env, error::Error, fs, path::PathBuf};

use alife_core::{FoundationWeightAsset, SensorProfile};
use alife_training::N2048EvolutionHardener;

const DEFAULT_HARDENING_SEED: u64 = 0x4E32_3034_385F_4556;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let output = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("target/artifacts/n2048-v1-grounded-hardened.alife-foundation")
    });
    let source = if let Some(path) = args.next().map(PathBuf::from) {
        FoundationWeightAsset::decode_canonical(&fs::read(path)?)?
    } else {
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)?
    };
    let outcome = N2048EvolutionHardener::new_required(source)?
        .harden_one_generation(DEFAULT_HARDENING_SEED)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, outcome.promoted_foundation.encode_canonical()?)?;
    println!(
        "export={} bytes={} adapter={} backend={} evaluated={} nonviable={} finalists={} descendants={} winner={} survival={:.6} learning={:.6} language={:.6} narration={:.6} robustness={:.6} efficiency={:.6} regression_stages={}",
        output.display(),
        fs::metadata(&output)?.len(),
        outcome.receipt.adapter_name,
        outcome.receipt.backend_api,
        outcome.receipt.evaluated_genomes,
        outcome.receipt.nonviable_genomes,
        outcome.receipt.pareto_finalists.len(),
        outcome.receipt.descendant_evaluations,
        outcome.receipt.winner_genome_id.raw(),
        outcome.receipt.winner_fitness.survival,
        outcome.receipt.winner_fitness.learning,
        outcome.receipt.winner_fitness.language_acquisition,
        outcome.receipt.winner_fitness.narration_fidelity,
        outcome.receipt.winner_fitness.mutation_robustness,
        outcome.receipt.winner_fitness.compute_efficiency,
        outcome.receipt.curated_regression_stage_count,
    );
    Ok(())
}
