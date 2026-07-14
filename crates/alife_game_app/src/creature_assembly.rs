use std::collections::{BTreeMap, BTreeSet};

use alife_world::{CreaturePartFamilyId, CreaturePartSources};
use thiserror::Error;

use crate::{CreaturePartCatalog, CreaturePartLodId, CreaturePartSlot, SocketFrame};

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureAssemblyPartRecipe {
    pub family: CreaturePartFamilyId,
    pub lod: CreaturePartLodId,
    pub slot: CreaturePartSlot,
    pub mesh_asset_path: String,
    pub texture_asset_path: String,
    pub socket: SocketFrame,
    pub local_scale: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedJoinCover {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreatureAssemblyWarning {
    UnknownFamilyFallback {
        requested: CreaturePartFamilyId,
        fallback: CreaturePartFamilyId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureAssemblyRecipe {
    pub root_family: CreaturePartFamilyId,
    pub parts: BTreeMap<CreaturePartSlot, CreatureAssemblyPartRecipe>,
    pub join_covers: Vec<ResolvedJoinCover>,
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
}

pub fn resolve_creature_assembly(
    requested_sources: CreaturePartSources,
    lod: CreaturePartLodId,
    catalog: &CreaturePartCatalog,
) -> Result<CreatureAssemblyRecipe, CreatureAssemblyError> {
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
            CreatureAssemblyPartRecipe {
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
            Ok(ResolvedJoinCover {
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
    Ok(CreatureAssemblyRecipe {
        root_family: sources.torso,
        parts,
        join_covers,
        warning,
        display_only: true,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreaturePartMeshKey {
    pub family: CreaturePartFamilyId,
    pub lod: CreaturePartLodId,
    pub slot: CreaturePartSlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreaturePartMaterialKey {
    pub family: CreaturePartFamilyId,
    pub palette_family: u8,
    pub fur_pattern: u8,
    pub expression_bucket: u8,
}

#[derive(Debug, Default)]
pub struct CreaturePartAssetKeyCache {
    mesh_keys: BTreeSet<CreaturePartMeshKey>,
    material_keys: BTreeSet<CreaturePartMaterialKey>,
    requested_meshes: usize,
}

impl CreaturePartAssetKeyCache {
    pub fn register_recipe(
        &mut self,
        recipe: &CreatureAssemblyRecipe,
        palette_family: u8,
        fur_pattern: u8,
        expression_bucket: u8,
    ) {
        for part in recipe.parts.values() {
            self.requested_meshes += 1;
            self.mesh_keys.insert(CreaturePartMeshKey {
                family: part.family,
                lod: part.lod,
                slot: part.slot,
            });
            self.material_keys.insert(CreaturePartMaterialKey {
                family: part.family,
                palette_family,
                fur_pattern,
                expression_bucket,
            });
        }
    }

    pub fn mesh_key_count(&self) -> usize {
        self.mesh_keys.len()
    }

    pub fn material_key_count(&self) -> usize {
        self.material_keys.len()
    }

    pub fn requested_mesh_count(&self) -> usize {
        self.requested_meshes
    }
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

    #[test]
    fn thirty_creatures_reuse_bounded_asset_keys() {
        let catalog = load_production_creature_part_catalog().unwrap();
        let mut cache = CreaturePartAssetKeyCache::default();
        for index in 0..30_u16 {
            let family = CreaturePartFamilyId(index % 8);
            let recipe = resolve_creature_assembly(
                CreaturePartSources::coherent(family),
                CreaturePartLodId::Compact,
                &catalog,
            )
            .unwrap();
            cache.register_recipe(&recipe, index as u8 % 16, index as u8 % 16, 0);
        }
        assert!(cache.mesh_key_count() <= 8 * 7);
        assert!(cache.mesh_key_count() < cache.requested_mesh_count());
    }
}
