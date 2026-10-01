//! Bounded CPU topology attribution; no devices, brain or world stepping.
pub use alife_core::*;
use std::hint::black_box;
use std::time::Instant;

#[allow(dead_code, unused_imports)]
mod fixture {
    include!("../../../crates/alife_core/tests/candidate_memory_retrieval.rs");
    pub fn patch(seq: u64, tracked: u64, count: usize) -> ExperiencePatch {
        let object = slot(0, tracked, 0.4, [0.0, 0.8, 0.9]);
        let tick = Tick::new(seq * 2);
        let draft = PerceptionFrameDraft::new(
            ORGANISM,
            tick,
            SensorProfile::GroundedObjectSlotsV1,
            SensorySnapshot::new(
                ORGANISM,
                tick,
                Vec3f::ZERO,
                SensoryChannels::ZERO,
                Default::default(),
            )
            .unwrap(),
            grounded_draft(0.4).body(),
            HomeostaticSnapshot::baseline(tick),
            (0..count)
                .map(|i| {
                    candidate_for_family(
                        &object,
                        i as u16,
                        if i % 2 == 0 {
                            CandidateActionFamily::Ingest
                        } else {
                            CandidateActionFamily::Avoid
                        },
                    )
                })
                .collect(),
            SensorProfileProvenance::new(
                SensorProfile::GroundedObjectSlotsV1,
                SensoryAbiVersion::CURRENT,
                tick,
            )
            .unwrap(),
            vec![object],
        )
        .unwrap();
        observation_from_draft(&empty_bank(), seq, draft, -0.8, 0.7, |_| {})
    }
}
#[allow(dead_code)]
mod profiled {
    include!(env!("ALIFE_TOPOLOGY_PROFILE_SOURCE"));
}
include!(env!("ALIFE_TOPOLOGY_CONTEXT_HELPER"));

#[repr(C)]
struct ThreadTimespec {
    tv_sec: std::os::raw::c_long,
    tv_nsec: std::os::raw::c_long,
}
extern "C" {
    fn clock_gettime(
        clock_id: std::os::raw::c_int,
        value: *mut ThreadTimespec,
    ) -> std::os::raw::c_int;
}
fn cpu_ns() -> u64 {
    let mut t = ThreadTimespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // Linux CLOCK_THREAD_CPUTIME_ID = 3. No production dependency or device use.
    assert_eq!(unsafe { clock_gettime(3, &mut t) }, 0);
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
fn median(mut values: Vec<u64>) -> u64 {
    values.sort();
    values[values.len() / 2]
}
fn main() {
    assert!(!cfg!(debug_assertions));
    for observations in [8, 64, 256] {
        let first = fixture::patch(1, 1, 1);
        let profile = first.header().sensor_profile.identity();
        let mut base = TopologySidecar::new_profiled(
            OrganismId(811),
            profile,
            TopologicalMapConfig::default(),
        )
        .unwrap();
        for seq in 1..=observations {
            let receipt = base.observe_sealed_patch(&fixture::patch(seq, seq, 1));
            assert!(!receipt.rejected_invalid);
        }
        base.validate_contract().unwrap();
        let encoded = serde_json::to_value(&base).unwrap();
        for candidates in [1, 26] {
            let next = fixture::patch(observations + 1, observations + 1, candidates);
            let draft = next.pre_action().perception();
            let draft = PerceptionFrameDraft::new(
                draft.organism_id(),
                draft.tick(),
                draft.sensor_profile(),
                draft.sensory().clone(),
                draft.body(),
                *draft.homeostasis(),
                draft.candidates().to_vec(),
                draft.profile_provenance(),
                draft.grounded_object_slots().to_vec(),
            )
            .unwrap();
            let recall =
                MemoryBank::new(MemoryBankConfig::new(8, 64, 4, 0.72, Confidence(0.0)).unwrap())
                    .unwrap()
                    .recall_frame(&draft)
                    .unwrap();

            for residents in [8, 16, 32, 50] {
                let mut official_ns = Vec::new();
                let mut profiled_ns = Vec::new();
                let mut stages = Vec::new();
                let mut context_ns = Vec::new();
                let mut official_cpu_ns = Vec::new();
                let mut context_cpu_ns = Vec::new();
                let mut result_json_blake3 = String::new();
                for _ in 0..5 {
                    let mut official = vec![base.clone(); residents];
                    let mut diagnostic: Vec<profiled::TopologySidecar> = (0..residents)
                        .map(|_| serde_json::from_value(encoded.clone()).unwrap())
                        .collect();
                    let start = Instant::now();
                    let cpu_start = cpu_ns();
                    let receipts: Vec<_> = official
                        .iter_mut()
                        .map(|map| map.observe_sealed_patch(&next))
                        .collect();
                    official_cpu_ns.push(cpu_ns() - cpu_start);
                    official_ns.push(start.elapsed().as_nanos() as u64);
                    profiled::profile_reset();
                    let start = Instant::now();
                    let measured: Vec<_> = diagnostic
                        .iter_mut()
                        .map(|map| map.observe_sealed_patch(&next))
                        .collect();
                    profiled_ns.push(start.elapsed().as_nanos() as u64);
                    stages.push(profiled::profile_read());
                    let start = Instant::now();
                    let cpu_start = cpu_ns();
                    for _ in 0..residents {
                        // Two calls model baseline and routed summary preparation;
                        // memory expectancies are rebuilt on every actual call.
                        black_box(
                            cognitive_context_for_recall(
                                OrganismId(811),
                                ExperienceSequenceId(observations + 1),
                                &recall,
                                &base,
                            )
                            .unwrap(),
                        );
                        black_box(
                            cognitive_context_for_recall(
                                OrganismId(811),
                                ExperienceSequenceId(observations + 1),
                                &recall,
                                &base,
                            )
                            .unwrap(),
                        );
                    }
                    context_cpu_ns.push(cpu_ns() - cpu_start);
                    context_ns.push(start.elapsed().as_nanos() as u64);
                    // Proof work and clone/reset work occur outside observation clocks.
                    assert_eq!(
                        serde_json::to_value(&receipts).unwrap(),
                        serde_json::to_value(&measured).unwrap()
                    );
                    assert_eq!(
                        serde_json::to_value(&official).unwrap(),
                        serde_json::to_value(&diagnostic).unwrap()
                    );
                    result_json_blake3 =
                        blake3::hash(&serde_json::to_vec(&(&receipts, &official)).unwrap())
                            .to_string();
                }
                let stage_medians: Vec<_> = (0..6)
                    .map(|i| median(stages.iter().map(|s| s[i]).collect()))
                    .collect();
                println!(
                    "{}",
                    serde_json::json!({"seed_observations":observations,"map_counts":base.counts(),"patch_candidates":candidates,"instances":residents,
                    "official_observe_ns":official_ns,"official_observe_cpu_ns":official_cpu_ns,"instrumented_observe_ns":profiled_ns,"stage_ns":stages,"stage_medians_ns":stage_medians,
                    "context_two_calls_ns":context_ns,"context_two_calls_cpu_ns":context_cpu_ns,"official_instrumented_state_receipt_equal":true,
                    "next_patch_json_blake3":blake3::hash(&serde_json::to_vec(&next).unwrap()).to_string(),
                    "result_json_blake3":result_json_blake3,"map_json_blake3":blake3::hash(&serde_json::to_vec(&base).unwrap()).to_string()})
                );
            }
        }
    }
}
