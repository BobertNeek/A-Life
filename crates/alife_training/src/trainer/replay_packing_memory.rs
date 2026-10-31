//! Separate fresh-process one-call allocation and Linux residency observations.
use super::*;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

#[derive(Clone, Copy, Default, serde::Serialize)]
struct Counts {
    alloc: u64,
    zeroed: u64,
    realloc: u64,
    dealloc: u64,
    requested: u64,
    live: i64,
    peak: u64,
}
thread_local! {
    static ACTIVE: Cell<bool> = const {Cell::new(false)};
    static COUNTS: Cell<Counts> = const {Cell::new(Counts {
        alloc:0,zeroed:0,realloc:0,dealloc:0,requested:0,live:0,peak:0,
    })};
}
fn record(new: usize, old: usize, kind: u8) {
    if ACTIVE.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNTS.try_with(|cell| {
            let mut c = cell.get();
            match kind {
                0 => c.alloc += 1,
                1 => c.zeroed += 1,
                2 => c.realloc += 1,
                _ => c.dealloc += 1,
            }
            c.requested += new as u64;
            c.live += new as i64 - old as i64;
            c.peak = c.peak.max(c.live.max(0) as u64);
            cell.set(c);
        });
    }
}
struct ObservedSystem;
// SAFETY: Forward System's allocation contract unchanged; optional constant
// thread-local counters neither allocate nor dereference allocation contents.
unsafe impl GlobalAlloc for ObservedSystem {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        // SAFETY: GlobalAlloc's caller supplies a valid layout.
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            record(l.size(), 0, 0)
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        // SAFETY: GlobalAlloc's caller supplies a valid layout.
        let p = unsafe { System.alloc_zeroed(l) };
        if !p.is_null() {
            record(l.size(), 0, 1)
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        record(0, l.size(), 3);
        // SAFETY: Caller supplies the original pointer/layout pair.
        unsafe { System.dealloc(p, l) };
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        // SAFETY: Caller supplies GlobalAlloc's pointer/layout/size contract.
        let q = unsafe { System.realloc(p, l, n) };
        if !q.is_null() {
            record(n, l.size(), 2)
        }
        q
    }
}
// CPU-enabled libtest only; normal libraries and applications retain System.
#[global_allocator]
static ALLOCATOR: ObservedSystem = ObservedSystem;
struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.with(|c| c.set(false));
    }
}
fn tracked<T>(f: impl FnOnce() -> T) -> T {
    assert!(!ACTIVE.with(Cell::get));
    ACTIVE.with(|c| c.set(true));
    let guard = Guard;
    let value = f();
    drop(guard);
    value
}

#[cfg(target_os = "linux")]
fn rss() -> serde_json::Value {
    fn value(s: &str, key: &str) -> u64 {
        s.lines()
            .find_map(|line| line.strip_prefix(key))
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            * 1024
    }
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let smaps = std::fs::read_to_string("/proc/self/smaps_rollup").unwrap();
    serde_json::json!({"status_rss":value(&status,"VmRSS:"),"reported_hwm":value(&status,"VmHWM:"),
        "smaps_rss":value(&smaps,"Rss:"),"smaps_pss":value(&smaps,"Pss:"),
        "smaps_private_dirty":value(&smaps,"Private_Dirty:")})
}

#[cfg(target_os = "linux")]
fn child(mode: &str, ticks: usize) -> serde_json::Value {
    use sha2::{Digest, Sha256};
    let p = phenotype();
    let q = sequence(p, ticks, if ticks == 384 { 128 } else { 0 });
    q.validate_for(p).unwrap();
    let (_, layout) = pack_metadata_and_layout(p, Some(&q)).unwrap();
    let before = rss();
    COUNTS.with(|c| c.set(Counts::default()));
    let packer = black_box(if mode == "candidate" {
        pack_replay_sequence as Packer
    } else {
        reference_pack as Packer
    });
    let words = tracked(|| black_box(packer(&q, layout, p.neuron_count())));
    let live = COUNTS.with(Cell::get);
    assert_eq!(live.zeroed, 1);
    assert_eq!(live.alloc, 0);
    assert_eq!(live.realloc, 0);
    assert_eq!(live.peak, layout.training_words * 4);
    assert_eq!(words.len(), words.capacity());
    let held = rss();
    let hash = format!("{:x}", Sha256::digest(bytemuck::cast_slice(&words)));
    tracked(|| drop(words));
    let final_counts = COUNTS.with(Cell::get);
    assert_eq!(final_counts.live, 0);
    serde_json::json!({"mode":mode,"ticks":ticks,"process_id":std::process::id(),
        "before":before,"packet_held":held,"after_drop":rss(),"counts_packet_held":live,
        "counts_after_drop":final_counts,"packet_sha256":hash,"device_requested":false,
        "retained_packet_after_drop":false,"allocation_scope":"packer only; fixture, validation, layout, hash and RSS sampling excluded",
        "os_scope":"whole fresh Linux process; source fixture and metadata remain alive",
        "os_limit":"status/smaps separate samples; VmHWM includes initialization and approximate accounting, not a strict continuous peak bound"})
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "fresh-process one-call memory assay; no device"]
fn single_call_packing_memory() {
    if let Ok(mode) = std::env::var("ALIFE_SINGLE_PACK_MEMORY_MODE") {
        let ticks = std::env::var("ALIFE_SINGLE_PACK_TICKS")
            .unwrap()
            .parse()
            .unwrap();
        println!("SINGLE_PACK_MEMORY_CHILD={}", child(&mode, ticks));
        return;
    }
    let mut children = Vec::new();
    for trial in 0..2 {
        for ticks in [512, 384] {
            for mode in ["baseline", "candidate"] {
                let output = std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "trainer::replay_packing_assay::memory::single_call_packing_memory",
                        "--ignored",
                        "--nocapture",
                        "--test-threads=1",
                    ])
                    .env("ALIFE_SINGLE_PACK_MEMORY_MODE", mode)
                    .env("ALIFE_SINGLE_PACK_TICKS", ticks.to_string())
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
                    .find_map(|line| line.split_once("SINGLE_PACK_MEMORY_CHILD=").map(|p| p.1))
                    .unwrap();
                let mut row: serde_json::Value = serde_json::from_str(line).unwrap();
                row["trial"] = serde_json::json!(trial);
                children.push(row);
            }
        }
    }
    for ticks in [512, 384] {
        let hashes: std::collections::BTreeSet<_> = children
            .iter()
            .filter(|c| c["ticks"] == ticks)
            .map(|c| c["packet_sha256"].as_str().unwrap())
            .collect();
        assert_eq!(hashes.len(), 1);
    }
    println!(
        "SINGLE_PACK_MEMORY_JSON={}",
        serde_json::json!({"assay":"single_call_packing_memory",
        "build_tag":std::env::var("ALIFE_SINGLE_PACK_PROFILE").unwrap_or_default(),
        "fresh_process_trials_per_mode_and_shape":2,"children":children})
    );
}
