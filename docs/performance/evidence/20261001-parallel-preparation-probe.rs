//! Bounded Linux CPU prototype; no live runtime, device, training or world mutation.
use alife_core::*;
use alife_gpu_backend::{GpuBrainSlot, GpuClassBucketPlan};
use std::hint::black_box;
use std::time::Instant;

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
fn cpu_ns(id: i32) -> u64 {
    let mut time = Timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    assert_eq!(unsafe { clock_gettime(id, &mut time) }, 0);
    time.tv_sec as u64 * 1_000_000_000 + time.tv_nsec as u64
}
fn memory_kib() -> (u64, u64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let read = |key: &str| {
        status
            .lines()
            .find_map(|line| {
                line.strip_prefix(key)
                    .map(|line| line.split_whitespace().next().unwrap().parse().unwrap())
            })
            .unwrap()
    };
    (read("VmRSS:"), read("VmHWM:"))
}

struct Case {
    draft: PerceptionFrameDraft,
    bank: MemoryBank,
    topology: TopologySidecar,
    slot: GpuBrainSlot,
}
struct Admitted<'a> {
    draft: PerceptionFrameDraft,
    bank: &'a MemoryBank,
    topology: &'a TopologySidecar,
    slot: &'a GpuBrainSlot,
}
struct Batch {
    outputs: Vec<reference::CpuResult>,
    wall_ns: u64,
    capture_wall_ns: u64,
    process_cpu_ns: u64,
    summed_worker_cpu_ns: u64,
    rss_before_kib: u64,
    rss_with_outputs_kib: u64,
    process_peak_rss_kib: u64,
}
fn prepare(input: &Admitted<'_>) -> reference::CpuResult {
    reference::full_cpu_preparation(&input.draft, input.bank, input.topology, input.slot, true)
}
fn run(cases: &[Case], workers: usize) -> Batch {
    assert!([1, 2, 4].contains(&workers));
    let rss_before_kib = memory_kib().0;
    let start = Instant::now();
    let process_start = cpu_ns(2); // Linux CLOCK_PROCESS_CPUTIME_ID.
                                   // Owned draft capture and borrowed immutable owner-specific banks/topologies.
    let admitted: Vec<_> = cases
        .iter()
        .map(|case| Admitted {
            draft: case.draft.clone(),
            bank: &case.bank,
            topology: &case.topology,
            slot: &case.slot,
        })
        .collect();
    let capture_wall_ns = start.elapsed().as_nanos() as u64;
    let (outputs, summed_worker_cpu_ns) = if workers == 1 {
        let cpu = cpu_ns(3); // Linux CLOCK_THREAD_CPUTIME_ID.
        let outputs = admitted.iter().map(prepare).collect();
        (outputs, cpu_ns(3) - cpu)
    } else {
        std::thread::scope(|scope| {
            let chunk_len = admitted.len().div_ceil(workers);
            let handles: Vec<_> = admitted
                .chunks(chunk_len)
                .map(|chunk| {
                    scope.spawn(move || {
                        let cpu = cpu_ns(3);
                        let outputs: Vec<_> = chunk.iter().map(prepare).collect();
                        (outputs, cpu_ns(3) - cpu)
                    })
                })
                .collect();
            let mut outputs = Vec::with_capacity(cases.len());
            let mut sum = 0;
            // Joining by input chunk order makes result order deterministic.
            for handle in handles {
                let (chunk, cpu) = handle.join().unwrap();
                outputs.extend(chunk);
                sum += cpu;
            }
            (outputs, sum)
        })
    };
    drop(admitted);
    let process_cpu_ns = cpu_ns(2) - process_start;
    let wall_ns = start.elapsed().as_nanos() as u64;
    let (rss_with_outputs_kib, process_peak_rss_kib) = memory_kib();
    Batch {
        outputs,
        wall_ns,
        capture_wall_ns,
        process_cpu_ns,
        summed_worker_cpu_ns,
        rss_before_kib,
        rss_with_outputs_kib,
        process_peak_rss_kib,
    }
}
fn scheduling_only(rows: usize, workers: usize) -> (u64, u64) {
    let start = Instant::now();
    let cpu = cpu_ns(2);
    let inputs: Vec<_> = (0..rows).collect();
    let output: Vec<_> = if workers == 1 {
        inputs.clone()
    } else {
        std::thread::scope(|scope| {
            let handles: Vec<_> = inputs
                .chunks(rows.div_ceil(workers))
                .map(|chunk| scope.spawn(move || chunk.to_vec()))
                .collect();
            handles
                .into_iter()
                .flat_map(|handle| handle.join().unwrap())
                .collect()
        })
    };
    assert_eq!(output, inputs);
    black_box(output);
    (start.elapsed().as_nanos() as u64, cpu_ns(2) - cpu)
}
fn proof(outputs: &[reference::CpuResult]) -> String {
    let proof: Vec<_> = outputs
        .iter()
        .map(|row| {
            let memory_words: Vec<_> = row
                .6
                .header
                .words()
                .iter()
                .copied()
                .chain(row.6.records.iter().flat_map(|r| r.words().iter().copied()))
                .chain(
                    row.6
                        .cognitive_records
                        .iter()
                        .flat_map(|r| r.words().iter().copied()),
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
                memory_words,
                row.6.neural_receptor_effects,
            )
        })
        .collect();
    blake3::hash(&serde_json::to_vec(&proof).unwrap()).to_string()
}
fn main() {
    assert!(!cfg!(debug_assertions));
    let asset = FoundationWeightAsset::decode_canonical(include_bytes!(
        "../../../assets/founders/scaled-choice-nociceptive-v1/candidate.alife-foundation"
    ))
    .unwrap();
    let manifest = asset.manifest();
    let identity = FoundationGeneticIdentity::new(
        manifest.foundation_id().raw(),
        manifest.foundation_version().raw() as u16,
        manifest.compatibility_family_id().raw(),
        manifest.capacity_class_id(),
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
    let reverse = std::env::args().any(|a| a == "--reverse");
    for candidates in [2, 26] {
        for records in [0, 64, 256] {
            for residents in [8, 16, 32, 50] {
                let mut bucket =
                    GpuClassBucketPlan::new(BrainCapacityClass::n512(), residents as u32).unwrap();
                let cases: Vec<_> = (0..residents)
                    .map(|index| {
                        let owner = OrganismId(811 + index as u64);
                        let (draft, bank) = reference::input(owner, candidates, records);
                        assert_eq!(bank.len(), records);
                        let topology = reference::topology_fixture(&draft);
                        let slot = bucket
                            .insert_phenotype(index as u32, 1, &phenotype)
                            .unwrap();
                        assert_eq!(slot.decoder_input_stride(), 54);
                        Case {
                            draft,
                            bank,
                            topology,
                            slot,
                        }
                    })
                    .collect();
                // Static input ownership/compiler Sync checks are exercised by run().
                let input_json: Vec<_> = cases
                    .iter()
                    .map(|c| (&c.draft, &c.bank, &c.topology))
                    .collect();
                let input_bytes = serde_json::to_vec(&input_json).unwrap();
                let input_hash = blake3::hash(&input_bytes).to_string();
                let input_json_bytes = input_bytes.len();
                drop(input_bytes);
                let expected = run(&cases, 1).outputs;
                let expected_hash = proof(&expected);
                for (index, row) in expected.iter().enumerate() {
                    assert_eq!(row.3.organism_id(), OrganismId(811 + index as u64));
                }
                let mut samples: [Vec<serde_json::Value>; 3] = std::array::from_fn(|_| Vec::new());
                let modes = if reverse { [4, 2, 1] } else { [1, 2, 4] };
                // One warmup for each exact work mode; all complete outputs agree.
                for workers in modes {
                    assert_eq!(run(&cases, workers).outputs, expected);
                }
                for round in 0..5 {
                    for index in 0..3 {
                        let workers = modes[(round + index) % 3];
                        let batch = run(&cases, workers);
                        assert_eq!(batch.outputs, expected);
                        assert_eq!(proof(&batch.outputs), expected_hash);
                        let (empty_wall_ns, empty_cpu_ns) = scheduling_only(residents, workers);
                        samples[match workers {1=>0,2=>1,_=>2}].push(serde_json::json!({
                            "wall_ns":batch.wall_ns,"capture_wall_ns":batch.capture_wall_ns,
                            "process_cpu_ns":batch.process_cpu_ns,"summed_worker_cpu_ns":batch.summed_worker_cpu_ns,
                            "rss_before_kib":batch.rss_before_kib,"rss_with_outputs_kib":batch.rss_with_outputs_kib,
                            "process_peak_rss_kib":batch.process_peak_rss_kib,
                            "scheduling_only_wall_ns":empty_wall_ns,"scheduling_only_process_cpu_ns":empty_cpu_ns
                        }));
                    }
                }
                assert_eq!(
                    blake3::hash(&serde_json::to_vec(&input_json).unwrap()).to_string(),
                    input_hash
                );
                // Direct public validator attribution, with fresh recall captured outside clocks.
                let recalls: Vec<_> = cases
                    .iter()
                    .map(|c| c.bank.recall_frame(&c.draft).unwrap())
                    .collect();
                let mut validation_ns = Vec::new();
                for _ in 0..5 {
                    let start = Instant::now();
                    for (case, recall) in cases.iter().zip(&recalls) {
                        recall.validate_for_draft(&case.draft).unwrap();
                    }
                    validation_ns.push(start.elapsed().as_nanos() as u64);
                }
                println!(
                    "{}",
                    serde_json::json!({"candidates":candidates,"records":records,
                    "instances":residents,"owner_range":[811,810+residents],"reverse":reverse,
                    "available_parallelism":std::thread::available_parallelism().unwrap().get(),
                    "input_json_bytes":input_json_bytes,"input_blake3":input_hash,
                    "result_blake3":expected_hash,"complete_results_equal":true,"input_unchanged":true,
                    "serial":samples[0],"two_workers":samples[1],"four_workers":samples[2],
                    "fresh_recall_public_validation_wall_ns":validation_ns})
                );
            }
        }
    }
}
