//! Explicit CPU timing and separate fresh-process Linux residency observations.
use super::*;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

#[derive(Clone, Copy, Default, serde::Serialize)]
struct Allocations {
    alloc_calls: u64,
    zeroed_calls: u64,
    realloc_calls: u64,
    dealloc_calls: u64,
    requested_bytes: u64,
    released_bytes: u64,
    live_bytes: i64,
    peak_live_bytes: u64,
}
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<Allocations> = const { Cell::new(Allocations {
        alloc_calls:0,zeroed_calls:0,realloc_calls:0,dealloc_calls:0,
        requested_bytes:0,released_bytes:0,live_bytes:0,peak_live_bytes:0,
    }) };
}
fn account(allocated: usize, released: usize, kind: u8) {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCS.try_with(|cell| {
            let mut c = cell.get();
            match kind {
                0 => c.alloc_calls += 1,
                1 => c.zeroed_calls += 1,
                2 => c.realloc_calls += 1,
                _ => c.dealloc_calls += 1,
            }
            c.requested_bytes += allocated as u64;
            c.released_bytes += released as u64;
            c.live_bytes += allocated as i64 - released as i64;
            c.peak_live_bytes = c.peak_live_bytes.max(c.live_bytes.max(0) as u64);
            cell.set(c);
        });
    }
}
struct ObservedSystem;
// SAFETY: All operations forward System's allocation contract unchanged. The
// optional counters use only nonallocating, constant-initialized thread locals.
unsafe impl GlobalAlloc for ObservedSystem {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: GlobalAlloc's caller supplies a valid layout.
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            account(layout.size(), 0, 0);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: GlobalAlloc's caller supplies a valid layout.
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            account(layout.size(), 0, 1);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        account(0, layout.size(), 3);
        // SAFETY: The caller supplies the original pointer/layout pair.
        unsafe { System.dealloc(p, layout) };
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: The caller supplies GlobalAlloc's pointer/layout/size contract.
        let next = unsafe { System.realloc(p, layout, size) };
        if !next.is_null() {
            account(size, layout.size(), 2);
        }
        next
    }
}
// This pass-through wrapper is present only in the CPU-enabled libtest binary.
#[global_allocator]
static ALLOCATOR: ObservedSystem = ObservedSystem;
struct TrackingGuard;
impl Drop for TrackingGuard {
    fn drop(&mut self) {
        TRACK.with(|c| c.set(false));
    }
}
fn tracked<T>(run: impl FnOnce() -> T) -> T {
    assert!(!TRACK.with(Cell::get));
    TRACK.with(|c| c.set(true));
    let guard = TrackingGuard;
    let value = run();
    drop(guard);
    assert!(ALLOCS.with(|c| c.get().live_bytes) >= 0);
    value
}

fn host_operation(
    owner: &Arc<PpoTrainingWindow>,
    cache: Option<&mut CohortPackedReplay>,
    step: u32,
    previous: &mut Option<super::super::super::ReplayShape>,
    admit: &mut dyn FnMut(u64) -> bool,
) -> u64 {
    let p = phenotype();
    owner.sequence.validate_for(p).unwrap();
    let shape = (
        owner.sequence.ticks.len(),
        owner.sequence.initial.dendrites.clone(),
    );
    // This fixture always alternates lengths. Preserve the real shape comparison
    // and measure the pure metadata helper, excluding all device construction.
    if let Some(previous) = previous.as_ref() {
        assert!(!super::super::super::replay_shapes_equal(previous, &shape));
    }
    let (metadata, layout) = pack_metadata_and_layout(p, Some(&owner.sequence)).unwrap();
    drop(metadata);
    let words = match cache {
        Some(cache) => cache.words(owner, layout, p.neuron_count(), admit),
        None => Cow::Owned(pack_replay_sequence(
            &owner.sequence,
            layout,
            p.neuron_count(),
        )),
    };
    let headers = build_step_headers(p, layout, AdamWConfig::default(), step).unwrap();
    assert_eq!(headers[10], step);
    // Timed paths sample outputs; full-word equality is checked in a separate
    // untimed pass and the varied-input unit tests. No transfer is simulated here.
    let checksum = words.len() as u64
        ^ u64::from(words[layout.context as usize])
        ^ u64::from(words[layout.initial as usize])
        ^ u64::from(headers[10]);
    black_box(&words);
    black_box(&headers);
    *previous = Some(shape);
    checksum
}

fn eight_call_fixture(cache_enabled: bool, owners: &[Arc<PpoTrainingWindow>]) -> u64 {
    let mut cache = CohortPackedReplay::new(2);
    let mut previous = None;
    let mut result = 0;
    for (index, owner_index) in [0, 1, 2, 3, 2, 3, 2, 3].into_iter().enumerate() {
        let step = 19801 + index.saturating_sub(2) as u32 / 2;
        result ^= host_operation(
            &owners[owner_index],
            if cache_enabled && index >= 2 {
                Some(&mut cache)
            } else {
                None
            },
            step,
            &mut previous,
            &mut |_| true,
        );
    }
    black_box(&cache); // Cache is dropped inside the timed scope.
    result
}

#[test]
#[ignore = "bounded adapter-free host timing; no device or runtime integration"]
fn cohort_packed_replay_host_benchmark() {
    let p = phenotype();
    let owners: Vec<_> = [(512, 0), (1, 0), (256, 0), (384, 128)]
        .into_iter()
        .map(|(ticks, burn)| window(sequence(p, ticks, burn)))
        .collect();
    let mut equality_cache = CohortPackedReplay::new(2);
    for index in [0, 1, 2, 3, 2, 3, 2, 3] {
        let owner = &owners[index];
        owner.sequence.validate_for(p).unwrap();
        let (_, layout) = pack_metadata_and_layout(p, Some(&owner.sequence)).unwrap();
        if index >= 2 {
            let expected = pack_replay_sequence(&owner.sequence, layout, p.neuron_count());
            assert_eq!(
                &*equality_cache.words(owner, layout, p.neuron_count(), &mut |_| true),
                expected
            );
        }
    }
    assert_eq!(equality_cache.packets.len(), 2);
    drop(equality_cache);
    for _ in 0..2 {
        assert_eq!(
            eight_call_fixture(false, &owners),
            eight_call_fixture(true, &owners)
        );
    }
    let mut trials = Vec::new();
    for trial in 0..11 {
        for enabled in if trial % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let start = Instant::now();
            let checksum = black_box(eight_call_fixture(enabled, &owners));
            let ns = start.elapsed().as_nanos() as u64;
            trials.push(serde_json::json!({"trial":trial,"cache_enabled":enabled,
                "elapsed_ns":ns,"sample_checksum":checksum}));
        }
    }
    println!(
        "COHORT_TIMING_JSON={}",
        serde_json::json!({
            "assay":"cohort_packed_replay_host_benchmark","device_requested":false,
            "runtime_integration":false,"warmups_per_mode":2,"trials_per_mode":11,
            "call_shapes":[512,1,256,384,256,384,256,384],"epochs":2,
            "cache_dropped_inside_timing":true,"all_calls_validate_fresh":true,
            "full_packet_equality_checked_before_timing":true,
            "fixture_payload_bytes":[85395456,191316,42710016,64052736],"trials":trials,
        })
    );
}

#[cfg(target_os = "linux")]
fn resident_status() -> serde_json::Value {
    let raw = std::fs::read_to_string("/proc/self/status").unwrap();
    let field = |key: &str| {
        raw.lines()
            .find_map(|line| line.strip_prefix(key))
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            * 1024
    };
    let rollup = std::fs::read_to_string("/proc/self/smaps_rollup").unwrap();
    let rollup_field = |key: &str| {
        rollup
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            * 1024
    };
    serde_json::json!({"rss_bytes":field("VmRSS:"),"high_water_rss_bytes":field("VmHWM:"),
        "smaps_rss_bytes":rollup_field("Rss:"),"smaps_pss_bytes":rollup_field("Pss:"),
        "smaps_private_dirty_bytes":rollup_field("Private_Dirty:")})
}

#[cfg(target_os = "linux")]
fn memory_child(mode: &str) -> serde_json::Value {
    let p = phenotype();
    let owners = [window(sequence(p, 256, 0)), window(sequence(p, 384, 128))];
    for owner in &owners {
        drop(pack_metadata_and_layout(p, Some(&owner.sequence)).unwrap());
    }
    let mut cache = CohortPackedReplay::new(2);
    let mut previous = None;
    let mut points = Vec::with_capacity(10);
    ALLOCS.with(|c| c.set(Allocations::default()));
    points.push(
        serde_json::json!({"phase":"fixture_ready","os":resident_status(),
        "allocations":ALLOCS.with(Cell::get),"cache":stats(&cache)}),
    );
    for operation in 0..6 {
        if mode == "cap" && operation == 1 {
            cache.byte_limit = cache.charged_bytes;
        }
        let enabled = mode != "baseline";
        let allow = mode != "refusal" && !(mode == "eviction" && operation >= 2);
        tracked(|| {
            black_box(host_operation(
                &owners[operation % 2],
                if enabled { Some(&mut cache) } else { None },
                19801 + (operation / 2) as u32,
                &mut previous,
                &mut |_| allow,
            ));
        });
        points.push(
            serde_json::json!({"phase":format!("after_operation_{operation}"),
            "ticks":owners[operation % 2].sequence.ticks.len(),"os":resident_status(),
            "allocations":ALLOCS.with(Cell::get),"cache":stats(&cache)}),
        );
    }
    tracked(|| {
        drop(cache);
        drop(previous);
    });
    assert_eq!(ALLOCS.with(|c| c.get().live_bytes), 0);
    points.push(
        serde_json::json!({"phase":"after_cohort_drop","os":resident_status(),
        "allocations":ALLOCS.with(Cell::get)}),
    );
    serde_json::json!({"mode":mode,"process_id":std::process::id(),"points":points,
        "fixture_shapes":[256,384],"fixture_sources_alive_through_final_sample":true,
        "os_high_water_scope":"whole fresh process, including fixture construction",
        "os_sampling_limit":"status and smaps are separate boundary reads; VmRSS/VmHWM accounting can disagree; no strict continuous peak bound is inferred",
        "allocator_scope":"operation allocations only; fixture and /proc sampling excluded",
        "admission":"deterministic test callback; not production Windows admission",
        "device_requested":false,"runtime_integration":false})
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "fresh-process Linux RSS/high-water assay; not production admission"]
fn cohort_packed_replay_memory() {
    if let Ok(mode) = std::env::var("ALIFE_PACKED_REPLAY_MEMORY_MODE") {
        println!("COHORT_MEMORY_JSON={}", memory_child(&mode));
        return;
    }
    let mut children = Vec::new();
    for trial in 0..2 {
        for mode in ["baseline", "cache", "refusal", "eviction", "cap"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "trainer::cohort_packing::tests::experiment::cohort_packed_replay_memory",
                    "--ignored",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env("ALIFE_PACKED_REPLAY_MEMORY_MODE", mode)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let stdout = String::from_utf8(output.stdout).unwrap();
            let line = stdout
                .lines()
                .find_map(|line| line.split_once("COHORT_MEMORY_JSON=").map(|p| p.1))
                .unwrap();
            let mut child: serde_json::Value = serde_json::from_str(line).unwrap();
            child["trial"] = serde_json::json!(trial);
            children.push(child);
        }
    }
    println!(
        "COHORT_MEMORY_SUITE_JSON={}",
        serde_json::json!({
            "assay":"cohort_packed_replay_memory","fresh_process_trials_per_mode":2,"children":children,
        })
    );
}
