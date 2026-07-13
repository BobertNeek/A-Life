use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
};

use alife_world::CreaturePartFamilyId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CREATURE_PART_CATALOG_SCHEMA: &str = "alife.creature_part_catalog.v1";
const PRODUCTION_CATALOG_JSON: &str =
    include_str!("../assets/production_voxel_v1/creature_parts/catalog.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CreaturePartSlot {
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
    TailBack,
}

impl CreaturePartSlot {
    pub const REQUIRED_RUNTIME_SLOTS: [Self; 6] = [
        Self::Head,
        Self::Torso,
        Self::LeftArm,
        Self::RightArm,
        Self::LeftLeg,
        Self::RightLeg,
    ];

    pub const ALL: [Self; 7] = [
        Self::Head,
        Self::Torso,
        Self::LeftArm,
        Self::RightArm,
        Self::LeftLeg,
        Self::RightLeg,
        Self::TailBack,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CreaturePartLodId {
    Full,
    Compact,
    Impostor,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SocketFrame {
    pub translation: [f32; 3],
    pub rotation_xyzw: [f32; 4],
    pub scale: [f32; 3],
}

impl SocketFrame {
    pub const IDENTITY: Self = Self {
        translation: [0.0; 3],
        rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0; 3],
    };

    fn validate(self) -> Result<(), CreaturePartCatalogError> {
        if !self
            .translation
            .into_iter()
            .chain(self.rotation_xyzw)
            .chain(self.scale)
            .all(f32::is_finite)
        {
            return Err(CreaturePartCatalogError::InvalidScalar("socket frame"));
        }
        let norm_squared = self
            .rotation_xyzw
            .into_iter()
            .map(|value| value * value)
            .sum::<f32>();
        if (norm_squared - 1.0).abs() > 1.0e-3 {
            return Err(CreaturePartCatalogError::InvalidScalar("socket quaternion"));
        }
        if !self
            .scale
            .into_iter()
            .all(|value| (0.25..=4.0).contains(&value))
        {
            return Err(CreaturePartCatalogError::InvalidScalar("socket scale"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CutPlane {
    pub normal: [f32; 3],
    pub offset: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CutVolume {
    pub planes: Vec<CutPlane>,
}

impl CutVolume {
    fn validate(&self) -> Result<(), CreaturePartCatalogError> {
        if self.planes.len() < 4 {
            return Err(CreaturePartCatalogError::InvalidCutVolume);
        }
        for plane in &self.planes {
            if !plane.offset.is_finite() || !plane.normal.into_iter().all(f32::is_finite) {
                return Err(CreaturePartCatalogError::InvalidCutVolume);
            }
            let length_squared = plane.normal.into_iter().map(|v| v * v).sum::<f32>();
            if length_squared <= 1.0e-6 {
                return Err(CreaturePartCatalogError::InvalidCutVolume);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoinCoverDefinition {
    pub slot: CreaturePartSlot,
    pub socket: String,
    pub cover_kind: String,
    pub overlap_depth: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreaturePartLod {
    pub lod: CreaturePartLodId,
    pub source_obj: String,
    pub generated_obj: String,
    pub socket_manifest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreaturePartFamilyDefinition {
    pub id: CreaturePartFamilyId,
    pub label: String,
    #[serde(default)]
    pub template_family: Option<CreaturePartFamilyId>,
    pub texture_asset: String,
    pub compatibility_tags: BTreeSet<String>,
    pub ordinary_substitutions: BTreeMap<CreaturePartSlot, BTreeSet<String>>,
    pub source_to_canonical: SocketFrame,
    pub cuts: BTreeMap<CreaturePartSlot, CutVolume>,
    pub sockets: BTreeMap<String, SocketFrame>,
    pub join_covers: Vec<JoinCoverDefinition>,
    pub lods: Vec<CreaturePartLod>,
}

impl CreaturePartFamilyDefinition {
    pub fn validate(&self) -> Result<(), CreaturePartCatalogError> {
        self.id
            .validate()
            .map_err(|_| CreaturePartCatalogError::InvalidFamilyId(self.id))?;
        if self.label.is_empty() || self.compatibility_tags.len() < 2 {
            return Err(CreaturePartCatalogError::InvalidFamily(self.id));
        }
        validate_production_path(&self.texture_asset)?;
        self.source_to_canonical.validate()?;

        for slot in CreaturePartSlot::ALL {
            self.cuts
                .get(&slot)
                .ok_or(CreaturePartCatalogError::MissingSlot(self.id, slot))?
                .validate()?;
        }
        for slot in CreaturePartSlot::REQUIRED_RUNTIME_SLOTS {
            if !self.ordinary_substitutions.contains_key(&slot) {
                return Err(CreaturePartCatalogError::MissingSlot(self.id, slot));
            }
        }

        const REQUIRED_SOCKETS: [&str; 6] = [
            "neck",
            "left-shoulder",
            "right-shoulder",
            "left-hip",
            "right-hip",
            "tail-base",
        ];
        for socket in REQUIRED_SOCKETS {
            self.sockets
                .get(socket)
                .ok_or_else(|| CreaturePartCatalogError::MissingSocket(socket.to_string()))?
                .validate()?;
        }

        if self.join_covers.len() < 5 || self.join_covers.len() > 12 {
            return Err(CreaturePartCatalogError::InvalidFamily(self.id));
        }
        for cover in &self.join_covers {
            if cover.cover_kind.is_empty()
                || !cover.overlap_depth.is_finite()
                || !(0.005..=0.25).contains(&cover.overlap_depth)
                || !self.sockets.contains_key(&cover.socket)
            {
                return Err(CreaturePartCatalogError::InvalidFamily(self.id));
            }
        }

        let lods = self.lods.iter().map(|lod| lod.lod).collect::<BTreeSet<_>>();
        if lods
            != BTreeSet::from([
                CreaturePartLodId::Full,
                CreaturePartLodId::Compact,
                CreaturePartLodId::Impostor,
            ])
            || self.lods.len() != 3
        {
            return Err(CreaturePartCatalogError::InvalidLods(self.id));
        }
        for lod in &self.lods {
            validate_developer_source_path(&lod.source_obj)?;
            validate_generated_path(&lod.generated_obj)?;
            validate_generated_path(&lod.socket_manifest)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreaturePartCatalog {
    pub schema: String,
    pub schema_version: u16,
    pub families: Vec<CreaturePartFamilyDefinition>,
}

impl CreaturePartCatalog {
    pub fn from_json_str(text: &str) -> Result<Self, CreaturePartCatalogError> {
        let mut catalog: Self = serde_json::from_str(text)?;
        catalog.expand_templates()?;
        catalog.validate()?;
        Ok(catalog)
    }

    fn expand_templates(&mut self) -> Result<(), CreaturePartCatalogError> {
        let templates = self
            .families
            .iter()
            .map(|family| (family.id, family.clone()))
            .collect::<BTreeMap<_, _>>();
        for family in &mut self.families {
            let Some(template_id) = family.template_family else {
                continue;
            };
            let template = templates
                .get(&template_id)
                .ok_or(CreaturePartCatalogError::UnknownTemplateFamily(template_id))?;
            if family.cuts.is_empty() {
                family.cuts = template.cuts.clone();
            }
            if family.sockets.is_empty() {
                family.sockets = template.sockets.clone();
            }
            if family.join_covers.is_empty() {
                family.join_covers = template.join_covers.clone();
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), CreaturePartCatalogError> {
        if self.schema != CREATURE_PART_CATALOG_SCHEMA || self.schema_version != 1 {
            return Err(CreaturePartCatalogError::Schema);
        }
        if self.families.is_empty() {
            return Err(CreaturePartCatalogError::Empty);
        }
        let mut ids = BTreeSet::new();
        let mut labels = BTreeSet::new();
        for family in &self.families {
            if !ids.insert(family.id) {
                return Err(CreaturePartCatalogError::DuplicateFamilyId(family.id));
            }
            if !labels.insert(family.label.as_str()) {
                return Err(CreaturePartCatalogError::DuplicateFamilyLabel(
                    family.label.clone(),
                ));
            }
            family.validate()?;
        }
        Ok(())
    }

    pub fn family(&self, id: CreaturePartFamilyId) -> Option<&CreaturePartFamilyDefinition> {
        self.families.iter().find(|family| family.id == id)
    }

    pub fn ordinarily_compatible(
        &self,
        torso: CreaturePartFamilyId,
        slot: CreaturePartSlot,
        candidate: CreaturePartFamilyId,
    ) -> bool {
        if torso == candidate {
            return self.family(torso).is_some();
        }
        let Some(torso) = self.family(torso) else {
            return false;
        };
        let Some(candidate) = self.family(candidate) else {
            return false;
        };
        let Some(required_tags) = torso.ordinary_substitutions.get(&slot) else {
            return false;
        };
        required_tags.is_subset(&candidate.compatibility_tags)
    }

    pub fn coherent_fallback(&self, requested: CreaturePartFamilyId) -> CreaturePartFamilyId {
        self.family(requested)
            .or_else(|| self.families.iter().min_by_key(|family| family.id))
            .map(|family| family.id)
            .unwrap_or(CreaturePartFamilyId(0))
    }

    pub fn texture_path(
        &self,
        family: CreaturePartFamilyId,
    ) -> Result<&str, CreaturePartCatalogError> {
        self.family(family)
            .map(|definition| definition.texture_asset.as_str())
            .ok_or(CreaturePartCatalogError::UnknownFamily(family))
    }
}

#[derive(Debug, Error)]
pub enum CreaturePartCatalogError {
    #[error("creature part catalog JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("creature part catalog schema is invalid")]
    Schema,
    #[error("creature part catalog has no families")]
    Empty,
    #[error("invalid creature part family ID {0:?}")]
    InvalidFamilyId(CreaturePartFamilyId),
    #[error("duplicate creature part family ID {0:?}")]
    DuplicateFamilyId(CreaturePartFamilyId),
    #[error("duplicate creature part family label {0}")]
    DuplicateFamilyLabel(String),
    #[error("unknown creature part family {0:?}")]
    UnknownFamily(CreaturePartFamilyId),
    #[error("invalid creature part family {0:?}")]
    InvalidFamily(CreaturePartFamilyId),
    #[error("missing slot {1:?} for family {0:?}")]
    MissingSlot(CreaturePartFamilyId, CreaturePartSlot),
    #[error("missing socket {0}")]
    MissingSocket(String),
    #[error("invalid LOD set for family {0:?}")]
    InvalidLods(CreaturePartFamilyId),
    #[error("invalid cut volume")]
    InvalidCutVolume,
    #[error("invalid scalar in {0}")]
    InvalidScalar(&'static str),
    #[error("invalid catalog path {0}")]
    InvalidPath(String),
    #[error("unknown template family {0:?}")]
    UnknownTemplateFamily(CreaturePartFamilyId),
    #[error("no compatible family for torso {torso:?} slot {slot:?}")]
    NoCompatibleFamily {
        torso: CreaturePartFamilyId,
        slot: CreaturePartSlot,
    },
}

pub fn load_production_creature_part_catalog(
) -> Result<CreaturePartCatalog, CreaturePartCatalogError> {
    CreaturePartCatalog::from_json_str(PRODUCTION_CATALOG_JSON)
}

fn validate_relative_path(path: &str) -> Result<(), CreaturePartCatalogError> {
    let path_ref = Path::new(path);
    if path.is_empty()
        || path_ref.is_absolute()
        || path_ref
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(CreaturePartCatalogError::InvalidPath(path.to_string()));
    }
    Ok(())
}

fn validate_production_path(path: &str) -> Result<(), CreaturePartCatalogError> {
    validate_relative_path(path)?;
    if !path.starts_with("production_voxel_v1/") {
        return Err(CreaturePartCatalogError::InvalidPath(path.to_string()));
    }
    Ok(())
}

fn validate_generated_path(path: &str) -> Result<(), CreaturePartCatalogError> {
    validate_production_path(path)?;
    if !path.starts_with("production_voxel_v1/creature_parts/generated/") {
        return Err(CreaturePartCatalogError::InvalidPath(path.to_string()));
    }
    Ok(())
}

fn validate_developer_source_path(path: &str) -> Result<(), CreaturePartCatalogError> {
    validate_relative_path(path)?;
    if !path.starts_with("source_creature_meshes/") || path.contains("production_voxel_v1") {
        return Err(CreaturePartCatalogError::InvalidPath(path.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn production_catalog_has_append_only_unique_ids_and_all_required_lods() {
        let catalog = load_production_creature_part_catalog().unwrap();
        assert_eq!(catalog.schema, CREATURE_PART_CATALOG_SCHEMA);
        assert_eq!(catalog.families.len(), 8);
        assert_eq!(
            catalog
                .families
                .iter()
                .map(|family| family.id)
                .collect::<BTreeSet<_>>()
                .len(),
            8
        );
        for family in &catalog.families {
            assert_eq!(family.lods.len(), 3);
            assert!(family.compatibility_tags.len() >= 2);
            family.validate().unwrap();
        }
    }

    #[test]
    fn synthetic_ninth_family_requires_no_rust_match_arm() {
        let mut catalog = load_production_creature_part_catalog().unwrap();
        let mut ninth = catalog.families[0].clone();
        ninth.id = CreaturePartFamilyId(100);
        ninth.label = "future-family".into();
        catalog.families.push(ninth);

        catalog.validate().unwrap();

        assert_eq!(
            catalog.family(CreaturePartFamilyId(100)).unwrap().label,
            "future-family"
        );
    }

    #[test]
    fn duplicate_family_ids_are_rejected() {
        let mut catalog = load_production_creature_part_catalog().unwrap();
        catalog.families.push(catalog.families[0].clone());

        assert!(catalog.validate().is_err());
    }
}
