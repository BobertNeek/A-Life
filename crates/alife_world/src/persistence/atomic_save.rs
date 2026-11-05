use std::{
    fs::{File, OpenOptions},
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(super) fn write(destination: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let (staged, mut file) = create_staged_file(parent)?;
    let result = (|| {
        alife_storage::write_all(&staged, &mut file, bytes)?;
        file.sync_all()
    })();
    // Windows requires the staging handle to be closed before replacement or
    // cleanup. Only the file created by this call is eligible for cleanup.
    drop(file);
    if let Err(error) = result.and_then(|()| alife_storage::rename(&staged, destination)) {
        let _ = alife_storage::remove_file(&staged);
        return Err(error);
    }
    // A failure here is reported even though the complete new save is already
    // visible. Never remove the published save after a directory-sync error.
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn create_staged_file(parent: &Path) -> io::Result<(PathBuf, File)> {
    loop {
        let staged = parent.join(format!(
            ".alife-portable-save-{}-{}.tmp",
            std::process::id(),
            SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)
        {
            Ok(file) => return Ok((staged, file)),
            // A previous process may have left recoverable staging data with
            // a reused PID. Keep it intact and allocate a different sibling.
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}
