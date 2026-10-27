use super::*;
use std::sync::{atomic::AtomicU64, Barrier};

static SEQUENCE: AtomicU64 = AtomicU64::new(1);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "alife-output-budget-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn rejects_growth_before_the_underlying_write_and_releases_failed_staging() {
    let f = Fixture::new();
    let b = OutputBudget::register(&f.0, 4).unwrap();
    let path = f.0.join("payload");
    let file = File::create(&path).unwrap();
    let mut w = writer(&path, file).unwrap();
    assert_eq!(
        w.write(b"12345").unwrap_err().kind(),
        io::ErrorKind::StorageFull
    );
    assert_eq!(fs::metadata(&path).unwrap().len(), 0);
    assert_eq!(b.usage().unwrap().used, 0);
    w.write_all(b"1234").unwrap();
    drop(w);
    assert_eq!(b.usage().unwrap().used, 4);
    remove_file(&path).unwrap();
    assert_eq!(b.usage().unwrap().used, 0);
    write(&path, b"next").unwrap();
    assert!(b.usage().unwrap().high_water <= 4);
}

#[test]
fn concurrent_writers_share_one_cap() {
    let f = Fixture::new();
    let b = OutputBudget::register(&f.0, 8).unwrap();
    let gate = Arc::new(Barrier::new(3));
    let jobs = (0..2)
        .map(|i| {
            let path = f.0.join(format!("worker-{i}"));
            let gate = gate.clone();
            std::thread::spawn(move || {
                let file = File::create(&path).unwrap();
                let mut w = writer(path, file).unwrap();
                gate.wait();
                w.write_all(b"12345678")
            })
        })
        .collect::<Vec<_>>();
    gate.wait();
    let success = jobs
        .into_iter()
        .map(|j| j.join().unwrap().is_ok())
        .filter(|ok| *ok)
        .count();
    assert_eq!(success, 1);
    assert_eq!(b.usage().unwrap().used, 8);
    assert!(b.usage().unwrap().high_water <= 8);
}

#[test]
fn retired_other_scope_does_not_break_an_independent_writer() {
    let retired = Fixture::new();
    let a = OutputBudget::register(&retired.0, 4).unwrap();
    let live = Fixture::new();
    let b = OutputBudget::register(&live.0, 4).unwrap();
    remove_dir_all(&retired.0).unwrap();
    write(live.0.join("payload"), b"1234").unwrap();
    assert_eq!(a.usage().unwrap().used, 0);
    assert_eq!(b.usage().unwrap().used, 4);
}

#[test]
fn retained_worker_handle_survives_initiating_handle_drop() {
    let f = Fixture::new();
    let b = OutputBudget::register(&f.0, 8).unwrap();
    let path = f.0.join("worker");
    let worker = budget_for(&path).unwrap().unwrap();
    drop(b);
    let worker = std::thread::spawn(move || {
        write(path, b"12345678").unwrap();
        worker
    })
    .join()
    .unwrap();
    assert_eq!(worker.usage().unwrap().used, 8);
}

#[test]
fn hard_link_staging_is_counted_and_duplicate_publication_does_not_charge_again() {
    let f = Fixture::new();
    let b = OutputBudget::register(&f.0, 12).unwrap();
    let staged = f.0.join("staged");
    let final_path = f.0.join("final");
    write(&staged, b"123456").unwrap();
    hard_link(&staged, &final_path).unwrap();
    assert_eq!(b.usage().unwrap().used, 12);
    assert_eq!(
        hard_link(&staged, &final_path).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(b.usage().unwrap().used, 12);
    remove_file(staged).unwrap();
    assert_eq!(b.usage().unwrap().used, 6);
    assert!(write(&final_path, b"changed").is_err());
    assert_eq!(fs::read(final_path).unwrap(), b"123456");
}

#[test]
fn staged_replace_preserves_existing_bytes_on_quota_error() {
    let f = Fixture::new();
    let original = f.0.join("current");
    fs::write(&original, b"saved").unwrap();
    let b = OutputBudget::register(&f.0, 9).unwrap();
    let staged = f.0.join("pending");
    assert!(write(&staged, b"oversize").is_err());
    assert_eq!(fs::read(&original).unwrap(), b"saved");
    write(&staged, b"next").unwrap();
    rename(&staged, &original).unwrap();
    assert_eq!(b.usage().unwrap().used, 4);
    assert_eq!(fs::read(original).unwrap(), b"next");
}

#[test]
fn rename_cannot_unlink_a_live_destination_writer() {
    let f = Fixture::new();
    let budget = OutputBudget::register(&f.0, 16).unwrap();
    let destination = f.0.join("destination");
    let staged = f.0.join("staged");
    let mut active = writer(&destination, File::create(&destination).unwrap()).unwrap();
    active.write_all(b"old").unwrap();
    write(&staged, b"replacement").unwrap();
    let before = budget.usage().unwrap();
    assert!(rename(&staged, &destination).is_err());
    assert_eq!(fs::read(&staged).unwrap(), b"replacement");
    assert_eq!(fs::read(&destination).unwrap(), b"old");
    assert_eq!(budget.usage().unwrap(), before);
    drop(active);
    rename(&staged, &destination).unwrap();
    assert_eq!(fs::read(destination).unwrap(), b"replacement");
    assert_eq!(budget.usage().unwrap().used, 11);
}

struct PartialFailure(File);
impl Write for PartialFailure {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write_all(&buf[..2])?;
        Err(io::Error::other("injected partial write"))
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
impl Seek for PartialFailure {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.0.seek(pos)
    }
}
#[test]
fn partial_io_error_keeps_actual_bytes_charged_until_cleanup() {
    let f = Fixture::new();
    let b = OutputBudget::register(&f.0, 8).unwrap();
    let path = f.0.join("partial");
    let mut w = writer_inner(&path, PartialFailure(File::create(&path).unwrap())).unwrap();
    assert!(w.write_all(b"12345678").is_err());
    drop(w);
    assert_eq!(b.usage().unwrap().used, 2);
    assert_eq!(fs::metadata(&path).unwrap().len(), 2);
    remove_file(path).unwrap();
    assert_eq!(b.usage().unwrap().used, 0);
}

#[test]
fn reserved_recovery_capacity_is_not_spent_by_active_writers() {
    let f = Fixture::new();
    let b = OutputBudget::register(&f.0, 20).unwrap();
    let recovery = b.reserve(12).unwrap();
    write(f.0.join("episode"), b"12345678").unwrap();
    assert!(write(f.0.join("too-much"), b"x").is_err());
    drop(recovery);
    write(f.0.join("recovered"), b"abcdefghijkl").unwrap();
    assert_eq!(b.usage().unwrap().used, 20);
    assert!(b.usage().unwrap().high_water <= 20);
}

#[test]
fn cleanup_and_directory_publication_update_all_child_accounting() {
    let f = Fixture::new();
    let b = OutputBudget::register(&f.0, 20).unwrap();
    let stage = f.0.join("stage");
    fs::create_dir(&stage).unwrap();
    write(stage.join("a"), b"1234").unwrap();
    write(stage.join("b"), b"123").unwrap();
    let final_path = f.0.join("final");
    rename(&stage, &final_path).unwrap();
    assert_eq!(b.usage().unwrap().used, 7);
    remove_dir_all(&final_path).unwrap();
    assert_eq!(b.usage().unwrap().used, 0);
}

#[test]
fn partial_cleanup_releases_only_confirmed_removed_files() {
    let f = Fixture::new();
    let budget = OutputBudget::register(&f.0, 8).unwrap();
    write(f.0.join("a"), b"1234").unwrap();
    write(f.0.join("b"), b"5678").unwrap();
    let error = remove_tree_with(&f.0, |root| {
        fs::remove_file(root.join("a"))?;
        Err(io::Error::other("injected cleanup interruption"))
    })
    .unwrap_err();
    assert!(error.to_string().contains("cleanup interruption"));
    assert_eq!(budget.usage().unwrap().used, 4);
    assert_eq!(fs::read(f.0.join("b")).unwrap(), b"5678");
    write(f.0.join("c"), b"next").unwrap();
    assert_eq!(budget.usage().unwrap().used, 8);
}

#[test]
fn streamed_copy_cannot_exceed_cap_and_does_not_modify_source() {
    let source = Fixture::new();
    fs::write(source.0.join("source"), b"12345678").unwrap();
    let f = Fixture::new();
    let b = OutputBudget::register(&f.0, 4).unwrap();
    assert!(copy(source.0.join("source"), f.0.join("copy")).is_err());
    assert_eq!(fs::read(source.0.join("source")).unwrap(), b"12345678");
    assert!(b.usage().unwrap().used <= 4);
}

#[test]
fn exact_cap_seek_overwrite_has_zero_growth_but_append_after_seek_is_charged() {
    let f = Fixture::new();
    let b = OutputBudget::register(&f.0, 4).unwrap();
    let path = f.0.join("seek");
    let mut w = writer(&path, File::create(&path).unwrap()).unwrap();
    w.write_all(b"1234").unwrap();
    w.seek(SeekFrom::Start(0)).unwrap();
    w.write_all(b"HEAD").unwrap();
    drop(w);
    let file = fs::OpenOptions::new().append(true).open(&path).unwrap();
    let mut append = append_writer(&path, file).unwrap();
    append.seek(SeekFrom::Start(0)).unwrap();
    assert!(append.write_all(b"x").is_err());
    drop(append);
    assert_eq!(fs::read(path).unwrap(), b"HEAD");
    assert_eq!(b.usage().unwrap().used, 4);
}

#[test]
fn second_create_cannot_truncate_an_active_writer() {
    let f = Fixture::new();
    let _b = OutputBudget::register(&f.0, 8).unwrap();
    let path = f.0.join("active");
    let mut w = writer(&path, File::create(&path).unwrap()).unwrap();
    w.write_all(b"saved").unwrap();
    assert!(write(&path, b"x").is_err());
    assert_eq!(fs::read(path).unwrap(), b"saved");
}

#[test]
fn unaccounted_nonempty_link_or_rename_is_refused() {
    let f = Fixture::new();
    let b = OutputBudget::register(&f.0, 8).unwrap();
    let path = f.0.join("missed-writer");
    fs::write(&path, b"untracked").unwrap();
    assert!(hard_link(&path, f.0.join("link")).is_err());
    assert!(rename(&path, f.0.join("renamed")).is_err());
    assert!(write(&path, b"x").is_err());
    assert_eq!(b.usage().unwrap().used, 0);
}

#[cfg(windows)]
#[test]
fn windows_verbatim_and_case_variants_resolve_to_same_scope() {
    let f = Fixture::new();
    let b = OutputBudget::register(&f.0, 4).unwrap();
    let canonical = fs::canonicalize(&f.0).unwrap();
    let text = canonical.to_str().unwrap();
    let ordinary = text
        .strip_prefix(r"\\?\")
        .unwrap_or(text)
        .to_ascii_lowercase();
    write(PathBuf::from(ordinary).join("payload"), b"1234").unwrap();
    assert_eq!(b.usage().unwrap().used, 4);
    assert!(write(canonical.join("other"), b"x").is_err());
}

#[cfg(unix)]
#[test]
fn descendant_symlink_cannot_escape_scope() {
    use std::os::unix::fs::symlink;
    let outside = Fixture::new();
    let f = Fixture::new();
    let _b = OutputBudget::register(&f.0, 4).unwrap();
    symlink(&outside.0, f.0.join("alias")).unwrap();
    assert!(write(f.0.join("alias/escaped"), b"12345").is_err());
    assert!(!outside.0.join("escaped").exists());
}
