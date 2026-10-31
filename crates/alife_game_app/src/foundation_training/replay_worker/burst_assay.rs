//! Typed-record burst overhead only. No snapshot conversion, production memory
//! guard/cache, GPU work or concurrent collector workload is exercised here.
use super::*;
use std::time::Instant;

fn verify_rows(
    root: &Path,
    finished: &FinishedReplayWindow,
    rows: &[FoundationReplayRecord],
    phenotype: &alife_core::BrainPhenotype,
) {
    assert_eq!(finished.references.len(), rows.len());
    assert_eq!(finished.commit.records, rows.len());
    assert_eq!(finished.commit.data_sync_calls, 1);
    assert_eq!(finished.commit.durable_cohort_commits, 1);
    let source = &rows[0].source;
    let mut reader = cohort_replay::CohortReplayReader::open(root, source).unwrap();
    for (reference, expected) in finished.references.iter().zip(rows) {
        // Authenticate actual storage and decode each row. The optional decoded
        // record cache is explicitly refused, rather than hiding external I/O.
        let actual = load_foundation_replay_record_from_storage(
            &root.join("individual"),
            reference,
            source,
            phenotype,
            Some(&mut reader),
            |_| false,
        )
        .unwrap();
        assert_eq!(actual.source, *source);
        assert_eq!(
            bincode::serde::encode_to_vec(&actual, bincode::config::standard()).unwrap(),
            bincode::serde::encode_to_vec(expected, bincode::config::standard()).unwrap(),
        );
    }
    let mut foreign = source.clone();
    foreign.policy_version += 1;
    assert!(cohort_replay::CohortReplayReader::open(root, &foreign).is_err());
}

#[test]
#[ignore = "manual typed-record queue/validation/codec/I/O burst risk assay; not training throughput"]
fn replay_worker_burst_risk_assay() {
    let count = std::env::var("ALIFE_REPLAY_BURST_ROWS")
        .ok()
        .map_or(32, |value| value.parse::<usize>().unwrap());
    assert!(matches!(count, 32 | 64));
    let fixture = Fixture::new();
    let (first, phenotype) = super::super::super::replay_io_tests::record_fixture_at(0);
    let source = first.source.clone();
    let mut rows = vec![first];
    for index in 1..count {
        // Existing CPU-world/synthetic-neural typed-record fixture only. Each
        // row's setup is outside timers; these are not a captured neural life.
        let (mut row, _) = super::super::super::replay_io_tests::record_fixture_at(index as u64);
        row.source = source.clone();
        row.record_index = index as u64;
        row.segment = 0;
        rows.push(row);
    }
    let mut input_digest = blake3::Hasher::new();
    let mut uncompressed_bytes = 0;
    for row in &rows {
        row.validate(&source, &phenotype).unwrap();
        let bytes = bincode::serde::encode_to_vec(row, bincode::config::standard()).unwrap();
        uncompressed_bytes += bytes.len();
        input_digest.update(&bytes);
    }
    let input_digest = input_digest.finalize().to_hex().to_string();
    // Persistent session worker, matching its production lifetime. Creation and
    // all fixture/owned-list cloning are excluded from every timed interval.
    let mut worker = OrderedReplayWorker::new().unwrap();
    for trial in 0..3 {
        let serial_root = fixture.0.join(format!("serial-{trial}"));
        let worker_root = fixture.0.join(format!("worker-{trial}"));
        let mut serial = Some(EncodedWindow::new(&serial_root, &rows[0], &phenotype));
        worker
            .start(EncodedWindow::new(&worker_root, &rows[0], &phenotype))
            .unwrap();
        let mut serial_rows = Some(rows.clone());
        let mut worker_rows = Some(rows.clone());
        let mut serial_finished = None;
        let mut worker_finished = None;
        let mut seconds = [0.0; 2];
        for asynchronous in if trial % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let started = Instant::now();
            if asynchronous {
                // Include owned append moves, channel backpressure, final
                // drain and durable seal/publication; start was setup above.
                for row in worker_rows.take().unwrap() {
                    worker.append(row).unwrap();
                }
                worker_finished = Some(worker.finish().unwrap());
            } else {
                let mut window = serial.take().unwrap();
                for row in serial_rows.take().unwrap() {
                    window.append(row).unwrap();
                }
                serial_finished = Some(window.seal().unwrap());
            }
            seconds[usize::from(asynchronous)] = started.elapsed().as_secs_f64();
        }
        let serial_finished = serial_finished.unwrap();
        let worker_finished = worker_finished.unwrap();
        // All evidence checks happen after both clocks have closed.
        assert_eq!(serial_finished.references, worker_finished.references);
        assert_eq!(
            serial_finished.commit.index_blake3,
            worker_finished.commit.index_blake3
        );
        let serial_bytes = std::fs::read(serial_root.join("cohort-replay.bundle")).unwrap();
        let worker_bytes = std::fs::read(worker_root.join("cohort-replay.bundle")).unwrap();
        assert_eq!(serial_bytes, worker_bytes);
        verify_rows(&serial_root, &serial_finished, &rows, &phenotype);
        verify_rows(&worker_root, &worker_finished, &rows, &phenotype);
        eprintln!(
            "REPLAY_BURST_TRIAL {}",
            serde_json::json!({
                "trial":trial,"rows":count,"order":if trial%2==0 {"serial,worker"} else {"worker,serial"},
                "serial_seconds":seconds[0],"worker_seconds":seconds[1],"serial_over_worker":seconds[0]/seconds[1],
                "neurons":phenotype.neuron_count(),"synapses":phenotype.synapses().len(),
                "input_uncompressed_bytes":uncompressed_bytes,"input_blake3":input_digest,"source":source,
                "bundle_bytes":serial_finished.commit.bundle_bytes,"bundle_blake3":blake3::hash(&serial_bytes).to_hex().to_string(),
                "index_blake3":serial_finished.commit.index_blake3,"full_refs_and_bundle_equal":true,
                "all_rows_authenticated_decoded_and_bitwise_equal":true,"foreign_source_rejected":true,
                "scope":"owned append/validation/unchanged codec/I/O/drain/durable seal; persistent worker and window start/creation/list clones excluded; no producer work, snapshot conversion, Windows guard/cache, GPU or actual collection overlap"
            })
        );
    }
}
