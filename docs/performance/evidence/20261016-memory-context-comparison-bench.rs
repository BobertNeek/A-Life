//! Isolated comparison allocation probe. The runner injects the current
//! production comparison function at PRODUCTION_HELPER below.
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

struct Counting;
static ENABLED: AtomicBool = AtomicBool::new(false);
static CALLS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ENABLED.load(Relaxed) {
            CALLS.fetch_add(1, Relaxed);
            BYTES.fetch_add(layout.size(), Relaxed);
        }
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if ENABLED.load(Relaxed) {
            CALLS.fetch_add(1, Relaxed);
            BYTES.fetch_add(size, Relaxed);
        }
        System.realloc(ptr, layout, size)
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;

struct GpuCandidateMemoryRecord {
    target_latent: [f32; 8],
    family_value: [f32; 4],
    target_confidence: f32,
    family_confidence: f32,
    source_counts_packed: u32,
}

// PRODUCTION_HELPER

fn legacy_values(records: &[GpuCandidateMemoryRecord]) -> Vec<f32> {
    let mut reencoded = Vec::with_capacity(records.len() * 16);
    for row in records {
        reencoded.extend_from_slice(&row.target_latent);
        reencoded.extend_from_slice(&row.family_value);
        reencoded.push(row.target_confidence);
        reencoded.push(row.family_confidence);
        reencoded.push((row.source_counts_packed & 0xffff) as f32);
        reencoded.push((row.source_counts_packed >> 16) as f32);
    }
    reencoded
}

fn measure(mut compare: impl FnMut() -> bool) -> (usize, usize) {
    CALLS.store(0, Relaxed);
    BYTES.store(0, Relaxed);
    ENABLED.store(true, Relaxed);
    for _ in 0..100 {
        assert!(black_box(compare()));
    }
    ENABLED.store(false, Relaxed);
    (CALLS.load(Relaxed), BYTES.load(Relaxed))
}

fn main() {
    for count in [0, 1, 2, 32, 256, 1024] {
        let records = (0..count)
            .map(|index| GpuCandidateMemoryRecord {
                target_latent: [index as f32 * 0.125; 8],
                family_value: [-0.25; 4],
                target_confidence: 0.5,
                family_confidence: 0.75,
                source_counts_packed: 17 | (65535 << 16),
            })
            .collect::<Vec<_>>();
        let values = legacy_values(&records);
        let records = black_box(records.as_slice());
        let values = black_box(values.as_slice());
        let legacy = measure(|| legacy_values(records) == values);
        let current = measure(|| memory_records_match_context(records, values));
        assert_eq!(legacy, (if count == 0 { 0 } else { 100 }, count * 64 * 100));
        assert_eq!(current, (0, 0));
        println!("{{\"rows\":{count},\"comparisons\":100,\"legacy_alloc_realloc_calls\":{},\"legacy_requested_bytes\":{},\"current_alloc_realloc_calls\":{},\"current_requested_bytes\":{}}}", legacy.0, legacy.1, current.0, current.1);
    }
}
