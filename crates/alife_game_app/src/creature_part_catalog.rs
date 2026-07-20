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
pub struct GeneForgeAnatomyProjection {
    pub schema: String,
    pub texel_sample: String,
    pub triangle_tie_break: String,
    pub classifier: String,
    pub detail_group_channels: BTreeMap<String, GeneForgeAnatomyChannel>,
    pub source_geometry: GeneForgeAnatomySourceGeometry,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeAnatomySourceGeometry {
    pub schema: String,
    pub groups: BTreeSet<String>,
    pub landmarks: BTreeMap<String, [f32; 3]>,
    pub canonical_bounds: GeneForgeAnatomySourceBounds,
    pub feature_landmarks: BTreeMap<String, GeneForgeAnatomyFeatureLandmark>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeAnatomySourceBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeAnatomyLodAudit {
    pub obj_sha256: String,
    pub semantic_sha256: String,
    pub projection_sha256: String,
    pub projected_texels: usize,
    pub inside_texels: usize,
    pub nearest_texels: usize,
    pub overlap_texels: usize,
    pub runtime_group_counts: BTreeMap<String, usize>,
    pub channel_counts: BTreeMap<GeneForgeAnatomyChannel, usize>,
    pub geometry_classification: BTreeMap<GeneForgeAnatomyChannel, GeneForgeAnatomyClassification>,
    pub source_landmark_projections: BTreeMap<String, GeneForgeSourceLandmarkProjection>,
    pub feature_anchor_ownership: BTreeMap<String, GeneForgeFeatureAnchorOwnership>,
    pub source_bounds: GeneForgeAnatomySourceBounds,
    pub derived_landmarks: BTreeMap<String, [f32; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeAnatomyClassification {
    pub groups: Vec<String>,
    pub landmarks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeSourceLandmarkProjection {
    pub x: usize,
    pub y: usize,
    pub source: [f32; 3],
    pub projected: [f32; 3],
    pub face: usize,
    pub group: String,
    pub weights: [f32; 3],
    pub distance: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeAnatomyFeatureLandmark {
    pub channel: GeneForgeAnatomyChannel,
    pub runtime_group: String,
    pub source_group: String,
    pub point: [f32; 3],
    pub source_position: [f32; 3],
    pub source_basis: Vec<String>,
    pub method: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeFeatureAnchorOwnership {
    pub channel: GeneForgeAnatomyChannel,
    pub runtime_group: String,
    pub owned_channel: GeneForgeAnatomyChannel,
    pub canonical: [f32; 3],
    pub x: usize,
    pub y: usize,
    pub source: [f32; 3],
    pub projected: [f32; 3],
    pub face: usize,
    pub group: String,
    pub weights: [f32; 3],
    pub distance: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeAnatomySourceProjectionAudit {
    pub schema: String,
    pub lods: BTreeMap<CreaturePartLodId, GeneForgeAnatomyLodAudit>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeAnatomyAuthoring {
    pub schema: String,
    pub coordinate_space: String,
    pub default_channel: GeneForgeAnatomyChannel,
    pub required_channels: BTreeSet<GeneForgeAnatomyChannel>,
    pub projection: GeneForgeAnatomyProjection,
    pub source_projection_audit: GeneForgeAnatomySourceProjectionAudit,
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

pub const GENEFORGE_ASSEMBLY_PREPARATION_SCHEMA: &str = "alife.geneforge_assembly_preparation.v2";
pub const GENEFORGE_ASSEMBLY_AUGMENTOR_VERSION: &str = "alife.geneforge_assembly_augmentor.v1";
pub const GENEFORGE_ASSEMBLY_TRANSFORM_SPACE: &str =
    "alife.creature.canonical.rhs-y-up-neg-z-forward.v1";
pub const GENEFORGE_ASSEMBLY_MATRIX_LAYOUT: &str =
    "row-major-4x4-affine;point=[x,y,z,1];translation=[3,7,11];bottom-row=[0,0,0,1]";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeGroupKeyCounts {
    pub canonical: usize,
    pub cross_torso: usize,
    pub total: usize,
}

impl GeneForgeGroupKeyCounts {
    fn validate(&self) -> Result<(), GeneForgeCatalogError> {
        if (self.canonical, self.cross_torso, self.total) != (252, 432, 684) {
            return Err(GeneForgeCatalogError::InvalidCatalogMetadata {
                reason: "assembly preparation group-key counts must be 252/432/684",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeAssemblyPreparationContract {
    pub schema: String,
    pub version: u16,
    pub augmentor_version: String,
    pub transform_space: String,
    pub matrix_layout: String,
    pub key_fields: Vec<String>,
    pub required_record_fields: Vec<String>,
    pub lod_order: Vec<String>,
    pub runtime_group_order: Vec<String>,
    pub socket_order: Vec<String>,
    pub residual_limit: f64,
    pub schema_digest: String,
}

impl GeneForgeAssemblyPreparationContract {
    fn validate(&self) -> Result<(), GeneForgeCatalogError> {
        if self.schema != GENEFORGE_ASSEMBLY_PREPARATION_SCHEMA
            || self.version != 2
            || self.augmentor_version != GENEFORGE_ASSEMBLY_AUGMENTOR_VERSION
            || self.transform_space != GENEFORGE_ASSEMBLY_TRANSFORM_SPACE
            || self.matrix_layout != GENEFORGE_ASSEMBLY_MATRIX_LAYOUT
            || self.key_fields
                != [
                    "source_family_id",
                    "source_asset_id",
                    "target_torso_asset_id",
                    "lod",
                    "runtime_group",
                    "socket",
                ]
            || self.required_record_fields
                != [
                    "source_family_id",
                    "source_asset_id",
                    "target_torso_asset_id",
                    "lod",
                    "runtime_group",
                    "socket",
                    "transform_space",
                    "schema_digest",
                    "prepared_matrix",
                    "residual",
                ]
            || self.lod_order != ["full", "compact", "impostor"]
            || self.runtime_group_order
                != [
                    "head",
                    "torso",
                    "left-arm",
                    "right-arm",
                    "left-leg",
                    "right-leg",
                    "tail-back",
                ]
            || self.socket_order
                != [
                    "neck",
                    "left-shoulder",
                    "right-shoulder",
                    "left-hip",
                    "right-hip",
                    "tail-base",
                    "torso-frame",
                ]
            || self.residual_limit != 0.025
            || !valid_sha256(&self.schema_digest)
            || self.schema_digest != self.calculated_schema_digest()?
        {
            return Err(GeneForgeCatalogError::InvalidCatalogMetadata {
                reason: "invalid assembly preparation contract or schema digest",
            });
        }
        Ok(())
    }

    pub fn calculated_schema_digest(&self) -> Result<String, GeneForgeCatalogError> {
        let mut descriptor = BTreeMap::new();
        descriptor.insert(
            "augmentor_version",
            serde_json::json!(self.augmentor_version),
        );
        descriptor.insert("key_fields", serde_json::json!(self.key_fields));
        descriptor.insert("lod_order", serde_json::json!(self.lod_order));
        descriptor.insert("matrix_layout", serde_json::json!(self.matrix_layout));
        descriptor.insert(
            "required_record_fields",
            serde_json::json!(self.required_record_fields),
        );
        descriptor.insert("residual_limit", serde_json::json!(self.residual_limit));
        descriptor.insert(
            "runtime_group_order",
            serde_json::json!(self.runtime_group_order),
        );
        descriptor.insert("schema", serde_json::json!(self.schema));
        descriptor.insert("schema_digest", serde_json::json!("0".repeat(64)));
        descriptor.insert("socket_order", serde_json::json!(self.socket_order));
        descriptor.insert("transform_space", serde_json::json!(self.transform_space));
        descriptor.insert("version", serde_json::json!(self.version));
        let bytes = serde_json::to_vec(&descriptor).map_err(|_| {
            GeneForgeCatalogError::InvalidCatalogMetadata {
                reason: "assembly preparation descriptor serialization failed",
            }
        })?;
        Ok(sha256_hex(&bytes))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeSocketEvidence {
    pub socket: String,
    pub source_anchor: [f64; 3],
    pub target_anchor: [f64; 3],
    pub transformed_source_anchor: [f64; 3],
    pub residual: f64,
    pub prepared_vertex_count: usize,
    pub applied_overlap_depth: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneForgeGroupTransform {
    pub source_family_id: u16,
    pub source_asset_id: CreaturePartAssetId,
    pub target_torso_asset_id: CreaturePartAssetId,
    pub lod: CreaturePartLodId,
    pub runtime_group: String,
    pub socket: String,
    pub transform_space: String,
    pub schema_digest: String,
    pub prepared_matrix: [f64; 16],
    pub residual: f64,
    #[serde(default)]
    pub socket_evidence: Vec<GeneForgeSocketEvidence>,
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
    pub assembly_preparation_contract: GeneForgeAssemblyPreparationContract,
    pub group_key_counts: GeneForgeGroupKeyCounts,
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

    pub fn validate_group_transform(
        &self,
        transform: &GeneForgeGroupTransform,
        cross_torso: bool,
    ) -> Result<(), GeneForgeCatalogError> {
        let family = self
            .families
            .get(transform.source_family_id as usize)
            .filter(|family| family.id.0 == transform.source_family_id)
            .ok_or(GeneForgeCatalogError::InvalidCatalogMetadata {
                reason: "assembly transform source family is outside 0..=11",
            })?;
        let asset = self.asset(&transform.source_asset_id).ok_or_else(|| {
            GeneForgeCatalogError::UnknownAsset {
                family: family.id,
                slot: CreaturePartSlotKey::Head,
                asset: transform.source_asset_id.clone(),
            }
        })?;
        let expected_group: &[&str] = match asset.logical_slot {
            CreaturePartSlotKey::Head => &["head"],
            CreaturePartSlotKey::Torso => &["torso"],
            CreaturePartSlotKey::Arms => &["left-arm", "right-arm"],
            CreaturePartSlotKey::Legs => &["left-leg", "right-leg"],
            CreaturePartSlotKey::Tail => &["tail-back"],
        };
        if family
            .parts
            .get(&asset.logical_slot)
            .map(|part| &part.asset_id)
            != Some(&transform.source_asset_id)
            || !expected_group.contains(&transform.runtime_group.as_str())
            || (transform.runtime_group == "torso" && transform.socket != "torso-frame")
            || (transform.runtime_group != "torso"
                && transform.socket
                    != match transform.runtime_group.as_str() {
                        "head" => "neck",
                        "left-arm" => "left-shoulder",
                        "right-arm" => "right-shoulder",
                        "left-leg" => "left-hip",
                        "right-leg" => "right-hip",
                        "tail-back" => "tail-base",
                        _ => "",
                    })
            || transform.transform_space != self.assembly_preparation_contract.transform_space
            || transform.schema_digest != self.assembly_preparation_contract.schema_digest
            || !transform
                .prepared_matrix
                .iter()
                .all(|value| value.is_finite())
            || transform.prepared_matrix[12..] != [0.0, 0.0, 0.0, 1.0]
            || !transform.residual.is_finite()
            || !(0.0..=self.assembly_preparation_contract.residual_limit)
                .contains(&transform.residual)
            || !matches!(
                transform.target_torso_asset_id.0.as_str(),
                "norn-torso" | "ettin-torso" | "grendel-torso"
            )
            || asset.logical_slot == CreaturePartSlotKey::Torso && cross_torso
            || (cross_torso
                && family.parts[&CreaturePartSlotKey::Torso].asset_id
                    == transform.target_torso_asset_id)
            || (!cross_torso
                && family.parts[&CreaturePartSlotKey::Torso].asset_id
                    != transform.target_torso_asset_id)
        {
            return Err(GeneForgeCatalogError::InvalidCatalogMetadata {
                reason: "invalid assembly group transform identity or affine contract",
            });
        }
        Ok(())
    }

    pub fn validate_group_transforms<'a>(
        &self,
        transforms: impl IntoIterator<Item = &'a GeneForgeGroupTransform>,
        cross_torso: bool,
    ) -> Result<(), GeneForgeCatalogError> {
        let mut keys = BTreeSet::new();
        for transform in transforms {
            self.validate_group_transform(transform, cross_torso)?;
            let key = (
                transform.source_family_id,
                transform.source_asset_id.clone(),
                transform.target_torso_asset_id.clone(),
                transform.lod,
                transform.runtime_group.clone(),
                transform.socket.clone(),
            );
            if !keys.insert(key) {
                return Err(GeneForgeCatalogError::InvalidCatalogMetadata {
                    reason: "duplicate assembly group transform key",
                });
            }
        }
        Ok(())
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
        self.assembly_preparation_contract.validate()?;
        self.group_key_counts.validate()?;
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

fn required_feature_anchors(
    logical_slot: CreaturePartSlotKey,
) -> BTreeMap<&'static str, (GeneForgeAnatomyChannel, &'static str)> {
    use GeneForgeAnatomyChannel::*;
    match logical_slot {
        CreaturePartSlotKey::Head => BTreeMap::from([
            ("left-ear", (InnerEar, "head")),
            ("right-ear", (InnerEar, "head")),
            ("muzzle", (Muzzle, "head")),
        ]),
        CreaturePartSlotKey::Torso => BTreeMap::from([("belly", (Belly, "torso"))]),
        CreaturePartSlotKey::Arms => BTreeMap::from([
            ("left-hand", (HandsFeet, "left-arm")),
            ("right-hand", (HandsFeet, "right-arm")),
        ]),
        CreaturePartSlotKey::Legs => BTreeMap::from([
            ("left-foot", (HandsFeet, "left-leg")),
            ("right-foot", (HandsFeet, "right-leg")),
        ]),
        CreaturePartSlotKey::Tail => BTreeMap::from([("tail-tip", (KeratinSkin, "tail-back"))]),
    }
}

fn validate_anatomy_authoring(asset: &GeneForgePartAssetDefinition) -> Result<(), &'static str> {
    let profile = &asset.anatomy_authoring;
    let mut expected_source_groups = asset.groups.values().cloned().collect::<BTreeSet<_>>();
    for role in asset.detail_groups.keys() {
        let role = serde_json::to_value(role)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .ok_or("detail role cannot be represented as a stable string")?;
        expected_source_groups.insert(format!("head.{role}"));
    }
    let expected_source_landmarks = asset
        .landmarks
        .keys()
        .map(|landmark| {
            serde_json::to_value(landmark)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .ok_or("landmark cannot be represented as a stable string")
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let expected_feature_anchors = required_feature_anchors(asset.logical_slot);
    if profile.schema != "alife.geneforge_anatomy_authoring.v2"
        || profile.coordinate_space != "same-lod-staged-obj"
        || profile.default_channel != GeneForgeAnatomyChannel::Primary
        || profile.projection.schema != "alife.geneforge_anatomy_projection.v1"
        || profile.projection.texel_sample != "pixel-center"
        || profile.projection.triangle_tie_break
            != "inside-max-min-barycentric-then-face-index;nearest-uv-then-face-index"
        || profile.projection.classifier != "source-geometry-feature-anchors.v3"
        || profile.projection.detail_group_channels
            != BTreeMap::from([
                (
                    "head.hair".to_string(),
                    GeneForgeAnatomyChannel::SecondaryMarking,
                ),
                (
                    "head.teeth".to_string(),
                    GeneForgeAnatomyChannel::KeratinSkin,
                ),
            ])
        || profile.projection.source_geometry.schema
            != "alife.geneforge_source_geometry_classifier.v2"
        || profile.projection.source_geometry.groups != expected_source_groups
        || profile
            .projection
            .source_geometry
            .landmarks
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            != expected_source_landmarks
        || profile
            .projection
            .source_geometry
            .landmarks
            .iter()
            .any(|(name, point)| {
                name.is_empty()
                    || point.iter().any(|value| !value.is_finite())
                    || !expected_source_landmarks.contains(name)
            })
        || profile
            .projection
            .source_geometry
            .feature_landmarks
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            != expected_feature_anchors
                .keys()
                .map(|name| (*name).to_string())
                .collect::<BTreeSet<_>>()
        || profile
            .projection
            .source_geometry
            .feature_landmarks
            .iter()
            .any(|(name, feature)| {
                let Some((channel, runtime_group)) = expected_feature_anchors.get(name.as_str())
                else {
                    return true;
                };
                feature.channel != *channel
                    || feature.runtime_group != *runtime_group
                    || feature.source_group != *runtime_group
                    || feature.method != "source-geometry-anchor-v1"
                    || feature.source_basis.is_empty()
                    || feature.source_basis.iter().any(|basis| basis.is_empty())
                    || feature.point.iter().any(|value| !value.is_finite())
                    || feature
                        .source_position
                        .iter()
                        .any(|value| !value.is_finite())
                    || !profile
                        .projection
                        .source_geometry
                        .groups
                        .contains(&feature.source_group)
            })
        || profile.projection.source_geometry.canonical_bounds.min != asset.canonical_bounds.min
        || profile.projection.source_geometry.canonical_bounds.max != asset.canonical_bounds.max
        || profile.source_projection_audit.schema != "alife.geneforge_source_projection_audit.v1"
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
            BTreeSet::from([
                "head",
                "head.eyes",
                "head.lids",
                "head.hair",
                "head.teeth",
                "head.tongue",
            ]),
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
    let _ = allowed;
    let expected_lods = BTreeSet::from([
        CreaturePartLodId::Full,
        CreaturePartLodId::Compact,
        CreaturePartLodId::Impostor,
    ]);
    if profile
        .source_projection_audit
        .lods
        .keys()
        .copied()
        .collect::<BTreeSet<_>>()
        != expected_lods
    {
        return Err("source projection audit must cover every LOD");
    }
    for (lod_id, evidence) in &profile.source_projection_audit.lods {
        let Some(lod) = asset.lods.iter().find(|lod| lod.lod == *lod_id) else {
            return Err("source projection audit references an unknown LOD");
        };
        if !valid_sha256(&evidence.obj_sha256)
            || !valid_sha256(&evidence.semantic_sha256)
            || !valid_sha256(&evidence.projection_sha256)
            || (valid_sha256(&lod.generated_obj_sha256)
                && evidence.obj_sha256 != lod.generated_obj_sha256)
            || (valid_sha256(&lod.semantic_mask_sha256)
                && evidence.semantic_sha256 != lod.semantic_mask_sha256)
            || evidence.projected_texels == 0
            || evidence.projected_texels > 64 * 64
            || evidence.inside_texels + evidence.nearest_texels != evidence.projected_texels
            || evidence.overlap_texels > evidence.inside_texels
            || evidence.runtime_group_counts.is_empty()
            || evidence
                .runtime_group_counts
                .iter()
                .any(|(group, count)| *count == 0 || !groups.contains(group.as_str()))
            || evidence.runtime_group_counts.values().sum::<usize>() != evidence.projected_texels
            || evidence
                .channel_counts
                .keys()
                .copied()
                .collect::<BTreeSet<_>>()
                != required
            || evidence.channel_counts.values().any(|count| *count == 0)
            || evidence.channel_counts.values().sum::<usize>() != evidence.projected_texels
            || evidence
                .source_landmark_projections
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>()
                != profile
                    .projection
                    .source_geometry
                    .landmarks
                    .keys()
                    .cloned()
                    .collect()
            || evidence
                .source_landmark_projections
                .iter()
                .any(|(name, projection)| {
                    name.is_empty()
                        || projection.group.is_empty()
                        || !profile
                            .projection
                            .source_geometry
                            .groups
                            .contains(&projection.group)
                        || projection.source.iter().any(|value| !value.is_finite())
                        || projection.projected.iter().any(|value| !value.is_finite())
                        || projection.weights.iter().any(|value| {
                            !value.is_finite() || *value < -1.0e-6 || *value > 1.0 + 1.0e-6
                        })
                        || (projection.weights.iter().sum::<f32>() - 1.0).abs() > 1.0e-5
                        || !projection.distance.is_finite()
                        || projection.distance < 0.0
                })
            || evidence
                .feature_anchor_ownership
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>()
                != expected_feature_anchors
                    .keys()
                    .map(|name| (*name).to_string())
                    .collect::<BTreeSet<_>>()
            || evidence
                .feature_anchor_ownership
                .iter()
                .any(|(name, ownership)| {
                    let Some((channel, runtime_group)) =
                        expected_feature_anchors.get(name.as_str())
                    else {
                        return true;
                    };
                    ownership.channel != *channel
                        || ownership.owned_channel != *channel
                        || ownership.runtime_group != *runtime_group
                        || ownership.group != *runtime_group
                        || ownership.x >= 64
                        || ownership.y >= 64
                        || ownership.canonical.iter().any(|value| !value.is_finite())
                        || ownership.source.iter().any(|value| !value.is_finite())
                        || ownership.projected.iter().any(|value| !value.is_finite())
                        || ownership.weights.iter().any(|value| !value.is_finite())
                        || (ownership.weights.iter().sum::<f32>() - 1.0).abs() > 1.0e-5
                        || !ownership.distance.is_finite()
                        || ownership.distance < 0.0
                })
            || evidence
                .geometry_classification
                .keys()
                .copied()
                .collect::<BTreeSet<_>>()
                != evidence.channel_counts.keys().copied().collect()
            || evidence
                .geometry_classification
                .iter()
                .any(|(_, classification)| {
                    classification.groups.is_empty()
                        || classification
                            .groups
                            .iter()
                            .any(|group| !profile.projection.source_geometry.groups.contains(group))
                        || classification.landmarks.iter().any(|name| {
                            !profile
                                .projection
                                .source_geometry
                                .landmarks
                                .contains_key(name)
                                && !profile
                                    .projection
                                    .source_geometry
                                    .feature_landmarks
                                    .contains_key(name)
                        })
                })
            || evidence
                .source_bounds
                .min
                .iter()
                .chain(evidence.source_bounds.max.iter())
                .any(|value| !value.is_finite())
            || evidence
                .source_bounds
                .min
                .iter()
                .zip(evidence.source_bounds.max.iter())
                .any(|(min, max)| min > max)
            || evidence.derived_landmarks.is_empty()
            || evidence.derived_landmarks.iter().any(|(name, point)| {
                name.is_empty() || point.iter().any(|value| !value.is_finite())
            })
        {
            return Err("source projection audit is malformed or detached from its LOD");
        }
    }
    Ok(())
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

fn sha256_hex(input: &[u8]) -> String {
    const INITIAL: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut bytes = input.to_vec();
    let bit_len = (bytes.len() as u64) * 8;
    bytes.push(0x80);
    while bytes.len() % 64 != 56 {
        bytes.push(0);
    }
    bytes.extend_from_slice(&bit_len.to_be_bytes());
    let mut state = INITIAL;
    for chunk in bytes.chunks_exact(64) {
        let mut words = [0_u32; 64];
        for (index, word) in chunk.chunks_exact(4).take(16).enumerate() {
            words[index] = u32::from_be_bytes(word.try_into().unwrap());
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let k = K[index];
            let sum1 = h
                .wrapping_add(e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25))
                .wrapping_add((e & f) ^ (!e & g))
                .wrapping_add(k)
                .wrapping_add(words[index]);
            let sum0 = (a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22))
                .wrapping_add((a & b) ^ (a & c) ^ (b & c));
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(sum1);
            d = c;
            c = b;
            b = a;
            a = sum0.wrapping_add(sum1);
        }
        for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
    state
        .into_iter()
        .map(|value| format!("{value:08x}"))
        .collect()
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
            "316d0a54faee2e10ca6df244c6bcc1a1f072bb7f4f8f5b45135500ef2cef6df2"
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
        ownership.part_assets[0]
            .anatomy_authoring
            .projection
            .detail_group_channels
            .insert("head.teeth".into(), GeneForgeAnatomyChannel::Belly);
        assert!(matches!(
            ownership.validate(),
            Err(GeneForgeCatalogError::InvalidAsset { .. })
        ));

        let mut detached = load_geneforge_creature_part_catalog().unwrap();
        detached.part_assets[0]
            .anatomy_authoring
            .source_projection_audit
            .lods
            .get_mut(&CreaturePartLodId::Full)
            .unwrap()
            .obj_sha256 = "0".repeat(64);
        assert!(matches!(
            detached.validate(),
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
    fn geneforge_v2_requires_same_lod_obj_projection_evidence() {
        let json: serde_json::Value = serde_json::from_str(GENEFORGE_RECIPE_CATALOG_JSON).unwrap();
        for asset in json["part_assets"].as_array().unwrap() {
            let authoring = &asset["anatomy_authoring"];
            assert_eq!(authoring["schema"], "alife.geneforge_anatomy_authoring.v2");
            assert_eq!(authoring["coordinate_space"], "same-lod-staged-obj");
            assert_eq!(
                authoring["projection"]["schema"],
                "alife.geneforge_anatomy_projection.v1"
            );
            assert!(authoring.get("zones").is_none());
            let audit = &authoring["source_projection_audit"];
            assert_eq!(
                audit["schema"],
                "alife.geneforge_source_projection_audit.v1"
            );
            for lod in ["full", "compact", "impostor"] {
                let evidence = &audit["lods"][lod];
                for digest in ["obj_sha256", "semantic_sha256", "projection_sha256"] {
                    assert_eq!(evidence[digest].as_str().unwrap().len(), 64);
                }
                assert!(evidence["projected_texels"].as_u64().unwrap() > 0);
                assert_eq!(
                    evidence["projected_texels"].as_u64().unwrap(),
                    evidence["inside_texels"].as_u64().unwrap()
                        + evidence["nearest_texels"].as_u64().unwrap()
                );
            }
        }

        let mut missing = json;
        missing["part_assets"][0]["anatomy_authoring"]
            .as_object_mut()
            .unwrap()
            .remove("source_projection_audit");
        assert!(matches!(
            GeneForgeCreaturePartCatalog::from_json_str(&missing.to_string()),
            Err(GeneForgeCatalogError::Json(_))
        ));
    }

    #[test]
    fn geneforge_v2_rejects_malformed_source_projection_counts_and_bounds() {
        for mutation in 0..3 {
            let mut catalog = load_geneforge_creature_part_catalog().unwrap();
            let evidence = catalog.part_assets[0]
                .anatomy_authoring
                .source_projection_audit
                .lods
                .get_mut(&CreaturePartLodId::Full)
                .unwrap();
            match mutation {
                0 => evidence.projected_texels += 1,
                1 => evidence.source_bounds.min[0] = evidence.source_bounds.max[0] + 1.0,
                _ => {
                    evidence.runtime_group_counts.insert("alternate".into(), 1);
                }
            }
            assert!(matches!(
                catalog.validate(),
                Err(GeneForgeCatalogError::InvalidAsset { .. })
            ));
        }
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

    #[test]
    fn task_5c_catalog_exposes_and_validates_preparation_contract() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        let contract = &catalog.assembly_preparation_contract;
        assert_eq!(contract.schema, "alife.geneforge_assembly_preparation.v2");
        assert_eq!(contract.version, 2);
        assert_eq!(
            contract.augmentor_version,
            "alife.geneforge_assembly_augmentor.v1"
        );
        assert_eq!(contract.residual_limit, 0.025);
        assert_eq!(catalog.group_key_counts.canonical, 252);
        assert_eq!(catalog.group_key_counts.cross_torso, 432);
        assert_eq!(catalog.group_key_counts.total, 684);
        assert_eq!(
            contract.schema_digest,
            contract.calculated_schema_digest().unwrap()
        );

        let valid = GeneForgeGroupTransform {
            source_family_id: 0,
            source_asset_id: CreaturePartAssetId("norn-head".into()),
            target_torso_asset_id: CreaturePartAssetId("ettin-torso".into()),
            lod: CreaturePartLodId::Full,
            runtime_group: "head".into(),
            socket: "neck".into(),
            transform_space: contract.transform_space.clone(),
            schema_digest: contract.schema_digest.clone(),
            prepared_matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            residual: 0.0,
            socket_evidence: Vec::new(),
        };
        assert!(catalog.validate_group_transform(&valid, false).is_ok());
        assert!(catalog.validate_group_transforms([&valid], false).is_ok());
        assert!(catalog
            .validate_group_transforms([&valid, &valid], false)
            .is_err());

        let mut mutations = Vec::new();
        let mut value = valid.clone();
        value.source_family_id = 12;
        mutations.push(value);
        let mut value = valid.clone();
        value.source_asset_id = CreaturePartAssetId("grendel-head".into());
        mutations.push(value);
        let mut value = valid.clone();
        value.target_torso_asset_id = CreaturePartAssetId("unknown-torso".into());
        mutations.push(value);
        let mut value = valid.clone();
        value.runtime_group = "left-arm".into();
        mutations.push(value);
        let mut value = valid.clone();
        value.socket = "left-shoulder".into();
        mutations.push(value);
        let mut value = valid.clone();
        value.transform_space = "wrong-space".into();
        mutations.push(value);
        let mut value = valid.clone();
        value.schema_digest = "0".repeat(64);
        mutations.push(value);
        let mut value = valid.clone();
        value.prepared_matrix[0] = f64::NAN;
        mutations.push(value);
        let mut value = valid.clone();
        value.prepared_matrix[15] = 2.0;
        mutations.push(value);
        let mut value = valid.clone();
        value.residual = 0.025_000_1;
        mutations.push(value);
        for mutation in mutations {
            assert!(catalog.validate_group_transform(&mutation, false).is_err());
        }
    }
}
