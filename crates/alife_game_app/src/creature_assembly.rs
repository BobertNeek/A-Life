use std::collections::{BTreeMap, BTreeSet};

use alife_world::{CreaturePartFamilyId, CreaturePartSlotKey, CreaturePartSources};
use thiserror::Error;

use crate::{
    CreatureCoatKey, CreaturePartAssetId, CreaturePartCatalog, CreaturePartLodId, CreaturePartSlot,
    CreatureVisualBounds, GeneForgeCreaturePartCatalog, GeneForgeGroupTransform,
    GeneForgeLandmarkId, GeneForgeSocketEvidence, SocketFrame,
};

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyCreatureAssemblyPartRecipe {
    pub family: CreaturePartFamilyId,
    pub lod: CreaturePartLodId,
    pub slot: CreaturePartSlot,
    pub mesh_asset_path: String,
    pub texture_asset_path: String,
    pub socket: SocketFrame,
    pub local_scale: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyResolvedJoinCover {
    pub slot: CreaturePartSlot,
    pub socket: SocketFrame,
    pub primitive: JoinCoverPrimitive,
    pub overlap_depth: f32,
    pub source_family: CreaturePartFamilyId,
    pub torso_family: CreaturePartFamilyId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JoinCoverPrimitive {
    Ruff,
    ShoulderTuft,
    HipFur,
    TailRuff,
    Cuff,
}

impl JoinCoverPrimitive {
    fn from_catalog(value: &str) -> Option<Self> {
        match value {
            "ruff" => Some(Self::Ruff),
            "shoulder-tuft" => Some(Self::ShoulderTuft),
            "hip-fur" => Some(Self::HipFur),
            "tail-ruff" => Some(Self::TailRuff),
            "cuff" => Some(Self::Cuff),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Ruff => "ruff",
            Self::ShoulderTuft => "shoulder-tuft",
            Self::HipFur => "hip-fur",
            Self::TailRuff => "tail-ruff",
            Self::Cuff => "cuff",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureAssemblyWarning {
    UnknownFamilyFallback {
        requested: CreaturePartFamilyId,
        fallback: CreaturePartFamilyId,
    },
}

impl CreatureAssemblyWarning {
    pub fn visible_message(self) -> String {
        match self {
            Self::UnknownFamilyFallback {
                requested,
                fallback,
            } => format!(
                "Creature part family {} is unavailable for display; showing family {}. The save and lineage data remain unchanged.",
                requested.0, fallback.0
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyCreatureAssemblyRecipe {
    pub root_family: CreaturePartFamilyId,
    pub parts: BTreeMap<CreaturePartSlot, LegacyCreatureAssemblyPartRecipe>,
    pub join_covers: Vec<LegacyResolvedJoinCover>,
    pub warning: Option<CreatureAssemblyWarning>,
    pub display_only: bool,
}

#[derive(Debug, Error)]
pub enum CreatureAssemblyError {
    #[error("catalog has no creature part families")]
    EmptyCatalog,
    #[error("family {0:?} is missing LOD {1:?}")]
    MissingLod(CreaturePartFamilyId, CreaturePartLodId),
    #[error("torso family is missing socket {0}")]
    MissingSocket(String),
    #[error("assembly transform is non-finite or outside scale bounds")]
    InvalidTransform,
    #[error("paired feet differ by more than 0.04 canonical units")]
    FootHeightMismatch,
    #[error("assembly is missing a hidden join cover")]
    MissingJoinCover,
    #[error("invalid authored assembly preparation: {reason}")]
    InvalidPreparation { reason: String },
    #[error("duplicate authored assembly preparation for {key:?}")]
    DuplicatePreparation { key: GeneForgePreparationKey },
    #[error("missing authored assembly preparation for {key:?}")]
    MissingPreparation { key: GeneForgePreparationKey },
    #[error("GeneForge catalog has no creature families")]
    EmptyGeneForgeCatalog,
    #[error("GeneForge family {0:?} is missing a required part recipe")]
    MissingGeneForgePart(CreaturePartFamilyId),
    #[error("GeneForge catalog is missing asset {0:?}")]
    MissingGeneForgeAsset(CreaturePartAssetId),
}

/// Renderer-only compatibility adapter. Task 7 owns its removal and cutover.
#[deprecated(note = "Task 7 must switch the renderer to resolve_geneforge_creature_assembly")]
pub fn resolve_creature_assembly(
    requested_sources: CreaturePartSources,
    lod: CreaturePartLodId,
    catalog: &CreaturePartCatalog,
) -> Result<LegacyCreatureAssemblyRecipe, CreatureAssemblyError> {
    let fallback = catalog
        .families
        .iter()
        .min_by_key(|family| family.id)
        .map(|family| family.id)
        .ok_or(CreatureAssemblyError::EmptyCatalog)?;
    let unknown = requested_sources.torso;
    let (sources, warning) = if catalog.family(unknown).is_none() {
        (
            CreaturePartSources::coherent(fallback),
            Some(CreatureAssemblyWarning::UnknownFamilyFallback {
                requested: unknown,
                fallback,
            }),
        )
    } else {
        let mut normalized = requested_sources;
        let mut warning = None;
        for slot in [
            CreaturePartSlot::Head,
            CreaturePartSlot::LeftArm,
            CreaturePartSlot::LeftLeg,
            CreaturePartSlot::TailBack,
        ] {
            let current = family_for_slot(normalized, slot);
            if catalog.family(current).is_some() {
                continue;
            }
            normalized = with_family_for_slot(normalized, slot, normalized.torso);
            if warning.is_none() {
                warning = Some(CreatureAssemblyWarning::UnknownFamilyFallback {
                    requested: current,
                    fallback: normalized.torso,
                });
            }
        }
        (normalized, warning)
    };
    let torso = catalog
        .family(sources.torso)
        .ok_or(CreatureAssemblyError::EmptyCatalog)?;
    let mut parts = BTreeMap::new();
    for slot in CreaturePartSlot::ALL {
        let source_family = family_for_slot(sources, slot);
        let family = catalog
            .family(source_family)
            .ok_or(CreatureAssemblyError::EmptyCatalog)?;
        let lod_entry = family
            .lods
            .iter()
            .find(|entry| entry.lod == lod)
            .ok_or(CreatureAssemblyError::MissingLod(source_family, lod))?;
        let socket = if let Some(name) = socket_name(slot) {
            *torso
                .sockets
                .get(name)
                .ok_or_else(|| CreatureAssemblyError::MissingSocket(name.to_string()))?
        } else {
            SocketFrame::IDENTITY
        };
        validate_frame(socket)?;
        parts.insert(
            slot,
            LegacyCreatureAssemblyPartRecipe {
                family: source_family,
                lod,
                slot,
                mesh_asset_path: lod_entry.generated_obj.clone(),
                texture_asset_path: family.texture_asset.clone(),
                socket,
                local_scale: [1.0; 3],
            },
        );
    }

    let left_hip = torso
        .sockets
        .get("left-hip")
        .ok_or_else(|| CreatureAssemblyError::MissingSocket("left-hip".to_string()))?;
    let right_hip = torso
        .sockets
        .get("right-hip")
        .ok_or_else(|| CreatureAssemblyError::MissingSocket("right-hip".to_string()))?;
    if (left_hip.translation[2] - right_hip.translation[2]).abs() > 0.04 {
        return Err(CreatureAssemblyError::FootHeightMismatch);
    }

    let join_covers = torso
        .join_covers
        .iter()
        .filter(|_| lod != CreaturePartLodId::Impostor)
        .map(|cover| {
            let source_family = family_for_slot(sources, cover.slot);
            let socket = torso
                .sockets
                .get(&cover.socket)
                .copied()
                .ok_or_else(|| CreatureAssemblyError::MissingSocket(cover.socket.clone()))?;
            let primitive = JoinCoverPrimitive::from_catalog(&cover.cover_kind)
                .ok_or(CreatureAssemblyError::MissingJoinCover)?;
            Ok(LegacyResolvedJoinCover {
                slot: cover.slot,
                socket,
                primitive,
                overlap_depth: cover.overlap_depth,
                source_family,
                torso_family: sources.torso,
            })
        })
        .collect::<Result<Vec<_>, CreatureAssemblyError>>()?;
    if lod != CreaturePartLodId::Impostor && (join_covers.len() < 5 || join_covers.len() > 12) {
        return Err(CreatureAssemblyError::MissingJoinCover);
    }
    Ok(LegacyCreatureAssemblyRecipe {
        root_family: sources.torso,
        parts,
        join_covers,
        warning,
        display_only: true,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreaturePartMeshKey {
    pub asset_id: CreaturePartAssetId,
    pub lod: CreaturePartLodId,
    pub runtime_group: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GeneForgePreparationKey {
    pub source_family: CreaturePartFamilyId,
    pub source_asset_id: CreaturePartAssetId,
    pub target_torso_asset_id: CreaturePartAssetId,
    pub lod: CreaturePartLodId,
    pub runtime_group: String,
    pub socket: String,
}

impl GeneForgePreparationKey {
    fn from_record(record: &GeneForgeGroupTransform) -> Self {
        Self {
            source_family: CreaturePartFamilyId(record.source_family_id),
            source_asset_id: record.source_asset_id.clone(),
            target_torso_asset_id: record.target_torso_asset_id.clone(),
            lod: record.lod,
            runtime_group: record.runtime_group.clone(),
            socket: record.socket.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneForgeAssemblyPreparationIndex {
    records: BTreeMap<GeneForgePreparationKey, GeneForgeGroupTransform>,
}

impl GeneForgeAssemblyPreparationIndex {
    pub fn new(
        catalog: &GeneForgeCreaturePartCatalog,
        records: impl IntoIterator<Item = GeneForgeGroupTransform>,
    ) -> Result<Self, CreatureAssemblyError> {
        let mut indexed = BTreeMap::new();
        for record in records {
            let family = gene_forge_family(catalog, CreaturePartFamilyId(record.source_family_id))
                .ok_or_else(|| CreatureAssemblyError::InvalidPreparation {
                    reason: format!(
                        "source family {} is not present in the GeneForge catalog",
                        record.source_family_id
                    ),
                })?;
            let canonical_torso = family
                .parts
                .get(&CreaturePartSlotKey::Torso)
                .ok_or(CreatureAssemblyError::MissingGeneForgePart(family.id))?
                .asset_id
                .clone();
            let cross_torso = record.target_torso_asset_id != canonical_torso;
            catalog
                .validate_group_transform(&record, cross_torso)
                .map_err(|error| CreatureAssemblyError::InvalidPreparation {
                    reason: error.to_string(),
                })?;
            validate_preparation_evidence(catalog, &record)?;
            let key = GeneForgePreparationKey::from_record(&record);
            if indexed.insert(key.clone(), record).is_some() {
                return Err(CreatureAssemblyError::DuplicatePreparation { key });
            }
        }
        Ok(Self { records: indexed })
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    fn get(&self, key: &GeneForgePreparationKey) -> Option<&GeneForgeGroupTransform> {
        self.records.get(key)
    }

    #[cfg(test)]
    fn without_target(&self, target: &CreaturePartAssetId) -> Self {
        Self {
            records: self
                .records
                .iter()
                .filter(|(key, _)| &key.target_torso_asset_id != target)
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        }
    }

    #[cfg(test)]
    fn without_runtime_group(&self, runtime_group: &str) -> Self {
        Self {
            records: self
                .records
                .iter()
                .filter(|(key, _)| key.runtime_group != runtime_group)
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureAssemblyPartRecipe {
    pub source_family: CreaturePartFamilyId,
    pub asset_id: CreaturePartAssetId,
    pub target_torso_asset_id: CreaturePartAssetId,
    pub lod: CreaturePartLodId,
    pub slot: CreaturePartSlot,
    pub runtime_group: String,
    pub socket: String,
    pub authored_transform: [f64; 16],
    pub canonical_bounds: CreatureVisualBounds,
    pub transformed_bounds: CreatureVisualBounds,
    pub landmarks: BTreeMap<GeneForgeLandmarkId, [f32; 3]>,
    pub attachment_residual: f64,
    pub socket_evidence: Vec<GeneForgeSocketEvidence>,
    pub coat_key: CreatureCoatKey,
}

impl CreatureAssemblyPartRecipe {
    pub fn mesh_key(&self) -> CreaturePartMeshKey {
        CreaturePartMeshKey {
            asset_id: self.asset_id.clone(),
            lod: self.lod,
            runtime_group: self.runtime_group.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedJoinCover {
    pub slot: CreaturePartSlot,
    pub runtime_group: String,
    pub cover_kind: String,
    pub authored_transform: [f64; 16],
    pub overlap_depth: f32,
    pub coat_key: CreatureCoatKey,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureAssemblyRecipe {
    pub saved_sources: CreaturePartSources,
    pub displayed_sources: CreaturePartSources,
    pub target_torso_asset_id: CreaturePartAssetId,
    pub parts: BTreeMap<CreaturePartSlot, CreatureAssemblyPartRecipe>,
    pub join_covers: Vec<ResolvedJoinCover>,
    pub coat_key: CreatureCoatKey,
    pub warning: Option<CreatureAssemblyWarning>,
    pub display_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreaturePartDisplayResolution {
    pub saved_sources: CreaturePartSources,
    pub displayed_sources: CreaturePartSources,
    pub warning: Option<CreatureAssemblyWarning>,
    pub display_only: bool,
}

#[derive(Debug, Default)]
pub struct CreaturePartAssetKeyCache {
    mesh_keys: BTreeSet<CreaturePartMeshKey>,
    coat_keys: BTreeSet<CreatureCoatKey>,
    requested_meshes: usize,
}

impl CreaturePartAssetKeyCache {
    pub fn register_geneforge_recipe(&mut self, recipe: &CreatureAssemblyRecipe) {
        for part in recipe.parts.values() {
            self.requested_meshes += 1;
            self.mesh_keys.insert(part.mesh_key());
            self.coat_keys.insert(part.coat_key);
        }
        self.coat_keys
            .extend(recipe.join_covers.iter().map(|cover| cover.coat_key));
    }

    pub fn mesh_key_count(&self) -> usize {
        self.mesh_keys.len()
    }

    pub fn coat_key_count(&self) -> usize {
        self.coat_keys.len()
    }

    pub fn requested_mesh_count(&self) -> usize {
        self.requested_meshes
    }

    pub fn mesh_keys(&self) -> impl Iterator<Item = &CreaturePartMeshKey> {
        self.mesh_keys.iter()
    }
}

pub fn resolve_geneforge_creature_assembly(
    requested_sources: CreaturePartSources,
    lod: CreaturePartLodId,
    coat_key: CreatureCoatKey,
    catalog: &GeneForgeCreaturePartCatalog,
    preparations: &GeneForgeAssemblyPreparationIndex,
) -> Result<CreatureAssemblyRecipe, CreatureAssemblyError> {
    let display = resolve_creature_part_display_sources(requested_sources, catalog)?;
    let displayed_sources = display.displayed_sources;
    let warning = display.warning;
    let torso_family = gene_forge_family(catalog, displayed_sources.torso)
        .ok_or(CreatureAssemblyError::EmptyGeneForgeCatalog)?;
    let target_torso_asset_id = torso_family
        .parts
        .get(&CreaturePartSlotKey::Torso)
        .ok_or(CreatureAssemblyError::MissingGeneForgePart(torso_family.id))?
        .asset_id
        .clone();

    let mut parts = BTreeMap::new();
    for slot in CreaturePartSlot::ALL {
        let source_family_id = family_for_slot(displayed_sources, slot);
        let source_family = gene_forge_family(catalog, source_family_id).ok_or(
            CreatureAssemblyError::MissingGeneForgePart(source_family_id),
        )?;
        let logical_slot = logical_slot(slot);
        let part_recipe = source_family.parts.get(&logical_slot).ok_or(
            CreatureAssemblyError::MissingGeneForgePart(source_family_id),
        )?;
        let asset = catalog.asset(&part_recipe.asset_id).ok_or_else(|| {
            CreatureAssemblyError::MissingGeneForgeAsset(part_recipe.asset_id.clone())
        })?;
        let runtime_group = runtime_group(slot).to_string();
        let socket = socket_name(slot).unwrap_or("torso-frame").to_string();
        let key = GeneForgePreparationKey {
            source_family: source_family_id,
            source_asset_id: part_recipe.asset_id.clone(),
            target_torso_asset_id: target_torso_asset_id.clone(),
            lod,
            runtime_group: runtime_group.clone(),
            socket: socket.clone(),
        };
        let preparation = preparations
            .get(&key)
            .ok_or_else(|| CreatureAssemblyError::MissingPreparation { key: key.clone() })?;
        let canonical_bounds =
            CreatureVisualBounds::new(asset.canonical_bounds.min, asset.canonical_bounds.max);
        let transformed_bounds = transform_bounds(canonical_bounds, preparation.prepared_matrix)?;
        parts.insert(
            slot,
            CreatureAssemblyPartRecipe {
                source_family: source_family_id,
                asset_id: part_recipe.asset_id.clone(),
                target_torso_asset_id: target_torso_asset_id.clone(),
                lod,
                slot,
                runtime_group,
                socket,
                authored_transform: preparation.prepared_matrix,
                canonical_bounds,
                transformed_bounds,
                landmarks: asset.landmarks.clone(),
                attachment_residual: preparation.residual,
                socket_evidence: preparation.socket_evidence.clone(),
                coat_key,
            },
        );
    }

    let join_covers = if lod == CreaturePartLodId::Impostor {
        Vec::new()
    } else {
        parts
            .values()
            .filter(|part| part.slot != CreaturePartSlot::Torso)
            .map(|part| {
                let source_family = gene_forge_family(catalog, part.source_family).ok_or(
                    CreatureAssemblyError::MissingGeneForgePart(part.source_family),
                )?;
                let cover_kind = source_family
                    .parts
                    .get(&logical_slot(part.slot))
                    .ok_or(CreatureAssemblyError::MissingGeneForgePart(
                        part.source_family,
                    ))?
                    .join_cover_kind
                    .clone();
                Ok(ResolvedJoinCover {
                    slot: part.slot,
                    runtime_group: part.runtime_group.clone(),
                    cover_kind,
                    authored_transform: part.authored_transform,
                    overlap_depth: catalog.assembly_contract.default_overlap_depth,
                    coat_key,
                })
            })
            .collect::<Result<Vec<_>, CreatureAssemblyError>>()?
    };

    Ok(CreatureAssemblyRecipe {
        saved_sources: requested_sources,
        displayed_sources,
        target_torso_asset_id,
        parts,
        join_covers,
        coat_key,
        warning,
        display_only: true,
    })
}

pub fn resolve_creature_part_display_sources(
    requested: CreaturePartSources,
    catalog: &GeneForgeCreaturePartCatalog,
) -> Result<CreaturePartDisplayResolution, CreatureAssemblyError> {
    let fallback = catalog
        .families
        .iter()
        .min_by_key(|family| family.id)
        .map(|family| family.id)
        .ok_or(CreatureAssemblyError::EmptyGeneForgeCatalog)?;
    if gene_forge_family(catalog, requested.torso).is_none() {
        return Ok(CreaturePartDisplayResolution {
            saved_sources: requested,
            displayed_sources: CreaturePartSources::coherent(fallback),
            warning: Some(CreatureAssemblyWarning::UnknownFamilyFallback {
                requested: requested.torso,
                fallback,
            }),
            display_only: true,
        });
    }
    let mut displayed = requested;
    let mut warning = None;
    for slot in [
        CreaturePartSlot::Head,
        CreaturePartSlot::LeftArm,
        CreaturePartSlot::LeftLeg,
        CreaturePartSlot::TailBack,
    ] {
        let source = family_for_slot(displayed, slot);
        if gene_forge_family(catalog, source).is_some() {
            continue;
        }
        displayed = with_family_for_slot(displayed, slot, displayed.torso);
        warning.get_or_insert(CreatureAssemblyWarning::UnknownFamilyFallback {
            requested: source,
            fallback: displayed.torso,
        });
    }
    Ok(CreaturePartDisplayResolution {
        saved_sources: requested,
        displayed_sources: displayed,
        warning,
        display_only: true,
    })
}

fn gene_forge_family(
    catalog: &GeneForgeCreaturePartCatalog,
    id: CreaturePartFamilyId,
) -> Option<&crate::GeneForgeCreatureFamilyDefinition> {
    catalog.families.iter().find(|family| family.id == id)
}

fn logical_slot(slot: CreaturePartSlot) -> CreaturePartSlotKey {
    match slot {
        CreaturePartSlot::Head => CreaturePartSlotKey::Head,
        CreaturePartSlot::Torso => CreaturePartSlotKey::Torso,
        CreaturePartSlot::LeftArm | CreaturePartSlot::RightArm => CreaturePartSlotKey::Arms,
        CreaturePartSlot::LeftLeg | CreaturePartSlot::RightLeg => CreaturePartSlotKey::Legs,
        CreaturePartSlot::TailBack => CreaturePartSlotKey::Tail,
    }
}

fn runtime_group(slot: CreaturePartSlot) -> &'static str {
    match slot {
        CreaturePartSlot::Head => "head",
        CreaturePartSlot::Torso => "torso",
        CreaturePartSlot::LeftArm => "left-arm",
        CreaturePartSlot::RightArm => "right-arm",
        CreaturePartSlot::LeftLeg => "left-leg",
        CreaturePartSlot::RightLeg => "right-leg",
        CreaturePartSlot::TailBack => "tail-back",
    }
}

fn validate_preparation_evidence(
    catalog: &GeneForgeCreaturePartCatalog,
    record: &GeneForgeGroupTransform,
) -> Result<(), CreatureAssemblyError> {
    if record.socket_evidence.is_empty() {
        return Err(CreatureAssemblyError::InvalidPreparation {
            reason: "socket evidence is required".into(),
        });
    }
    for evidence in &record.socket_evidence {
        let finite = evidence
            .source_anchor
            .into_iter()
            .chain(evidence.target_anchor)
            .chain(evidence.transformed_source_anchor)
            .chain([evidence.residual, evidence.applied_overlap_depth])
            .all(f64::is_finite);
        let transformed = transform_point_f64(record.prepared_matrix, evidence.source_anchor);
        let authored_transform_error =
            euclidean_distance(transformed, evidence.transformed_source_anchor);
        let measured_residual =
            euclidean_distance(evidence.transformed_source_anchor, evidence.target_anchor);
        if !finite
            || evidence.socket != record.socket
            || evidence.prepared_vertex_count == 0
            || !(0.0..=catalog.assembly_preparation_contract.residual_limit)
                .contains(&evidence.residual)
            || evidence.applied_overlap_depth < 0.0
            || authored_transform_error > 1.0e-9
            || (measured_residual - evidence.residual).abs() > 1.0e-9
            || (evidence.residual - record.residual).abs() > 1.0e-9
        {
            return Err(CreatureAssemblyError::InvalidPreparation {
                reason: format!("invalid socket evidence for {}", record.runtime_group),
            });
        }
    }
    Ok(())
}

fn transform_point_f64(matrix: [f64; 16], point: [f64; 3]) -> [f64; 3] {
    [
        matrix[0] * point[0] + matrix[1] * point[1] + matrix[2] * point[2] + matrix[3],
        matrix[4] * point[0] + matrix[5] * point[1] + matrix[6] * point[2] + matrix[7],
        matrix[8] * point[0] + matrix[9] * point[1] + matrix[10] * point[2] + matrix[11],
    ]
}

fn euclidean_distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    left.into_iter()
        .zip(right)
        .map(|(left, right)| (left - right).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn transform_bounds(
    bounds: CreatureVisualBounds,
    matrix: [f64; 16],
) -> Result<CreatureVisualBounds, CreatureAssemblyError> {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for corner in bounds.corners() {
        let point = [
            matrix[0] * f64::from(corner[0])
                + matrix[1] * f64::from(corner[1])
                + matrix[2] * f64::from(corner[2])
                + matrix[3],
            matrix[4] * f64::from(corner[0])
                + matrix[5] * f64::from(corner[1])
                + matrix[6] * f64::from(corner[2])
                + matrix[7],
            matrix[8] * f64::from(corner[0])
                + matrix[9] * f64::from(corner[1])
                + matrix[10] * f64::from(corner[2])
                + matrix[11],
        ];
        if !point.into_iter().all(f64::is_finite)
            || point
                .into_iter()
                .any(|component| component < f64::from(f32::MIN) || component > f64::from(f32::MAX))
        {
            return Err(CreatureAssemblyError::InvalidPreparation {
                reason: "prepared bounds are non-finite".into(),
            });
        }
        for axis in 0..3 {
            let component = point[axis] as f32;
            min[axis] = min[axis].min(component);
            max[axis] = max[axis].max(component);
        }
    }
    let transformed = CreatureVisualBounds::new(min, max);
    if !transformed.is_valid() {
        return Err(CreatureAssemblyError::InvalidPreparation {
            reason: "prepared bounds are invalid".into(),
        });
    }
    Ok(transformed)
}

fn family_for_slot(sources: CreaturePartSources, slot: CreaturePartSlot) -> CreaturePartFamilyId {
    match slot {
        CreaturePartSlot::Head => sources.head,
        CreaturePartSlot::Torso => sources.torso,
        CreaturePartSlot::LeftArm | CreaturePartSlot::RightArm => sources.arms,
        CreaturePartSlot::LeftLeg | CreaturePartSlot::RightLeg => sources.legs,
        CreaturePartSlot::TailBack => sources.tail,
    }
}

fn with_family_for_slot(
    mut sources: CreaturePartSources,
    slot: CreaturePartSlot,
    family: CreaturePartFamilyId,
) -> CreaturePartSources {
    match slot {
        CreaturePartSlot::Head => sources.head = family,
        CreaturePartSlot::Torso => sources.torso = family,
        CreaturePartSlot::LeftArm | CreaturePartSlot::RightArm => sources.arms = family,
        CreaturePartSlot::LeftLeg | CreaturePartSlot::RightLeg => sources.legs = family,
        CreaturePartSlot::TailBack => sources.tail = family,
    }
    sources
}

fn socket_name(slot: CreaturePartSlot) -> Option<&'static str> {
    match slot {
        CreaturePartSlot::Head => Some("neck"),
        CreaturePartSlot::Torso => None,
        CreaturePartSlot::LeftArm => Some("left-shoulder"),
        CreaturePartSlot::RightArm => Some("right-shoulder"),
        CreaturePartSlot::LeftLeg => Some("left-hip"),
        CreaturePartSlot::RightLeg => Some("right-hip"),
        CreaturePartSlot::TailBack => Some("tail-base"),
    }
}

fn validate_frame(frame: SocketFrame) -> Result<(), CreatureAssemblyError> {
    if !frame
        .translation
        .into_iter()
        .chain(frame.rotation_xyzw)
        .chain(frame.scale)
        .all(f32::is_finite)
        || !frame
            .scale
            .into_iter()
            .all(|value| (0.25..=4.0).contains(&value))
    {
        return Err(CreatureAssemblyError::InvalidTransform);
    }
    Ok(())
}

#[cfg(test)]
mod geneforge_tests {
    use std::collections::BTreeSet;

    use alife_world::{CreaturePartFamilyId, CreaturePartSlotKey, CreaturePartSources};

    use super::*;
    use crate::{
        load_geneforge_creature_part_catalog, CreatureCoatKey, CreaturePartAssetId,
        GeneForgeCreaturePartCatalog, GeneForgeGroupTransform, GeneForgeSocketEvidence,
    };

    fn authored_matrix(
        family: u16,
        target: usize,
        lod: CreaturePartLodId,
        group: usize,
    ) -> [f64; 16] {
        let lod = match lod {
            CreaturePartLodId::Full => 0.0,
            CreaturePartLodId::Compact => 0.01,
            CreaturePartLodId::Impostor => 0.02,
        };
        [
            1.0,
            0.0,
            0.0,
            f64::from(family) * 0.001,
            0.0,
            1.0,
            0.0,
            target as f64 * 0.002 + lod,
            0.0,
            0.0,
            1.0,
            group as f64 * -0.001,
            0.0,
            0.0,
            0.0,
            1.0,
        ]
    }

    fn runtime_groups(slot: CreaturePartSlotKey) -> &'static [(&'static str, &'static str)] {
        match slot {
            CreaturePartSlotKey::Head => &[("head", "neck")],
            CreaturePartSlotKey::Torso => &[("torso", "torso-frame")],
            CreaturePartSlotKey::Arms => &[
                ("left-arm", "left-shoulder"),
                ("right-arm", "right-shoulder"),
            ],
            CreaturePartSlotKey::Legs => &[("left-leg", "left-hip"), ("right-leg", "right-hip")],
            CreaturePartSlotKey::Tail => &[("tail-back", "tail-base")],
        }
    }

    fn authored_records(catalog: &GeneForgeCreaturePartCatalog) -> Vec<GeneForgeGroupTransform> {
        let torso_assets = ["norn-torso", "ettin-torso", "grendel-torso"];
        let lods = [
            CreaturePartLodId::Full,
            CreaturePartLodId::Compact,
            CreaturePartLodId::Impostor,
        ];
        let mut records = Vec::new();
        for family in &catalog.families {
            let canonical_torso = &family.parts[&CreaturePartSlotKey::Torso].asset_id;
            for (target_index, target) in torso_assets.into_iter().enumerate() {
                let target_torso_asset_id = CreaturePartAssetId(target.into());
                let cross_torso = target_torso_asset_id != *canonical_torso;
                for lod in lods {
                    for logical_slot in [
                        CreaturePartSlotKey::Head,
                        CreaturePartSlotKey::Torso,
                        CreaturePartSlotKey::Arms,
                        CreaturePartSlotKey::Legs,
                        CreaturePartSlotKey::Tail,
                    ] {
                        if cross_torso && logical_slot == CreaturePartSlotKey::Torso {
                            continue;
                        }
                        let asset_id = family.parts[&logical_slot].asset_id.clone();
                        for (group_index, (runtime_group, socket)) in
                            runtime_groups(logical_slot).iter().enumerate()
                        {
                            let matrix =
                                authored_matrix(family.id.0, target_index, lod, group_index);
                            let source_anchor = [0.0, 0.0, 0.0];
                            let transformed_source_anchor = [matrix[3], matrix[7], matrix[11]];
                            let target_anchor = [matrix[3] + 0.001, matrix[7], matrix[11]];
                            records.push(GeneForgeGroupTransform {
                                source_family_id: family.id.0,
                                source_asset_id: asset_id.clone(),
                                target_torso_asset_id: target_torso_asset_id.clone(),
                                lod,
                                runtime_group: (*runtime_group).into(),
                                socket: (*socket).into(),
                                transform_space: catalog
                                    .assembly_preparation_contract
                                    .transform_space
                                    .clone(),
                                schema_digest: catalog
                                    .assembly_preparation_contract
                                    .schema_digest
                                    .clone(),
                                prepared_matrix: matrix,
                                residual: 0.001,
                                socket_evidence: vec![GeneForgeSocketEvidence {
                                    socket: (*socket).into(),
                                    source_anchor,
                                    target_anchor,
                                    transformed_source_anchor,
                                    residual: 0.001,
                                    prepared_vertex_count: 12,
                                    applied_overlap_depth: 0.01,
                                }],
                            });
                        }
                    }
                }
            }
        }
        records
    }

    fn coat_key(sources: CreaturePartSources) -> CreatureCoatKey {
        CreatureCoatKey::new(sources, 3, 5, 7)
    }

    #[test]
    fn all_twelve_families_resolve_exact_authored_assets_at_full_and_compact() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        let records = authored_records(&catalog);
        assert_eq!(records.len(), 684);
        let preparations = GeneForgeAssemblyPreparationIndex::new(&catalog, records).unwrap();

        for family in &catalog.families {
            for lod in [CreaturePartLodId::Full, CreaturePartLodId::Compact] {
                let sources = CreaturePartSources::coherent(family.id);
                let recipe = resolve_geneforge_creature_assembly(
                    sources,
                    lod,
                    coat_key(sources),
                    &catalog,
                    &preparations,
                )
                .unwrap();
                for slot in CreaturePartSlot::ALL {
                    let logical_slot = logical_slot(slot);
                    assert_eq!(
                        recipe.parts[&slot].asset_id,
                        family.parts[&logical_slot].asset_id
                    );
                    assert_eq!(recipe.parts[&slot].lod, lod);
                }
            }
        }
    }

    #[test]
    fn mixed_assembly_resolves_saved_slot_assets_against_target_torso() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        let preparations =
            GeneForgeAssemblyPreparationIndex::new(&catalog, authored_records(&catalog)).unwrap();
        let sources = CreaturePartSources {
            head: CreaturePartFamilyId(11),
            torso: CreaturePartFamilyId(9),
            arms: CreaturePartFamilyId(8),
            legs: CreaturePartFamilyId(10),
            tail: CreaturePartFamilyId(6),
        };
        let recipe = resolve_geneforge_creature_assembly(
            sources,
            CreaturePartLodId::Full,
            coat_key(sources),
            &catalog,
            &preparations,
        )
        .unwrap();

        let expected = [
            (CreaturePartSlot::Head, "grendel-head"),
            (CreaturePartSlot::Torso, "grendel-torso"),
            (CreaturePartSlot::LeftArm, "norn-arms"),
            (CreaturePartSlot::RightArm, "norn-arms"),
            (CreaturePartSlot::LeftLeg, "norn-legs"),
            (CreaturePartSlot::RightLeg, "norn-legs"),
            (CreaturePartSlot::TailBack, "grendel-tail"),
        ];
        for (slot, asset_id) in expected {
            let part = &recipe.parts[&slot];
            assert_eq!(part.asset_id, CreaturePartAssetId(asset_id.into()));
            assert_eq!(
                part.target_torso_asset_id,
                CreaturePartAssetId("grendel-torso".into())
            );
        }
    }

    #[test]
    fn exact_target_lod_matrix_bounds_and_residual_evidence_survive_resolution() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        let records = authored_records(&catalog);
        let preparations = GeneForgeAssemblyPreparationIndex::new(&catalog, records).unwrap();
        let sources = CreaturePartSources {
            head: CreaturePartFamilyId(0),
            torso: CreaturePartFamilyId(1),
            arms: CreaturePartFamilyId(0),
            legs: CreaturePartFamilyId(0),
            tail: CreaturePartFamilyId(0),
        };
        let full = resolve_geneforge_creature_assembly(
            sources,
            CreaturePartLodId::Full,
            coat_key(sources),
            &catalog,
            &preparations,
        )
        .unwrap();
        let compact = resolve_geneforge_creature_assembly(
            sources,
            CreaturePartLodId::Compact,
            coat_key(sources),
            &catalog,
            &preparations,
        )
        .unwrap();
        let full_head = &full.parts[&CreaturePartSlot::Head];
        let compact_head = &compact.parts[&CreaturePartSlot::Head];
        assert_ne!(
            full_head.authored_transform,
            compact_head.authored_transform
        );
        assert!(full_head.authored_transform.into_iter().all(f64::is_finite));
        assert_eq!(full_head.authored_transform[12..], [0.0, 0.0, 0.0, 1.0]);
        assert!(full_head.canonical_bounds.is_valid());
        assert!(full_head.transformed_bounds.is_valid());
        assert!(full_head.attachment_residual <= 0.025);
        assert!(!full_head.landmarks.is_empty());
        assert_eq!(full_head.socket_evidence.len(), 1);
    }

    #[test]
    fn mesh_keys_deduplicate_by_asset_lod_group_and_one_coat_covers_entire_assembly() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        let preparations =
            GeneForgeAssemblyPreparationIndex::new(&catalog, authored_records(&catalog)).unwrap();
        let mut cache = CreaturePartAssetKeyCache::default();
        let mut coat_keys = BTreeSet::new();
        for family_id in [0, 3, 6, 9] {
            let sources = CreaturePartSources::coherent(CreaturePartFamilyId(family_id));
            let recipe = resolve_geneforge_creature_assembly(
                sources,
                CreaturePartLodId::Compact,
                coat_key(sources),
                &catalog,
                &preparations,
            )
            .unwrap();
            cache.register_geneforge_recipe(&recipe);
            coat_keys.extend(recipe.parts.values().map(|part| part.coat_key));
            coat_keys.extend(recipe.join_covers.iter().map(|cover| cover.coat_key));
        }
        assert_eq!(cache.requested_mesh_count(), 28);
        assert_eq!(cache.mesh_key_count(), 17);
        assert_eq!(cache.coat_key_count(), 4);
        assert_eq!(coat_keys.len(), 4);
        assert!(cache
            .mesh_keys()
            .all(|key| !key.asset_id.0.is_empty() && !key.runtime_group.is_empty()));
    }

    #[test]
    fn preparation_index_rejects_duplicate_wrong_target_missing_group_and_non_finite_records() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        let records = authored_records(&catalog);

        let mut duplicate = records.clone();
        duplicate.push(records[0].clone());
        assert!(matches!(
            GeneForgeAssemblyPreparationIndex::new(&catalog, duplicate),
            Err(CreatureAssemblyError::DuplicatePreparation { .. })
        ));

        let mut non_finite = records.clone();
        non_finite[0].prepared_matrix[3] = f64::NAN;
        assert!(matches!(
            GeneForgeAssemblyPreparationIndex::new(&catalog, non_finite),
            Err(CreatureAssemblyError::InvalidPreparation { .. })
        ));

        let mut forged_evidence = records.clone();
        forged_evidence[0].socket_evidence[0].transformed_source_anchor[0] += 1.0;
        assert!(matches!(
            GeneForgeAssemblyPreparationIndex::new(&catalog, forged_evidence),
            Err(CreatureAssemblyError::InvalidPreparation { .. })
        ));

        let preparations = GeneForgeAssemblyPreparationIndex::new(&catalog, records).unwrap();
        let sources = CreaturePartSources::coherent(CreaturePartFamilyId(0));
        let wrong_target = preparations.without_target(&CreaturePartAssetId("ettin-torso".into()));
        assert!(matches!(
            resolve_geneforge_creature_assembly(
                sources,
                CreaturePartLodId::Full,
                coat_key(sources),
                &catalog,
                &wrong_target,
            ),
            Err(CreatureAssemblyError::MissingPreparation { .. })
        ));

        let missing_group = preparations.without_runtime_group("head");
        assert!(matches!(
            resolve_geneforge_creature_assembly(
                sources,
                CreaturePartLodId::Full,
                coat_key(sources),
                &catalog,
                &missing_group,
            ),
            Err(CreatureAssemblyError::MissingPreparation { .. })
        ));
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use std::collections::BTreeSet;

    use alife_world::{CreaturePartFamilyId, CreaturePartSources};

    use super::*;
    use crate::{load_production_creature_part_catalog, CreaturePartLodId, CreaturePartSlot};

    #[test]
    fn mixed_recipe_uses_saved_sources_and_torso_sockets() {
        let sources = CreaturePartSources {
            head: CreaturePartFamilyId(1),
            torso: CreaturePartFamilyId(0),
            arms: CreaturePartFamilyId(6),
            legs: CreaturePartFamilyId(0),
            tail: CreaturePartFamilyId(7),
        };
        let recipe = resolve_creature_assembly(
            sources,
            CreaturePartLodId::Compact,
            &load_production_creature_part_catalog().unwrap(),
        )
        .unwrap();

        assert_eq!(recipe.root_family, CreaturePartFamilyId(0));
        assert_eq!(
            recipe.parts[&CreaturePartSlot::Head].family,
            CreaturePartFamilyId(1)
        );
        assert!(recipe.join_covers.len() >= 5);
        assert!(recipe.display_only);
    }

    #[test]
    fn unknown_family_uses_coherent_visible_fallback() {
        let recipe = resolve_creature_assembly(
            CreaturePartSources::coherent(CreaturePartFamilyId(999)),
            CreaturePartLodId::Compact,
            &load_production_creature_part_catalog().unwrap(),
        )
        .unwrap();

        assert_eq!(
            recipe.warning,
            Some(CreatureAssemblyWarning::UnknownFamilyFallback {
                requested: CreaturePartFamilyId(999),
                fallback: CreaturePartFamilyId(0),
            })
        );
        assert_eq!(
            recipe
                .parts
                .values()
                .map(|part| part.family)
                .collect::<BTreeSet<_>>()
                .len(),
            1
        );
    }

    #[test]
    fn unknown_attached_family_falls_back_to_saved_torso() {
        let torso = CreaturePartFamilyId(3);
        let recipe = resolve_creature_assembly(
            CreaturePartSources {
                head: CreaturePartFamilyId(999),
                torso,
                arms: CreaturePartFamilyId(6),
                legs: CreaturePartFamilyId(0),
                tail: CreaturePartFamilyId(7),
            },
            CreaturePartLodId::Compact,
            &load_production_creature_part_catalog().unwrap(),
        )
        .unwrap();

        assert_eq!(recipe.root_family, torso);
        assert_eq!(recipe.parts[&CreaturePartSlot::Head].family, torso);
        assert_eq!(
            recipe.parts[&CreaturePartSlot::LeftArm].family,
            CreaturePartFamilyId(6)
        );
        assert_eq!(
            recipe.warning,
            Some(CreatureAssemblyWarning::UnknownFamilyFallback {
                requested: CreaturePartFamilyId(999),
                fallback: torso,
            })
        );
    }
}
