// One-off CPU-only evidence probe; link against the tested workspace rlibs.
use alife_core::{BrainCapacityClass, PerceptionFrame};
use alife_gpu_backend::{GpuBrainSlot, GpuClassBucketPlan, GpuPerceptionUpload};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::time::Instant;

#[path = "../../../crates/alife_gpu_backend/tests/closed_loop_buffer_contracts/support.rs"]
#[allow(dead_code)]
mod support;

struct CountingAllocator;
static COUNT: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicU64 = AtomicU64::new(0);
static REALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNT.load(Relaxed) {
            ALLOCS.fetch_add(1, Relaxed);
            BYTES.fetch_add(layout.size() as u64, Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if COUNT.load(Relaxed) {
            REALLOCS.fetch_add(1, Relaxed);
            BYTES.fetch_add(size as u64, Relaxed);
        }
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn prepare(frame: &PerceptionFrame, slot: &GpuBrainSlot, row: u32) -> GpuPerceptionUpload {
    // Mirrors the memory binding construction, then batch construction/revalidation.
    let binding_upload = GpuPerceptionUpload::try_from_frame(frame, slot, 0).unwrap();
    black_box(binding_upload.frame_binding);
    drop(binding_upload);
    let mut upload = GpuPerceptionUpload::try_from_frame(frame, slot, 0).unwrap();
    upload.rebase(row * 1024, row * 4096).unwrap();
    upload.validate_against(frame, slot).unwrap();
    black_box(upload)
}

fn main() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = support::compile(capacity.id(), 41);
    let mut bucket = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = bucket.insert_phenotype(0, 7, &phenotype).unwrap();
    let base = support::perception_fixture();
    for candidates in [2_usize, 8, 32] {
        let frame = PerceptionFrame::new(
            base.organism_id(),
            base.tick(),
            base.sensor_profile(),
            base.sensory().clone(),
            base.body(),
            *base.homeostasis(),
            (0..candidates)
                .map(|i| {
                    let mut candidate = base.candidates()[i % base.candidates().len()];
                    candidate.candidate_index = i as u16;
                    candidate
                })
                .collect(),
            base.profile_provenance(),
            Vec::new(),
        )
        .unwrap();
        for _ in 0..200 {
            for row in 0..8 {
                drop(prepare(&frame, &slot, row));
            }
        }
        ALLOCS.store(0, Relaxed);
        REALLOCS.store(0, Relaxed);
        BYTES.store(0, Relaxed);
        COUNT.store(true, Relaxed);
        for _ in 0..100 {
            for row in 0..8 {
                drop(prepare(&frame, &slot, row));
            }
        }
        COUNT.store(false, Relaxed);
        let mut timings = Vec::new();
        for _ in 0..9 {
            let started = Instant::now();
            for _ in 0..1000 {
                for row in 0..8 {
                    drop(prepare(&frame, &slot, row));
                }
            }
            timings.push(started.elapsed().as_nanos() as u64 / 1000);
        }
        let mut digest = blake3::Hasher::new();
        for row in 0..8 {
            let upload = prepare(&frame, &slot, row);
            for word in upload
                .dispatch_header_words
                .iter()
                .chain(&upload.frame_payload_words)
            {
                digest.update(&word.to_le_bytes());
            }
        }
        println!("candidates={candidates} rows=8 allocations={} reallocations={} requested_bytes={} ns_per_cohort={timings:?} payload_blake3={}",
            ALLOCS.load(Relaxed) / 100, REALLOCS.load(Relaxed) / 100,
            BYTES.load(Relaxed) / 100, digest.finalize());
    }
}
