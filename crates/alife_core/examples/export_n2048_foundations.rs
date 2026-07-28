use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, FoundationWeightAsset, NormalizedScalar,
    PhenotypeCompiler, SensorProfile, Tick,
};

const FOUNDATION_SEED: u64 = 0x4E32_3034_385F_0001;

fn main() -> Result<(), Box<dyn Error>> {
    let output_root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("assets/brain_foundations"));
    fs::create_dir_all(&output_root)?;
    export(
        &output_root,
        SensorProfile::PrivilegedAffordanceV1,
        "n2048-v1-privileged.alife-foundation",
    )?;
    export(
        &output_root,
        SensorProfile::GroundedObjectSlotsV1,
        "n2048-v1-grounded.alife-foundation",
    )?;
    Ok(())
}

fn export(
    output_root: &Path,
    sensor_profile: SensorProfile,
    file_name: &str,
) -> Result<(), Box<dyn Error>> {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(FOUNDATION_SEED, capacity.id());
    let development = DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0)?);
    let phenotype = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        sensor_profile,
    )?;
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&phenotype)?;
    let bytes = asset.encode_canonical()?;
    let output_path = output_root.join(file_name);
    fs::write(&output_path, bytes)?;
    println!(
        "{} {}",
        output_path.display(),
        hex_digest(asset.digest().bytes())
    );
    Ok(())
}

fn hex_digest(bytes: &[u8; 32]) -> String {
    let mut result = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut result, "{byte:02x}").expect("writing to a String cannot fail");
    }
    result
}
