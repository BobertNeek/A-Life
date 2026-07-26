use std::{fs, path::PathBuf, time::SystemTime};

use alife_archive::{
    GeneticArchiveInput, LifeArchiveInput, LineageLibrary, LineageLibraryConfig, ARCHIVE_PAGE_BYTES,
};
use alife_core::{
    ArchiveCheckpointDisposition, ArchiveCheckpointRetention, ArchiveLearnedCapturePolicy,
    BrainCapacityClass, BrainGenome, DevelopmentState, FoundationWeightAsset, NormalizedScalar,
    OrganismId, PhenotypeCompiler, SensorProfile, Tick,
};

fn temp_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "alife-lineage-{label}-{}-{nonce}",
        std::process::id()
    ))
}

fn fixture(seed: u64, organism_raw: u64) -> (BrainGenome, alife_core::BrainPhenotype) {
    let capacity = BrainCapacityClass::production_for_id(BrainCapacityClass::N512_ID).unwrap();
    let genome = BrainGenome::scaffold(seed, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.25).unwrap());
    let phenotype = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    assert_ne!(organism_raw, 0);
    (genome, phenotype)
}

#[test]
fn genetic_birth_life_checkpoint_and_rebuilt_index_are_durable() {
    let root = temp_root("roundtrip");
    fs::create_dir_all(root.join("staging")).unwrap();
    fs::write(root.join("staging").join("partial.tmp"), b"partial").unwrap();
    let mut library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    assert!(fs::read_dir(root.join("staging")).unwrap().next().is_none());

    let (genome, phenotype) = fixture(101, 7);
    let birth = library
        .archive_birth(GeneticArchiveInput {
            source_run_id: "run-alpha",
            organism_id: OrganismId(7),
            birth_tick: Tick(4),
            genome: &genome,
            phenotype: &phenotype,
            foundation_asset_bytes: None,
        })
        .unwrap();
    let checkpoint_bytes = (0..(ARCHIVE_PAGE_BYTES * 2 + 137))
        .map(|index| (index % 251) as u8)
        .collect::<Vec<_>>();
    let receipt = library
        .archive_life(LifeArchiveInput {
            birth_manifest_digest: birth,
            death_tick: Tick(88),
            final_experience_sequence: None,
            statistics_bytes: br#"{"survival_ticks":84}"#,
            learned_checkpoint_bytes: Some(&checkpoint_bytes),
            checkpoint_retention: ArchiveCheckpointRetention::TemporaryPeak,
        })
        .unwrap();
    let manifest = library
        .load_manifest(receipt.committed_manifest_digest)
        .unwrap();
    let life = manifest.life.unwrap();
    let checkpoint = match &life.checkpoint {
        ArchiveCheckpointDisposition::Stored(reference) => reference,
        other => panic!("expected stored checkpoint, got {other:?}"),
    };
    assert_eq!(checkpoint.pages.len(), 3);
    assert_eq!(
        library.read_checkpoint(checkpoint).unwrap(),
        checkpoint_bytes
    );
    assert_eq!(library.manifest_count().unwrap(), 2);
    drop(library);

    fs::remove_file(root.join("lineage.db")).unwrap();
    let rebuilt = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    assert_eq!(rebuilt.manifest_count().unwrap(), 2);
    assert_eq!(
        rebuilt
            .latest_manifest_for("run-alpha", OrganismId(7))
            .unwrap(),
        Some(receipt.committed_manifest_digest)
    );
    drop(rebuilt);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn quota_downgrades_automatic_capture_but_never_blocks_genetic_archive_or_pin() {
    assert_eq!(ArchiveLearnedCapturePolicy::GeneticOnly.retention(), None);
    assert_eq!(
        ArchiveLearnedCapturePolicy::Pinned.retention(),
        Some(ArchiveCheckpointRetention::Pinned)
    );
    let root = temp_root("quota");
    let mut config = LineageLibraryConfig::profile_default(&root);
    config.full_state_quota_bytes = 1;
    let mut library = LineageLibrary::open(config).unwrap();

    let (genome_a, phenotype_a) = fixture(201, 8);
    let birth_a = library
        .archive_birth(GeneticArchiveInput {
            source_run_id: "run-quota",
            organism_id: OrganismId(8),
            birth_tick: Tick::ZERO,
            genome: &genome_a,
            phenotype: &phenotype_a,
            foundation_asset_bytes: None,
        })
        .unwrap();
    let automatic = library
        .archive_life(LifeArchiveInput {
            birth_manifest_digest: birth_a,
            death_tick: Tick(9),
            final_experience_sequence: None,
            statistics_bytes: b"{}",
            learned_checkpoint_bytes: Some(&[7; 1024]),
            checkpoint_retention: ArchiveCheckpointRetention::AutomaticPermanent,
        })
        .unwrap();
    let automatic_manifest = library
        .load_manifest(automatic.committed_manifest_digest)
        .unwrap();
    assert!(matches!(
        automatic_manifest.life.unwrap().checkpoint,
        ArchiveCheckpointDisposition::DowngradedToGeneticOnly { .. }
    ));

    let (genome_b, phenotype_b) = fixture(202, 9);
    let birth_b = library
        .archive_birth(GeneticArchiveInput {
            source_run_id: "run-quota",
            organism_id: OrganismId(9),
            birth_tick: Tick::ZERO,
            genome: &genome_b,
            phenotype: &phenotype_b,
            foundation_asset_bytes: None,
        })
        .unwrap();
    let pinned = library
        .archive_life(LifeArchiveInput {
            birth_manifest_digest: birth_b,
            death_tick: Tick(10),
            final_experience_sequence: None,
            statistics_bytes: b"{}",
            learned_checkpoint_bytes: Some(&[9; 1024]),
            checkpoint_retention: ArchiveCheckpointRetention::Pinned,
        })
        .unwrap();
    let pinned_manifest = library
        .load_manifest(pinned.committed_manifest_digest)
        .unwrap();
    assert!(matches!(
        pinned_manifest.life.unwrap().checkpoint,
        ArchiveCheckpointDisposition::Stored(_)
    ));
    assert_eq!(library.manifest_count().unwrap(), 4);
    drop(library);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn n2048_birth_copies_the_exact_shipped_foundation_asset() {
    let root = temp_root("foundation");
    let mut library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(303, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.25).unwrap());
    let foundation =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        &foundation,
    )
    .unwrap();
    let foundation_bytes = foundation.encode_canonical().unwrap();
    let birth = library
        .archive_birth(GeneticArchiveInput {
            source_run_id: "run-foundation",
            organism_id: OrganismId(10),
            birth_tick: Tick::ZERO,
            genome: &genome,
            phenotype: &phenotype,
            foundation_asset_bytes: Some(&foundation_bytes),
        })
        .unwrap();
    let manifest = library.load_manifest(birth).unwrap();
    assert_eq!(
        manifest.genetic.foundation_payload_digest,
        Some(foundation.digest())
    );
    assert_eq!(
        manifest.genetic.foundation_asset.unwrap().size_bytes,
        foundation_bytes.len() as u64
    );
    assert_eq!(fs::read_dir(root.join("assets")).unwrap().count(), 2);
    drop(library);
    fs::remove_dir_all(root).unwrap();
}
