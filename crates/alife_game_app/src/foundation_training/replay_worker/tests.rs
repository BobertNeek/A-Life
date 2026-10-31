use super::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "alife-replay-worker-{}-{}",
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

// Exercise the actual record validator, unchanged codec and durable cohort on
// this CPU-only fixture. It does not claim GPU capture or neural equivalence.
struct EncodedWindow {
    cohort: CohortReplayWriter,
    source: FoundationReplaySource,
    phenotype: alife_core::BrainPhenotype,
    references: Vec<FoundationReplayRecordRef>,
}
impl EncodedWindow {
    fn new(
        root: &Path,
        record: &FoundationReplayRecord,
        phenotype: &alife_core::BrainPhenotype,
    ) -> Self {
        Self {
            cohort: CohortReplayWriter::new(root, record.source.clone()).unwrap(),
            source: record.source.clone(),
            phenotype: phenotype.clone(),
            references: Vec::new(),
        }
    }
}
impl ReplayWindow for EncodedWindow {
    type Capture = FoundationReplayRecord;
    type Finished = FinishedReplayWindow;
    fn append(&mut self, capture: Self::Capture) -> WorkerResult<()> {
        capture
            .validate(&self.source, &self.phenotype)
            .map_err(|e| e.to_string())?;
        self.references
            .push(self.cohort.append(&capture).map_err(|e| e.to_string())?);
        Ok(())
    }
    fn seal(self) -> WorkerResult<Self::Finished> {
        Ok(FinishedReplayWindow {
            references: self.references,
            commit: self.cohort.seal().map_err(|e| e.to_string())?,
        })
    }
}

fn records() -> (Vec<FoundationReplayRecord>, alife_core::BrainPhenotype) {
    let (first, phenotype) = super::super::replay_io_tests::record_fixture_at(0);
    let mut records = vec![first];
    for (index, tick) in [1, 4, 5].into_iter().enumerate() {
        let (mut row, _) = super::super::replay_io_tests::record_fixture_at(tick);
        row.source = records[0].source.clone();
        row.record_index = index as u64 + 1;
        row.segment = u64::from(tick >= 4);
        records.push(row);
    }
    (records, phenotype)
}

#[test]
fn worker_preserves_ordered_refs_bundle_bytes_segments_and_corruption_checks() {
    let fixture = Fixture::new();
    let (rows, phenotype) = records();
    let serial_root = fixture.0.join("serial");
    let mut serial = EncodedWindow::new(&serial_root, &rows[0], &phenotype);
    for row in &rows {
        serial.append(row.clone()).unwrap();
    }
    let expected = serial.seal().unwrap();
    let mut worker = OrderedReplayWorker::new().unwrap();
    // Reuse the same worker for two complete windows to catch retained sources
    // or references accidentally crossing a policy/cohort boundary.
    for index in 0..2 {
        let root = fixture.0.join(format!("threaded-{index}"));
        worker
            .start(EncodedWindow::new(&root, &rows[0], &phenotype))
            .unwrap();
        for row in &rows {
            worker.append(row.clone()).unwrap();
        }
        let actual = worker.finish().unwrap();
        assert_eq!(actual.references, expected.references);
        assert_eq!(actual.commit.index_blake3, expected.commit.index_blake3);
        assert_eq!(actual.commit.data_sync_calls, 1);
        let expected_bytes = std::fs::read(serial_root.join("cohort-replay.bundle")).unwrap();
        let path = root.join("cohort-replay.bundle");
        assert_eq!(std::fs::read(&path).unwrap(), expected_bytes);
        let mut reader = cohort_replay::CohortReplayReader::open(&root, &rows[0].source).unwrap();
        for reference in &actual.references {
            reader.read(reference).unwrap();
        }
        let mut corrupted = expected_bytes;
        corrupted[actual.references[0].encoded_bytes as usize - 1] ^= 1;
        std::fs::write(&path, corrupted).unwrap();
        assert!(reader.read(&actual.references[0]).is_err());
    }
    eprintln!(
        "worker object sizes: capture={} command={} window={}",
        std::mem::size_of::<Capture>(),
        std::mem::size_of::<Command<CapturedReplayWindow>>(),
        std::mem::size_of::<CapturedReplayWindow>()
    );
}

#[test]
fn validation_failure_and_disk_budget_failure_never_publish_partial_cohorts() {
    let fixture = Fixture::new();
    let (mut rows, phenotype) = records();
    let record = rows.remove(0);
    let root = fixture.0.join("invalid");
    let mut worker = OrderedReplayWorker::new().unwrap();
    worker
        .start(EncodedWindow::new(&root, &record, &phenotype))
        .unwrap();
    worker.append(record.clone()).unwrap();
    let mut changed = rows.remove(0);
    changed.before.recurrent_digest[0] ^= 1;
    // The asynchronous failure may be observed either at enqueue's final poll
    // or at finish; neither path can return authenticated, sealed references.
    let _ = worker.append(changed);
    assert!(worker.finish().is_err());
    assert!(!root.join("cohort-replay.bundle").exists());
    assert!(std::fs::metadata(root.join("cohort-replay.pending")).unwrap().len() > 0);

    let root = fixture.0.join("budget");
    std::fs::create_dir(&root).unwrap();
    let budget = alife_storage::OutputBudget::register(&root, 8).unwrap();
    worker
        .start(EncodedWindow::new(&root, &record, &phenotype))
        .unwrap();
    let _ = worker.append(record);
    assert!(worker.finish().is_err());
    assert!(!root.join("cohort-replay.bundle").exists());
    assert!(budget.usage().unwrap().high_water <= 8);
}

struct ControlledWindow {
    began: mpsc::SyncSender<()>,
    release: mpsc::Receiver<()>,
    visited: Arc<AtomicUsize>,
    panic: bool,
}
impl ReplayWindow for ControlledWindow {
    type Capture = usize;
    type Finished = usize;
    fn append(&mut self, _: usize) -> WorkerResult<()> {
        self.began.send(()).unwrap();
        self.release.recv().unwrap();
        assert!(!self.panic, "injected replay worker panic");
        self.visited.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    fn seal(self) -> WorkerResult<usize> {
        Ok(self.visited.load(Ordering::Relaxed))
    }
}

#[test]
fn one_queued_capture_bounds_retention_and_finish_drains_every_capture() {
    let (began, seen) = mpsc::sync_channel(2);
    let (release, released) = mpsc::channel();
    let visited = Arc::new(AtomicUsize::new(0));
    let mut worker = OrderedReplayWorker::new().unwrap();
    worker
        .start(ControlledWindow {
            began,
            release: released,
            visited: visited.clone(),
            panic: false,
        })
        .unwrap();
    worker.append(0).unwrap();
    seen.recv().unwrap(); // first capture is held in-flight
    worker.append(1).unwrap(); // exactly one additional capture may queue
    assert!(matches!(
        worker.commands.try_send(Command::Append(2)),
        Err(mpsc::TrySendError::Full(_))
    ));
    release.send(()).unwrap();
    release.send(()).unwrap();
    assert_eq!(worker.finish().unwrap(), 2);
    assert_eq!(visited.load(Ordering::Relaxed), 2);
}

#[test]
fn discard_acknowledges_file_drop_and_drop_joins_all_pending_work() {
    let fixture = Fixture::new();
    let (record, phenotype) = super::super::replay_io_tests::record_fixture();
    let root = fixture.0.join("discard");
    let mut worker = OrderedReplayWorker::new().unwrap();
    worker
        .start(EncodedWindow::new(&root, &record, &phenotype))
        .unwrap();
    worker.append(record.clone()).unwrap();
    worker.discard().unwrap();
    assert!(!root.join("cohort-replay.bundle").exists());
    std::fs::remove_dir_all(&root).unwrap();
    let root = fixture.0.join("drop");
    worker
        .start(EncodedWindow::new(&root, &record, &phenotype))
        .unwrap();
    worker.append(record).unwrap();
    drop(worker);
    assert!(!root.join("cohort-replay.bundle").exists());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn worker_panic_is_reported_and_cannot_produce_a_successful_finish() {
    let (began, seen) = mpsc::sync_channel(1);
    let (release, released) = mpsc::channel();
    let mut worker = OrderedReplayWorker::new().unwrap();
    worker
        .start(ControlledWindow {
            began,
            release: released,
            visited: Arc::new(AtomicUsize::new(0)),
            panic: true,
        })
        .unwrap();
    worker.append(0).unwrap();
    seen.recv().unwrap();
    release.send(()).unwrap();
    let error = worker.finish().err().unwrap().to_string();
    assert!(error.contains("replay writer panicked"), "{error}");
}

/// The CPU oracle above begins at typed records. This separate gate exercises
/// the complete capture conversion, cross-row continuity and cache/TLS path.
/// It creates only a disposable procedural individual, never a real checkpoint.
#[cfg(all(windows, feature = "gpu-tests"))]
#[test]
#[ignore = "requires admitted hardware GPU and Windows process-memory admission"]
fn captured_steps_match_serial_full_writer_including_explicit_segments() {
    let fixture = Fixture::new();
    let world = super::super::held_food_lesson_tests::unconfigured(42);
    let mut runtime = GpuLiveBrainRuntime::new_profiled_foundation_training(
        GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap(),
        world,
        42,
        BrainScaleTier::Standard2048,
        SensorProfile::GroundedTerrainVisionV1,
        alife_archive::LineageLibraryConfig::profile_default(fixture.0.join("lineage")),
        "disposable-replay-worker-qualification",
        alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
        GpuTrainingSamplingConfig {
            seed: 42,
            counter: 0,
            temperature: 1.0,
            demonstrator: None,
        },
    )
    .unwrap();
    let mut steps = Vec::new();
    for _ in 0..32 {
        runtime.tick_outcome().unwrap();
        assert!(runtime.last_memory_preparation_errors().is_empty());
        assert!(runtime.last_post_seal_learning_failures().is_empty());
        steps.extend(runtime.take_foundation_training_steps());
        if steps.len() >= 4 {
            break;
        }
    }
    assert_eq!(steps.len(), 4);
    let phenotype = steps[0].before.phenotype.as_ref().clone();
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&phenotype).unwrap();
    let source = foundation_replay_source(&phenotype, &asset, 7, b"disposable fixture").unwrap();
    let build = |root: &Path| {
        let cohort = CohortReplayWriter::new(root, source.clone()).unwrap();
        let writer = FoundationReplayWriter::new(
            &root.join("individual"),
            source.clone(),
            phenotype.clone(),
            &asset,
            FoundationReplayBudget::default(),
        )
        .unwrap();
        (writer, cohort)
    };
    let serial_root = fixture.0.join("serial-full");
    let (mut serial, mut cohort) = build(&serial_root);
    let mut expected = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        if index == 2 {
            // Exercise explicit segment restart without fabricating missing
            // decisions, body state or sleep in this disposable capture trace.
            serial.start_segment().unwrap();
        }
        expected.push(serial.append_cohort(step, &mut cohort).unwrap());
    }
    cohort.seal().unwrap();
    drop(serial);
    let threaded_root = fixture.0.join("threaded-full");
    let (writer, cohort) = build(&threaded_root);
    let mut worker = FoundationReplayWorker::new().unwrap();
    worker.start(writer, cohort).unwrap();
    for (index, step) in steps.into_iter().enumerate() {
        worker.append(step, index == 2).unwrap();
    }
    assert_eq!(worker.finish().unwrap().references, expected);
    assert_eq!(
        std::fs::read(serial_root.join("cohort-replay.bundle")).unwrap(),
        std::fs::read(threaded_root.join("cohort-replay.bundle")).unwrap()
    );
}
