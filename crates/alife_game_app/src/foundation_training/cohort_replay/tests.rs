use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "alife-cohort-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn source() -> FoundationReplaySource {
    FoundationReplaySource {
        source_revision: "synthetic durability fixture".into(),
        policy_version: 7,
        actor_checkpoint_digest: alife_core::Blake3Digest::from_bytes([1; 32]),
        foundation_asset_digest: alife_core::Blake3Digest::from_bytes([2; 32]),
        phenotype_hash: alife_core::PhenotypeHash([3; 4]),
        compiler_inputs_digest: [4; 4],
    }
}

#[cfg(windows)]
#[test]
fn resident_continuous_replay_loader_uses_the_individuals_cohort_parent() {
    // Authenticated I/O/codec fixture, not neural execution. A valid zstd
    // stream with an unsupported schema proves the real loader found and read
    // the intended cohort before rejecting the deliberately incomplete record.
    let fixture = Fixture::new();
    let window = fixture.0.join("window-00000000");
    let individual = window.join("individual");
    std::fs::create_dir_all(&individual).unwrap();
    let mut writer = CohortReplayWriter::new(&window, source()).unwrap();
    let raw = bincode::serde::encode_to_vec(1u32, bincode::config::standard()).unwrap();
    let bytes = zstd::stream::encode_all(raw.as_slice(), 3).unwrap();
    let template = FoundationReplayRecordRef {
        record_index: 0,
        segment: 0,
        tick: 1,
        encoded_bytes: 0,
        digest: [0; 32],
        cohort: Some(FoundationCohortReplayLocation {
            offset: 0,
            organism_id: 1,
        }),
    };
    let reference = writer
        .append_encoding(template, |out| {
            out.write_all(&bytes)?;
            Ok((bytes.len() as u64, *blake3::hash(&bytes).as_bytes()))
        })
        .unwrap();
    writer.seal().unwrap();
    let capacity = alife_core::BrainCapacityClass::n2048();
    let genome = alife_core::BrainGenome::scaffold(91305, capacity.id());
    let development = alife_core::DevelopmentState::new(
        genome.id,
        alife_core::Tick::ZERO,
        alife_core::NormalizedScalar::new(1.0).unwrap(),
    );
    let phenotype = alife_core::PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        alife_core::SensorProfile::GroundedTerrainVisionV1,
    )
    .unwrap();
    let refs = [reference];
    let wrong = load_foundation_replay_window(
        &window,
        &refs,
        0,
        &source(),
        &phenotype,
        FoundationReplayBudget::default(),
    )
    .err()
    .expect("the old one-level-high lookup must fail");
    assert_eq!(
        wrong.downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::NotFound
    );
    let correct = load_foundation_replay_window(
        &individual,
        &refs,
        0,
        &source(),
        &phenotype,
        FoundationReplayBudget::default(),
    )
    .err()
    .expect("the incomplete schema fixture must remain rejected");
    assert!(correct
        .to_string()
        .contains("unsupported compact replay schema 1"));
}

#[test]
fn output_budget_replay_stream_refuses_before_disk_growth() {
    let fixture = Fixture::new();
    let budget = alife_storage::OutputBudget::register(&fixture.0, 8).unwrap();
    let mut replay = CohortReplayWriter::new(&fixture.0, source()).unwrap();
    append(&mut replay, 1, 0);
    assert!(replay.file.flush().is_err());
    drop(replay);
    assert_eq!(std::fs::metadata(fixture.0.join(PENDING)).unwrap().len(), 0);
    assert_eq!(budget.usage().unwrap().used, 0);
    assert!(budget.usage().unwrap().high_water <= 8);
    assert!(!fixture.0.join(SEALED).exists());
}

// Opaque bytes exercise storage integrity and crash semantics, not neural math.
fn append(
    writer: &mut CohortReplayWriter,
    organism: u64,
    row: u64,
) -> (FoundationReplayRecordRef, Vec<u8>) {
    let bytes = [
        organism.to_le_bytes(),
        row.to_le_bytes(),
        (-0.0f64).to_bits().to_le_bytes(),
    ]
    .concat();
    let reference = FoundationReplayRecordRef {
        cohort: Some(FoundationCohortReplayLocation {
            offset: writer.bytes,
            organism_id: organism,
        }),
        record_index: row,
        segment: 0,
        tick: row + 1,
        encoded_bytes: 0,
        digest: [0; 32],
    };
    let reference = writer
        .append_encoding(reference, |out| {
            out.write_all(&bytes)?;
            Ok((bytes.len() as u64, *blake3::hash(&bytes).as_bytes()))
        })
        .unwrap();
    (reference, bytes)
}

#[test]
fn fifty_by_seventeen_rows_have_one_sync_and_authenticated_private_offsets() {
    let fixture = Fixture::new();
    let mut writer = CohortReplayWriter::new(&fixture.0, source()).unwrap();
    let mut rows = Vec::new();
    for row in 0..17 {
        for organism in 1..=50 {
            rows.push(append(&mut writer, organism, row));
        }
    }
    assert!(!fixture.0.join(SEALED).exists());
    assert!(matches!(
        inspect_foundation_cohort_replay(&fixture.0, &source()).unwrap(),
        FoundationCohortReplayRecovery::Uncommitted { .. }
    ));
    let commit = writer.seal().unwrap();
    assert_eq!(
        (
            commit.records,
            commit.data_sync_calls,
            commit.durable_cohort_commits
        ),
        (850, 1, 1)
    );
    assert!(!fixture.0.join(PENDING).exists());
    assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), 1);
    let mut reader = CohortReplayReader::open(&fixture.0, &source()).unwrap();
    for (reference, bytes) in &rows {
        assert_eq!(reader.read(reference).unwrap(), *bytes);
    }
    let mut other = rows[0].0.clone();
    other.cohort.as_mut().unwrap().organism_id = 2;
    assert!(reader.read(&other).is_err());
    let mut wrong_source = source();
    wrong_source.policy_version += 1;
    assert!(CohortReplayReader::open(&fixture.0, &wrong_source).is_err());
    assert!(matches!(
        inspect_foundation_cohort_replay(&fixture.0, &source()).unwrap(),
        FoundationCohortReplayRecovery::Committed { records: 850, .. }
    ));
}

#[test]
fn dropped_partial_collection_is_never_readmitted_or_overwritten() {
    let fixture = Fixture::new();
    let mut writer = CohortReplayWriter::new(&fixture.0, source()).unwrap();
    append(&mut writer, 1, 0);
    drop(writer);
    assert!(CohortReplayReader::open(&fixture.0, &source()).is_err());
    assert!(matches!(
        inspect_foundation_cohort_replay(&fixture.0, &source()).unwrap(),
        FoundationCohortReplayRecovery::Uncommitted { .. }
    ));
    let bytes = std::fs::read(fixture.0.join(PENDING)).unwrap();
    assert!(CohortReplayWriter::new(&fixture.0, source()).is_err());
    assert_eq!(std::fs::read(fixture.0.join(PENDING)).unwrap(), bytes);
}

#[test]
fn newly_created_output_ancestry_is_persisted_before_cohort_acknowledgement() {
    let fixture = Fixture::new();
    let root = fixture.0.join("new-output/replay");
    let mut writer = CohortReplayWriter::new(&root, source()).unwrap();
    append(&mut writer, 1, 0);
    writer.seal().unwrap();
    assert!(matches!(
        inspect_foundation_cohort_replay(&root, &source()).unwrap(),
        FoundationCohortReplayRecovery::Committed { records: 1, .. }
    ));
}

#[test]
fn encoding_failure_poison_prevents_a_partial_commit() {
    let fixture = Fixture::new();
    let mut writer = CohortReplayWriter::new(&fixture.0, source()).unwrap();
    let (mut reference, _) = append(&mut writer, 1, 0);
    reference.record_index = 1;
    reference.tick = 2;
    reference.cohort.as_mut().unwrap().offset = writer.bytes;
    assert!(writer
        .append_encoding(reference, |out| {
            out.write_all(b"torn frame")?;
            Err("injected encode failure".into())
        })
        .is_err());
    assert!(writer.seal().is_err());
    assert!(!fixture.0.join(SEALED).exists());
}

#[test]
fn synced_footer_without_publication_is_still_uncommitted() {
    let fixture = Fixture::new();
    let mut writer = CohortReplayWriter::new(&fixture.0, source()).unwrap();
    append(&mut writer, 1, 0);
    assert!(writer
        .seal_with_publish(|pending, sealed| {
            assert!(!sealed.exists());
            let bytes = std::fs::read(pending)?;
            assert_eq!(
                &bytes
                    [bytes.len() - FOOTER_BYTES as usize..bytes.len() - FOOTER_BYTES as usize + 8],
                MAGIC
            );
            Err(std::io::Error::other("injected crash before publication"))
        })
        .is_err());
    assert!(matches!(
        inspect_foundation_cohort_replay(&fixture.0, &source()).unwrap(),
        FoundationCohortReplayRecovery::Uncommitted { .. }
    ));
}

#[test]
fn uncertain_publish_return_is_resolved_by_read_only_recovery() {
    let fixture = Fixture::new();
    let mut writer = CohortReplayWriter::new(&fixture.0, source()).unwrap();
    append(&mut writer, 1, 0);
    assert!(writer
        .seal_with_publish(|pending, sealed| {
            publish_new(pending, sealed)?;
            Err(std::io::Error::other(
                "injected lost publication acknowledgement",
            ))
        })
        .is_err());
    assert!(matches!(
        inspect_foundation_cohort_replay(&fixture.0, &source()).unwrap(),
        FoundationCohortReplayRecovery::Committed { records: 1, .. }
    ));
    assert!(CohortReplayWriter::new(&fixture.0, source()).is_err());
}

#[test]
fn publication_never_replaces_a_foreign_or_accepted_bundle() {
    let fixture = Fixture::new();
    let mut writer = CohortReplayWriter::new(&fixture.0, source()).unwrap();
    append(&mut writer, 1, 0);
    std::fs::write(fixture.0.join(SEALED), b"accepted foreign bytes").unwrap();
    assert!(writer.seal().is_err());
    assert_eq!(
        std::fs::read(fixture.0.join(SEALED)).unwrap(),
        b"accepted foreign bytes"
    );
}

#[test]
fn torn_corrupt_extra_and_reindexed_committed_bytes_fail_closed() {
    for damage in 0..4 {
        let fixture = Fixture::new();
        let mut writer = CohortReplayWriter::new(&fixture.0, source()).unwrap();
        append(&mut writer, 1, 0);
        writer.seal().unwrap();
        let path = fixture.0.join(SEALED);
        let mut bytes = std::fs::read(&path).unwrap();
        match damage {
            0 => {
                bytes.pop();
            }
            1 => bytes[0] ^= 1,
            2 => bytes.push(0),
            3 => {
                let i = bytes.len() - FOOTER_BYTES as usize - 1;
                bytes[i] ^= 1;
            }
            _ => unreachable!(),
        }
        std::fs::write(path, bytes).unwrap();
        assert!(inspect_foundation_cohort_replay(&fixture.0, &source()).is_err());
    }
}

#[test]
fn private_sequence_and_overlap_validation_precedes_commit() {
    for damage in 0..4 {
        let fixture = Fixture::new();
        let mut writer = CohortReplayWriter::new(&fixture.0, source()).unwrap();
        append(&mut writer, 1, 0);
        append(&mut writer, 1, 1);
        match damage {
            0 => writer.index.records[1].cohort.as_mut().unwrap().offset = 0,
            1 => writer.index.records[1].record_index = 0,
            2 => writer.index.records[1].tick = 4,
            3 => writer.index.records[1].cohort.as_mut().unwrap().organism_id = 2,
            _ => unreachable!(),
        }
        assert!(writer.seal().is_err());
        assert!(!fixture.0.join(SEALED).exists());
    }
}
