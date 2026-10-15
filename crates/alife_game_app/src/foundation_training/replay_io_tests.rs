use super::*;
use std::io::{Cursor, Read, Seek};

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
#[ignore = "CPU-only filesystem/codec timing; run separately without concurrent builds"]
fn replay_verified_read_cpu_timing() {
    let raw = bincode::serde::encode_to_vec(sequence(), bincode::config::standard()).unwrap();
    let encoded = zstd::stream::encode_all(raw.as_slice(), 3).unwrap();
    let digest = *blake3::hash(&encoded).as_bytes();
    let path = std::env::temp_dir().join(format!("alife-replay-io-{}.zst", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    std::io::Write::write_all(&mut file, &encoded).unwrap();
    drop(file);
    let load = |new: bool| {
        let mut file = std::fs::File::open(&path).unwrap();
        assert_eq!(file.metadata().unwrap().len(), encoded.len() as u64);
        let decoded = if new {
            let bytes = read_verified_replay_bytes(file, encoded.len() as u64, &digest).unwrap();
            decode(bytes.as_slice())
        } else {
            // Frozen main: hash the entire file, rewind, decode from the file.
            let mut hasher = blake3::Hasher::new();
            let mut buffer = [0; 64 * 1024];
            loop {
                let n = file.read(&mut buffer).unwrap();
                if n == 0 {
                    break;
                }
                hasher.update(&buffer[..n]);
            }
            assert_eq!(*hasher.finalize().as_bytes(), digest);
            file.rewind().unwrap();
            decode(std::io::BufReader::new(file))
        };
        std::hint::black_box(decoded);
    };
    for new in [false, true] {
        load(new);
    }
    for sample in 0..9 {
        for new in if sample % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let started = Instant::now();
            for _ in 0..200 {
                load(new);
            }
            println!(
                "replay_io sample={sample} new={new} iterations=200 encoded_bytes={} elapsed_ns={}",
                encoded.len(),
                started.elapsed().as_nanos()
            );
        }
    }
    std::fs::remove_file(path).unwrap();
}
