use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use alife_archive::{
    CompositeGeneticArchiveBatchInput, CompositeGeneticArchiveInput, GeneticArchiveInput,
    LifeArchiveInput, LineageLibrary, LineageLibraryConfig, ARCHIVE_PAGE_BYTES,
    MAX_COMPOSITE_BIRTH_BATCH_BYTES,
};
use alife_core::{
    ArchiveCheckpointDisposition, ArchiveCheckpointRetention, ArchiveLearnedCapturePolicy,
    Blake3Digest, BrainCapacityClass, BrainGenome, BrainPhenotype, CreatureGenome,
    DevelopmentState, FoundationGeneticIdentity, FoundationWeightAsset, GenomeId, LineageId,
    N512FounderFoundationProjection, N512FounderProjectionReceipt, NormalizedScalar, OrganismId,
    PassiveLifeEvent, PassiveLifeStatistics, PhenotypeCompiler, SensorProfile, Tick,
};
use rusqlite::{params, Connection, OpenFlags};

struct CompositeFixture {
    creature_genome: CreatureGenome,
    phenotype: BrainPhenotype,
    foundation_identity: FoundationGeneticIdentity,
    foundation_content_digest: Blake3Digest,
    foundation_asset_bytes: Vec<u8>,
    projection_receipt: Option<N512FounderProjectionReceipt>,
}

impl CompositeFixture {
    fn input<'a>(
        &'a self,
        source_run_id: &'a str,
        organism_id: OrganismId,
        birth_tick: Tick,
    ) -> CompositeGeneticArchiveBatchInput<'a> {
        CompositeGeneticArchiveBatchInput {
            source_run_id,
            organism_id,
            genome_id: self.creature_genome.id,
            lineage_id: self.creature_genome.lineage_id,
            birth_tick,
            foundation: self.foundation_identity,
            foundation_content_digest: self.foundation_content_digest,
            sensor_profile: self.phenotype.sensor_profile(),
            projection_receipt: self.projection_receipt.as_ref(),
            phenotype_hash: self.phenotype.phenotype_hash(),
            creature_genome: &self.creature_genome,
            phenotype: &self.phenotype,
            foundation_asset_bytes: &self.foundation_asset_bytes,
        }
    }
}

fn generic_composite_fixture(seed: u64, sensor_profile: SensorProfile) -> CompositeFixture {
    let foundation_identity = FoundationGeneticIdentity::new(
        0x4E32_3034_385F_5631,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    let creature_genome = CreatureGenome::early_mammal_founder(seed, foundation_identity).unwrap();
    let expressed = creature_genome.express().unwrap();
    let development = expressed
        .development_state_at(Tick::new(u64::from(
            expressed.development.maturation_duration_ticks,
        )))
        .unwrap();
    let capacity = BrainCapacityClass::n2048();
    let foundation = FoundationWeightAsset::builtin_n2048_v1(sensor_profile).unwrap();
    let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
        &expressed.brain_genome,
        &capacity,
        &development,
        sensor_profile,
        &foundation,
    )
    .unwrap();
    let foundation_content_digest = foundation.digest();
    let foundation_asset_bytes = foundation.encode_canonical().unwrap();
    CompositeFixture {
        creature_genome,
        phenotype,
        foundation_identity,
        foundation_content_digest,
        foundation_asset_bytes,
        projection_receipt: None,
    }
}

fn curated_n512_fixture(seed: u64, sensor_profile: SensorProfile) -> CompositeFixture {
    let foundation_identity = FoundationGeneticIdentity::new(
        0x004E_3531_325F_5631,
        1,
        0x4E35_3132_5F00_FA11,
        BrainCapacityClass::N512_ID,
    )
    .unwrap();
    let creature_genome = CreatureGenome::early_mammal_founder(seed, foundation_identity).unwrap();
    let expressed = creature_genome.express().unwrap();
    let foundation = FoundationWeightAsset::builtin_nano512_v1(sensor_profile).unwrap();
    let projection =
        N512FounderFoundationProjection::compile(&expressed, sensor_profile, &foundation).unwrap();
    let phenotype = projection.compiled_phenotype().clone();
    let foundation_content_digest = foundation.digest();
    let foundation_asset_bytes = foundation.encode_canonical().unwrap();
    CompositeFixture {
        creature_genome,
        phenotype,
        foundation_identity,
        foundation_content_digest,
        foundation_asset_bytes,
        projection_receipt: Some(projection.receipt().clone()),
    }
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArchiveStateSnapshot {
    manifest_count: u64,
    latest: Vec<Blake3Digest>,
    manifest_rows: Vec<(String, String, String, String, i64, Option<String>)>,
    topology: BTreeSet<String>,
    files: BTreeMap<String, Vec<u8>>,
}

fn snapshot_archive_files(root: &Path) -> (BTreeSet<String>, BTreeMap<String, Vec<u8>>) {
    fn visit(
        root: &Path,
        directory: &Path,
        topology: &mut BTreeSet<String>,
        files: &mut BTreeMap<String, Vec<u8>>,
    ) {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .to_string();
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
                topology.insert(format!("L:{relative}"));
            } else if metadata.is_dir() {
                topology.insert(format!("D:{relative}"));
                visit(root, &path, topology, files);
            } else {
                topology.insert(format!("F:{relative}"));
                files.insert(relative, fs::read(path).unwrap());
            }
        }
    }

    let mut topology = BTreeSet::new();
    let mut files = BTreeMap::new();
    for directory in ["manifests", "assets", "staging"] {
        topology.insert(format!("D:{directory}"));
        visit(root, &root.join(directory), &mut topology, &mut files);
    }
    (topology, files)
}

fn snapshot_manifest_rows(
    root: &Path,
) -> Vec<(String, String, String, String, i64, Option<String>)> {
    let connection =
        Connection::open_with_flags(root.join("lineage.db"), OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let mut statement = connection
        .prepare(
            "SELECT digest,source_run_id,organism_id,genome_id,is_life,death_tick \
             FROM manifests ORDER BY rowid",
        )
        .unwrap();
    statement
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn snapshot_archive_state(library: &LineageLibrary, root: &Path) -> ArchiveStateSnapshot {
    let manifest_count = library.manifest_count().unwrap();
    let latest = library.latest_manifest_digests().unwrap();
    let manifest_rows = snapshot_manifest_rows(root);
    let (topology, files) = snapshot_archive_files(root);
    ArchiveStateSnapshot {
        manifest_count,
        latest,
        manifest_rows,
        topology,
        files,
    }
}

fn digest_hex_for_test(digest: Blake3Digest) -> String {
    digest
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).unwrap();
        }
    }
}

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

#[test]
fn composite_birth_batch_prepare_preserves_order_digests_and_archive_bytes() {
    let root = temp_root("batch-order");
    let library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let first = generic_composite_fixture(901, SensorProfile::GroundedObjectSlotsV1);
    let second = generic_composite_fixture(902, SensorProfile::GroundedObjectSlotsV1);
    let before = snapshot_archive_state(&library, &root);

    let first_input = first.input("batch-order", OrganismId(901), Tick::new(11));
    let second_input = second.input("batch-order", OrganismId(902), Tick::new(12));
    let prepared = library
        .prepare_composite_birth_batch(&[first_input, second_input])
        .unwrap();
    let first_digest = prepared.items()[0].manifest_digest();
    let second_digest = prepared.items()[1].manifest_digest();
    assert_eq!(prepared.len(), 2);
    assert_eq!(
        prepared.manifest_digests(),
        vec![first_digest, second_digest]
    );
    assert_eq!(prepared.items()[0].organism_id(), OrganismId(901));
    assert_eq!(prepared.items()[1].organism_id(), OrganismId(902));
    assert_eq!(
        prepared.items()[0].manifest().genetic.sensor_profile,
        SensorProfile::GroundedObjectSlotsV1
    );
    assert!(root.join("manifests").read_dir().unwrap().next().is_none());

    let reordered = library
        .prepare_composite_birth_batch(&[second_input, first_input])
        .unwrap();
    assert_eq!(
        reordered.manifest_digests(),
        vec![second_digest, first_digest]
    );
    assert_eq!(snapshot_archive_state(&library, &root), before);

    drop(library);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn composite_birth_batch_prepare_uses_the_actual_non_default_profile() {
    let root = temp_root("batch-profile");
    let library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let fixture = generic_composite_fixture(903, SensorProfile::PrivilegedAffordanceV1);
    let before = snapshot_archive_state(&library, &root);

    let prepared = library
        .prepare_composite_birth_batch(&[fixture.input(
            "batch-profile",
            OrganismId(903),
            Tick::new(13),
        )])
        .unwrap();
    assert_eq!(
        prepared.items()[0].manifest().genetic.sensor_profile,
        SensorProfile::PrivilegedAffordanceV1
    );
    assert_eq!(snapshot_archive_state(&library, &root), before);

    drop(library);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn composite_birth_batch_prepare_rejects_a_curated_receipt_mismatch_without_writes() {
    let root = temp_root("batch-receipt-mismatch");
    let library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let fixture = curated_n512_fixture(904, SensorProfile::GroundedObjectSlotsV1);
    let other = curated_n512_fixture(905, SensorProfile::GroundedObjectSlotsV1);
    let mut input = fixture.input("batch-receipt-mismatch", OrganismId(904), Tick::new(14));
    input.projection_receipt = other.projection_receipt.as_ref();
    let before = snapshot_archive_state(&library, &root);

    let error = library.prepare_composite_birth_batch(&[input]).unwrap_err();
    assert!(matches!(
        error,
        alife_archive::ArchiveError::Contract(_) | alife_archive::ArchiveError::Integrity(_)
    ));
    assert_eq!(snapshot_archive_state(&library, &root), before);

    drop(library);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn composite_birth_batch_prepare_rejects_projection_provenance_mismatch_without_writes() {
    let root = temp_root("batch-receipt-provenance");
    let library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let fixture = curated_n512_fixture(914, SensorProfile::GroundedObjectSlotsV1);
    let other = curated_n512_fixture(915, SensorProfile::GroundedObjectSlotsV1);
    let mut input = fixture.input("batch-receipt-provenance", OrganismId(914), Tick::new(20));
    input.projection_receipt = other.projection_receipt.as_ref();
    let before = snapshot_archive_state(&library, &root);

    let error = library.prepare_composite_birth_batch(&[input]).unwrap_err();
    assert!(matches!(
        error,
        alife_archive::ArchiveError::Contract(_) | alife_archive::ArchiveError::Integrity(_)
    ));
    assert_eq!(snapshot_archive_state(&library, &root), before);

    drop(library);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn composite_birth_batch_prepare_accepts_a_contract_valid_curated_receipt() {
    let root = temp_root("batch-receipt-valid");
    let library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let fixture = curated_n512_fixture(905, SensorProfile::PrivilegedAffordanceV1);
    let before = snapshot_archive_state(&library, &root);

    let prepared = library
        .prepare_composite_birth_batch(&[fixture.input(
            "batch-receipt-valid",
            OrganismId(905),
            Tick::new(14),
        )])
        .unwrap();
    assert_eq!(prepared.len(), 1);
    assert_eq!(
        prepared.items()[0].manifest().genetic.sensor_profile,
        SensorProfile::PrivilegedAffordanceV1
    );
    assert_eq!(snapshot_archive_state(&library, &root), before);

    drop(library);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn composite_birth_batch_prepare_rejects_duplicate_targets_and_ids_without_writes() {
    let root = temp_root("batch-duplicates");
    let library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let first = generic_composite_fixture(906, SensorProfile::GroundedObjectSlotsV1);
    let second = generic_composite_fixture(907, SensorProfile::GroundedObjectSlotsV1);
    let first_input = first.input("batch-duplicates", OrganismId(906), Tick::new(15));
    let before = snapshot_archive_state(&library, &root);

    let mut duplicate_target = second.input("batch-duplicates", OrganismId(907), Tick::new(16));
    duplicate_target.organism_id = first_input.organism_id;
    assert!(library
        .prepare_composite_birth_batch(&[first_input, duplicate_target])
        .is_err());

    let mut duplicate_organism = second.input("batch-duplicates", OrganismId(907), Tick::new(16));
    duplicate_organism.organism_id = first_input.organism_id;
    duplicate_organism.source_run_id = "batch-duplicates-other";
    assert!(library
        .prepare_composite_birth_batch(&[first_input, duplicate_organism])
        .is_err());

    let mut duplicate_genome = second.input("batch-duplicates", OrganismId(907), Tick::new(16));
    duplicate_genome.genome_id = first_input.genome_id;
    assert!(library
        .prepare_composite_birth_batch(&[first_input, duplicate_genome])
        .is_err());

    let mut duplicate_lineage = second.input("batch-duplicates", OrganismId(907), Tick::new(16));
    duplicate_lineage.lineage_id = first_input.lineage_id;
    assert!(library
        .prepare_composite_birth_batch(&[first_input, duplicate_lineage])
        .is_err());

    assert_eq!(snapshot_archive_state(&library, &root), before);
    drop(library);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn composite_birth_batch_prepare_rejects_invalid_input_before_any_write() {
    let root = temp_root("batch-invalid-input");
    let library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let fixture = generic_composite_fixture(908, SensorProfile::GroundedObjectSlotsV1);
    let other = generic_composite_fixture(909, SensorProfile::GroundedObjectSlotsV1);
    let before = snapshot_archive_state(&library, &root);

    let mut wrong_run = fixture.input("batch-invalid-input", OrganismId(908), Tick::new(17));
    wrong_run.source_run_id = "invalid run";
    assert!(library.prepare_composite_birth_batch(&[wrong_run]).is_err());

    let mut wrong_foundation = fixture.input("batch-invalid-input", OrganismId(908), Tick::new(17));
    wrong_foundation.foundation = FoundationGeneticIdentity::new(
        0xDEAD,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    assert!(library
        .prepare_composite_birth_batch(&[wrong_foundation])
        .is_err());

    let mut wrong_digest = fixture.input("batch-invalid-input", OrganismId(908), Tick::new(17));
    wrong_digest.foundation_content_digest = Blake3Digest::default();
    assert!(library
        .prepare_composite_birth_batch(&[wrong_digest])
        .is_err());

    let mut wrong_bytes = fixture.input("batch-invalid-input", OrganismId(908), Tick::new(17));
    wrong_bytes.foundation_asset_bytes = &[1, 2, 3];
    assert!(library
        .prepare_composite_birth_batch(&[wrong_bytes])
        .is_err());

    let mut wrong_phenotype = fixture.input("batch-invalid-input", OrganismId(908), Tick::new(17));
    wrong_phenotype.phenotype = &other.phenotype;
    wrong_phenotype.phenotype_hash = other.phenotype.phenotype_hash();
    assert!(library
        .prepare_composite_birth_batch(&[wrong_phenotype])
        .is_err());

    let mut wrong_profile = fixture.input("batch-invalid-input", OrganismId(908), Tick::new(17));
    wrong_profile.sensor_profile = SensorProfile::PrivilegedAffordanceV1;
    assert!(library
        .prepare_composite_birth_batch(&[wrong_profile])
        .is_err());

    let mut wrong_genome_id = fixture.input("batch-invalid-input", OrganismId(908), Tick::new(17));
    wrong_genome_id.genome_id = GenomeId(999_001);
    assert!(library
        .prepare_composite_birth_batch(&[wrong_genome_id])
        .is_err());

    let mut wrong_lineage_id = fixture.input("batch-invalid-input", OrganismId(908), Tick::new(17));
    wrong_lineage_id.lineage_id = LineageId(999_002);
    assert!(library
        .prepare_composite_birth_batch(&[wrong_lineage_id])
        .is_err());

    assert_eq!(snapshot_archive_state(&library, &root), before);
    drop(library);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn composite_birth_batch_prepare_rejects_empty_and_oversized_batches_without_writes() {
    let root = temp_root("batch-bounds");
    let library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let fixture = generic_composite_fixture(913, SensorProfile::GroundedObjectSlotsV1);
    let before = snapshot_archive_state(&library, &root);

    let empty: [CompositeGeneticArchiveBatchInput<'_>; 0] = [];
    assert!(library.prepare_composite_birth_batch(&empty).is_err());

    let oversized = vec![
        fixture.input("batch-bounds", OrganismId(913), Tick::new(19));
        alife_archive::MAX_COMPOSITE_BIRTH_BATCH_ITEMS + 1
    ];
    assert!(library.prepare_composite_birth_batch(&oversized).is_err());
    assert_eq!(snapshot_archive_state(&library, &root), before);

    drop(library);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn composite_birth_batch_prepare_rejects_indexed_conflicts_and_unindexed_orphans() {
    let indexed_root = temp_root("batch-indexed-conflict");
    let mut indexed_library =
        LineageLibrary::open(LineageLibraryConfig::profile_default(&indexed_root)).unwrap();
    let indexed_fixture = generic_composite_fixture(910, SensorProfile::GroundedObjectSlotsV1);
    let indexed_expressed = indexed_fixture.creature_genome.express().unwrap();
    indexed_library
        .archive_birth(GeneticArchiveInput {
            source_run_id: "batch-indexed-conflict",
            organism_id: OrganismId(910),
            birth_tick: Tick::new(1),
            genome: &indexed_expressed.brain_genome,
            phenotype: &indexed_fixture.phenotype,
            foundation_asset_bytes: Some(&indexed_fixture.foundation_asset_bytes),
        })
        .unwrap();
    let indexed_before = snapshot_archive_state(&indexed_library, &indexed_root);
    assert!(indexed_library
        .prepare_composite_birth_batch(&[indexed_fixture.input(
            "batch-indexed-conflict",
            OrganismId(910),
            Tick::new(2),
        )])
        .is_err());
    assert_eq!(
        snapshot_archive_state(&indexed_library, &indexed_root),
        indexed_before
    );
    drop(indexed_library);
    fs::remove_dir_all(indexed_root).unwrap();

    let source_root = temp_root("batch-orphan-source");
    let target_root = temp_root("batch-orphan-target");
    let mut source_library =
        LineageLibrary::open(LineageLibraryConfig::profile_default(&source_root)).unwrap();
    let orphan_fixture = generic_composite_fixture(911, SensorProfile::GroundedObjectSlotsV1);
    let orphan_digest = source_library
        .archive_composite_birth(CompositeGeneticArchiveInput {
            source_run_id: "batch-orphan-target",
            organism_id: OrganismId(911),
            birth_tick: Tick::new(99),
            creature_genome: &orphan_fixture.creature_genome,
            phenotype: &orphan_fixture.phenotype,
            foundation_asset_bytes: &orphan_fixture.foundation_asset_bytes,
        })
        .unwrap();
    drop(source_library);

    let target_library =
        LineageLibrary::open(LineageLibraryConfig::profile_default(&target_root)).unwrap();
    fs::copy(
        source_root
            .join("manifests")
            .join(format!("{}.json", digest_hex_for_test(orphan_digest))),
        target_root
            .join("manifests")
            .join(format!("{}.json", digest_hex_for_test(orphan_digest))),
    )
    .unwrap();
    copy_tree(&source_root.join("assets"), &target_root.join("assets"));
    let orphan_before = snapshot_archive_state(&target_library, &target_root);
    assert!(target_library
        .prepare_composite_birth_batch(&[orphan_fixture.input(
            "batch-orphan-target",
            OrganismId(911),
            Tick::new(100),
        )])
        .is_err());
    assert_eq!(
        snapshot_archive_state(&target_library, &target_root),
        orphan_before
    );

    drop(target_library);
    fs::remove_dir_all(source_root).unwrap();
    fs::remove_dir_all(target_root).unwrap();
}

#[test]
fn composite_birth_batch_prepare_rejects_a_wrong_target_indexed_row() {
    let source_root = temp_root("batch-wrong-target-source");
    let target_root = temp_root("batch-wrong-target-index");
    let mut source_library =
        LineageLibrary::open(LineageLibraryConfig::profile_default(&source_root)).unwrap();
    let fixture = generic_composite_fixture(916, SensorProfile::GroundedObjectSlotsV1);
    let source_run_id = "batch-wrong-target-index";
    let manifest_digest = source_library
        .archive_composite_birth(CompositeGeneticArchiveInput {
            source_run_id,
            organism_id: OrganismId(916),
            birth_tick: Tick::new(21),
            creature_genome: &fixture.creature_genome,
            phenotype: &fixture.phenotype,
            foundation_asset_bytes: &fixture.foundation_asset_bytes,
        })
        .unwrap();
    drop(source_library);

    let target_library =
        LineageLibrary::open(LineageLibraryConfig::profile_default(&target_root)).unwrap();
    fs::copy(
        source_root
            .join("manifests")
            .join(format!("{}.json", digest_hex_for_test(manifest_digest))),
        target_root
            .join("manifests")
            .join(format!("{}.json", digest_hex_for_test(manifest_digest))),
    )
    .unwrap();
    copy_tree(&source_root.join("assets"), &target_root.join("assets"));

    let connection = Connection::open(target_root.join("lineage.db")).unwrap();
    connection
        .execute(
            "INSERT INTO manifests(digest,source_run_id,organism_id,genome_id,is_life,death_tick) \
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                digest_hex_for_test(manifest_digest),
                "contradictory-target",
                "999916",
                fixture.creature_genome.id.raw().to_string(),
                0_i64,
                Option::<String>::None,
            ],
        )
        .unwrap();
    drop(connection);

    let before = snapshot_archive_state(&target_library, &target_root);
    let error = target_library
        .prepare_composite_birth_batch(&[fixture.input(
            source_run_id,
            OrganismId(916),
            Tick::new(21),
        )])
        .unwrap_err();
    assert!(error.to_string().contains("index"));
    assert_eq!(
        snapshot_archive_state(&target_library, &target_root),
        before
    );

    drop(target_library);
    fs::remove_dir_all(source_root).unwrap();
    fs::remove_dir_all(target_root).unwrap();
}

#[test]
fn composite_birth_batch_prepare_rejects_corrupted_referenced_manifest_without_writes() {
    let root = temp_root("batch-corrupt-manifest");
    let mut library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let fixture = generic_composite_fixture(917, SensorProfile::GroundedObjectSlotsV1);
    let digest = library
        .archive_composite_birth(CompositeGeneticArchiveInput {
            source_run_id: "batch-corrupt-manifest",
            organism_id: OrganismId(917),
            birth_tick: Tick::new(22),
            creature_genome: &fixture.creature_genome,
            phenotype: &fixture.phenotype,
            foundation_asset_bytes: &fixture.foundation_asset_bytes,
        })
        .unwrap();
    let manifest_path = root
        .join("manifests")
        .join(format!("{}.json", digest_hex_for_test(digest)));
    fs::write(&manifest_path, b"corrupted manifest").unwrap();
    let before = snapshot_archive_state(&library, &root);

    let error = library
        .prepare_composite_birth_batch(&[fixture.input(
            "batch-corrupt-manifest",
            OrganismId(917),
            Tick::new(22),
        )])
        .unwrap_err();
    assert!(error.to_string().contains("digest") || error.to_string().contains("JSON"));
    assert_eq!(snapshot_archive_state(&library, &root), before);

    drop(library);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn composite_birth_batch_prepare_rejects_corrupted_referenced_cas_without_writes() {
    let root = temp_root("batch-corrupt-cas");
    let mut library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let fixture = generic_composite_fixture(918, SensorProfile::GroundedObjectSlotsV1);
    let digest = library
        .archive_composite_birth(CompositeGeneticArchiveInput {
            source_run_id: "batch-corrupt-cas",
            organism_id: OrganismId(918),
            birth_tick: Tick::new(23),
            creature_genome: &fixture.creature_genome,
            phenotype: &fixture.phenotype,
            foundation_asset_bytes: &fixture.foundation_asset_bytes,
        })
        .unwrap();
    let foundation_digest =
        Blake3Digest::from_bytes(*blake3::hash(&fixture.foundation_asset_bytes).as_bytes());
    let cas_path = root
        .join("assets")
        .join(digest_hex_for_test(foundation_digest))
        .join("payload.bin");
    fs::write(&cas_path, b"corrupted foundation bytes").unwrap();
    let before = snapshot_archive_state(&library, &root);

    let error = library
        .prepare_composite_birth_batch(&[fixture.input(
            "batch-corrupt-cas",
            OrganismId(918),
            Tick::new(23),
        )])
        .unwrap_err();
    assert!(error.to_string().contains("digest") || error.to_string().contains("foundation"));
    assert_eq!(snapshot_archive_state(&library, &root), before);
    assert!(digest.bytes().iter().any(|byte| *byte != 0));

    drop(library);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn composite_birth_batch_prepare_rejects_wrong_content_destination_collision_without_writes() {
    let root = temp_root("batch-destination-collision");
    let library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let fixture = generic_composite_fixture(919, SensorProfile::GroundedObjectSlotsV1);
    let foundation_digest =
        Blake3Digest::from_bytes(*blake3::hash(&fixture.foundation_asset_bytes).as_bytes());
    let destination = root
        .join("assets")
        .join(digest_hex_for_test(foundation_digest));
    fs::create_dir_all(&destination).unwrap();
    let payload = destination.join("payload.bin");
    fs::write(&payload, b"wrong content at expected CAS path").unwrap();
    let before = snapshot_archive_state(&library, &root);

    let error = library
        .prepare_composite_birth_batch(&[fixture.input(
            "batch-destination-collision",
            OrganismId(919),
            Tick::new(24),
        )])
        .unwrap_err();
    assert!(error.to_string().contains("collision"));
    assert_eq!(snapshot_archive_state(&library, &root), before);
    assert_eq!(
        fs::read(payload).unwrap(),
        b"wrong content at expected CAS path"
    );

    drop(library);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn composite_birth_batch_prepare_rejects_an_aggregate_byte_limit_overrun_without_writes() {
    let root = temp_root("batch-byte-limit");
    let library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let fixture = generic_composite_fixture(920, SensorProfile::GroundedObjectSlotsV1);
    let oversized_len = usize::try_from(MAX_COMPOSITE_BIRTH_BATCH_BYTES)
        .unwrap()
        .checked_add(1)
        .unwrap();
    let oversized_foundation = vec![0_u8; oversized_len];
    let mut input = fixture.input("batch-byte-limit", OrganismId(920), Tick::new(25));
    input.foundation_asset_bytes = &oversized_foundation;
    let before = snapshot_archive_state(&library, &root);

    let error = library.prepare_composite_birth_batch(&[input]).unwrap_err();
    assert!(error.to_string().contains("aggregate"));
    assert_eq!(snapshot_archive_state(&library, &root), before);

    drop(library);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn composite_birth_batch_prepare_rejects_archive_reparse_descendant_without_writes() {
    let root = temp_root("batch-reparse-descendant");
    let outside = temp_root("batch-reparse-outside");
    let library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let fixture = generic_composite_fixture(921, SensorProfile::GroundedObjectSlotsV1);
    fs::create_dir_all(&outside).unwrap();
    let foundation_digest =
        Blake3Digest::from_bytes(*blake3::hash(&fixture.foundation_asset_bytes).as_bytes());
    let link = root
        .join("assets")
        .join(digest_hex_for_test(foundation_digest));

    #[cfg(windows)]
    let link_created = match std::os::windows::fs::symlink_dir(&outside, &link) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("symlink test skipped: {error}");
            false
        }
    };
    #[cfg(unix)]
    let link_created = match std::os::unix::fs::symlink(&outside, &link) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("symlink test skipped: {error}");
            false
        }
    };
    #[cfg(not(any(windows, unix)))]
    let link_created = false;

    if link_created {
        fs::write(outside.join("payload.bin"), &fixture.foundation_asset_bytes).unwrap();
        let before = snapshot_archive_state(&library, &root);
        let error = library
            .prepare_composite_birth_batch(&[fixture.input(
                "batch-reparse-descendant",
                OrganismId(921),
                Tick::new(26),
            )])
            .unwrap_err();
        assert!(error.to_string().contains("reparse") || error.to_string().contains("symbolic"));
        assert_eq!(snapshot_archive_state(&library, &root), before);
        assert_eq!(
            fs::read(outside.join("payload.bin")).unwrap(),
            fixture.foundation_asset_bytes
        );
    }

    drop(library);
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

#[test]
fn composite_birth_batch_prepare_accepts_only_exact_existing_birth_idempotently() {
    let root = temp_root("batch-idempotent");
    let mut library = LineageLibrary::open(LineageLibraryConfig::profile_default(&root)).unwrap();
    let fixture = generic_composite_fixture(912, SensorProfile::GroundedObjectSlotsV1);
    let existing = library
        .archive_composite_birth(CompositeGeneticArchiveInput {
            source_run_id: "batch-idempotent",
            organism_id: OrganismId(912),
            birth_tick: Tick::new(18),
            creature_genome: &fixture.creature_genome,
            phenotype: &fixture.phenotype,
            foundation_asset_bytes: &fixture.foundation_asset_bytes,
        })
        .unwrap();
    let before = snapshot_archive_state(&library, &root);

    let prepared = library
        .prepare_composite_birth_batch(&[fixture.input(
            "batch-idempotent",
            OrganismId(912),
            Tick::new(18),
        )])
        .unwrap();
    assert_eq!(prepared.manifest_digests(), vec![existing]);
    assert_eq!(snapshot_archive_state(&library, &root), before);

    drop(library);
    fs::remove_dir_all(root).unwrap();
}
