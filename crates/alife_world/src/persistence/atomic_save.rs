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

static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct Destination {
    file: File,
    metadata: Metadata,
    #[cfg(target_os = "linux")]
    attributes: linux_metadata::Attributes,
}

pub(super) fn write(destination: &Path, bytes: &[u8]) -> io::Result<()> {
    let previous = open_destination(destination).map_err(|e| before_publication(destination, e))?;
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let (staged, mut file) =
        create_staged_file(parent).map_err(|e| before_publication(destination, e))?;
    let result = (|| {
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
        file.sync_all()
    })();
    // Close before cleanup/publication on Windows. Only this call's newly
    // created staging file is eligible for cleanup.
    drop(file);
    if let Err(error) = result {
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
    Ok(Some(Destination {
        file,
        metadata,
        #[cfg(target_os = "linux")]
        attributes,
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
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&staged) {
            Ok(file) => return Ok((staged, file)),
            // Preserve recoverable siblings left by previous processes.
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}
