//! One dated CPU finalization comparison and narrow worker cost screen; no device.
use alife_core::*;
use alife_gpu_backend::{GpuBrainSlot, GpuClassBucketPlan};
use std::{hint::black_box, time::Instant};

#[allow(dead_code, unused_imports)]
mod reference {
    include!(env!("ALIFE_PARALLEL_REFERENCE"));
}

#[repr(C)]
struct Timespec {
    tv_sec: std::os::raw::c_long,
    tv_nsec: std::os::raw::c_long,
}
extern "C" {
    fn clock_gettime(id: std::os::raw::c_int, value: *mut Timespec) -> std::os::raw::c_int;
}
fn cpu_ns() -> u64 {
    let mut value = Timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    assert_eq!(unsafe { clock_gettime(2, &mut value) }, 0);
    value.tv_sec as u64 * 1_000_000_000 + value.tv_nsec as u64
}
fn memory_kib() -> (u64, u64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let read = |key: &str| {
        status
            .lines()
            .find_map(|line| {
                line.strip_prefix(key)
                    .map(|rest| rest.split_whitespace().next().unwrap().parse().unwrap())
            })
            .unwrap()
    };
    (read("VmRSS:"), read("VmHWM:"))
}
struct Input {
    draft: PerceptionFrameDraft,
    bank: MemoryBank,
    topology: TopologySidecar,
    slot: GpuBrainSlot,
}
fn preparation(inputs: &[Input]) -> (Vec<reference::CpuResult>, [u64; 4]) {
    let start = Instant::now();
    let cpu = cpu_ns();
    let outputs = inputs
        .iter()
        .map(|input| {
            reference::full_cpu_preparation(
                &input.draft,
                &input.bank,
                &input.topology,
                &input.slot,
                true,
            )
        })
        .collect();
    let cpu = cpu_ns() - cpu;
    let wall = start.elapsed().as_nanos() as u64;
    let (rss, peak) = memory_kib();
    (outputs, [wall, cpu, rss, peak])
}
fn proof(outputs: &[reference::CpuResult]) -> String {
    let rows: Vec<_> = outputs
        .iter()
        .map(|row| {
            let words: Vec<_> = row
                .6
                .header
                .words()
                .iter()
                .copied()
                .chain(
                    row.6
                        .records
                        .iter()
                        .flat_map(|record| record.words().iter().copied()),
                )
                .chain(
                    row.6
                        .cognitive_records
                        .iter()
                        .flat_map(|record| record.words().iter().copied()),
                )
                .collect();
            (
                &row.0,
                &row.1,
                &row.2,
                &row.3,
                row.4.context(),
                row.4.candidate_keys(),
                row.4.receipt(),
                row.4.cognitive_context(),
                &row.5.dispatch_header_words,
                &row.5.frame_payload_words,
                words,
                row.6.neural_receptor_effects,
            )
        })
        .collect();
    blake3::hash(&serde_json::to_vec(&rows).unwrap()).to_string()
}
fn frame_validation(outputs: &[reference::CpuResult], parallel: bool) -> [u64; 2] {
    let validate =
        |part: &[reference::CpuResult]| part.iter().map(|row| row.3.validate()).collect::<Vec<_>>();
    let start = Instant::now();
    let cpu = cpu_ns();
    let results = if parallel {
        std::thread::scope(|scope| {
            let (left, right) = outputs.split_at(outputs.len().div_ceil(2));
            let worker = scope.spawn(move || validate(right));
            let mut result = validate(left);
            result.extend(worker.join().unwrap());
            result
        })
    } else {
        validate(outputs)
    };
    let cpu = cpu_ns() - cpu;
    let wall = start.elapsed().as_nanos() as u64;
    assert_eq!(results.len(), outputs.len());
    assert!(results.iter().all(Result::is_ok));
    black_box(results);
    [wall, cpu]
}
fn main() {
    assert!(!cfg!(debug_assertions));
    let asset = FoundationWeightAsset::decode_canonical(include_bytes!(
        "../../../assets/founders/scaled-choice-nociceptive-v1/candidate.alife-foundation"
    ))
    .unwrap();
    let m = asset.manifest();
    let identity = FoundationGeneticIdentity::new(
        m.foundation_id().raw(),
        m.foundation_version().raw() as u16,
        m.compatibility_family_id().raw(),
        m.capacity_class_id(),
    )
    .unwrap();
    let extension = CognitiveChannelExtensionV1::try_new_v1(
        identity,
        ActionCandidateCreditProfileV1::SignedChoiceReadouts,
    )
    .unwrap();
    let candidate =
        Nano512ActionCreditCandidateV2::new_with_cognitive_extension(&asset, extension).unwrap();
    let (phenotype, _) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&candidate).unwrap();
    let reverse = std::env::args().any(|arg| arg == "--reverse");
    for candidates in [2, 26] {
        for records in [0, 64, 256] {
            for count in [8, 16, 32, 50] {
                let mut bucket =
                    GpuClassBucketPlan::new(BrainCapacityClass::n512(), count).unwrap();
                let inputs: Vec<_> = (0..count)
                    .map(|index| {
                        let (draft, bank) =
                            reference::input(OrganismId(811 + index as u64), candidates, records);
                        assert_eq!(bank.len(), records);
                        let topology = reference::topology_fixture(&draft);
                        let slot = bucket.insert_phenotype(index, 1, &phenotype).unwrap();
                        assert_eq!(slot.decoder_input_stride(), 54);
                        Input {
                            draft,
                            bank,
                            topology,
                            slot,
                        }
                    })
                    .collect();
                let digest = || {
                    blake3::hash(
                        &serde_json::to_vec(
                            &inputs
                                .iter()
                                .map(|i| (&i.draft, &i.bank, &i.topology))
                                .collect::<Vec<_>>(),
                        )
                        .unwrap(),
                    )
                    .to_string()
                };
                let input_digest = digest();
                let expected = preparation(&inputs).0;
                let output_digest = proof(&expected);
                for (index, row) in expected.iter().enumerate() {
                    assert_eq!(row.3.organism_id(), OrganismId(811 + index as u64));
                    row.4.validate_for_frame(&row.3).unwrap();
                }
                // Attribute one retained public finalized-validator pass, outside preparation.
                let mut validation = Vec::new();
                let mut samples = Vec::new();
                for _ in 0..9 {
                    let (actual, sample) = preparation(&inputs);
                    assert_eq!(actual, expected);
                    assert_eq!(proof(&actual), output_digest);
                    samples.push(sample);
                    let wall = Instant::now();
                    let cpu = cpu_ns();
                    for row in &actual {
                        row.4.validate_for_frame(&row.3).unwrap();
                    }
                    validation.push([wall.elapsed().as_nanos() as u64, cpu_ns() - cpu]);
                }
                let mut serial = Vec::new();
                let mut parallel = Vec::new();
                frame_validation(&expected, false);
                frame_validation(&expected, true);
                for round in 0..21 {
                    for index in 0..2 {
                        let parallel_mode = (index + round + usize::from(reverse)) % 2 == 1;
                        let sample = frame_validation(&expected, parallel_mode);
                        if parallel_mode {
                            parallel.push(sample);
                        } else {
                            serial.push(sample);
                        }
                    }
                }
                assert_eq!(digest(), input_digest);
                println!(
                    "{}",
                    serde_json::json!({"candidates":candidates,"records":records,
            "instances":count,"reverse":reverse,"input_blake3":input_digest,
            "output_blake3":output_digest,"exact_outputs_equal":true,"inputs_unchanged":true,
            "preparation_samples":samples,"public_finalized_validation_samples":validation,
            "serial_frame_validation_samples":serial,"two_worker_frame_validation_samples":parallel})
                );
            }
        }
    }
}
