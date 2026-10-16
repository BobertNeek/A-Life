//! Allocation traffic for exact CPU recall preparation, not a live-game timing benchmark.
//! Run the same source against the baseline and retained-context alife_core rlibs.
use alife_core::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct Counting;
static ENABLED: AtomicBool = AtomicBool::new(false);
static CALLS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ENABLED.load(Ordering::Relaxed) {
            CALLS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if ENABLED.load(Ordering::Relaxed) {
            CALLS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if ENABLED.load(Ordering::Relaxed) {
            CALLS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(size, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;
fn counted<T>(run: impl FnOnce() -> T) -> (T, usize, usize) {
    CALLS.store(0, Ordering::Relaxed);
    BYTES.store(0, Ordering::Relaxed);
    ENABLED.store(true, Ordering::Relaxed);
    let result = run();
    ENABLED.store(false, Ordering::Relaxed);
    (
        result,
        CALLS.load(Ordering::Relaxed),
        BYTES.load(Ordering::Relaxed),
    )
}
fn fixture(count: usize) -> (MemoryBank, PerceptionFrameDraft) {
    let owner = OrganismId(811);
    let tick = Tick::new(40);
    let profile = SensorProfile::PrivilegedAffordanceV1;
    let candidates = (0..count)
        .map(|index| {
            ActionCandidate::new(
                index as u16,
                ActionId(200 + index as u32),
                ActionKind::Idle,
                CandidateActionFamily::Idle,
                CandidateObservationRef::None,
                ActionTarget::NONE,
                CandidateFeatureVector::zero(),
                Confidence::new(0.9).unwrap(),
                NormalizedScalar::new(0.2).unwrap(),
                DurationTicks::new(1),
                DurationTicks::new(2),
            )
            .unwrap()
        })
        .collect();
    let draft = PerceptionFrameDraft::new(
        owner,
        tick,
        profile,
        SensorySnapshot::new(
            owner,
            tick,
            Vec3f::ZERO,
            SensoryChannels::ZERO,
            Default::default(),
        )
        .unwrap(),
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::new(
            tick,
            DriveSnapshot::baseline(),
            EndocrineSnapshot::baseline(),
        )
        .unwrap(),
        candidates,
        SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, tick).unwrap(),
        Vec::new(),
    )
    .unwrap();
    let bank = MemoryBank::new(
        MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    (bank, draft)
}

fn main() {
    let label = std::env::args().nth(1).expect("baseline or retained label");
    for count in [1, 3, MAX_ACTION_CANDIDATES] {
        let (bank, draft) = fixture(count);
        let (_, cache) = bank
            .recall_frame_runtime(&draft, &Default::default(), true)
            .unwrap();
        let cache = cache.unwrap();
        for mode in ["audited", "runtime_cold", "runtime_cached"] {
            let mut expected = None;
            let mut measured = None;
            for _ in 0..16 {
                // Captured inputs and immutable fixture/cache setup are outside counting.
                let owned = draft.clone();
                let cognitive = CognitiveContextFrame::empty(
                    owned.organism_id(),
                    ExperienceSequenceId(101),
                    owned.tick(),
                )
                .unwrap();
                let (result, calls, bytes) = counted(|| {
                    let recall = match mode {
                        "audited" => bank.recall_frame(&owned).unwrap(),
                        "runtime_cold" => {
                            bank.recall_frame_runtime(&owned, &Default::default(), true)
                                .unwrap()
                                .0
                        }
                        _ => bank.recall_frame_runtime(&owned, &cache, false).unwrap().0,
                    };
                    recall
                        .with_cognitive_context(cognitive)
                        .unwrap()
                        .finalize(owned)
                        .unwrap()
                });
                result.1.validate_for_frame(&result.0).unwrap();
                let proof = (
                    result.0.frame_digest().0,
                    result
                        .1
                        .candidate_keys()
                        .iter()
                        .map(|key| key.canonical_digest())
                        .collect::<Vec<_>>(),
                );
                if let Some(ref prior) = expected {
                    assert_eq!(prior, &proof);
                } else {
                    expected = Some(proof);
                }
                if let Some(prior) = measured {
                    assert_eq!(prior, (calls, bytes));
                } else {
                    measured = Some((calls, bytes));
                }
            }
            let (calls, bytes) = measured.unwrap();
            let proof = expected.unwrap();
            println!("{{\"variant\":\"{}\",\"mode\":\"{}\",\"candidates\":{},\"alloc_realloc_calls\":{},\"requested_bytes\":{},\"frame_digest\":{:?},\"candidate_key_digests\":{:?}}}", label, mode, count, calls, bytes, proof.0, proof.1);
        }
        let recall = bank.recall_frame_runtime(&draft, &cache, false).unwrap().0;
        let (_, calls, bytes) = counted(|| recall.clone());
        println!("{{\"variant\":\"{}\",\"mode\":\"clone_runtime_prepared\",\"candidates\":{},\"alloc_realloc_calls\":{},\"requested_bytes\":{}}}", label, count, calls, bytes);
    }
}
