use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
};

use alife_world::{CreaturePartFamilyId, CreaturePartSlotKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CREATURE_PART_CATALOG_SCHEMA: &str = "alife.creature_part_catalog.v1";
pub const GENEFORGE_CREATURE_PART_CATALOG_SCHEMA: &str = "alife.geneforge_creature_part_catalog.v2";
const PRODUCTION_CATALOG_JSON: &str =
    include_str!("../assets/production_voxel_v1/creature_parts/catalog.json");
const GENEFORGE_RECIPE_CATALOG_JSON: &str =
    include_str!("../assets/production_voxel_v1/creature_parts/geneforge_recipes.json");

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
    pub source_digest: String,
    pub generated_obj: String,
    pub socket_manifest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreaturePartSourceAttribution {
    pub asset_id: String,
    pub source: String,
    pub author: String,
    pub license: String,
    pub license_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreaturePartFamilyDefinition {
    pub id: CreaturePartFamilyId,
    pub label: String,
    #[serde(default)]
    pub template_family: Option<CreaturePartFamilyId>,
    pub texture_asset: String,
    pub source_attribution: CreaturePartSourceAttribution,
    pub builder_version: String,
    pub output_schema: String,
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
        if self.source_attribution.asset_id.is_empty()
            || self.source_attribution.source.is_empty()
            || self.source_attribution.author.is_empty()
            || self.source_attribution.license.is_empty()
            || self
                .source_attribution
                .license
                .eq_ignore_ascii_case("unknown")
            || self.builder_version.is_empty()
            || self.output_schema.is_empty()
        {
            return Err(CreaturePartCatalogError::InvalidAttribution(self.id));
        }
        validate_production_path(&self.source_attribution.license_ref)?;
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
            validate_fnv1a_digest(&lod.source_digest)?;
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CreaturePartAssetId(pub String);

impl CreaturePartAssetId {
    fn validate(&self) -> bool {
        !self.0.is_empty()
            && self.0.len() <= 64
            && self
                .0
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeneForgeDonorId {
    Norn,
    Ettin,
    Grendel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneForgeSourceDefinition {
    pub donor: GeneForgeDonorId,
    pub blend_file: String,
    pub sha256: String,
    pub texture_root: String,
    pub microdetail_root: String,
    pub audited_non_marker_properties: BTreeMap<String, i32>,
    pub source_url: String,
    pub author: String,
    pub license: String,
    pub license_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeneForgeMarkerSemantic {
    Head,
    Torso,
    LeftThigh,
    LeftShin,
    LeftFoot,
    RightThigh,
    RightShin,
    RightFoot,
    LeftUpperArm,
    LeftLowerArm,
    RightUpperArm,
    RightLowerArm,
    TailRoot,
    TailTip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeneForgeSelectionPolicy {
    ExactCaseSensitiveNames,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeneForgeGeometryPolicy {
    EvaluatedDepsgraph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeneForgeTopologyRepair {
    RemoveZeroAreaFaces,
    RepairDeclaredNonManifoldEdges,
    RemoveLooseVertices,
    RepairDeclaredBoundaryEdges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeneForgeEvaluatedEmptyPolicy {
    ValidatedRawMesh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeneForgeUvFallbackPolicy {
    SemanticDetailRegion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeneForgeDetailRole {
    Eyes,
    Lids,
    Hair,
    Teeth,
    Tongue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeneForgeAnatomyChannel {
    Primary,
    Belly,
    Muzzle,
    InnerEar,
    HandsFeet,
    KeratinSkin,
    SecondaryMarking,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum GeneForgeAnatomyShape {
    Ellipse { center: [f32; 2], radius: [f32; 2] },
    Polygon { points: Vec<[f32; 2]> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeAnatomyZone {
    pub id: String,
    pub channel: GeneForgeAnatomyChannel,
    pub semantic_groups: BTreeSet<String>,
    pub shape: GeneForgeAnatomyShape,
    pub strength: u8,
    pub priority: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeAnatomyAuthoring {
    pub schema: String,
    pub coordinate_space: String,
    pub default_channel: GeneForgeAnatomyChannel,
    pub required_channels: BTreeSet<GeneForgeAnatomyChannel>,
    pub zones: Vec<GeneForgeAnatomyZone>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneForgePartSelector {
    pub selection_policy: GeneForgeSelectionPolicy,
    pub geometry_policy: GeneForgeGeometryPolicy,
    pub marker_ids: Vec<u8>,
    #[serde(default)]
    pub include_objects: Vec<String>,
    #[serde(default)]
    pub object_visscripts: BTreeMap<String, String>,
    #[serde(default)]
    pub selector_tags: BTreeMap<String, String>,
    #[serde(default)]
    pub topology_repairs: BTreeMap<String, Vec<GeneForgeTopologyRepair>>,
    #[serde(default)]
    pub evaluated_empty_policy: BTreeMap<String, GeneForgeEvaluatedEmptyPolicy>,
    #[serde(default)]
    pub uv_fallbacks: BTreeMap<String, GeneForgeUvFallbackPolicy>,
    pub mirror_policy: String,
    pub uv_map: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeCanonicalBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl GeneForgeCanonicalBounds {
    fn validate(self) -> Result<(), &'static str> {
        if !self.min.into_iter().chain(self.max).all(f32::is_finite) {
            return Err("bounds contain a non-finite coordinate");
        }
        if !(0..3).all(|axis| self.min[axis] < self.max[axis]) {
            return Err("bounds must have positive extent on every axis");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeneForgeSemanticRegion {
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
    Tail,
    Eyes,
    Mouth,
    JoinCover,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeneForgeLandmarkId {
    LeftEye,
    RightEye,
    Muzzle,
    LeftBrow,
    RightBrow,
    LeftLid,
    RightLid,
    NeckAttachment,
    LeftShoulderAttachment,
    RightShoulderAttachment,
    LeftHipAttachment,
    RightHipAttachment,
    LeftHand,
    RightHand,
    LeftFoot,
    RightFoot,
    TailRoot,
    TailTip,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeGeneratedPartLod {
    pub lod: CreaturePartLodId,
    pub generated_obj: String,
    pub generated_obj_sha256: String,
    pub socket_manifest: String,
    pub socket_manifest_sha256: String,
    pub semantic_mask: String,
    pub semantic_mask_sha256: String,
    pub anatomy_mask: String,
    pub anatomy_mask_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgePartAssetDefinition {
    pub id: CreaturePartAssetId,
    pub donor: GeneForgeDonorId,
    pub logical_slot: CreaturePartSlotKey,
    pub selector: GeneForgePartSelector,
    pub groups: BTreeMap<CreaturePartSlot, String>,
    #[serde(default)]
    pub detail_groups: BTreeMap<GeneForgeDetailRole, Vec<String>>,
    pub attachment_frames: BTreeMap<CreaturePartSlot, SocketFrame>,
    pub canonical_bounds: GeneForgeCanonicalBounds,
    pub semantic_regions: BTreeSet<GeneForgeSemanticRegion>,
    pub landmarks: BTreeMap<GeneForgeLandmarkId, [f32; 3]>,
    pub anatomy_authoring: GeneForgeAnatomyAuthoring,
    pub lods: Vec<GeneForgeGeneratedPartLod>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeFamilyPartRecipe {
    pub asset_id: CreaturePartAssetId,
    pub fit: SocketFrame,
    pub seam_offset: [f32; 3],
    pub variant_label: String,
    pub join_cover_kind: String,
}

impl GeneForgeFamilyPartRecipe {
    pub fn has_authored_tweak(&self) -> bool {
        self.fit != SocketFrame::IDENTITY
            || self
                .seam_offset
                .into_iter()
                .any(|value| value.abs() > 1.0e-6)
    }

    fn validate_authored_fit(&self) -> Result<(), &'static str> {
        if !self
            .fit
            .translation
            .into_iter()
            .chain(self.fit.rotation_xyzw)
            .chain(self.fit.scale)
            .chain(self.seam_offset)
            .all(f32::is_finite)
        {
            return Err("fit contains a non-finite value");
        }
        let rotation_norm_squared = self
            .fit
            .rotation_xyzw
            .into_iter()
            .map(|value| value * value)
            .sum::<f32>();
        if (rotation_norm_squared - 1.0).abs() > 1.0e-3 {
            return Err("fit quaternion is not normalized");
        }
        let scale = self.fit.scale[0];
        if !(0.88..=1.12).contains(&scale)
            || self
                .fit
                .scale
                .into_iter()
                .any(|component| (component - scale).abs() > 1.0e-6)
        {
            return Err("fit scale must be uniform and within 0.88..=1.12");
        }
        if self
            .fit
            .translation
            .into_iter()
            .any(|component| component.abs() > 0.12)
        {
            return Err("fit translation exceeds 0.12 canonical units");
        }
        if self
            .seam_offset
            .into_iter()
            .any(|component| component.abs() > 0.025)
        {
            return Err("fit seam offset exceeds 0.025 canonical units");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeCreatureFamilyDefinition {
    pub id: CreaturePartFamilyId,
    pub label: String,
    pub parts: BTreeMap<CreaturePartSlotKey, GeneForgeFamilyPartRecipe>,
    pub compatibility_tags: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeAssemblyContract {
    pub schema: String,
    pub attachment_error_limit: f32,
    pub default_overlap_depth: f32,
    pub slot_sockets: BTreeMap<CreaturePartSlotKey, Vec<String>>,
}

impl GeneForgeAssemblyContract {
    fn validate(&self) -> Result<(), GeneForgeCatalogError> {
        let expected = BTreeMap::from([
            (CreaturePartSlotKey::Head, vec!["neck"]),
            (
                CreaturePartSlotKey::Torso,
                vec![
                    "neck",
                    "left-shoulder",
                    "right-shoulder",
                    "left-hip",
                    "right-hip",
                    "tail-base",
                ],
            ),
            (
                CreaturePartSlotKey::Arms,
                vec!["left-shoulder", "right-shoulder"],
            ),
            (CreaturePartSlotKey::Legs, vec!["left-hip", "right-hip"]),
            (CreaturePartSlotKey::Tail, vec!["tail-base"]),
        ]);
        let actual = self
            .slot_sockets
            .iter()
            .map(|(slot, sockets)| {
                (
                    *slot,
                    sockets.iter().map(String::as_str).collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        if self.schema != "alife.geneforge_family_assembly.v1"
            || self.attachment_error_limit != 0.025
            || !self.default_overlap_depth.is_finite()
            || !(0.005..=0.25).contains(&self.default_overlap_depth)
            || actual != expected
        {
            return Err(GeneForgeCatalogError::InvalidCatalogMetadata {
                reason: "invalid family assembly bridge/seam contract",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeCreaturePartCatalog {
    pub schema: String,
    pub schema_version: u16,
    pub blender_version: String,
    pub importer_version: String,
    /// SHA-256 of key-sorted compact JSON with this field replaced by 64 zeroes.
    pub recipe_sha256: String,
    pub marker_map: BTreeMap<u8, GeneForgeMarkerSemantic>,
    pub assembly_contract: GeneForgeAssemblyContract,
    pub sources: Vec<GeneForgeSourceDefinition>,
    pub part_assets: Vec<GeneForgePartAssetDefinition>,
    pub families: Vec<GeneForgeCreatureFamilyDefinition>,
}

impl GeneForgeCreaturePartCatalog {
    pub fn from_json_str(text: &str) -> Result<Self, GeneForgeCatalogError> {
        let catalog: Self = serde_json::from_str(text)?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn asset(&self, id: &CreaturePartAssetId) -> Option<&GeneForgePartAssetDefinition> {
        self.part_assets.iter().find(|asset| asset.id == *id)
    }

    pub fn validate(&self) -> Result<(), GeneForgeCatalogError> {
        if self.schema != GENEFORGE_CREATURE_PART_CATALOG_SCHEMA
            || self.schema_version != 2
            || self.blender_version != "5.1.0"
        {
            return Err(GeneForgeCatalogError::Schema);
        }
        if self.importer_version != "alife.geneforge_importer.v2" {
            return Err(GeneForgeCatalogError::InvalidCatalogMetadata {
                reason: "unexpected importer version",
            });
        }
        if !valid_sha256(&self.recipe_sha256) {
            return Err(GeneForgeCatalogError::InvalidCatalogMetadata {
                reason: "invalid recipe SHA-256",
            });
        }
        if self.marker_map
            != BTreeMap::from([
                (1, GeneForgeMarkerSemantic::Head),
                (2, GeneForgeMarkerSemantic::Torso),
                (3, GeneForgeMarkerSemantic::LeftThigh),
                (4, GeneForgeMarkerSemantic::LeftShin),
                (5, GeneForgeMarkerSemantic::LeftFoot),
                (6, GeneForgeMarkerSemantic::RightThigh),
                (7, GeneForgeMarkerSemantic::RightShin),
                (8, GeneForgeMarkerSemantic::RightFoot),
                (9, GeneForgeMarkerSemantic::LeftUpperArm),
                (10, GeneForgeMarkerSemantic::LeftLowerArm),
                (11, GeneForgeMarkerSemantic::RightUpperArm),
                (12, GeneForgeMarkerSemantic::RightLowerArm),
                (13, GeneForgeMarkerSemantic::TailRoot),
                (14, GeneForgeMarkerSemantic::TailTip),
            ])
        {
            return Err(GeneForgeCatalogError::InvalidCatalogMetadata {
                reason: "marker map must be the stable semantic 1..=14 contract",
            });
        }
        self.assembly_contract.validate()?;
        if self.sources.is_empty() || self.part_assets.is_empty() || self.families.is_empty() {
            return Err(GeneForgeCatalogError::Empty);
        }

        let mut source_donors = BTreeSet::new();
        for source in &self.sources {
            if !source_donors.insert(source.donor) {
                return Err(GeneForgeCatalogError::SourceAttributionDrift {
                    donor: source.donor,
                    reason: "duplicate donor source",
                });
            }
            if !valid_sha256(&source.sha256)
                || !valid_relative_path(&source.blend_file)
                || !valid_relative_path(&source.texture_root)
                || !valid_relative_path(&source.microdetail_root)
                || source.source_url.trim().is_empty()
                || source.author.trim().is_empty()
                || source.license.trim().is_empty()
                || source.license.eq_ignore_ascii_case("unknown")
                || !valid_production_path(&source.license_ref)
            {
                return Err(GeneForgeCatalogError::SourceAttributionDrift {
                    donor: source.donor,
                    reason: "source record contains invalid metadata",
                });
            }
            if !source_matches_pinned_attribution(source) {
                return Err(GeneForgeCatalogError::SourceAttributionDrift {
                    donor: source.donor,
                    reason: "source record differs from pinned attribution",
                });
            }
        }
        if source_donors
            != BTreeSet::from([
                GeneForgeDonorId::Norn,
                GeneForgeDonorId::Ettin,
                GeneForgeDonorId::Grendel,
            ])
        {
            return Err(GeneForgeCatalogError::InvalidSourceSet);
        }

        let mut asset_ids = BTreeSet::new();
        for asset in &self.part_assets {
            if !asset_ids.insert(asset.id.clone()) {
                return Err(GeneForgeCatalogError::DuplicateAssetId(asset.id.clone()));
            }
            if !asset.id.validate()
                || !source_donors.contains(&asset.donor)
                || asset.selector.marker_ids.is_empty()
                || asset.selector.include_objects.is_empty()
                || asset.selector.selection_policy
                    != GeneForgeSelectionPolicy::ExactCaseSensitiveNames
                || asset.selector.geometry_policy != GeneForgeGeometryPolicy::EvaluatedDepsgraph
                || (asset.selector.object_visscripts.is_empty()
                    && !asset.selector.selector_tags.contains_key("pitch_id"))
                || !asset
                    .selector
                    .marker_ids
                    .iter()
                    .all(|id| (1..=14).contains(id))
                || asset.selector.mirror_policy.trim().is_empty()
                || asset.selector.uv_map.trim().is_empty()
                || (asset.logical_slot == CreaturePartSlotKey::Tail
                    && asset.donor == GeneForgeDonorId::Ettin)
            {
                return Err(GeneForgeCatalogError::InvalidAsset {
                    asset: asset.id.clone(),
                    reason: "invalid selector, donor, or logical slot contract",
                });
            }
            let selected = asset
                .selector
                .include_objects
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            if asset
                .selector
                .object_visscripts
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                != selected
            {
                return Err(GeneForgeCatalogError::InvalidAsset {
                    asset: asset.id.clone(),
                    reason: "every selected object needs an exact kc3dsbpy_visscript contract",
                });
            }
            if asset
                .selector
                .topology_repairs
                .iter()
                .any(|(name, repairs)| !selected.contains(name.as_str()) || repairs.is_empty())
                || asset
                    .selector
                    .evaluated_empty_policy
                    .keys()
                    .chain(asset.selector.uv_fallbacks.keys())
                    .any(|name| !selected.contains(name.as_str()))
            {
                return Err(GeneForgeCatalogError::InvalidAsset {
                    asset: asset.id.clone(),
                    reason: "importer repair metadata references an unselected object",
                });
            }
            let detail_objects = asset
                .detail_groups
                .values()
                .flatten()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            if asset.logical_slot == CreaturePartSlotKey::Head {
                let required_roles = BTreeSet::from([
                    GeneForgeDetailRole::Eyes,
                    GeneForgeDetailRole::Lids,
                    GeneForgeDetailRole::Hair,
                    GeneForgeDetailRole::Teeth,
                    GeneForgeDetailRole::Tongue,
                ]);
                if asset.detail_groups.keys().copied().collect::<BTreeSet<_>>() != required_roles
                    || detail_objects.is_empty()
                    || !detail_objects.is_subset(&selected)
                    || asset
                        .selector
                        .uv_fallbacks
                        .keys()
                        .any(|name| !detail_objects.contains(name.as_str()))
                {
                    return Err(GeneForgeCatalogError::InvalidAsset {
                        asset: asset.id.clone(),
                        reason: "head detail groups or UV fallback metadata are invalid",
                    });
                }
            } else if !asset.detail_groups.is_empty() || !asset.selector.uv_fallbacks.is_empty() {
                return Err(GeneForgeCatalogError::InvalidAsset {
                    asset: asset.id.clone(),
                    reason: "non-head asset declares head detail metadata",
                });
            }
            for slot in required_runtime_slots(asset.logical_slot) {
                if !asset.groups.contains_key(&slot) {
                    return Err(GeneForgeCatalogError::MissingRuntimeGroup {
                        asset: asset.id.clone(),
                        slot,
                    });
                }
                let frame = asset.attachment_frames.get(&slot).ok_or_else(|| {
                    GeneForgeCatalogError::MissingAttachmentFrame {
                        asset: asset.id.clone(),
                        slot,
                    }
                })?;
                frame.validate().map_err(|error| {
                    GeneForgeCatalogError::InvalidAttachmentFrame {
                        asset: asset.id.clone(),
                        slot,
                        reason: error.to_string(),
                    }
                })?;
            }
            asset.canonical_bounds.validate().map_err(|reason| {
                GeneForgeCatalogError::InvalidCanonicalBounds {
                    asset: asset.id.clone(),
                    reason,
                }
            })?;
            for region in required_semantic_regions(asset.logical_slot) {
                if !asset.semantic_regions.contains(&region) {
                    return Err(GeneForgeCatalogError::MissingSemanticRegion {
                        asset: asset.id.clone(),
                        region,
                    });
                }
            }
            for (landmark, point) in &asset.landmarks {
                if !point.iter().copied().all(f32::is_finite) {
                    return Err(GeneForgeCatalogError::InvalidLandmark {
                        asset: asset.id.clone(),
                        landmark: *landmark,
                        reason: "landmark contains a non-finite coordinate",
                    });
                }
            }
            for landmark in required_landmarks(asset.logical_slot) {
                if !asset.landmarks.contains_key(&landmark) {
                    return Err(GeneForgeCatalogError::MissingLandmark {
                        asset: asset.id.clone(),
                        landmark,
                    });
                }
            }
            validate_anatomy_authoring(asset).map_err(|reason| {
                GeneForgeCatalogError::InvalidAsset {
                    asset: asset.id.clone(),
                    reason,
                }
            })?;
            let lods = asset
                .lods
                .iter()
                .map(|lod| lod.lod)
                .collect::<BTreeSet<_>>();
            if lods
                != BTreeSet::from([
                    CreaturePartLodId::Full,
                    CreaturePartLodId::Compact,
                    CreaturePartLodId::Impostor,
                ])
                || asset.lods.len() != 3
            {
                return Err(GeneForgeCatalogError::InvalidAssetLodSet {
                    asset: asset.id.clone(),
                });
            }
            for lod in &asset.lods {
                if !valid_generated_path(&lod.generated_obj)
                    || !valid_generated_path(&lod.socket_manifest)
                    || !valid_production_path(&lod.semantic_mask)
                    || !valid_production_path(&lod.anatomy_mask)
                {
                    return Err(GeneForgeCatalogError::InvalidAssetLodPath {
                        asset: asset.id.clone(),
                        lod: lod.lod,
                    });
                }
                for (output, digest) in [
                    ("generated OBJ", &lod.generated_obj_sha256),
                    ("socket manifest", &lod.socket_manifest_sha256),
                    ("semantic mask", &lod.semantic_mask_sha256),
                    ("anatomy mask", &lod.anatomy_mask_sha256),
                ] {
                    if !valid_sha256(digest) {
                        return Err(GeneForgeCatalogError::InvalidOutputDigest {
                            asset: asset.id.clone(),
                            lod: lod.lod,
                            output,
                        });
                    }
                }
            }
        }

        for (donor, expected_markers) in [
            (GeneForgeDonorId::Norn, BTreeSet::from_iter(1_u8..=14)),
            (GeneForgeDonorId::Ettin, BTreeSet::from_iter(1_u8..=12)),
            (GeneForgeDonorId::Grendel, BTreeSet::from_iter(1_u8..=14)),
        ] {
            let actual = self
                .part_assets
                .iter()
                .filter(|asset| asset.donor == donor)
                .flat_map(|asset| asset.selector.marker_ids.iter().copied())
                .collect::<BTreeSet<_>>();
            if actual != expected_markers {
                return Err(GeneForgeCatalogError::InvalidCatalogMetadata {
                    reason: "donor marker selection does not match the exact audited set",
                });
            }
        }

        let expected_slots = BTreeSet::from([
            CreaturePartSlotKey::Head,
            CreaturePartSlotKey::Torso,
            CreaturePartSlotKey::Arms,
            CreaturePartSlotKey::Legs,
            CreaturePartSlotKey::Tail,
        ]);
        let mut labels = BTreeSet::new();
        for (index, family) in self.families.iter().enumerate() {
            if family.id.0 != index as u16
                || family.label.trim().is_empty()
                || !labels.insert(family.label.as_str())
                || family.parts.keys().copied().collect::<BTreeSet<_>>() != expected_slots
                || family.compatibility_tags.len() < 2
            {
                return Err(GeneForgeCatalogError::InvalidFamily(family.id));
            }
            let mut donors = BTreeSet::new();
            for (slot, recipe) in &family.parts {
                let asset = self.asset(&recipe.asset_id).ok_or_else(|| {
                    GeneForgeCatalogError::UnknownAsset {
                        family: family.id,
                        slot: *slot,
                        asset: recipe.asset_id.clone(),
                    }
                })?;
                if asset.logical_slot != *slot
                    || recipe.variant_label.trim().is_empty()
                    || recipe.join_cover_kind.trim().is_empty()
                {
                    return Err(GeneForgeCatalogError::InvalidFamily(family.id));
                }
                recipe.validate_authored_fit().map_err(|reason| {
                    GeneForgeCatalogError::InvalidFamilyFit {
                        family: family.id,
                        slot: *slot,
                        reason,
                    }
                })?;
                donors.insert(asset.donor);
            }
            if donors != source_donors
                || !family
                    .parts
                    .values()
                    .any(GeneForgeFamilyPartRecipe::has_authored_tweak)
            {
                return Err(GeneForgeCatalogError::InvalidFamily(family.id));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum GeneForgeCatalogError {
    #[error("GeneForge recipe catalog JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("GeneForge recipe catalog schema or pinned Blender version is invalid")]
    Schema,
    #[error("GeneForge recipe catalog is empty")]
    Empty,
    #[error("invalid GeneForge catalog metadata: {reason}")]
    InvalidCatalogMetadata { reason: &'static str },
    #[error("GeneForge source set must contain exactly Norn, Ettin, and Grendel")]
    InvalidSourceSet,
    #[error("GeneForge source attribution drift for {donor:?}: {reason}")]
    SourceAttributionDrift {
        donor: GeneForgeDonorId,
        reason: &'static str,
    },
    #[error("duplicate GeneForge part asset ID {0:?}")]
    DuplicateAssetId(CreaturePartAssetId),
    #[error("invalid GeneForge part asset {asset:?}: {reason}")]
    InvalidAsset {
        asset: CreaturePartAssetId,
        reason: &'static str,
    },
    #[error("missing runtime group {slot:?} for GeneForge part asset {asset:?}")]
    MissingRuntimeGroup {
        asset: CreaturePartAssetId,
        slot: CreaturePartSlot,
    },
    #[error("missing attachment frame {slot:?} for GeneForge part asset {asset:?}")]
    MissingAttachmentFrame {
        asset: CreaturePartAssetId,
        slot: CreaturePartSlot,
    },
    #[error("invalid attachment frame {slot:?} for GeneForge part asset {asset:?}: {reason}")]
    InvalidAttachmentFrame {
        asset: CreaturePartAssetId,
        slot: CreaturePartSlot,
        reason: String,
    },
    #[error("invalid canonical bounds for GeneForge part asset {asset:?}: {reason}")]
    InvalidCanonicalBounds {
        asset: CreaturePartAssetId,
        reason: &'static str,
    },
    #[error("missing semantic region {region:?} for GeneForge part asset {asset:?}")]
    MissingSemanticRegion {
        asset: CreaturePartAssetId,
        region: GeneForgeSemanticRegion,
    },
    #[error("missing landmark {landmark:?} for GeneForge part asset {asset:?}")]
    MissingLandmark {
        asset: CreaturePartAssetId,
        landmark: GeneForgeLandmarkId,
    },
    #[error("invalid landmark {landmark:?} for GeneForge part asset {asset:?}: {reason}")]
    InvalidLandmark {
        asset: CreaturePartAssetId,
        landmark: GeneForgeLandmarkId,
        reason: &'static str,
    },
    #[error("invalid LOD set for GeneForge part asset {asset:?}")]
    InvalidAssetLodSet { asset: CreaturePartAssetId },
    #[error("invalid {lod:?} output path for GeneForge part asset {asset:?}")]
    InvalidAssetLodPath {
        asset: CreaturePartAssetId,
        lod: CreaturePartLodId,
    },
    #[error("invalid {output} SHA-256 for GeneForge part asset {asset:?} LOD {lod:?}")]
    InvalidOutputDigest {
        asset: CreaturePartAssetId,
        lod: CreaturePartLodId,
        output: &'static str,
    },
    #[error("unknown GeneForge part asset {asset:?} for family {family:?} slot {slot:?}")]
    UnknownAsset {
        family: CreaturePartFamilyId,
        slot: CreaturePartSlotKey,
        asset: CreaturePartAssetId,
    },
    #[error("invalid GeneForge family {0:?}")]
    InvalidFamily(CreaturePartFamilyId),
    #[error("invalid authored fit for GeneForge family {family:?} slot {slot:?}: {reason}")]
    InvalidFamilyFit {
        family: CreaturePartFamilyId,
        slot: CreaturePartSlotKey,
        reason: &'static str,
    },
}

pub fn load_geneforge_creature_part_catalog(
) -> Result<GeneForgeCreaturePartCatalog, GeneForgeCatalogError> {
    GeneForgeCreaturePartCatalog::from_json_str(GENEFORGE_RECIPE_CATALOG_JSON)
}

fn required_runtime_slots(logical_slot: CreaturePartSlotKey) -> BTreeSet<CreaturePartSlot> {
    match logical_slot {
        CreaturePartSlotKey::Head => BTreeSet::from([CreaturePartSlot::Head]),
        CreaturePartSlotKey::Torso => BTreeSet::from([CreaturePartSlot::Torso]),
        CreaturePartSlotKey::Arms => {
            BTreeSet::from([CreaturePartSlot::LeftArm, CreaturePartSlot::RightArm])
        }
        CreaturePartSlotKey::Legs => {
            BTreeSet::from([CreaturePartSlot::LeftLeg, CreaturePartSlot::RightLeg])
        }
        CreaturePartSlotKey::Tail => BTreeSet::from([CreaturePartSlot::TailBack]),
    }
}

fn validate_anatomy_authoring(asset: &GeneForgePartAssetDefinition) -> Result<(), &'static str> {
    let profile = &asset.anatomy_authoring;
    if profile.schema != "alife.geneforge_anatomy_authoring.v1"
        || profile.coordinate_space != "semantic-group-local-uv"
        || profile.default_channel != GeneForgeAnatomyChannel::Primary
        || profile.zones.is_empty()
    {
        return Err("invalid anatomy authoring metadata");
    }
    let (required, allowed, groups) = match asset.logical_slot {
        CreaturePartSlotKey::Head => (
            BTreeSet::from([
                GeneForgeAnatomyChannel::Primary,
                GeneForgeAnatomyChannel::Muzzle,
                GeneForgeAnatomyChannel::InnerEar,
                GeneForgeAnatomyChannel::KeratinSkin,
                GeneForgeAnatomyChannel::SecondaryMarking,
            ]),
            BTreeSet::from([
                GeneForgeAnatomyChannel::Primary,
                GeneForgeAnatomyChannel::Muzzle,
                GeneForgeAnatomyChannel::InnerEar,
                GeneForgeAnatomyChannel::KeratinSkin,
                GeneForgeAnatomyChannel::SecondaryMarking,
            ]),
            BTreeSet::from(["head"]),
        ),
        CreaturePartSlotKey::Torso => (
            BTreeSet::from([
                GeneForgeAnatomyChannel::Primary,
                GeneForgeAnatomyChannel::Belly,
                GeneForgeAnatomyChannel::SecondaryMarking,
            ]),
            BTreeSet::from([
                GeneForgeAnatomyChannel::Primary,
                GeneForgeAnatomyChannel::Belly,
                GeneForgeAnatomyChannel::KeratinSkin,
                GeneForgeAnatomyChannel::SecondaryMarking,
            ]),
            BTreeSet::from(["torso"]),
        ),
        CreaturePartSlotKey::Arms => (
            BTreeSet::from([
                GeneForgeAnatomyChannel::Primary,
                GeneForgeAnatomyChannel::HandsFeet,
                GeneForgeAnatomyChannel::SecondaryMarking,
            ]),
            BTreeSet::from([
                GeneForgeAnatomyChannel::Primary,
                GeneForgeAnatomyChannel::HandsFeet,
                GeneForgeAnatomyChannel::KeratinSkin,
                GeneForgeAnatomyChannel::SecondaryMarking,
            ]),
            BTreeSet::from(["left-arm", "right-arm"]),
        ),
        CreaturePartSlotKey::Legs => (
            BTreeSet::from([
                GeneForgeAnatomyChannel::Primary,
                GeneForgeAnatomyChannel::HandsFeet,
                GeneForgeAnatomyChannel::SecondaryMarking,
            ]),
            BTreeSet::from([
                GeneForgeAnatomyChannel::Primary,
                GeneForgeAnatomyChannel::HandsFeet,
                GeneForgeAnatomyChannel::KeratinSkin,
                GeneForgeAnatomyChannel::SecondaryMarking,
            ]),
            BTreeSet::from(["left-leg", "right-leg"]),
        ),
        CreaturePartSlotKey::Tail => (
            BTreeSet::from([
                GeneForgeAnatomyChannel::Primary,
                GeneForgeAnatomyChannel::KeratinSkin,
                GeneForgeAnatomyChannel::SecondaryMarking,
            ]),
            BTreeSet::from([
                GeneForgeAnatomyChannel::Primary,
                GeneForgeAnatomyChannel::KeratinSkin,
                GeneForgeAnatomyChannel::SecondaryMarking,
            ]),
            BTreeSet::from(["tail-back"]),
        ),
    };
    if profile.required_channels != required {
        return Err("anatomy required-channel contract is invalid");
    }
    let mut ids = BTreeSet::new();
    let mut authored = BTreeSet::from([GeneForgeAnatomyChannel::Primary]);
    for zone in &profile.zones {
        if zone.id.is_empty()
            || !ids.insert(zone.id.as_str())
            || !allowed.contains(&zone.channel)
            || zone.semantic_groups.is_empty()
            || !zone
                .semantic_groups
                .iter()
                .all(|group| groups.contains(group.as_str()))
            || zone.strength == 0
            || !valid_anatomy_shape(&zone.shape)
        {
            return Err("anatomy zone is malformed or violates source ownership");
        }
        authored.insert(zone.channel);
    }
    if !required.is_subset(&authored) {
        return Err("anatomy zones do not author every required channel");
    }
    Ok(())
}

fn valid_anatomy_shape(shape: &GeneForgeAnatomyShape) -> bool {
    let unit = |value: f32| value.is_finite() && (0.0..=1.0).contains(&value);
    match shape {
        GeneForgeAnatomyShape::Ellipse { center, radius } => {
            center.iter().copied().all(unit)
                && radius.iter().copied().all(unit)
                && radius.iter().all(|value| *value > 0.0)
        }
        GeneForgeAnatomyShape::Polygon { points } => {
            points.len() >= 3 && points.iter().flatten().copied().all(unit)
        }
    }
}

fn required_semantic_regions(
    logical_slot: CreaturePartSlotKey,
) -> BTreeSet<GeneForgeSemanticRegion> {
    use GeneForgeSemanticRegion::*;
    match logical_slot {
        CreaturePartSlotKey::Head => BTreeSet::from([Head, Eyes, Mouth, JoinCover]),
        CreaturePartSlotKey::Torso => BTreeSet::from([Torso, JoinCover]),
        CreaturePartSlotKey::Arms => BTreeSet::from([LeftArm, RightArm, JoinCover]),
        CreaturePartSlotKey::Legs => BTreeSet::from([LeftLeg, RightLeg, JoinCover]),
        CreaturePartSlotKey::Tail => BTreeSet::from([Tail, JoinCover]),
    }
}

fn required_landmarks(logical_slot: CreaturePartSlotKey) -> BTreeSet<GeneForgeLandmarkId> {
    use GeneForgeLandmarkId::*;
    match logical_slot {
        CreaturePartSlotKey::Head => BTreeSet::from([
            LeftEye,
            RightEye,
            Muzzle,
            LeftBrow,
            RightBrow,
            LeftLid,
            RightLid,
            NeckAttachment,
        ]),
        CreaturePartSlotKey::Torso => BTreeSet::from([
            NeckAttachment,
            LeftShoulderAttachment,
            RightShoulderAttachment,
            LeftHipAttachment,
            RightHipAttachment,
            TailRoot,
        ]),
        CreaturePartSlotKey::Arms => BTreeSet::from([
            LeftShoulderAttachment,
            RightShoulderAttachment,
            LeftHand,
            RightHand,
        ]),
        CreaturePartSlotKey::Legs => {
            BTreeSet::from([LeftHipAttachment, RightHipAttachment, LeftFoot, RightFoot])
        }
        CreaturePartSlotKey::Tail => BTreeSet::from([TailRoot, TailTip]),
    }
}

fn source_matches_pinned_attribution(source: &GeneForgeSourceDefinition) -> bool {
    let (blend_file, sha256, texture_root, microdetail_root, non_marker_properties) =
        match source.donor {
            GeneForgeDonorId::Norn => (
                "Norn/Geneforge_r4.0_Norn.blend",
                "B6E5C1BC0E0EC69995748B211F45EFF787B9162DBC4856A1AB7F48F3E610FB4A",
                "Norn/Blueberry Norns Example/Geneforge Textures",
                "Norn/Alpha Textures",
                BTreeMap::new(),
            ),
            GeneForgeDonorId::Ettin => (
                "Ettin/geneforge_r4.0_ettin.blend",
                "CC1D2AA1D310BCEA3D39FE495BF756A9B3650ECF0F3C9EEE8AC8488609202B0B",
                "Ettin/Teal Ettin Textures",
                "Ettin/Alpha Textures",
                BTreeMap::from([("head1_Ettin_angry".to_string(), 0)]),
            ),
            GeneForgeDonorId::Grendel => (
                "Grendel/geneforge_r4.0_grendel.blend",
                "3289BBD6D7CAEDF7CCA44175E63B60B4140D26EE2E86CCAD7A89FA8724132E62",
                "Grendel/Purple Grendels Textures",
                "Grendel/Alpha Textures",
                BTreeMap::new(),
            ),
        };
    source.blend_file == blend_file
        && source.sha256 == sha256
        && source.texture_root == texture_root
        && source.microdetail_root == microdetail_root
        && source.audited_non_marker_properties == non_marker_properties
        && source.source_url == "https://eem.foo/geneforge/"
        && source.author == "Eem Foo"
        && source.license == "MIT"
        && source.license_ref == "production_voxel_v1/models/GENEFORGE_LICENSE_RECEIPT.md"
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_relative_path(path: &str) -> bool {
    let path_ref = Path::new(path);
    !path.is_empty()
        && !path_ref.is_absolute()
        && path_ref
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn valid_production_path(path: &str) -> bool {
    valid_relative_path(path) && path.starts_with("production_voxel_v1/")
}

fn valid_generated_path(path: &str) -> bool {
    valid_production_path(path)
        && path.starts_with("production_voxel_v1/creature_parts/generated/geneforge/")
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
    #[error("invalid source attribution for family {0:?}")]
    InvalidAttribution(CreaturePartFamilyId),
    #[error("invalid FNV-1a digest {0}")]
    InvalidDigest(String),
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

fn validate_fnv1a_digest(digest: &str) -> Result<(), CreaturePartCatalogError> {
    let Some(hex) = digest.strip_prefix("fnv1a64:") else {
        return Err(CreaturePartCatalogError::InvalidDigest(digest.to_string()));
    };
    if hex.len() != 16 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CreaturePartCatalogError::InvalidDigest(digest.to_string()));
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
                .map(|family| (family.id.0, family.label.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (0, "colobus"),
                (1, "gecko"),
                (2, "herring"),
                (3, "inkfish"),
                (4, "muskrat"),
                (5, "pudu"),
                (6, "sparrow"),
                (7, "taipan"),
            ]
        );
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
    fn production_families_use_explicit_readable_biped_socket_layouts() {
        let catalog = load_production_creature_part_catalog().unwrap();
        for family in &catalog.families {
            let left_shoulder = family.sockets["left-shoulder"].translation;
            let right_shoulder = family.sockets["right-shoulder"].translation;
            let left_hip = family.sockets["left-hip"].translation;
            let right_hip = family.sockets["right-hip"].translation;
            let neck = family.sockets["neck"].translation;
            let tail = family.sockets["tail-base"].translation;
            let shoulder_gap = right_shoulder[0] - left_shoulder[0];
            let hip_gap = right_hip[0] - left_hip[0];

            assert!(
                (0.46..=0.66).contains(&shoulder_gap),
                "{} shoulders collapse into the torso: {shoulder_gap}",
                family.label
            );
            assert!(
                (0.24..=0.40).contains(&hip_gap),
                "{} legs collapse into one column: {hip_gap}",
                family.label
            );
            assert!((0.38..=0.50).contains(&neck[2]));
            assert!((0.12..=0.34).contains(&tail[1]));
            assert!((-0.30..=-0.08).contains(&tail[2]));
        }
    }

    #[test]
    fn production_catalog_has_no_template_or_empty_anatomy_profiles() {
        let catalog: CreaturePartCatalog = serde_json::from_str(PRODUCTION_CATALOG_JSON)
            .expect("production creature catalog JSON parses");

        for family in &catalog.families {
            assert!(
                family.template_family.is_none(),
                "production family {} must own its anatomy profile",
                family.label
            );
            assert_eq!(
                family.cuts.len(),
                CreaturePartSlot::ALL.len(),
                "production family {} has incomplete cuts",
                family.label
            );
            assert!(
                family.sockets.len() >= 6 && family.join_covers.len() >= 5,
                "production family {} has incomplete sockets or covers",
                family.label
            );
        }
    }

    #[test]
    fn generated_socket_manifests_match_each_explicit_family_profile() {
        let catalog = load_production_creature_part_catalog().unwrap();
        let assets_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        for family in &catalog.families {
            for lod in &family.lods {
                let manifest: serde_json::Value = serde_json::from_str(
                    &std::fs::read_to_string(assets_root.join(&lod.socket_manifest)).unwrap(),
                )
                .unwrap();
                assert_eq!(manifest["family_id"], family.id.0);
                assert_eq!(manifest["lod"], serde_json::to_value(lod.lod).unwrap());
                let actual_sockets: BTreeMap<String, SocketFrame> =
                    serde_json::from_value(manifest["sockets"].clone()).unwrap();
                assert_eq!(
                    actual_sockets, family.sockets,
                    "{} {:?} socket manifest drifted from the production catalog",
                    family.label, lod.lod
                );
            }
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
    fn future_family_requires_explicit_known_source_attribution() {
        let mut catalog = load_production_creature_part_catalog().unwrap();
        let mut ninth = catalog.families[0].clone();
        ninth.id = CreaturePartFamilyId(100);
        ninth.label = "future-family".into();
        ninth.source_attribution.license = "unknown".into();
        catalog.families.push(ninth);

        assert!(matches!(
            catalog.validate(),
            Err(CreaturePartCatalogError::InvalidAttribution(
                CreaturePartFamilyId(100)
            ))
        ));
    }

    #[test]
    fn duplicate_family_ids_are_rejected() {
        let mut catalog = load_production_creature_part_catalog().unwrap();
        catalog.families.push(catalog.families[0].clone());

        assert!(catalog.validate().is_err());
    }

    #[test]
    fn geneforge_v2_catalog_has_the_exact_twelve_frankenstein_families() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();

        assert_eq!(catalog.schema, GENEFORGE_CREATURE_PART_CATALOG_SCHEMA);
        assert_eq!(catalog.schema_version, 2);
        assert_eq!(catalog.blender_version, "5.1.0");
        assert_eq!(
            catalog
                .families
                .iter()
                .map(|family| (family.id.0, family.label.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (0, "tuftback"),
                (1, "tideclimber"),
                (2, "mossknuckle"),
                (3, "emberloper"),
                (4, "duskmane"),
                (5, "reefburrower"),
                (6, "velvetreed"),
                (7, "copperskipper"),
                (8, "slateprowler"),
                (9, "cobaltbramble"),
                (10, "orchidstout"),
                (11, "amberlongstep"),
            ]
        );

        for family in &catalog.families {
            let donors = family
                .parts
                .values()
                .map(|recipe| catalog.asset(&recipe.asset_id).unwrap().donor)
                .collect::<BTreeSet<_>>();
            assert_eq!(
                donors,
                BTreeSet::from([
                    GeneForgeDonorId::Norn,
                    GeneForgeDonorId::Ettin,
                    GeneForgeDonorId::Grendel,
                ]),
                "{} must visibly combine all three donors",
                family.label
            );
            assert!(
                family
                    .parts
                    .values()
                    .any(|recipe| recipe.has_authored_tweak()),
                "{} must not be an unmodified stock donor recipe",
                family.label
            );
        }
    }

    #[test]
    fn geneforge_v2_sources_and_generated_assets_are_complete() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        assert_eq!(
            catalog.sources,
            vec![
                GeneForgeSourceDefinition {
                    donor: GeneForgeDonorId::Norn,
                    blend_file: "Norn/Geneforge_r4.0_Norn.blend".into(),
                    sha256: "B6E5C1BC0E0EC69995748B211F45EFF787B9162DBC4856A1AB7F48F3E610FB4A"
                        .into(),
                    texture_root: "Norn/Blueberry Norns Example/Geneforge Textures".into(),
                    microdetail_root: "Norn/Alpha Textures".into(),
                    audited_non_marker_properties: BTreeMap::new(),
                    source_url: "https://eem.foo/geneforge/".into(),
                    author: "Eem Foo".into(),
                    license: "MIT".into(),
                    license_ref: "production_voxel_v1/models/GENEFORGE_LICENSE_RECEIPT.md".into(),
                },
                GeneForgeSourceDefinition {
                    donor: GeneForgeDonorId::Ettin,
                    blend_file: "Ettin/geneforge_r4.0_ettin.blend".into(),
                    sha256: "CC1D2AA1D310BCEA3D39FE495BF756A9B3650ECF0F3C9EEE8AC8488609202B0B"
                        .into(),
                    texture_root: "Ettin/Teal Ettin Textures".into(),
                    microdetail_root: "Ettin/Alpha Textures".into(),
                    audited_non_marker_properties: BTreeMap::from([(
                        "head1_Ettin_angry".into(),
                        0,
                    )]),
                    source_url: "https://eem.foo/geneforge/".into(),
                    author: "Eem Foo".into(),
                    license: "MIT".into(),
                    license_ref: "production_voxel_v1/models/GENEFORGE_LICENSE_RECEIPT.md".into(),
                },
                GeneForgeSourceDefinition {
                    donor: GeneForgeDonorId::Grendel,
                    blend_file: "Grendel/geneforge_r4.0_grendel.blend".into(),
                    sha256: "3289BBD6D7CAEDF7CCA44175E63B60B4140D26EE2E86CCAD7A89FA8724132E62"
                        .into(),
                    texture_root: "Grendel/Purple Grendels Textures".into(),
                    microdetail_root: "Grendel/Alpha Textures".into(),
                    audited_non_marker_properties: BTreeMap::new(),
                    source_url: "https://eem.foo/geneforge/".into(),
                    author: "Eem Foo".into(),
                    license: "MIT".into(),
                    license_ref: "production_voxel_v1/models/GENEFORGE_LICENSE_RECEIPT.md".into(),
                },
            ]
        );
        assert_eq!(catalog.importer_version, "alife.geneforge_importer.v2");
        assert_eq!(
            catalog.recipe_sha256,
            "85b3a060ac11529d3d57db816de3eb41c773ac825d8cda9ab0bbcb909cf25b74"
        );

        for asset in &catalog.part_assets {
            assert!(asset.canonical_bounds.validate().is_ok());
            assert_eq!(
                asset.semantic_regions,
                required_semantic_regions(asset.logical_slot),
                "{} must declare the complete semantic atlas contract",
                asset.id.0
            );
            assert!(required_landmarks(asset.logical_slot)
                .is_subset(&asset.landmarks.keys().copied().collect()));
            assert!(validate_anatomy_authoring(asset).is_ok());
            assert_eq!(
                asset
                    .lods
                    .iter()
                    .map(|lod| lod.lod)
                    .collect::<BTreeSet<_>>(),
                BTreeSet::from([
                    CreaturePartLodId::Full,
                    CreaturePartLodId::Compact,
                    CreaturePartLodId::Impostor,
                ]),
                "{} must have every production LOD",
                asset.id.0
            );
            for lod in &asset.lods {
                assert!(valid_sha256(&lod.generated_obj_sha256));
                assert!(valid_sha256(&lod.socket_manifest_sha256));
                assert!(valid_sha256(&lod.semantic_mask_sha256));
                assert!(valid_production_path(&lod.anatomy_mask));
                assert!(valid_sha256(&lod.anatomy_mask_sha256));
            }
            assert!(asset
                .selector
                .marker_ids
                .iter()
                .all(|id| (1..=14).contains(id)));
            assert!(
                !asset.selector.include_objects.is_empty(),
                "{} must select source meshes by exact object name",
                asset.id.0
            );
            assert!(
                !asset.selector.object_visscripts.is_empty()
                    || asset.selector.selector_tags.contains_key("pitch_id"),
                "{} must preserve raw visibility or pitch selection metadata",
                asset.id.0
            );
        }

        let expected_head_objects = [
            (
                "norn-head",
                &[
                    "Head1.normal",
                    "Eye_L",
                    "Lid_L",
                    "ear_2L_chichi",
                    "ear_2R_chichi",
                ][..],
            ),
            (
                "ettin-head",
                &["head1_Ettin_normal", "Eye L", "Eyelid L"][..],
            ),
            ("grendel-head", &["Head1_Grendel", "Eye L", "ear 2L"][..]),
        ];
        for (asset_id, objects) in expected_head_objects {
            let asset = catalog
                .asset(&CreaturePartAssetId(asset_id.into()))
                .unwrap();
            for object in objects {
                assert!(
                    asset
                        .selector
                        .include_objects
                        .iter()
                        .any(|name| name == object),
                    "{asset_id} is missing exact selector {object}"
                );
            }
        }
    }

    #[test]
    fn geneforge_v2_types_deterministic_importer_metadata() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        assert_eq!(catalog.marker_map.len(), 14);
        assert_eq!(catalog.marker_map[&1], GeneForgeMarkerSemantic::Head);
        assert_eq!(catalog.marker_map[&14], GeneForgeMarkerSemantic::TailTip);

        let norn_head = catalog
            .asset(&CreaturePartAssetId("norn-head".into()))
            .unwrap();
        assert_eq!(
            norn_head.selector.selection_policy,
            GeneForgeSelectionPolicy::ExactCaseSensitiveNames
        );
        assert_eq!(
            norn_head.selector.geometry_policy,
            GeneForgeGeometryPolicy::EvaluatedDepsgraph
        );
        assert!(!norn_head
            .selector
            .include_objects
            .iter()
            .any(|name| name.contains("whiskers")));
        assert_eq!(
            norn_head.detail_groups[&GeneForgeDetailRole::Eyes],
            vec!["Eye_L", "Eye_R"]
        );

        let ettin_head = catalog
            .asset(&CreaturePartAssetId("ettin-head".into()))
            .unwrap();
        assert_eq!(
            ettin_head.selector.evaluated_empty_policy["Eyelid L"],
            GeneForgeEvaluatedEmptyPolicy::ValidatedRawMesh
        );
        assert_eq!(
            ettin_head.selector.uv_fallbacks["Eye L"],
            GeneForgeUvFallbackPolicy::SemanticDetailRegion
        );
    }

    #[test]
    fn geneforge_v2_accepts_a_data_only_thirteenth_family() {
        let mut catalog = load_geneforge_creature_part_catalog().unwrap();
        let mut family = catalog.families[0].clone();
        family.id = CreaturePartFamilyId(12);
        family.label = "future-frankenstein".into();
        catalog.families.push(family);

        catalog.validate().unwrap();
    }

    #[test]
    fn geneforge_v2_rejects_missing_assets_and_ettin_tails() {
        let mut missing = load_geneforge_creature_part_catalog().unwrap();
        missing.families[0]
            .parts
            .get_mut(&alife_world::CreaturePartSlotKey::Head)
            .unwrap()
            .asset_id = CreaturePartAssetId("missing-head".into());
        assert!(missing.validate().is_err());

        let mut impossible_tail = load_geneforge_creature_part_catalog().unwrap();
        let tail = impossible_tail
            .part_assets
            .iter_mut()
            .find(|asset| asset.logical_slot == alife_world::CreaturePartSlotKey::Tail)
            .unwrap();
        tail.donor = GeneForgeDonorId::Ettin;
        assert!(impossible_tail.validate().is_err());
    }

    #[test]
    fn geneforge_v2_rejects_duplicate_asset_ids() {
        let mut catalog = load_geneforge_creature_part_catalog().unwrap();
        catalog.part_assets.push(catalog.part_assets[0].clone());

        assert!(matches!(
            catalog.validate(),
            Err(GeneForgeCatalogError::DuplicateAssetId(ref asset))
                if asset == &CreaturePartAssetId("norn-head".into())
        ));
    }

    #[test]
    fn geneforge_v2_rejects_invalid_logical_slot_json() {
        let invalid = GENEFORGE_RECIPE_CATALOG_JSON.replacen(
            "\"logical_slot\": \"head\"",
            "\"logical_slot\": \"wings\"",
            1,
        );

        assert!(matches!(
            GeneForgeCreaturePartCatalog::from_json_str(&invalid),
            Err(GeneForgeCatalogError::Json(_))
        ));
    }

    #[test]
    fn geneforge_v2_rejects_missing_or_invalid_attachment_frames() {
        let mut missing = load_geneforge_creature_part_catalog().unwrap();
        missing.part_assets[0]
            .attachment_frames
            .remove(&CreaturePartSlot::Head);
        assert!(matches!(
            missing.validate(),
            Err(GeneForgeCatalogError::MissingAttachmentFrame { ref asset, slot })
                if asset == &CreaturePartAssetId("norn-head".into())
                    && slot == CreaturePartSlot::Head
        ));

        let mut invalid = load_geneforge_creature_part_catalog().unwrap();
        invalid.part_assets[0]
            .attachment_frames
            .get_mut(&CreaturePartSlot::Head)
            .unwrap()
            .rotation_xyzw = [0.0; 4];
        assert!(matches!(
            invalid.validate(),
            Err(GeneForgeCatalogError::InvalidAttachmentFrame {
                ref asset,
                slot: CreaturePartSlot::Head,
                ..
            }) if asset == &CreaturePartAssetId("norn-head".into())
        ));
    }

    #[test]
    fn geneforge_v2_rejects_missing_semantic_regions() {
        let mut catalog = load_geneforge_creature_part_catalog().unwrap();
        catalog.part_assets[0]
            .semantic_regions
            .remove(&GeneForgeSemanticRegion::JoinCover);

        assert!(matches!(
            catalog.validate(),
            Err(GeneForgeCatalogError::MissingSemanticRegion {
                ref asset,
                region: GeneForgeSemanticRegion::JoinCover,
            }) if asset == &CreaturePartAssetId("norn-head".into())
        ));
    }

    #[test]
    fn geneforge_v2_rejects_non_finite_and_out_of_range_family_fits() {
        let mut non_finite = load_geneforge_creature_part_catalog().unwrap();
        non_finite.families[0]
            .parts
            .get_mut(&CreaturePartSlotKey::Head)
            .unwrap()
            .seam_offset[0] = f32::NAN;
        assert!(matches!(
            non_finite.validate(),
            Err(GeneForgeCatalogError::InvalidFamilyFit {
                family: CreaturePartFamilyId(0),
                slot: CreaturePartSlotKey::Head,
                ..
            })
        ));

        let mut out_of_range = load_geneforge_creature_part_catalog().unwrap();
        out_of_range.families[0]
            .parts
            .get_mut(&CreaturePartSlotKey::Head)
            .unwrap()
            .fit
            .scale = [1.13; 3];
        assert!(matches!(
            out_of_range.validate(),
            Err(GeneForgeCatalogError::InvalidFamilyFit {
                family: CreaturePartFamilyId(0),
                slot: CreaturePartSlotKey::Head,
                ..
            })
        ));

        let mut non_uniform = load_geneforge_creature_part_catalog().unwrap();
        non_uniform.families[0]
            .parts
            .get_mut(&CreaturePartSlotKey::Head)
            .unwrap()
            .fit
            .scale = [1.0, 1.01, 1.0];
        assert!(matches!(
            non_uniform.validate(),
            Err(GeneForgeCatalogError::InvalidFamilyFit {
                family: CreaturePartFamilyId(0),
                slot: CreaturePartSlotKey::Head,
                ..
            })
        ));

        let mut translated = load_geneforge_creature_part_catalog().unwrap();
        translated.families[0]
            .parts
            .get_mut(&CreaturePartSlotKey::Head)
            .unwrap()
            .fit
            .translation = [0.121, 0.0, 0.0];
        assert!(matches!(
            translated.validate(),
            Err(GeneForgeCatalogError::InvalidFamilyFit {
                family: CreaturePartFamilyId(0),
                slot: CreaturePartSlotKey::Head,
                ..
            })
        ));

        let mut seam = load_geneforge_creature_part_catalog().unwrap();
        seam.families[0]
            .parts
            .get_mut(&CreaturePartSlotKey::Head)
            .unwrap()
            .seam_offset = [0.026, 0.0, 0.0];
        assert!(matches!(
            seam.validate(),
            Err(GeneForgeCatalogError::InvalidFamilyFit {
                family: CreaturePartFamilyId(0),
                slot: CreaturePartSlotKey::Head,
                ..
            })
        ));
    }

    #[test]
    fn geneforge_v2_rejects_invalid_output_digests() {
        let mut catalog = load_geneforge_creature_part_catalog().unwrap();
        catalog.part_assets[0].lods[0].generated_obj_sha256 = "not-a-sha256".into();

        assert!(matches!(
            catalog.validate(),
            Err(GeneForgeCatalogError::InvalidOutputDigest {
                ref asset,
                lod: CreaturePartLodId::Full,
                output: "generated OBJ",
            }) if asset == &CreaturePartAssetId("norn-head".into())
        ));
    }

    #[test]
    fn geneforge_v2_rejects_invalid_anatomy_authoring_and_paths() {
        let mut schema = load_geneforge_creature_part_catalog().unwrap();
        schema.part_assets[0].anatomy_authoring.schema = "unreviewed".into();
        assert!(matches!(
            schema.validate(),
            Err(GeneForgeCatalogError::InvalidAsset { .. })
        ));

        let mut ownership = load_geneforge_creature_part_catalog().unwrap();
        ownership.part_assets[0].anatomy_authoring.zones[0].channel =
            GeneForgeAnatomyChannel::Belly;
        assert!(matches!(
            ownership.validate(),
            Err(GeneForgeCatalogError::InvalidAsset { .. })
        ));

        let mut shape = load_geneforge_creature_part_catalog().unwrap();
        shape.part_assets[0].anatomy_authoring.zones[0].shape = GeneForgeAnatomyShape::Polygon {
            points: vec![[-0.1, 0.0], [0.5, 0.0], [0.5, 0.5]],
        };
        assert!(matches!(
            shape.validate(),
            Err(GeneForgeCatalogError::InvalidAsset { .. })
        ));

        let mut path = load_geneforge_creature_part_catalog().unwrap();
        path.part_assets[0].lods[0].anatomy_mask = "../escaped.png".into();
        assert!(matches!(
            path.validate(),
            Err(GeneForgeCatalogError::InvalidAssetLodPath { .. })
        ));
    }

    #[test]
    fn geneforge_v2_rejects_absent_required_metadata() {
        let mut json: serde_json::Value =
            serde_json::from_str(GENEFORGE_RECIPE_CATALOG_JSON).unwrap();
        json["part_assets"][0]
            .as_object_mut()
            .unwrap()
            .remove("canonical_bounds");

        assert!(matches!(
            GeneForgeCreaturePartCatalog::from_json_str(&json.to_string()),
            Err(GeneForgeCatalogError::Json(_))
        ));
    }

    #[test]
    fn geneforge_v2_rejects_invalid_catalog_bounds_and_landmark_metadata() {
        let mut catalog_metadata = load_geneforge_creature_part_catalog().unwrap();
        catalog_metadata.importer_version = "unreviewed-importer".into();
        assert!(matches!(
            catalog_metadata.validate(),
            Err(GeneForgeCatalogError::InvalidCatalogMetadata { .. })
        ));

        let mut recipe_digest = load_geneforge_creature_part_catalog().unwrap();
        recipe_digest.recipe_sha256 = "invalid".into();
        assert!(matches!(
            recipe_digest.validate(),
            Err(GeneForgeCatalogError::InvalidCatalogMetadata { .. })
        ));

        let mut bounds = load_geneforge_creature_part_catalog().unwrap();
        bounds.part_assets[0].canonical_bounds.max[0] =
            bounds.part_assets[0].canonical_bounds.min[0];
        assert!(matches!(
            bounds.validate(),
            Err(GeneForgeCatalogError::InvalidCanonicalBounds { .. })
        ));

        let mut landmark = load_geneforge_creature_part_catalog().unwrap();
        landmark.part_assets[0]
            .landmarks
            .get_mut(&GeneForgeLandmarkId::LeftEye)
            .unwrap()[0] = f32::NAN;
        assert!(matches!(
            landmark.validate(),
            Err(GeneForgeCatalogError::InvalidLandmark {
                landmark: GeneForgeLandmarkId::LeftEye,
                ..
            })
        ));
    }

    #[test]
    fn geneforge_v2_rejects_complete_source_attribution_drift() {
        let mut catalog = load_geneforge_creature_part_catalog().unwrap();
        catalog.sources[0].author = "Different Author".into();

        assert!(matches!(
            catalog.validate(),
            Err(GeneForgeCatalogError::SourceAttributionDrift {
                donor: GeneForgeDonorId::Norn,
                ..
            })
        ));
    }
}
