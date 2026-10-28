use super::*;
use std::io::{Cursor, Read};

fn continuity_boundary() -> ReplayContinuity {
    ReplayContinuity {
        organism_id: 17,
        slot: 2,
        slot_generation: 3,
        dispatch_generation: 41,
        activation_side: 1,
        weight_generation: 13,
        weight_bank: 1,
        recurrent_digest: [7; 32],
        weight_digest: [11; 32],
        dendrites_digest: [19; 32],
    }
}

#[test]
fn replay_boundary_refuses_recurrent_homeostasis_and_dendritic_resets() {
    // The recurrent digest covers both activations and homeostasis. These are
    // validator fixtures, not evidence of body or GPU trajectory equivalence.
    let previous = continuity_boundary();
    assert!(previous.validate_next(&previous).is_ok());
    let mut reset = previous.clone();
    reset.recurrent_digest = [0; 32];
    assert!(previous.validate_next(&reset).is_err());
    reset = previous.clone();
    reset.dendrites_digest = [0; 32];
    assert!(previous.validate_next(&reset).is_err());
}

#[test]
fn replay_boundary_preserves_recorded_plastic_state_and_allows_new_generations() {
    let previous = continuity_boundary();
    let mut next = previous.clone();
    next.weight_digest = [0; 32];
    assert!(previous.validate_next(&next).is_err());
    next = previous.clone();
    next.weight_bank ^= 1;
    assert!(previous.validate_next(&next).is_err());
    next = previous.clone();
    next.weight_generation -= 1;
    assert!(previous.validate_next(&next).is_err());

    // Ordinary lifetime learning may publish a new bank/generation between
    // decisions. Replay uses that row's recorded offsets; it must not demand
    // that acquired weights stay frozen for an entire collection.
    next = previous.clone();
    next.weight_generation += 1;
    next.weight_bank ^= 1;
    next.weight_digest = [23; 32];
    assert!(previous.validate_next(&next).is_ok());
}

#[test]
fn replay_boundary_requires_the_same_individual_and_recurrent_dispatch() {
    let previous = continuity_boundary();
    let mut changed = Vec::new();
    let mut next = previous.clone();
    next.organism_id += 1;
    changed.push(next);
    let mut next = previous.clone();
    next.slot += 1;
    changed.push(next);
    let mut next = previous.clone();
    next.slot_generation += 1;
    changed.push(next);
    let mut next = previous.clone();
    next.dispatch_generation += 1;
    changed.push(next);
    let mut next = previous.clone();
    next.activation_side ^= 1;
    changed.push(next);
    for next in changed {
        assert!(previous.validate_next(&next).is_err());
    }
}

#[test]
fn replay_boundary_validation_does_not_mutate_either_recorded_state() {
    let previous = continuity_boundary();
    let previous_before = previous.clone();
    let mut next = previous.clone();
    let next_before = next.clone();
    previous.validate_next(&next).unwrap();
    assert_eq!(previous, previous_before);
    assert_eq!(next, next_before);
    next.recurrent_digest = [0; 32];
    let invalid_before = next.clone();
    assert!(previous.validate_next(&next).is_err());
    assert_eq!(previous, previous_before);
    assert_eq!(next, invalid_before);
}

#[test]
fn verified_encoding_checks_all_bytes_and_bounds_the_read() {
    let bytes = b"complete compressed encoding";
    let digest = *blake3::hash(bytes).as_bytes();
    assert_eq!(
        read_verified_replay_bytes(bytes.as_slice(), bytes.len() as u64, &digest).unwrap(),
        bytes
    );
    for changed in [
        bytes[..bytes.len() - 1].to_vec(),
        [bytes.as_slice(), b"extra"].concat(),
        {
            let mut changed = bytes.to_vec();
            *changed.last_mut().unwrap() ^= 1;
            changed
        },
    ] {
        assert!(
            read_verified_replay_bytes(changed.as_slice(), bytes.len() as u64, &digest).is_err()
        );
    }
    let mut reader = Cursor::new(vec![1; 1024]);
    assert!(read_verified_replay_bytes(&mut reader, 10, &[0; 32]).is_err());
    assert_eq!(
        reader.position(),
        11,
        "growth is bounded before hashing or decoding"
    );
    for size in [0, FOUNDATION_REPLAY_RECORD_LIMIT_BYTES + 1] {
        reader.set_position(0);
        assert!(read_verified_replay_bytes(&mut reader, size, &[0; 32]).is_err());
        assert_eq!(reader.position(), 0);
    }
}

#[test]
fn verified_encoding_propagates_io_failure_instead_of_partial_data() {
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected replay read failure"))
        }
    }
    assert!(read_verified_replay_bytes(Broken, 20, &[0; 32])
        .unwrap_err()
        .to_string()
        .contains("injected replay read failure"));
}

// Deliberately synthetic codec/I/O fixture, not a captured episode or cognition
// benchmark. Match the production neuron/weight dimensions and mixed float bits.
fn sequence() -> TrainingSequence {
    let mut state = 123456789_u32;
    let offsets = (0..32768)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state as i32 as f32) / i32::MAX as f32
        })
        .collect();
    TrainingSequence {
        phenotype_hash: alife_core::PhenotypeHash([17; 4]),
        initial: alife_training::TrainingInitialState {
            activations: vec![-0.0; 2048],
            activity_ema: vec![0.5; 2048],
            metabolic_load: vec![0.75; 2048],
            dendrites: Default::default(),
        },
        ticks: vec![alife_training::TrainingReplayTick {
            encoded_inputs: vec![0.125; 2048],
            projection_gain: 0.5,
            local_threshold_shift: -0.25,
            microstep_count: 1,
            enabled_routes: vec![true, false],
            effective_weight_offsets: offsets,
            structural_synapses: Vec::new(),
            candidates: vec![alife_training::TrainingReplayCandidate {
                family: alife_core::CandidateActionFamily::Idle,
                decoder_inputs: [-0.0; 54],
                innate_bias: -0.0,
            }],
        }],
        burn_in_ticks: 0,
        memory_candidate_gain: 0.5,
    }
}

fn decode(reader: impl Read) -> TrainingSequence {
    let decoder = zstd::stream::read::Decoder::new(reader).unwrap();
    bincode::serde::decode_from_std_read(
        &mut decoder.take(FOUNDATION_REPLAY_RECORD_LIMIT_BYTES + 1),
        bincode::config::standard().with_limit::<268435456>(),
    )
    .unwrap()
}

#[test]
fn verified_encoding_preserves_every_replay_float_bit_and_rejects_late_corruption() {
    let original = sequence();
    let raw = bincode::serde::encode_to_vec(&original, bincode::config::standard()).unwrap();
    let mut encoded = zstd::stream::encode_all(raw.as_slice(), 3).unwrap();
    let digest = *blake3::hash(&encoded).as_bytes();
    let verified =
        read_verified_replay_bytes(encoded.as_slice(), encoded.len() as u64, &digest).unwrap();
    let decoded = decode(verified.as_slice());
    assert_eq!(decoded, original);
    assert_eq!(
        bincode::serde::encode_to_vec(decoded, bincode::config::standard()).unwrap(),
        raw
    );
    *encoded.last_mut().unwrap() ^= 1;
    assert!(read_verified_replay_bytes(encoded.as_slice(), encoded.len() as u64, &digest).is_err());
}

#[test]
fn dendritic_branches_digest_matches_canonical_json_blake3() {
    let empty = alife_core::DendriticBranchSet::default();
    let expected = *blake3::hash(&serde_json::to_vec(&empty).unwrap()).as_bytes();
    assert_eq!(dendritic_branches_digest(&empty).unwrap(), expected);
    assert_eq!(dendritic_branches_digest(&empty).unwrap(), expected);
}

#[test]
fn dendritic_digest_cache_preserves_signed_zero_in_every_float_lane() {
    let branches = |threshold, gain, weight| {
        alife_core::DendriticBranchSet::new(vec![alife_core::DendriticBranch::new(
            0,
            threshold,
            gain,
            vec![alife_core::DendriticInputRef::new(1, weight).unwrap()],
        )
        .unwrap()])
        .unwrap()
    };
    let original = branches(0.0, 0.0, 0.0);
    for changed in [
        branches(-0.0, 0.0, 0.0),
        branches(0.0, -0.0, 0.0),
        branches(0.0, 0.0, -0.0),
    ] {
        assert_eq!(
            original, changed,
            "ordinary float equality conflates the inputs"
        );
        let original_digest = dendritic_branches_digest(&original).unwrap();
        let expected = *blake3::hash(&serde_json::to_vec(&changed).unwrap()).as_bytes();
        assert_ne!(original_digest, expected);
        assert_eq!(dendritic_branches_digest(&changed).unwrap(), expected);
        assert_eq!(dendritic_branches_digest(&changed).unwrap(), expected);
    }
}

#[test]
fn replay_slice_hash_preserves_explicit_little_endian_bits_and_boundaries() {
    let words = [0_u32, 0x8000_0000, 0x7fc0_0041, 0x3f80_0000, u32::MAX];
    let mut expected = blake3::Hasher::new();
    for word in words {
        expected.update(&word.to_le_bytes());
    }
    for split in 0..=words.len() {
        let mut actual = blake3::Hasher::new();
        hash_replay_words_le(&mut actual, &words[..split]);
        hash_replay_words_le(&mut actual, &words[split..]);
        assert_eq!(actual.finalize(), expected.finalize());
    }
}

#[test]
fn replay_cache_bounds_decoded_charge_replaces_entries_and_evicts_lru() {
    let key = |id| ([id; 32], 10);
    let mut cache = ReplayRecordCache::new(10, 3);
    cache.insert(key(1), vec![1], 4);
    cache.insert(key(2), vec![2], 4);
    assert_eq!(cache.get(&key(1)), Some(vec![1]));
    cache.insert(key(3), vec![3], 4);
    assert_eq!(cache.charged_bytes, 8);
    assert_eq!(cache.get(&key(2)), None);
    cache.insert(key(1), vec![11], 2);
    assert_eq!(cache.charged_bytes, 6);
    assert_eq!(cache.get(&key(1)), Some(vec![11]));
    cache.insert(key(4), vec![4], 11);
    assert_eq!(
        cache.charged_bytes, 6,
        "oversize values never enter the cache"
    );
    assert_eq!(cache.get(&key(4)), None);
    cache.insert(key(4), vec![4], 1);
    cache.insert(key(5), vec![5], 1);
    assert_eq!(cache.entries.len(), 3);
    assert_eq!(
        cache.get(&key(3)),
        None,
        "entry count is bounded independently"
    );
    assert!(cache.charged_bytes <= cache.byte_limit);
}

#[test]
fn replay_cache_charge_is_uncompressed_and_rejects_oversize_without_allocating() {
    use std::io::Write;
    let mut writer = ReplayCacheChargeWriter(0);
    let block = [0; 1024];
    for _ in 0..VERIFIED_REPLAY_CACHE_CHARGE_LIMIT / 8 / block.len() as u64 {
        writer.write_all(&block).unwrap();
    }
    assert_eq!(writer.0 * 8, VERIFIED_REPLAY_CACHE_CHARGE_LIMIT);
    assert!(writer.write_all(&[0]).is_err());
    assert_eq!(writer.0 * 8, VERIFIED_REPLAY_CACHE_CHARGE_LIMIT);
}

// Synthetic GPU-receipt fields over a measured CPU-world patch exercise the
// codec/admission contract, not actual GPU behavior or cognition performance.
fn record_fixture() -> (FoundationReplayRecord, alife_core::BrainPhenotype) {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(42, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let phenotype = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedTerrainVisionV1,
    )
    .unwrap();
    let mut world = super::held_food_lesson_tests::unconfigured(42);
    let scenario = configure_foundation_scenario(
        &mut world,
        42,
        Some(FoundationTeacherLesson::GrabFood),
        None,
        false,
    )
    .unwrap();
    let frame = super::held_food_lesson_tests::frame(&mut world);
    let (_, outcome) = super::held_food_lesson_tests::eat_outcome(&mut world, scenario.food);
    let patch = super::held_food_lesson_tests::eating_narration_patch(
        frame.clone(),
        scenario.food,
        outcome,
    );
    let mut sequence = sequence();
    sequence.phenotype_hash = phenotype.phenotype_hash();
    sequence.memory_candidate_gain = phenotype
        .candidate_decoder()
        .memory_channel()
        .map_or(0.0, |p| p.max_candidate_gain());
    sequence.ticks[0].microstep_count = u32::from(phenotype.microstep_count());
    sequence.ticks[0].enabled_routes = vec![true; phenotype.projections().len()];
    sequence.ticks[0].candidates = frame
        .candidates()
        .iter()
        .map(|candidate| {
            let mut decoder_inputs = [0.0; 54];
            decoder_inputs[..24].copy_from_slice(&candidate.features.0);
            TrainingReplayCandidate {
                family: candidate.family,
                decoder_inputs,
                innate_bias: 0.0,
            }
        })
        .collect();
    let source = FoundationReplaySource {
        source_revision: format!(
            "synthetic-cache-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ),
        policy_version: 7,
        actor_checkpoint_digest: alife_core::Blake3Digest::from_bytes([1; 32]),
        foundation_asset_digest: alife_core::Blake3Digest::from_bytes([2; 32]),
        phenotype_hash: phenotype.phenotype_hash(),
        compiler_inputs_digest: phenotype.compiler_inputs_digest(),
    };
    let mut before = continuity_boundary();
    before.organism_id = frame.organism_id().raw();
    let mut recurrent = blake3::Hasher::new();
    for value in &sequence.initial.activations {
        recurrent.update(&value.to_bits().to_le_bytes());
    }
    for (activity, metabolic) in sequence
        .initial
        .activity_ema
        .iter()
        .zip(&sequence.initial.metabolic_load)
    {
        recurrent.update(&activity.to_bits().to_le_bytes());
        recurrent.update(&metabolic.to_bits().to_le_bytes());
    }
    before.recurrent_digest = *recurrent.finalize().as_bytes();
    before.dendrites_digest = dendritic_branches_digest(&sequence.initial.dendrites).unwrap();
    let mut after = before.clone();
    after.dispatch_generation += 1;
    after.activation_side ^= sequence.ticks[0].microstep_count as u8 & 1;
    let behavior = alife_gpu_backend::GpuTrainingRolloutReceipt {
        organism_id: before.organism_id,
        tick: frame.tick().raw(),
        dispatch_generation: after.dispatch_generation,
        frame_digest: frame.frame_digest(),
        active_weight_generation: before.weight_generation,
        sampling: GpuTrainingSamplingConfig {
            seed: 1,
            counter: 0,
            temperature: 1.0,
            demonstrator: None,
        },
        logits: vec![0.0; frame.candidates().len()],
        decoder_inputs: sequence.ticks[0]
            .candidates
            .iter()
            .flat_map(|c| c.decoder_inputs[..24].iter().copied())
            .collect(),
        decoder_input_stride: 24,
        representative_mask: 1,
        motor_masks: [0; 6],
        forced_motor_slots: vec![u8::MAX; frame.candidates().len()],
        representative_index: 0,
        motor_indices: [u16::MAX; 6],
        factor_log_probabilities: [0.0; 7],
        behavior: alife_gpu_backend::GpuTrainingBehaviorKind::OnPolicy,
        joint_log_probability: Some(0.0),
        policy_joint_log_probability: 0.0,
    };
    let record = FoundationReplayRecord {
        schema: FOUNDATION_REPLAY_SCHEMA,
        source,
        record_index: 0,
        segment: 0,
        tick: frame.tick().raw(),
        organism_id: before.organism_id,
        sequence,
        behavior,
        patch,
        before,
        after,
    };
    record.validate(&record.source, &phenotype).unwrap();
    (record, phenotype)
}

#[test]
fn warm_replay_cache_still_checks_external_bytes_source_and_reference() {
    let (record, phenotype) = record_fixture();
    let root = std::env::temp_dir().join(format!(
        "alife-replay-cache-{}",
        record.source.source_revision
    ));
    std::fs::create_dir(&root).unwrap();
    for cohort in [false, true] {
        let directory = root.join(if cohort {
            "cohort/individual"
        } else {
            "standalone"
        });
        std::fs::create_dir_all(&directory).unwrap();
        let (reference, path) = if cohort {
            let mut writer = cohort_replay::CohortReplayWriter::new(
                directory.parent().unwrap(),
                record.source.clone(),
            )
            .unwrap();
            let reference = writer.append(&record).unwrap();
            writer.seal().unwrap();
            (
                reference,
                directory.parent().unwrap().join("cohort-replay.bundle"),
            )
        } else {
            let reference = write_replay_record(&directory, &record).unwrap();
            let path = directory.join(reference.file_name());
            (reference, path)
        };
        let load = |reference: &FoundationReplayRecordRef, source: &FoundationReplaySource| {
            let mut reader = if cohort {
                Some(cohort_replay::CohortReplayReader::open(
                    directory.parent().unwrap(),
                    source,
                )?)
            } else {
                None
            };
            load_foundation_replay_record_from_storage(
                &directory,
                reference,
                source,
                &phenotype,
                reader.as_mut(),
            )
        };
        let original = bincode::serde::encode_to_vec(&record, bincode::config::standard()).unwrap();
        for _ in 0..2 {
            let loaded = load(&reference, &record.source).unwrap();
            assert_eq!(
                bincode::serde::encode_to_vec(loaded, bincode::config::standard()).unwrap(),
                original
            );
        }
        let mut wrong = reference.clone();
        wrong.segment += 1;
        assert!(load(&wrong, &record.source).is_err());
        let mut wrong_source = record.source.clone();
        wrong_source.policy_version += 1;
        assert!(load(&reference, &wrong_source).is_err());
        let bytes = std::fs::read(&path).unwrap();
        let mut changed = bytes.clone();
        changed[reference.encoded_bytes as usize - 1] ^= 1;
        std::fs::write(&path, changed).unwrap();
        assert!(
            load(&reference, &record.source).is_err(),
            "warm decoded cache cannot hide changed disk bytes"
        );
        std::fs::write(&path, &bytes).unwrap();
        let mut changed = bytes.clone();
        changed.push(0);
        std::fs::write(&path, changed).unwrap();
        assert!(
            load(&reference, &record.source).is_err(),
            "warm cache cannot hide changed file length"
        );
        std::fs::write(&path, bytes).unwrap();
        assert!(load(&reference, &record.source).is_ok());
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "manual CPU-only synthetic codec/cache microbenchmark; not training throughput"]
fn benchmark_replay_cache_cpu_only() {
    let (record, phenotype) = record_fixture();
    let root = std::env::temp_dir().join(format!(
        "alife-cache-benchmark-{}",
        record.source.source_revision
    ));
    std::fs::create_dir(&root).unwrap();
    let reference = write_replay_record(&root, &record).unwrap();
    let path = root.join(reference.file_name());
    let load = || {
        load_foundation_replay_record_from_storage(
            &root,
            &reference,
            &record.source,
            &phenotype,
            None,
        )
        .unwrap()
    };
    let baseline = || {
        let bytes = read_verified_replay_bytes(
            std::fs::File::open(&path).unwrap(),
            reference.encoded_bytes,
            &reference.digest,
        )
        .unwrap();
        let decoder = zstd::stream::read::Decoder::new(bytes.as_slice()).unwrap();
        let decoded: FoundationReplayRecord = bincode::serde::decode_from_std_read(
            &mut decoder.take(FOUNDATION_REPLAY_RECORD_LIMIT_BYTES + 1),
            bincode::config::standard().with_limit::<268435456>(),
        )
        .unwrap();
        decoded.validate(&record.source, &phenotype).unwrap();
        decoded
    };
    let exact = bincode::serde::encode_to_vec(&record, bincode::config::standard()).unwrap();
    assert_eq!(
        bincode::serde::encode_to_vec(baseline(), bincode::config::standard()).unwrap(),
        exact
    );
    assert_eq!(
        bincode::serde::encode_to_vec(load(), bincode::config::standard()).unwrap(),
        exact
    );
    let mut timings: [Vec<f64>; 4] = Default::default();
    // Sixteen repetitions of nine passes over one synthetic production-sized
    // record reduce timer noise. Inputs and fixture construction precede timing.
    const LOADS: usize = 16 * 9;
    for trial in 0..5 {
        // Rotate ordering to reduce cache/thermal order effects. All files are
        // cache-warm; this deliberately does not estimate disk cold-start cost.
        for offset in 0..4 {
            let variant = (trial + offset) % 4;
            let started = std::time::Instant::now();
            for _ in 0..LOADS {
                match variant {
                    0 => {
                        std::hint::black_box(baseline());
                    }
                    1 => {
                        std::hint::black_box(load());
                    }
                    2 => {
                        std::hint::black_box(record.sequence.clone());
                    }
                    _ => cache_verified_replay_record(&reference, record.clone()),
                }
            }
            timings[variant].push(started.elapsed().as_secs_f64());
        }
    }
    for values in &mut timings {
        values.sort_by(f64::total_cmp);
    }
    // Linux-only observation, not a replacement for the production Windows
    // memory guard or proof of a physical-memory cap for other records.
    let rss_kib = || -> Option<u64> {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        status
            .lines()
            .find(|line| line.starts_with("VmRSS:"))?
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()
    };
    let mut writer = ReplayCacheChargeWriter(0);
    bincode::serde::encode_into_std_write(&record, &mut writer, bincode::config::standard())
        .unwrap();
    let charge = writer.0 * 8 + std::mem::size_of::<FoundationReplayRecord>() as u64;
    let before_rss = rss_kib();
    let mut retained = ReplayRecordCache::new(
        VERIFIED_REPLAY_CACHE_CHARGE_LIMIT,
        VERIFIED_REPLAY_CACHE_ENTRIES,
    );
    for index in 0_u64..128 {
        let mut key = [0; 32];
        key[..8].copy_from_slice(&index.to_le_bytes());
        retained.insert((key, 1), record.clone(), charge);
    }
    let retained_rss = rss_kib();
    assert_eq!(retained.entries.len(), 128);
    eprintln!("synthetic_cpu_cache_benchmark encoded_bytes={} loads_per_trial={} trials=5 baseline_decode_validate_median_s={:.6} verified_record_hit_median_s={:.6} admitted_sequence_clone_median_s={:.6} record_clone_charge_and_insert_median_s={:.6} record_retention_charge_bytes={} retained_records=128 aggregate_retention_charge_bytes={} linux_rss_kib_before={:?} linux_rss_kib_retained={:?} bitwise_wire_equal=true",
        reference.encoded_bytes, LOADS, timings[0][2], timings[1][2], timings[2][2], timings[3][2], charge, retained.charged_bytes, before_rss, retained_rss);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn replay_cache_can_retain_production_sized_synthetic_burn_in_overlap() {
    let (record, _) = record_fixture();
    let mut writer = ReplayCacheChargeWriter(0);
    bincode::serde::encode_into_std_write(&record, &mut writer, bincode::config::standard())
        .unwrap();
    let charge = writer.0 * 8 + std::mem::size_of::<FoundationReplayRecord>() as u64;
    let mut cache = ReplayRecordCache::new(
        VERIFIED_REPLAY_CACHE_CHARGE_LIMIT,
        VERIFIED_REPLAY_CACHE_ENTRIES,
    );
    for index in 0_u64..128 {
        let mut key = [0; 32];
        key[..8].copy_from_slice(&index.to_le_bytes());
        cache.insert((key, 1), record.clone(), charge);
    }
    assert_eq!(
        cache.entries.len(),
        128,
        "retain ordinary burn-in overlap on this bounded fixture"
    );
    assert_eq!(cache.charged_bytes, charge * 128);
    assert!(cache.charged_bytes <= VERIFIED_REPLAY_CACHE_CHARGE_LIMIT);
}
