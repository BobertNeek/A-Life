//! Private raw framed witness sink. No compression, truncation or dropped bytes.
use std::io::Write;
use std::sync::{Mutex, OnceLock};
static FILE: OnceLock<Mutex<(std::fs::File, u64)>> = OnceLock::new();
pub(crate) fn write_frame(bytes: &[u8]) {
    let cap: u64 = std::env::var("DUAL_WITNESS_CAP").unwrap().parse().unwrap();
    let file = FILE.get_or_init(|| {
        let path = std::env::var("DUAL_WITNESS_PATH").unwrap();
        Mutex::new((std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(), 0))
    });
    let mut state = file.lock().unwrap();
    let next = state.1.checked_add(8).unwrap().checked_add(bytes.len() as u64).unwrap();
    assert!(next <= cap, "prospective complete raw witness cap exceeded");
    state.0.write_all(&(bytes.len() as u64).to_le_bytes()).unwrap();
    state.0.write_all(bytes).unwrap();
    state.0.sync_all().unwrap();
    state.1 = next;
    eprintln!("DUAL_WITNESS frame_bytes={} cumulative_framed_bytes={next}", bytes.len());
}
