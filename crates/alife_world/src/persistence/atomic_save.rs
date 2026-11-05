use std::{
    fs::{File, Metadata, OpenOptions},
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(target_os = "linux")]
mod linux_metadata;
#[cfg(target_os = "macos")]
mod macos_metadata;
#[cfg(windows)]
mod windows_metadata;

static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct Destination {
    file: File,
    metadata: Metadata,
    #[cfg(target_os = "linux")]
    attributes: linux_metadata::Attributes,
    #[cfg(windows)]
    security: windows_metadata::Snapshot,
}

pub(super) fn write(destination: &Path, bytes: &[u8]) -> io::Result<()> {
    write_with_sync(destination, bytes, File::sync_all)
}

fn write_with_sync(
    destination: &Path,
    bytes: &[u8],
    sync_file: impl FnOnce(&File) -> io::Result<()>,
) -> io::Result<()> {
    let previous = open_destination(destination).map_err(|e| before_publication(destination, e))?;
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let (staged, mut file) =
        create_staged_file(parent).map_err(|e| before_publication(destination, e))?;
    let result = (|| {
        // Windows keeps the staging handle exclusive. Prepare and verify the
        // old descriptor while the file is empty, before writing any new data.
        #[cfg(windows)]
        if let Some(previous) = &previous {
            windows_metadata::apply(&previous.security, &file)?;
        }
        #[cfg(target_os = "macos")]
        macos_metadata::make_private(&file)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if file.metadata()?.permissions().mode() & 0o077 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "this save location did not honor private staging permissions",
                ));
            }
        }
        alife_storage::write_all(&staged, &mut file, bytes)?;
        #[cfg(unix)]
        if let Some(previous) = &previous {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let created = file.metadata()?;
                if previous.metadata.uid() != created.uid()
                    || previous.metadata.gid() != created.gid()
                {
                    rustix::fs::fchown(
                        &file,
                        Some(rustix::fs::Uid::from_raw(previous.metadata.uid())),
                        Some(rustix::fs::Gid::from_raw(previous.metadata.gid())),
                    )?;
                }
            }
            #[cfg(target_os = "linux")]
            linux_metadata::apply(&previous.attributes, &file)?;
            #[cfg(target_os = "macos")]
            macos_metadata::copy(&previous.file, &file)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::{MetadataExt, PermissionsExt};
                file.set_permissions(std::fs::Permissions::from_mode(
                    previous.metadata.mode() & 0o777,
                ))?;
            }
        }
        recheck(destination, previous.as_ref(), &file)?;
        sync_file(&file)
    })();
    // Close before cleanup/publication on Windows. Only this call's newly
    // created staging file is eligible for cleanup.
    drop(file);
    if let Err(error) = result {
        // Cleanup is best effort: an I/O error can leave this recoverable
        // sibling behind. Never remove a pre-existing recovery sibling.
        let _ = alife_storage::remove_file(&staged);
        return Err(before_publication(destination, error));
    }
    if let Err(error) = alife_storage::rename(&staged, destination) {
        let _ = alife_storage::remove_file(&staged);
        return Err(error);
    }
    // A directory-sync error is reported after publication, without removing
    // the complete published save. Do not describe it as a refused write.
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn before_publication(destination: &Path, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!(
        "Save '{}' was not published: {error}. Any existing save is unchanged; keep it and choose an explicit alternative save path if needed.",
        destination.display()))
}

fn open_destination(path: &Path) -> io::Result<Option<Destination>> {
    let observed =
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.is_file() => metadata,
            Ok(_) => return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "save destination must be a regular file; use its real path instead of a symlink",
            )),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
    // Check write access without truncation. NOFOLLOW and NONBLOCK avoid
    // following a substituted symlink or waiting on a substituted FIFO.
    #[cfg(unix)]
    let file = File::from(rustix::fs::open(
        path,
        rustix::fs::OFlags::WRONLY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::NONBLOCK
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )?);
    #[cfg(not(unix))]
    let file = OpenOptions::new().write(true).open(path)?;
    let metadata = file.metadata()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.dev() != observed.dev() || metadata.ino() != observed.ino() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "save destination changed during the write-access check",
            ));
        }
        if metadata.mode() & 0o7000 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "cannot preserve special permission bits when replacing save content",
            ));
        }
    }
    #[cfg(not(unix))]
    let _ = observed;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "save destination is no longer a regular file",
        ));
    }
    #[cfg(target_os = "linux")]
    let attributes = linux_metadata::read(&file)?;
    #[cfg(windows)]
    let security = windows_metadata::capture(&file, path)?;
    Ok(Some(Destination {
        file,
        metadata,
        #[cfg(target_os = "linux")]
        attributes,
        #[cfg(windows)]
        security,
    }))
}

fn recheck(path: &Path, previous: Option<&Destination>, staged: &File) -> io::Result<()> {
    let current = match std::fs::symlink_metadata(path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    match (previous, current) {
        (None, None) => (),
        (Some(previous), Some(current)) if current.is_file() => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let old = &previous.metadata;
                let new = staged.metadata()?;
                if current.dev() != old.dev()
                    || current.ino() != old.ino()
                    || current.mode() != old.mode()
                    || current.uid() != old.uid()
                    || current.gid() != old.gid()
                    || new.mode() != old.mode()
                    || new.uid() != old.uid()
                    || new.gid() != old.gid()
                {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "save destination access controls changed before publication",
                    ));
                }
            }
            #[cfg(target_os = "linux")]
            if linux_metadata::read(&previous.file)? != previous.attributes
                || linux_metadata::read(staged)? != previous.attributes
            {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "save attributes changed before publication",
                ));
            }
            #[cfg(not(unix))]
            if current.permissions().readonly() != previous.metadata.permissions().readonly()
                || previous.file.metadata()?.permissions().readonly()
                    != previous.metadata.permissions().readonly()
                || staged.metadata()?.permissions().readonly()
                    != previous.metadata.permissions().readonly()
            {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "save destination read-only protection changed before publication",
                ));
            }
            #[cfg(windows)]
            windows_metadata::recheck(&previous.security, &previous.file, path, staged)?;
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "save destination changed before publication",
            ))
        }
    }
    // These observations do not make rename conditional on an inode or an ACL
    // snapshot. Callers must serialize save, permission and path changes.
    Ok(())
}

fn create_staged_file(parent: &Path) -> io::Result<(PathBuf, File)> {
    loop {
        let staged = parent.join(format!(
            ".alife-portable-save-{}-{}.tmp",
            std::process::id(),
            SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        #[cfg(not(windows))]
        let mut options = OpenOptions::new();
        #[cfg(not(windows))]
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        #[cfg(not(windows))]
        let opened = options.open(&staged);
        #[cfg(windows)]
        let opened = windows_metadata::create_private(&staged);
        match opened {
            Ok(file) => return Ok((staged, file)),
            // Preserve recoverable siblings left by previous processes.
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_sync_failure_after_write_preserves_previous_save_and_recovery() {
        let root = std::env::temp_dir().join(format!(
            "alife-save-sync-failure-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let path = root.join("save.json");
        let recovery = root.join(".alife-portable-save-previous.tmp");
        let previous = b"previous complete save";
        let replacement = b"replacement complete save with different bytes";
        let recovery_bytes = b"recoverable staging";
        let baseline = (previous.len() + recovery_bytes.len()) as u64;
        let peak = baseline + replacement.len() as u64;
        let budget = alife_storage::OutputBudget::register(&root, peak).unwrap();
        alife_storage::write(&path, previous).unwrap();
        alife_storage::write(&recovery, recovery_bytes).unwrap();
        let error = write_with_sync(&path, replacement, |file| {
            assert_eq!(file.metadata().unwrap().len(), replacement.len() as u64);
            use std::io::{Read, Seek, SeekFrom};
            let mut reader = file.try_clone().unwrap();
            reader.seek(SeekFrom::Start(0)).unwrap();
            let mut actual = Vec::new();
            reader.read_to_end(&mut actual).unwrap();
            assert_eq!(actual, replacement);
            assert_eq!(budget.usage().unwrap().used, peak);
            Err(io::Error::other("injected file-sync failure"))
        })
        .unwrap_err();
        assert!(
            error.to_string().contains("injected file-sync failure"),
            "{error}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), previous);
        assert_eq!(std::fs::read(&recovery).unwrap(), recovery_bytes);
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
        let usage = budget.usage().unwrap();
        assert_eq!(
            (usage.high_water, usage.used, usage.reserved),
            (peak, baseline, 0)
        );
        // The same quota and destination remain usable after the failed sync.
        write(&path, replacement).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), replacement);
        assert_eq!(std::fs::read(&recovery).unwrap(), recovery_bytes);
        alife_storage::remove_dir_all(&root).unwrap();
    }
}
