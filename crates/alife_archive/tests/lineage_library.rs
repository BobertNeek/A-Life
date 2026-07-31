use std::{fs, path::PathBuf, time::SystemTime};

use alife_archive::{
    CompositeGeneticArchiveInput, GeneticArchiveInput, LifeArchiveInput, LineageLibrary,
    LineageLibraryConfig, ARCHIVE_PAGE_BYTES,
};
use alife_core::{
    ArchiveCheckpointDisposition, ArchiveCheckpointRetention, ArchiveLearnedCapturePolicy,
    BrainCapacityClass, BrainGenome, CreatureGenome, DevelopmentState, FoundationGeneticIdentity,
    FoundationWeightAsset, NormalizedScalar, OrganismId, PassiveLifeEvent, PassiveLifeStatistics,
    PhenotypeCompiler, SensorProfile, Tick,
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
    let birth_manifest = library.load_manifest(birth).unwrap();
    assert_eq!(library.load_brain_genome(&birth_manifest).unwrap(), genome);
    assert_eq!(library.latest_manifest_digests().unwrap(), vec![birth]);
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
    assert_eq!(
        library.latest_manifest_digests().unwrap(),
        vec![receipt.committed_manifest_digest]
    );
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
fn typed_life_statistics_round_trip_from_the_content_store() {
    let root = temp_root("statistics");
    let mut library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let (genome, phenotype) = fixture(151, 77);
    let birth = library
        .archive_birth(GeneticArchiveInput {
            source_run_id: "run-statistics",
            organism_id: OrganismId(77),
            birth_tick: Tick(2),
            genome: &genome,
            phenotype: &phenotype,
            foundation_asset_bytes: None,
        })
        .unwrap();
    let mut statistics = PassiveLifeStatistics::new(OrganismId(77), Tick(2)).unwrap();
    statistics
        .observe(PassiveLifeEvent::FoodOutcome { beneficial: true })
        .unwrap();
    statistics.finalize(Tick(9), "hazard").unwrap();
    let bytes = serde_json::to_vec(&statistics).unwrap();
    let receipt = library
        .archive_life(LifeArchiveInput {
            birth_manifest_digest: birth,
            death_tick: Tick(9),
            final_experience_sequence: None,
            statistics_bytes: &bytes,
            learned_checkpoint_bytes: None,
            checkpoint_retention: ArchiveCheckpointRetention::TemporaryPeak,
        })
        .unwrap();
    let manifest = library
        .load_manifest(receipt.committed_manifest_digest)
        .unwrap();
    assert_eq!(library.load_life_statistics(&manifest).unwrap(), statistics);
    assert_eq!(
        library.life_manifest_digests().unwrap(),
        vec![receipt.committed_manifest_digest]
    );
    drop(library);
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

#[test]
fn composite_birth_round_trips_complete_creature_genome_provenance() {
    let root = temp_root("composite-genome");
    let mut library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let foundation_identity = FoundationGeneticIdentity::new(
        0x4E32_3034_385F_5631,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    let maternal = CreatureGenome::early_mammal_founder(701, foundation_identity).unwrap();
    let paternal = CreatureGenome::early_mammal_founder(702, foundation_identity).unwrap();
    let child = CreatureGenome::reproduce(&maternal, &paternal, 703).unwrap();
    let expressed = child.express().unwrap();
    let development = expressed
        .development_state_at(Tick::new(u64::from(
            expressed.development.maturation_duration_ticks,
        )))
        .unwrap();
    let foundation =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        &foundation,
    )
    .unwrap();
    let manifest_digest = library
        .archive_composite_birth(CompositeGeneticArchiveInput {
            source_run_id: "ei0-composite-roundtrip",
            organism_id: OrganismId(703),
            birth_tick: Tick::new(9),
            creature_genome: &child,
            phenotype: &phenotype,
            foundation_asset_bytes: &foundation.encode_canonical().unwrap(),
        })
        .unwrap();
    let manifest = library.load_manifest(manifest_digest).unwrap();
    let restored = library.load_creature_genome(&manifest).unwrap();

    assert_eq!(restored, child);
    assert_eq!(restored.parent_genome_ids, vec![maternal.id, paternal.id]);
    assert_eq!(restored.lineage_id, child.lineage_id);
    assert_eq!(restored.provenance, child.provenance);
    assert_eq!(restored.foundation, foundation_identity);
    assert!(manifest.genetic.composite_genome_asset.is_some());

    drop(library);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn composite_birth_rejects_a_phenotype_compiled_from_another_genome() {
    let root = temp_root("composite-mismatch");
    let mut library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let foundation_identity = FoundationGeneticIdentity::new(
        0x4E32_3034_385F_5631,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    let archived = CreatureGenome::early_mammal_founder(801, foundation_identity).unwrap();
    let wrong = CreatureGenome::early_mammal_founder(802, foundation_identity).unwrap();
    let wrong_expressed = wrong.express().unwrap();
    let wrong_development = wrong_expressed
        .development_state_at(Tick::new(u64::from(
            wrong_expressed.development.maturation_duration_ticks,
        )))
        .unwrap();
    let foundation =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let wrong_phenotype = PhenotypeCompiler::compile_from_foundation_asset(
        &wrong_expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &wrong_development,
        SensorProfile::GroundedObjectSlotsV1,
        &foundation,
    )
    .unwrap();
    let foundation_bytes = foundation.encode_canonical().unwrap();

    let error = library
        .archive_composite_birth(CompositeGeneticArchiveInput {
            source_run_id: "ei0-composite-mismatch",
            organism_id: OrganismId(801),
            birth_tick: Tick::new(1),
            creature_genome: &archived,
            phenotype: &wrong_phenotype,
            foundation_asset_bytes: &foundation_bytes,
        })
        .unwrap_err();

    assert!(matches!(error, alife_archive::ArchiveError::Integrity(_)));
    drop(library);
    fs::remove_dir_all(root).unwrap();
}
