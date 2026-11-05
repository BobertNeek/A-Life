use std::{
    fs::{File, OpenOptions},
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(super) fn write(destination: &Path, bytes: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    let previous = match std::fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.is_file() => Some(metadata),
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "save destination must be a regular file",
            ))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let (staged, mut file) = create_staged_file(parent)?;
    let result = (|| {
        #[cfg(unix)]
        if let Some(previous) = &previous {
            use std::os::unix::fs::MetadataExt;
            let created = file.metadata()?;
            if previous.uid() != created.uid() || previous.gid() != created.gid() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "save replacement would change ownership",
                ));
            }
        }
        alife_storage::write_all(&staged, &mut file, bytes)?;
        // Retain stricter owner permissions without enabling group/other access.
        // Restoring group bits could unmask ACL entries inherited from this
        // directory instead of preserving the old file's ACL. This does not
        // copy extended attributes or lock against concurrent chmod/replacement.
        #[cfg(unix)]
        if let Some(previous) = &previous {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(
                previous.permissions().mode() & 0o600,
            ))?;
        }
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
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&staged) {
            Ok(file) => return Ok((staged, file)),
            // A previous process may have left recoverable staging data with
            // a reused PID. Keep it intact and allocate a different sibling.
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}
