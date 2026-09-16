use std::{env, error::Error, ffi::OsString, fs, io, path::PathBuf};

use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, FoundationWeightAsset, NormalizedScalar,
    PhenotypeCompiler, SensorProfile, Tick, TrainingStageManifest,
};
use alife_training::{
    AdamWConfig, FoundationCurriculumStage, FoundationTrainer, N2048CurriculumV1,
    N2048FoundationProgram, N2048_FOUNDATION_TRAINING_SEED,
};

const MAX_ATTEMPTS_PER_STAGE: u32 = 128;
const OPTIMIZER_STEPS_PER_ATTEMPT: u32 = 8;
const DEFAULT_OUTPUT: &str = "target/artifacts/n2048-v1-grounded-trained.alife-foundation";

#[derive(Debug, PartialEq, Eq)]
struct TrainingOptions {
    output: PathBuf,
    retrain_language: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = parse_args(env::args_os().skip(1))?;
    let output = options.output;
    let retrain_language = options.retrain_language;
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(N2048_FOUNDATION_TRAINING_SEED, capacity.id());
    let development = DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0)?);
    let partial = output.with_extension("partial.alife-foundation");
    if retrain_language && !partial.exists() && output.exists() {
        let completed = FoundationWeightAsset::decode_canonical(&fs::read(&output)?)?;
        let completed_phenotype = PhenotypeCompiler::compile_from_foundation_asset(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
            &completed,
        )?;
        let language_partial = FoundationWeightAsset::from_trained_weights(
            &completed_phenotype,
            completed.weights().to_vec(),
            TrainingStageManifest::new(1, 1, 5),
        )?;
        fs::write(&partial, language_partial.encode_canonical()?)?;
    }
    let source = if partial.exists() {
        FoundationWeightAsset::decode_canonical(&fs::read(&partial)?)?
    } else {
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)?
    };
    let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        &source,
    )?;
    let curriculum = N2048CurriculumV1::new();
    let completed = source.manifest().training_stage().completed_stage_count() as usize;
    let next_stage = FoundationCurriculumStage::ALL
        .get(completed)
        .copied()
        .unwrap_or(FoundationCurriculumStage::HeldOutGeneralization);
    let initial_mask = curriculum.stage_mask(&phenotype, next_stage)?;
    let trainer =
        FoundationTrainer::new_required(phenotype, source, initial_mask, AdamWConfig::default())?;
    let mut program = N2048FoundationProgram::resume(trainer)?;
    for stage in FoundationCurriculumStage::ALL.into_iter().skip(completed) {
        let mut passed = false;
        for attempt in 0..MAX_ATTEMPTS_PER_STAGE {
            let receipt = program.run_stage(
                stage,
                OPTIMIZER_STEPS_PER_ATTEMPT,
                N2048_FOUNDATION_TRAINING_SEED.wrapping_add(u64::from(stage.ordinal()) << 32),
            )?;
            println!(
                "stage={} attempt={} successes={}/{} lower_bound={:.6} loss={:.6} regression={:.6} frozen={} passed={}",
                stage.ordinal(),
                attempt + 1,
                receipt.evaluation.successes(),
                receipt.evaluation.episodes(),
                receipt.evaluation.lower_confidence_bound()?,
                receipt.evaluation.mean_loss(),
                receipt.evaluation.maximum_locked_stage_regression(),
                receipt.evaluation.frozen_weights_bit_identical(),
                receipt.gate_passed,
            );
            if receipt.gate_passed {
                passed = true;
                break;
            }
        }
        if !passed {
            return Err(format!("stage {} did not pass its bounded gate", stage.ordinal()).into());
        }
        let checkpoint = program.export_stage_candidate()?;
        if let Some(parent) = partial.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&partial, checkpoint.encode_canonical()?)?;
    }
    let asset = program.export_completed_candidate()?;
    let bytes = asset.encode_canonical()?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, bytes)?;
    if partial.exists() {
        fs::remove_file(&partial)?;
    }
    println!(
        "export={} bytes={} completed_stages={} digest={}",
        output.display(),
        fs::metadata(&output)?.len(),
        asset.manifest().training_stage().completed_stage_count(),
        hex(asset.digest().bytes()),
    );
    Ok(())
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<TrainingOptions, io::Error> {
    let mut output = None;
    let mut retrain_language = false;
    for arg in args {
        if arg.to_str() == Some("--retrain-language") {
            retrain_language = true;
        } else if arg.to_string_lossy().starts_with("--") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown option: {}", arg.to_string_lossy()),
            ));
        } else if output.replace(PathBuf::from(arg)).is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "only one output path may be supplied",
            ));
        }
    }
    Ok(TrainingOptions {
        output: output.unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT)),
        retrain_language,
    })
}

fn hex(bytes: &[u8; 32]) -> String {
    let mut result = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut result, "{byte:02x}").expect("writing to a String cannot fail");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrain_flag_does_not_become_the_output_path() {
        let options = parse_args([OsString::from("--retrain-language")]).unwrap();
        assert_eq!(options.output, PathBuf::from(DEFAULT_OUTPUT));
        assert!(options.retrain_language);
    }

    #[test]
    fn output_and_retrain_flag_are_order_independent() {
        let expected = PathBuf::from("candidate.alife-foundation");
        for args in [
            ["candidate.alife-foundation", "--retrain-language"],
            ["--retrain-language", "candidate.alife-foundation"],
        ] {
            let options = parse_args(args.map(OsString::from)).unwrap();
            assert_eq!(options.output, expected);
            assert!(options.retrain_language);
        }
    }
}
