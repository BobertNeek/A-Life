//! Atomic content-addressed storage for portable GPU checkpoint envelopes.

use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use alife_world::persistence::{
    AssetKind, AssetManifest, AssetManifestEntry, AssetPresence, GpuBrainAssetRef,
    PortableAssetDigest, GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::GameAppShellError;

#[cfg(unix)]
use std::fs::File;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuCheckpointAssetStore {
    root: PathBuf,
}

impl GpuCheckpointAssetStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, GameAppShellError> {
        let root = root.into();
        if root.as_os_str().is_empty() {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint asset root is empty".to_string(),
            });
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn write_json<T: Serialize>(
        &self,
        kind: &'static str,
        value: &T,
    ) -> Result<(GpuBrainAssetRef, AssetManifestEntry), GameAppShellError> {
        validate_kind(kind)?;
        let bytes = serde_json::to_vec(value)?;
        let digest = PortableAssetDigest::for_bytes(&bytes);
        let suffix = digest.0.strip_prefix("fnv1a64:").ok_or_else(|| {
            GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint digest did not use the portable digest ABI".to_string(),
            }
        })?;
        let asset_id = format!("gpu-brain.{kind}.{suffix}");
        let relative_path = format!("gpu-brain/{kind}-{suffix}.json");
        let destination = self.root.join(&relative_path);
        write_content_addressed(&destination, &bytes)?;
        let size_bytes = u64::try_from(bytes.len()).map_err(|_| {
            GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint asset length overflowed u64".to_string(),
            }
        })?;
        let asset_ref = GpuBrainAssetRef {
            asset_id: asset_id.clone(),
            digest: digest.clone(),
        };
        asset_ref.validate()?;
        let entry = AssetManifestEntry {
            asset_id,
            kind: AssetKind::Other,
            relative_path,
            digest,
            presence: AssetPresence::Required,
            schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
            size_bytes: Some(size_bytes),
            provenance: Some(format!("gpu-checkpoint:{kind}")),
        };
        Ok((asset_ref, entry))
    }

    pub(crate) fn read_json<T: DeserializeOwned>(
        &self,
        manifest: &AssetManifest,
        asset_ref: &GpuBrainAssetRef,
    ) -> Result<(T, Vec<u8>), GameAppShellError> {
        asset_ref.validate()?;
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.asset_id == asset_ref.asset_id)
            .ok_or_else(
                || alife_world::persistence::PersistenceError::MissingAssetReference {
                    asset_id: asset_ref.asset_id.clone(),
                },
            )?;
        if entry.digest != asset_ref.digest
            || entry.presence != AssetPresence::Required
            || entry.schema_version != GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION
        {
            return Err(
                alife_world::persistence::PersistenceError::InvalidAssetManifest {
                    asset_id: asset_ref.asset_id.clone(),
                    message: "GPU checkpoint reference does not match its required manifest entry",
                }
                .into(),
            );
        }
        let path = self.root.join(&entry.relative_path);
        let bytes = fs::read(&path)?;
        let actual = PortableAssetDigest::for_bytes(&bytes);
        if actual != asset_ref.digest {
            return Err(alife_world::persistence::PersistenceError::DigestMismatch {
                asset_id: asset_ref.asset_id.clone(),
                expected: asset_ref.digest.0.clone(),
                actual: actual.0,
            }
            .into());
        }
        if entry
            .size_bytes
            .is_some_and(|expected| expected != bytes.len() as u64)
        {
            return Err(
                alife_world::persistence::PersistenceError::InvalidAssetManifest {
                    asset_id: asset_ref.asset_id.clone(),
                    message: "GPU checkpoint asset size does not match its manifest entry",
                }
                .into(),
            );
        }
        Ok((serde_json::from_slice(&bytes)?, bytes))
    }
}

pub fn merge_gpu_checkpoint_manifest_entries(
    manifest: &mut AssetManifest,
    entries: impl IntoIterator<Item = AssetManifestEntry>,
) -> Result<(), GameAppShellError> {
    let mut existing = manifest
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.asset_id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    for entry in entries {
        if let Some(index) = existing.get(&entry.asset_id).copied() {
            if manifest.entries[index] != entry {
                return Err(
                    alife_world::persistence::PersistenceError::InvalidAssetManifest {
                        asset_id: entry.asset_id,
                        message: "content-addressed GPU asset id maps to different metadata",
                    }
                    .into(),
                );
            }
            continue;
        }
        existing.insert(entry.asset_id.clone(), manifest.entries.len());
        manifest.entries.push(entry);
    }
    manifest
        .entries
        .sort_by(|left, right| left.asset_id.cmp(&right.asset_id));
    Ok(())
}

fn validate_kind(kind: &str) -> Result<(), GameAppShellError> {
    if kind.is_empty()
        || kind.len() > 48
        || !kind
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "GPU checkpoint asset kind is not a bounded lowercase identifier".to_string(),
        });
    }
    Ok(())
}

fn write_content_addressed(path: &Path, bytes: &[u8]) -> Result<(), GameAppShellError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        if fs::read(path)? == bytes {
            return Ok(());
        }
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: format!(
                "content-addressed GPU checkpoint collision at {}",
                path.display()
            ),
        });
    }
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
            message: "GPU checkpoint destination has no portable file name".to_string(),
        })?;
    let temporary = path.with_file_name(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        sequence
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    match fs::rename(&temporary, path) {
        Ok(()) => sync_parent(path),
        Err(_error) if path.exists() && fs::read(path)? == bytes => {
            fs::remove_file(&temporary)?;
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(error.into())
        }
    }
}

fn sync_parent(path: &Path) -> Result<(), GameAppShellError> {
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
