//! Atomic portable-save publication for durable GPU checkpoint transactions.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

use alife_world::persistence::{PortableAssetDigest, PortableSaveFile};

use crate::GameAppShellError;

static SAVE_CAS_GUARD: Mutex<()> = Mutex::new(());
static SAVE_CAS_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuSaveManifestDigest(PortableAssetDigest);

impl GpuSaveManifestDigest {
    fn for_bytes(bytes: &[u8]) -> Self {
        Self(PortableAssetDigest::for_bytes(bytes))
    }

    pub fn as_str(&self) -> &str {
        &self.0 .0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuLoadedSaveManifest {
    pub save: PortableSaveFile,
    pub digest: GpuSaveManifestDigest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuSaveManifestCasOutcome {
    Replaced {
        replacement_digest: GpuSaveManifestDigest,
    },
    AlreadyApplied {
        replacement_digest: GpuSaveManifestDigest,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuDurableSaveManifest {
    save_path: PathBuf,
    asset_root: PathBuf,
}

impl GpuDurableSaveManifest {
    pub fn open(
        save_path: impl Into<PathBuf>,
        asset_root: impl AsRef<Path>,
    ) -> Result<Self, GameAppShellError> {
        let save_path = save_path.into();
        let asset_root = fs::canonicalize(asset_root)?;
        let canonical_save = fs::canonicalize(&save_path)?;
        let durable = Self {
            save_path: canonical_save,
            asset_root,
        };
        durable.load()?;
        Ok(durable)
    }

    pub fn save_path(&self) -> &Path {
        &self.save_path
    }

    pub fn asset_root(&self) -> &Path {
        &self.asset_root
    }

    pub fn load(&self) -> Result<GpuLoadedSaveManifest, GameAppShellError> {
        let bytes = fs::read(&self.save_path)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| {
            GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint save manifest must be valid UTF-8 JSON".to_string(),
            }
        })?;
        let save = PortableSaveFile::from_json_str(text)?;
        save.validate_with_asset_root(&self.asset_root)?;
        Ok(GpuLoadedSaveManifest {
            save,
            digest: GpuSaveManifestDigest::for_bytes(&bytes),
        })
    }

    /// Atomically publishes a complete manual/autosave checkpoint, including
    /// first creation of the target manifest. The save may live in a selected
    /// save directory while neural assets remain validated against the
    /// separate asset root.
    pub fn publish_snapshot(
        save_path: impl AsRef<Path>,
        asset_root: impl AsRef<Path>,
        replacement: &PortableSaveFile,
    ) -> Result<GpuLoadedSaveManifest, GameAppShellError> {
        let asset_root = fs::canonicalize(asset_root)?;
        let requested = save_path.as_ref();
        let requested = if requested.is_absolute() {
            requested.to_path_buf()
        } else {
            std::env::current_dir()?.join(requested)
        };
        let parent =
            requested
                .parent()
                .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                    message: "GPU checkpoint save manifest has no parent directory".to_string(),
                })?;
        fs::create_dir_all(parent)?;
        let parent = fs::canonicalize(parent)?;
        let file_name =
            requested
                .file_name()
                .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                    message: "GPU checkpoint save manifest requires a file name".to_string(),
                })?;
        let save_path = parent.join(file_name);
        replacement.validate_with_asset_root(&asset_root)?;
        // This is a durable file manifest, not the bounded in-memory save-slot
        // payload governed by P34_MAX_INLINE_SAVE_BYTES. Bulk neural arrays are
        // already external content-addressed assets.
        let replacement_bytes = serde_json::to_vec_pretty(replacement)?;
        let _guard =
            SAVE_CAS_GUARD
                .lock()
                .map_err(|_| GameAppShellError::InvalidProductionFrontend {
                    message: "GPU checkpoint save CAS lock was poisoned".to_string(),
                })?;
        write_atomic_manifest(&save_path, &replacement_bytes)?;
        drop(_guard);

        let durable = Self {
            save_path: fs::canonicalize(&save_path)?,
            asset_root,
        };
        let published = durable.load()?;
        if published.save != *replacement {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "atomic GPU checkpoint publication changed the replacement save"
                    .to_string(),
            });
        }
        Ok(published)
    }

    pub fn compare_and_swap(
        &self,
        expected: &GpuSaveManifestDigest,
        replacement: &PortableSaveFile,
    ) -> Result<GpuSaveManifestCasOutcome, GameAppShellError> {
        replacement.validate_with_asset_root(&self.asset_root)?;
        let replacement_bytes = serde_json::to_vec_pretty(replacement)?;
        let replacement_digest = GpuSaveManifestDigest::for_bytes(&replacement_bytes);
        let _guard =
            SAVE_CAS_GUARD
                .lock()
                .map_err(|_| GameAppShellError::InvalidProductionFrontend {
                    message: "GPU checkpoint save CAS lock was poisoned".to_string(),
                })?;

        let current_bytes = fs::read(&self.save_path)?;
        let current_digest = GpuSaveManifestDigest::for_bytes(&current_bytes);
        if current_digest == replacement_digest {
            return Ok(GpuSaveManifestCasOutcome::AlreadyApplied { replacement_digest });
        }
        if &current_digest != expected {
            return Err(GameAppShellError::GpuCheckpointManifestConflict {
                expected: expected.as_str().to_string(),
                actual: current_digest.as_str().to_string(),
            });
        }

        let pre_replace_digest = GpuSaveManifestDigest::for_bytes(&fs::read(&self.save_path)?);
        if &pre_replace_digest != expected {
            return Err(GameAppShellError::GpuCheckpointManifestConflict {
                expected: expected.as_str().to_string(),
                actual: pre_replace_digest.as_str().to_string(),
            });
        }
        write_atomic_manifest(&self.save_path, &replacement_bytes)?;

        let published = fs::read(&self.save_path)?;
        let published_digest = GpuSaveManifestDigest::for_bytes(&published);
        if published_digest != replacement_digest {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "atomic GPU checkpoint publication digest mismatch".to_string(),
            });
        }
        Ok(GpuSaveManifestCasOutcome::Replaced { replacement_digest })
    }
}

fn write_atomic_manifest(save_path: &Path, bytes: &[u8]) -> Result<(), GameAppShellError> {
    let parent =
        save_path
            .parent()
            .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint save manifest has no parent directory".to_string(),
            })?;
    let nonce = SAVE_CAS_NONCE.fetch_add(1, Ordering::Relaxed);
    let file_name = save_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
            message: "GPU checkpoint save manifest requires a UTF-8 file name".to_string(),
        })?;
    let temporary = parent.join(format!(
        ".{file_name}.gpu-cas-{}-{nonce}.tmp",
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    if let Err(error) = (|| -> std::io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        atomic_replace(&temporary, save_path)?;
        sync_parent_directory(parent)
    })() {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    if fs::read(save_path)? != bytes {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "atomic GPU checkpoint publication digest mismatch".to_string(),
        });
    }
    Ok(())
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: both pointers reference live, NUL-terminated UTF-16 buffers for
    // the duration of the call, and both paths are on the same directory tree.
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> std::io::Result<()> {
    std::fs::File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent_directory(_parent: &Path) -> std::io::Result<()> {
    Ok(())
}
