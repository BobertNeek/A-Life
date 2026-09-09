//! Atomic content-addressed storage for portable GPU checkpoint envelopes.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use alife_world::persistence::{
    AssetKind, AssetManifest, AssetManifestEntry, AssetPresence, GpuBrainAssetRef,
    PortableAssetDigest, PortableSaveFile, GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
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

/// Removes stale entries emitted by the GPU checkpoint writer while preserving
/// every asset referenced by the current save and every entry owned elsewhere.
pub fn retain_current_gpu_checkpoint_manifest_entries(save: &mut PortableSaveFile) {
    let mut current_asset_ids = save
        .generated_weight_asset_refs
        .iter()
        .chain(save.etf_prototype_asset_refs.iter())
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    for creature in &save.creatures {
        current_asset_ids.extend(
            creature
                .weights
                .generated_weight_asset_id
                .iter()
                .map(String::as_str),
        );
        if let Some(composite) = &creature.composite_genetics {
            current_asset_ids.insert(composite.creature_genome_asset_id.as_str());
            current_asset_ids.insert(composite.foundation_asset_id.as_str());
        }
        if let Some(lifetime) = &creature.lifetime_state_asset {
            current_asset_ids.insert(lifetime.asset_id.as_str());
        }
        if let Some(gpu_brain) = &creature.gpu_brain {
            current_asset_ids.extend(
                gpu_brain
                    .asset_references()
                    .into_iter()
                    .map(|asset| asset.asset_id.as_str()),
            );
        }
    }

    retain_manifest_entries_for_current_gpu_roots(&mut save.assets, current_asset_ids);
}

fn retain_manifest_entries_for_current_gpu_roots<'a>(
    manifest: &mut AssetManifest,
    current_asset_ids: impl IntoIterator<Item = &'a str>,
) {
    let current_asset_ids = current_asset_ids.into_iter().collect::<BTreeSet<_>>();
    manifest.entries.retain(|entry| {
        current_asset_ids.contains(entry.asset_id.as_str())
            || !is_exact_gpu_checkpoint_writer_entry(entry)
    });
}

fn is_exact_gpu_checkpoint_writer_entry(entry: &AssetManifestEntry) -> bool {
    if entry.kind != AssetKind::Other
        || entry.presence != AssetPresence::Required
        || entry.schema_version != GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION
        || entry.size_bytes.is_none()
        || entry.digest.validate_format().is_err()
    {
        return false;
    }
    let Some(kind) = entry
        .provenance
        .as_deref()
        .and_then(|provenance| provenance.strip_prefix("gpu-checkpoint:"))
    else {
        return false;
    };
    if validate_kind(kind).is_err() {
        return false;
    }
    let Some(digest_suffix) = entry.digest.0.strip_prefix("fnv1a64:") else {
        return false;
    };

    entry.asset_id == format!("gpu-brain.{kind}.{digest_suffix}")
        && entry.relative_path == format!("gpu-brain/{kind}-{digest_suffix}.json")
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
    publish_temporary_content_addressed(&temporary, path, bytes)
}

fn publish_temporary_content_addressed(
    temporary: &Path,
    path: &Path,
    bytes: &[u8],
) -> Result<(), GameAppShellError> {
    match fs::hard_link(temporary, path) {
        Ok(()) => {
            fs::remove_file(temporary)?;
            sync_parent(path)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = fs::read(path);
            let cleanup = fs::remove_file(temporary);
            match existing {
                Ok(existing) if existing == bytes => {
                    cleanup?;
                    Ok(())
                }
                Ok(_) => {
                    let _ = cleanup;
                    Err(GameAppShellError::InvalidProductionFrontend {
                        message: format!(
                            "content-addressed GPU checkpoint collision at {}",
                            path.display()
                        ),
                    })
                }
                Err(read_error) => {
                    let _ = cleanup;
                    Err(read_error.into())
                }
            }
        }
        Err(error) => {
            let _ = fs::remove_file(temporary);
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

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{BrainScaleTier, FoundationWeightAsset, PhenotypeHash, SensorProfile};
    use alife_world::{
        create_canonical_new_game,
        persistence::{
            AssetManifest, CompositeGeneticSaveRef, CreatureLifetimeStateSaveRef, PortableSaveFile,
            RuntimeConfig,
        },
        CanonicalNewGameConfig,
    };

    fn current_save() -> PortableSaveFile {
        let foundation =
            FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1)
                .unwrap();
        let game = create_canonical_new_game(
            &CanonicalNewGameConfig::phase3(73_129, 4).unwrap(),
            &foundation,
        )
        .unwrap();
        PortableSaveFile::from_headless_world(
            "current-gpu-manifest-filter",
            &game.world,
            RuntimeConfig::deterministic_default(game.world.seed(), BrainScaleTier::Nano512),
            AssetManifest::empty(),
            game.creatures,
        )
        .unwrap()
    }

    fn writer_entry(kind: &'static str, payload: &[u8]) -> AssetManifestEntry {
        let digest = PortableAssetDigest::for_bytes(payload);
        let suffix = digest.0.strip_prefix("fnv1a64:").unwrap();
        AssetManifestEntry {
            asset_id: format!("gpu-brain.{kind}.{suffix}"),
            kind: AssetKind::Other,
            relative_path: format!("gpu-brain/{kind}-{suffix}.json"),
            digest,
            presence: AssetPresence::Required,
            schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
            size_bytes: Some(payload.len() as u64),
            provenance: Some(format!("gpu-checkpoint:{kind}")),
        }
    }

    #[test]
    fn current_manifest_filter_removes_only_stale_exact_writer_entries() {
        let mut save = current_save();
        let stale_neural = writer_entry("stale-neural", b"stale-neural");
        let generated = writer_entry("generated-root", b"generated-root");
        let etf = writer_entry("etf-root", b"etf-root");
        save.generated_weight_asset_refs = vec![generated.asset_id.clone()];
        save.etf_prototype_asset_refs = vec![etf.asset_id.clone()];

        let arbitrary_other = AssetManifestEntry {
            asset_id: "arbitrary-other".to_owned(),
            kind: AssetKind::Other,
            relative_path: "other/arbitrary.json".to_owned(),
            digest: PortableAssetDigest::for_bytes(b"arbitrary-other"),
            presence: AssetPresence::Required,
            schema_version: 73,
            size_bytes: None,
            provenance: Some("independent-writer".to_owned()),
        };
        save.assets.entries = vec![
            stale_neural.clone(),
            generated.clone(),
            etf.clone(),
            arbitrary_other.clone(),
        ];

        retain_current_gpu_checkpoint_manifest_entries(&mut save);

        assert_eq!(
            save.assets
                .entries
                .iter()
                .map(|entry| entry.asset_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                generated.asset_id.as_str(),
                etf.asset_id.as_str(),
                arbitrary_other.asset_id.as_str(),
            ]
        );
        assert!(!save
            .assets
            .entries
            .iter()
            .any(|entry| entry.asset_id == stale_neural.asset_id));
    }

    #[test]
    fn current_manifest_filter_retains_independent_creature_roots_and_shared_content() {
        let mut save = current_save();
        let shared = writer_entry("shared", b"shared");
        let stale = writer_entry("stale", b"stale");
        let composite_foundation = writer_entry("composite-foundation", b"composite-foundation");
        let lifetime_state = writer_entry("lifetime-state", b"lifetime-state");
        let creature = &mut save.creatures[0];
        creature.weights.generated_weight_asset_id = Some(shared.asset_id.clone());
        creature.composite_genetics = Some(CompositeGeneticSaveRef {
            schema_version: 1,
            creature_genome_asset_id: shared.asset_id.clone(),
            foundation_asset_id: composite_foundation.asset_id.clone(),
            phenotype_hash: PhenotypeHash([1, 2, 3, 4]),
        });
        creature.lifetime_state_asset = Some(CreatureLifetimeStateSaveRef {
            schema_version: 1,
            asset_id: lifetime_state.asset_id.clone(),
        });
        save.assets = AssetManifest {
            entries: vec![
                shared.clone(),
                stale,
                composite_foundation.clone(),
                lifetime_state.clone(),
            ],
            ..AssetManifest::empty()
        };

        retain_current_gpu_checkpoint_manifest_entries(&mut save);

        assert_eq!(
            save.assets.entries,
            vec![shared, composite_foundation, lifetime_state]
        );
    }

    #[test]
    fn current_manifest_filter_preserves_partial_or_conflicting_writer_metadata() {
        let mut save = current_save();
        let exact = writer_entry("ambiguous", b"ambiguous");
        let mut id_only = exact.clone();
        id_only.provenance = Some("independent-writer".to_owned());
        let mut path_only = exact.clone();
        path_only.asset_id = "independent-path-owner".to_owned();
        let mut provenance_only = exact.clone();
        provenance_only.relative_path = "other/provenance-only.json".to_owned();
        let mut malformed_kind = exact.clone();
        malformed_kind.provenance = Some("gpu-checkpoint:Bad Kind".to_owned());
        let mut conflicting_digest = exact.clone();
        conflicting_digest.digest = PortableAssetDigest::for_bytes(b"different");
        let mut malformed_digest = exact.clone();
        malformed_digest.digest = PortableAssetDigest("sha256:not-portable".to_owned());
        let mut wrong_schema = exact;
        wrong_schema.schema_version += 1;
        let mut missing_size = writer_entry("missing-size", b"missing-size");
        missing_size.size_bytes = None;
        let mut unrelated_kind = writer_entry("unrelated-kind", b"unrelated-kind");
        unrelated_kind.kind = AssetKind::GeneratedWeights;
        let expected = vec![
            id_only,
            path_only,
            provenance_only,
            malformed_kind,
            conflicting_digest,
            malformed_digest,
            wrong_schema,
            missing_size,
            unrelated_kind,
        ];
        save.assets = AssetManifest {
            entries: expected.clone(),
            ..AssetManifest::empty()
        };

        retain_current_gpu_checkpoint_manifest_entries(&mut save);

        assert_eq!(save.assets.entries, expected);
    }

    #[test]
    fn current_manifest_filter_does_not_delete_stale_content_files() {
        let nonce = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "alife-current-gpu-manifest-filter-{}-{nonce}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        let store = GpuCheckpointAssetStore::new(&root).unwrap();
        let (_, stale) = store
            .write_json("stale", &serde_json::json!({ "stale": true }))
            .unwrap();
        let stale_path = root.join(&stale.relative_path);
        let mut save = current_save();
        save.assets = AssetManifest {
            entries: vec![stale],
            ..AssetManifest::empty()
        };

        retain_current_gpu_checkpoint_manifest_entries(&mut save);

        assert!(save.assets.entries.is_empty());
        assert!(stale_path.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn raced_content_addressed_publication_never_replaces_existing_bytes() {
        let nonce = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "alife-content-addressed-race-{}-{nonce}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();
        let destination = root.join("asset.json");
        let temporary = root.join("asset.tmp");
        fs::write(&destination, b"existing-authority").unwrap();
        fs::write(&temporary, b"racing-candidate").unwrap();

        assert!(
            publish_temporary_content_addressed(&temporary, &destination, b"racing-candidate")
                .is_err()
        );
        assert_eq!(fs::read(&destination).unwrap(), b"existing-authority");
        assert!(!temporary.exists());

        fs::remove_dir_all(root).unwrap();
    }
}
