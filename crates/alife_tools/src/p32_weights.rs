//! Optional P32 generated initial-weight asset contracts and tooling.
//!
//! These assets are offline birth/initialization inputs. They can carry
//! D2NWG-style generated inherited weights, but they do not require a live ML
//! stack and they never contain lifetime-learned layers.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use alife_core::{
    validate_finite, BrainClassId, BrainClassRegistry, BrainClassSpec, BrainScaleTier, CooEntry,
    CooTile, LobeKind, NeuralProjectionSchema, NormalizedScalar, ProjectionTile,
    ScaffoldContractError, SparseTileCoord, SynapseWeightSplit, WeightSplitContract,
    MICROTILE_EDGE,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const P32_INITIAL_WEIGHT_ASSET_SCHEMA: &str = "alife.p32.initial_weight_asset.v1";
pub const P32_INITIAL_WEIGHT_ASSET_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Error)]
pub enum GeneratedWeightAssetError {
    #[error("asset schema mismatch: expected '{expected}', got '{actual}'")]
    WrongSchema {
        expected: &'static str,
        actual: String,
    },
    #[error("asset schema version mismatch: expected {expected}, got {actual}")]
    WrongSchemaVersion { expected: u16, actual: u16 },
    #[error("asset brain class does not match canonical registry")]
    BrainClassMismatch,
    #[error("asset lobe layout hash mismatch")]
    LobeLayoutHashMismatch,
    #[error("asset validation digest mismatch")]
    ValidationDigestMismatch,
    #[error("asset density or mask metadata is inconsistent")]
    DensityMetadataMismatch,
    #[error("asset contains lifetime-learned or operational payload")]
    LifetimePayloadPresent,
    #[error("asset references an invalid lobe boundary or sparse projection")]
    InvalidLobeBoundary,
    #[error("asset contains duplicate synapse entries")]
    DuplicateSynapse,
    #[error("asset contract failed: {0}")]
    Contract(#[from] ScaffoldContractError),
    #[error("failed to read or write generated weight asset: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse generated weight asset JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneratedWeightTemplate {
    SurvivalBaseline,
    CuriousExplorer,
    SocialLearner,
    LanguageBiasedLexicon,
    NeutralControl,
}

impl GeneratedWeightTemplate {
    pub const ALL: [Self; 5] = [
        Self::SurvivalBaseline,
        Self::CuriousExplorer,
        Self::SocialLearner,
        Self::LanguageBiasedLexicon,
        Self::NeutralControl,
    ];

    pub const fn stable_salt(self) -> u64 {
        match self {
            Self::SurvivalBaseline => 0x5032_0001,
            Self::CuriousExplorer => 0x5032_0002,
            Self::SocialLearner => 0x5032_0003,
            Self::LanguageBiasedLexicon => 0x5032_0004,
            Self::NeutralControl => 0x5032_0005,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneratedWeightProvenanceKind {
    ProceduralFallback,
    ExternalD2nwg,
    TinyFixture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct D2nwgExternalHook {
    pub hook_schema_version: u16,
    pub command_hint: String,
    pub expected_input_contract: String,
    pub expected_output_schema: String,
    pub python_or_ml_required_by_rust_runtime: bool,
}

impl D2nwgExternalHook {
    pub fn documented_optional_json_hook(command_hint: impl Into<String>) -> Self {
        Self {
            hook_schema_version: 1,
            command_hint: command_hint.into(),
            expected_input_contract:
                "offline script reads brain class, lobe layout hash, template, and seed".to_string(),
            expected_output_schema: P32_INITIAL_WEIGHT_ASSET_SCHEMA.to_string(),
            python_or_ml_required_by_rust_runtime: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedWeightProvenance {
    pub kind: GeneratedWeightProvenanceKind,
    pub template: GeneratedWeightTemplate,
    pub generator_name: String,
    pub generator_version: String,
    pub seed: u64,
    pub external_hook: Option<D2nwgExternalHook>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedWeightLobeRange {
    pub lobe: LobeKind,
    pub start: u32,
    pub len: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedWeightLobeLayoutDigest {
    pub hash_algorithm: String,
    pub hash: String,
    pub ranges: Vec<GeneratedWeightLobeRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeneratedSynapseWeight {
    pub projection_index: u32,
    pub source: u32,
    pub target: u32,
    pub genetic_fixed: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WGeneticFixedPayload {
    pub encoding: String,
    pub entries: Vec<GeneratedSynapseWeight>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeneratedSynapseAlpha {
    pub source: u32,
    pub target: u32,
    pub alpha: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeneratedTileAlpha {
    pub projection_index: u32,
    pub microtile_row: u32,
    pub microtile_col: u32,
    pub alpha: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedAlphaMaskPayload {
    pub storage_policy: String,
    pub default_alpha: f32,
    pub tile_overrides: Vec<GeneratedTileAlpha>,
    pub synapse_overrides: Vec<GeneratedSynapseAlpha>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedWeightDensityMetadata {
    pub active_synapses: u32,
    pub active_microtiles: u32,
    pub max_active_synapses: u32,
    pub max_active_microtiles: u32,
    pub density_ratio: f32,
    pub active_supertiles: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedWeightSplitMetadata {
    pub genetic_fixed_payload_present: bool,
    pub lifetime_consolidated_payload_present: bool,
    pub h_operational_payload_present: bool,
    pub h_shadow_payload_present: bool,
    pub generated_asset_is_birth_initialization_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedInitialWeightAsset {
    pub schema: String,
    pub schema_version: u16,
    pub brain_class: BrainScaleTier,
    pub brain_class_id: BrainClassId,
    pub neuron_count: u32,
    pub lobe_layout: GeneratedWeightLobeLayoutDigest,
    pub w_genetic_fixed: WGeneticFixedPayload,
    pub alpha_mask: GeneratedAlphaMaskPayload,
    pub density: GeneratedWeightDensityMetadata,
    pub split: GeneratedWeightSplitMetadata,
    pub provenance: GeneratedWeightProvenance,
    pub validation_digest: String,
}

impl GeneratedInitialWeightAsset {
    pub fn procedural_fallback(
        spec: &BrainClassSpec,
        template: GeneratedWeightTemplate,
        seed: u64,
    ) -> Result<Self, GeneratedWeightAssetError> {
        let schema = NeuralProjectionSchema::from_routing_for_fixture(
            spec,
            &WeightSplitContract::for_brain_class(
                spec.id,
                spec.max_active_synapses,
                spec.max_active_microtiles,
                seed.max(1),
            )?,
        )?;
        let mut rng = DeterministicGenerator::new(seed ^ template.stable_salt());
        let target_entries = procedural_entry_count(spec, template);
        let mut entries = Vec::with_capacity(target_entries as usize);

        while entries.len() < target_entries as usize {
            for projection in &schema.projections {
                if entries.len() >= target_entries as usize {
                    break;
                }
                let source =
                    projection.source_range.start + rng.next_bounded(projection.source_range.len);
                let target =
                    projection.target_range.start + rng.next_bounded(projection.target_range.len);
                let source = align_to_lobe_index(source, projection.source_range.start);
                let target = align_to_lobe_index(target, projection.target_range.start);
                let weight = template_weight(template, &mut rng);
                entries.push(GeneratedSynapseWeight {
                    projection_index: projection.projection_index,
                    source,
                    target,
                    genetic_fixed: weight,
                });
            }
        }

        let mut asset = Self::from_entries(
            spec,
            template,
            GeneratedWeightProvenanceKind::ProceduralFallback,
            seed,
            entries,
            Some(D2nwgExternalHook::documented_optional_json_hook(
                "external D2NWG scripts may emit this JSON schema; no Rust runtime hook is required",
            )),
        )?;
        asset
            .provenance
            .notes
            .push("deterministic procedural fallback, not a trained D2NWG model".to_string());
        asset.validation_digest = compute_validation_digest(&asset);
        asset.validate_against_spec(spec)?;
        Ok(asset)
    }

    pub fn from_entries(
        spec: &BrainClassSpec,
        template: GeneratedWeightTemplate,
        provenance_kind: GeneratedWeightProvenanceKind,
        seed: u64,
        entries: Vec<GeneratedSynapseWeight>,
        external_hook: Option<D2nwgExternalHook>,
    ) -> Result<Self, GeneratedWeightAssetError> {
        spec.validate()?;
        let active_tiles = count_active_microtiles(&entries);
        let active_supertiles = count_active_supertyles(&entries);
        let max_active_synapses = spec.max_active_synapses;
        let max_active_microtiles = spec.max_active_microtiles;
        let density_ratio = if max_active_synapses == 0 {
            0.0
        } else {
            entries.len() as f32 / max_active_synapses as f32
        };
        let default_alpha = match template {
            GeneratedWeightTemplate::SurvivalBaseline => 0.2,
            GeneratedWeightTemplate::CuriousExplorer => 0.45,
            GeneratedWeightTemplate::SocialLearner => 0.35,
            GeneratedWeightTemplate::LanguageBiasedLexicon => 0.4,
            GeneratedWeightTemplate::NeutralControl => 0.25,
        };
        let synapse_overrides = entries
            .iter()
            .map(|entry| GeneratedSynapseAlpha {
                source: entry.source,
                target: entry.target,
                alpha: default_alpha,
            })
            .collect();

        let mut asset = Self {
            schema: P32_INITIAL_WEIGHT_ASSET_SCHEMA.to_string(),
            schema_version: P32_INITIAL_WEIGHT_ASSET_SCHEMA_VERSION,
            brain_class: spec.tier,
            brain_class_id: spec.id,
            neuron_count: spec.neuron_count,
            lobe_layout: lobe_layout_digest(spec),
            w_genetic_fixed: WGeneticFixedPayload {
                encoding: "sparse_coo_f32_genetic_fixed_only".to_string(),
                entries,
            },
            alpha_mask: GeneratedAlphaMaskPayload {
                storage_policy: "hierarchical_sparse_with_synapse_overrides".to_string(),
                default_alpha,
                tile_overrides: Vec::new(),
                synapse_overrides,
            },
            density: GeneratedWeightDensityMetadata {
                active_synapses: 0,
                active_microtiles: active_tiles,
                max_active_synapses,
                max_active_microtiles,
                density_ratio,
                active_supertiles,
            },
            split: GeneratedWeightSplitMetadata {
                genetic_fixed_payload_present: true,
                lifetime_consolidated_payload_present: false,
                h_operational_payload_present: false,
                h_shadow_payload_present: false,
                generated_asset_is_birth_initialization_only: true,
            },
            provenance: GeneratedWeightProvenance {
                kind: provenance_kind,
                template,
                generator_name: "alife_tools::p32_weights".to_string(),
                generator_version: env!("CARGO_PKG_VERSION").to_string(),
                seed: seed.max(1),
                external_hook,
                notes: Vec::new(),
            },
            validation_digest: String::new(),
        };
        asset.density.active_synapses = asset.w_genetic_fixed.entries.len() as u32;
        asset.validation_digest = compute_validation_digest(&asset);
        asset.validate_against_spec(spec)?;
        Ok(asset)
    }

    pub fn validate_against_spec(
        &self,
        spec: &BrainClassSpec,
    ) -> Result<(), GeneratedWeightAssetError> {
        spec.validate()?;
        if self.schema != P32_INITIAL_WEIGHT_ASSET_SCHEMA {
            return Err(GeneratedWeightAssetError::WrongSchema {
                expected: P32_INITIAL_WEIGHT_ASSET_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        if self.schema_version != P32_INITIAL_WEIGHT_ASSET_SCHEMA_VERSION {
            return Err(GeneratedWeightAssetError::WrongSchemaVersion {
                expected: P32_INITIAL_WEIGHT_ASSET_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        if self.brain_class != spec.tier
            || self.brain_class_id != spec.id
            || self.neuron_count != spec.neuron_count
        {
            return Err(GeneratedWeightAssetError::BrainClassMismatch);
        }
        if self.lobe_layout.hash != lobe_layout_digest(spec).hash {
            return Err(GeneratedWeightAssetError::LobeLayoutHashMismatch);
        }
        if !self.split.genetic_fixed_payload_present
            || self.split.lifetime_consolidated_payload_present
            || self.split.h_operational_payload_present
            || self.split.h_shadow_payload_present
            || !self.split.generated_asset_is_birth_initialization_only
        {
            return Err(GeneratedWeightAssetError::LifetimePayloadPresent);
        }
        validate_density(self, spec)?;
        validate_alpha_payload(&self.alpha_mask)?;
        validate_entries(self, spec)?;
        if self.validation_digest != compute_validation_digest(self) {
            return Err(GeneratedWeightAssetError::ValidationDigestMismatch);
        }
        Ok(())
    }

    pub fn validate_canonical(&self) -> Result<(), GeneratedWeightAssetError> {
        let spec = BrainClassRegistry::spec_for_id(self.brain_class_id)
            .ok_or(GeneratedWeightAssetError::BrainClassMismatch)?;
        self.validate_against_spec(&spec)
    }

    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, GeneratedWeightAssetError> {
        let text = fs::read_to_string(path)?;
        let asset: Self = serde_json::from_str(&text)?;
        asset.validate_canonical()?;
        Ok(asset)
    }

    pub fn to_json_file(&self, path: impl AsRef<Path>) -> Result<(), GeneratedWeightAssetError> {
        self.validate_canonical()?;
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn import_to_projection_schema(
        &self,
    ) -> Result<NeuralProjectionSchema, GeneratedWeightAssetError> {
        let spec = BrainClassRegistry::spec_for_id(self.brain_class_id)
            .ok_or(GeneratedWeightAssetError::BrainClassMismatch)?;
        self.validate_against_spec(&spec)?;
        let split = WeightSplitContract::for_brain_class(
            spec.id,
            spec.max_active_synapses,
            spec.max_active_microtiles,
            self.provenance.seed.max(1),
        )?;
        let mut schema = NeuralProjectionSchema::from_routing_for_fixture(&spec, &split)?;
        let mut grouped: BTreeMap<(u32, u32, u32), Vec<CooEntry>> = BTreeMap::new();
        for entry in &self.w_genetic_fixed.entries {
            let coord = SparseTileCoord::from_neuron_indices(entry.target, entry.source)?;
            let alpha = self.alpha_for_synapse(entry.source, entry.target)?;
            let weights = SynapseWeightSplit::new(entry.genetic_fixed, 0.0, alpha, 0.0, 0.0)?;
            grouped
                .entry((
                    entry.projection_index,
                    coord.microtile_row,
                    coord.microtile_col,
                ))
                .or_default()
                .push(CooEntry::new(
                    (entry.target - coord.target_start()) as u8,
                    (entry.source - coord.source_start()) as u8,
                    weights,
                )?);
        }
        for ((projection_index, row, col), entries) in grouped {
            let projection = schema
                .projections
                .get_mut(projection_index as usize)
                .ok_or(GeneratedWeightAssetError::InvalidLobeBoundary)?;
            projection.tiles.push(ProjectionTile::new_coo(
                projection_index,
                SparseTileCoord::new(row, col)?,
                CooTile::new(entries)?,
            ));
        }
        schema.rebuild_supertile_masks();
        schema.validate()?;
        Ok(schema)
    }

    pub fn export_from_projection_schema(
        spec: &BrainClassSpec,
        schema: &NeuralProjectionSchema,
        template: GeneratedWeightTemplate,
        seed: u64,
    ) -> Result<Self, GeneratedWeightAssetError> {
        spec.validate()?;
        schema.validate()?;
        if schema.brain_class_id != spec.id || schema.neuron_count != spec.neuron_count {
            return Err(GeneratedWeightAssetError::BrainClassMismatch);
        }
        let mut entries = Vec::new();
        let mut alpha_overrides = Vec::new();
        for projection in &schema.projections {
            for tile in &projection.tiles {
                for synapse in tile.decode_synapses()? {
                    if synapse.weights.lifetime_consolidated != 0.0
                        || synapse.weights.h_operational != 0.0
                        || synapse.weights.h_shadow != 0.0
                    {
                        return Err(GeneratedWeightAssetError::LifetimePayloadPresent);
                    }
                    entries.push(GeneratedSynapseWeight {
                        projection_index: projection.projection_index,
                        source: synapse.source,
                        target: synapse.target,
                        genetic_fixed: synapse.weights.genetic_fixed,
                    });
                    alpha_overrides.push(GeneratedSynapseAlpha {
                        source: synapse.source,
                        target: synapse.target,
                        alpha: synapse.weights.alpha,
                    });
                }
            }
        }
        let mut asset = Self::from_entries(
            spec,
            template,
            GeneratedWeightProvenanceKind::ProceduralFallback,
            seed,
            entries,
            None,
        )?;
        asset.alpha_mask.default_alpha = 0.0;
        asset.alpha_mask.synapse_overrides = alpha_overrides;
        asset.validation_digest = compute_validation_digest(&asset);
        asset.validate_against_spec(spec)?;
        Ok(asset)
    }

    fn alpha_for_synapse(
        &self,
        source: u32,
        target: u32,
    ) -> Result<f32, GeneratedWeightAssetError> {
        for override_alpha in &self.alpha_mask.synapse_overrides {
            if override_alpha.source == source && override_alpha.target == target {
                NormalizedScalar::new(override_alpha.alpha)?;
                return Ok(override_alpha.alpha);
            }
        }
        NormalizedScalar::new(self.alpha_mask.default_alpha)?;
        Ok(self.alpha_mask.default_alpha)
    }
}

pub fn lobe_layout_digest(spec: &BrainClassSpec) -> GeneratedWeightLobeLayoutDigest {
    let ranges: Vec<_> = spec
        .lobe_layout
        .iter_regions()
        .map(|region| GeneratedWeightLobeRange {
            lobe: region.kind,
            start: region.start,
            len: region.len,
        })
        .collect();
    let mut hasher = StableHasher::new();
    hasher.feed_str("p32-lobe-layout-v1");
    hasher.feed_u16(spec.id.raw());
    hasher.feed_u32(spec.neuron_count);
    for range in &ranges {
        hasher.feed_u16(range.lobe.stable_id().raw());
        hasher.feed_u32(range.start);
        hasher.feed_u32(range.len);
    }
    GeneratedWeightLobeLayoutDigest {
        hash_algorithm: "fnv1a64".to_string(),
        hash: hasher.finish_hex(),
        ranges,
    }
}

pub fn compute_validation_digest(asset: &GeneratedInitialWeightAsset) -> String {
    let mut hasher = StableHasher::new();
    hasher.feed_str(&asset.schema);
    hasher.feed_u16(asset.schema_version);
    hasher.feed_u16(asset.brain_class_id.raw());
    hasher.feed_u32(asset.neuron_count);
    hasher.feed_str(&asset.lobe_layout.hash);
    for entry in &asset.w_genetic_fixed.entries {
        hasher.feed_u32(entry.projection_index);
        hasher.feed_u32(entry.source);
        hasher.feed_u32(entry.target);
        hasher.feed_u32(entry.genetic_fixed.to_bits());
    }
    hasher.feed_u32(asset.density.active_synapses);
    hasher.feed_u32(asset.density.active_microtiles);
    hasher.feed_u32(asset.density.max_active_synapses);
    hasher.feed_u32(asset.density.max_active_microtiles);
    hasher.feed_u32(asset.density.density_ratio.to_bits());
    hasher.feed_u32(asset.density.active_supertiles);
    hasher.feed_u32(asset.alpha_mask.default_alpha.to_bits());
    for entry in &asset.alpha_mask.tile_overrides {
        hasher.feed_u32(entry.projection_index);
        hasher.feed_u32(entry.microtile_row);
        hasher.feed_u32(entry.microtile_col);
        hasher.feed_u32(entry.alpha.to_bits());
    }
    for entry in &asset.alpha_mask.synapse_overrides {
        hasher.feed_u32(entry.source);
        hasher.feed_u32(entry.target);
        hasher.feed_u32(entry.alpha.to_bits());
    }
    hasher.feed_u64(asset.provenance.seed);
    hasher.finish_hex()
}

fn validate_density(
    asset: &GeneratedInitialWeightAsset,
    spec: &BrainClassSpec,
) -> Result<(), GeneratedWeightAssetError> {
    validate_finite(asset.density.density_ratio)?;
    let active_synapses = asset.w_genetic_fixed.entries.len() as u32;
    let active_tiles = count_active_microtiles(&asset.w_genetic_fixed.entries);
    if asset.density.active_synapses != active_synapses
        || asset.density.active_microtiles != active_tiles
        || asset.density.max_active_synapses != spec.max_active_synapses
        || asset.density.max_active_microtiles != spec.max_active_microtiles
        || active_synapses > spec.max_active_synapses
        || active_tiles > spec.max_active_microtiles
        || !(0.0..=1.0).contains(&asset.density.density_ratio)
    {
        return Err(GeneratedWeightAssetError::DensityMetadataMismatch);
    }
    Ok(())
}

fn validate_alpha_payload(
    alpha: &GeneratedAlphaMaskPayload,
) -> Result<(), GeneratedWeightAssetError> {
    NormalizedScalar::new(alpha.default_alpha)?;
    for entry in &alpha.tile_overrides {
        NormalizedScalar::new(entry.alpha)?;
    }
    for entry in &alpha.synapse_overrides {
        NormalizedScalar::new(entry.alpha)?;
    }
    Ok(())
}

fn validate_entries(
    asset: &GeneratedInitialWeightAsset,
    spec: &BrainClassSpec,
) -> Result<(), GeneratedWeightAssetError> {
    let split = WeightSplitContract::for_brain_class(
        spec.id,
        spec.max_active_synapses,
        spec.max_active_microtiles,
        asset.provenance.seed.max(1),
    )?;
    let schema = NeuralProjectionSchema::from_routing_for_fixture(spec, &split)?;
    let mut seen = BTreeSet::new();
    for entry in &asset.w_genetic_fixed.entries {
        validate_finite(entry.genetic_fixed)?;
        if !seen.insert((entry.projection_index, entry.source, entry.target)) {
            return Err(GeneratedWeightAssetError::DuplicateSynapse);
        }
        let projection = schema
            .projections
            .get(entry.projection_index as usize)
            .ok_or(GeneratedWeightAssetError::InvalidLobeBoundary)?;
        if entry.source >= spec.neuron_count
            || entry.target >= spec.neuron_count
            || spec.lobe_by_neuron_index(entry.source).is_none()
            || spec.lobe_by_neuron_index(entry.target).is_none()
            || entry.source < projection.source_range.start
            || entry.source >= projection.source_range.start + projection.source_range.len
            || entry.target < projection.target_range.start
            || entry.target >= projection.target_range.start + projection.target_range.len
        {
            return Err(GeneratedWeightAssetError::InvalidLobeBoundary);
        }
    }
    Ok(())
}

fn count_active_microtiles(entries: &[GeneratedSynapseWeight]) -> u32 {
    entries
        .iter()
        .map(|entry| {
            (
                entry.projection_index,
                entry.target / MICROTILE_EDGE,
                entry.source / MICROTILE_EDGE,
            )
        })
        .collect::<BTreeSet<_>>()
        .len() as u32
}

fn count_active_supertyles(entries: &[GeneratedSynapseWeight]) -> u32 {
    entries
        .iter()
        .map(|entry| {
            (
                entry.projection_index,
                entry.target / 128,
                entry.source / 128,
            )
        })
        .collect::<BTreeSet<_>>()
        .len() as u32
}

fn procedural_entry_count(spec: &BrainClassSpec, template: GeneratedWeightTemplate) -> u32 {
    let base = match template {
        GeneratedWeightTemplate::NeutralControl => 8,
        GeneratedWeightTemplate::SurvivalBaseline => 16,
        GeneratedWeightTemplate::CuriousExplorer => 20,
        GeneratedWeightTemplate::SocialLearner => 20,
        GeneratedWeightTemplate::LanguageBiasedLexicon => 24,
    };
    base.min(spec.max_active_synapses).max(1)
}

fn template_weight(template: GeneratedWeightTemplate, rng: &mut DeterministicGenerator) -> f32 {
    let raw = rng.next_unit_signed() * 0.35;
    match template {
        GeneratedWeightTemplate::SurvivalBaseline => raw + 0.1,
        GeneratedWeightTemplate::CuriousExplorer => raw + 0.05,
        GeneratedWeightTemplate::SocialLearner => raw + 0.025,
        GeneratedWeightTemplate::LanguageBiasedLexicon => raw + 0.075,
        GeneratedWeightTemplate::NeutralControl => raw * 0.5,
    }
}

fn align_to_lobe_index(value: u32, lobe_start: u32) -> u32 {
    lobe_start + (value - lobe_start)
}

#[derive(Debug, Clone, Copy)]
struct DeterministicGenerator {
    state: u64,
}

impl DeterministicGenerator {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn next_bounded(&mut self, upper: u32) -> u32 {
        if upper == 0 {
            0
        } else {
            (self.next_u64() % u64::from(upper)) as u32
        }
    }

    fn next_unit_signed(&mut self) -> f32 {
        let value = (self.next_u64() >> 40) as u32;
        (value as f32 / 16_777_215.0) * 2.0 - 1.0
    }
}

struct StableHasher {
    value: u64,
}

impl StableHasher {
    fn new() -> Self {
        Self {
            value: 0xcbf2_9ce4_8422_2325,
        }
    }

    fn feed_u8(&mut self, value: u8) {
        self.value ^= u64::from(value);
        self.value = self.value.wrapping_mul(0x0000_0100_0000_01b3);
    }

    fn feed_u16(&mut self, value: u16) {
        for byte in value.to_le_bytes() {
            self.feed_u8(byte);
        }
    }

    fn feed_u32(&mut self, value: u32) {
        for byte in value.to_le_bytes() {
            self.feed_u8(byte);
        }
    }

    fn feed_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.feed_u8(byte);
        }
    }

    fn feed_str(&mut self, value: &str) {
        for byte in value.as_bytes() {
            self.feed_u8(*byte);
        }
        self.feed_u8(0xff);
    }

    fn finish_hex(self) -> String {
        format!("fnv1a64:{:016x}", self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GeneratedInitialWeightAsset, GeneratedWeightAssetError, GeneratedWeightProvenanceKind,
        GeneratedWeightTemplate,
    };
    use alife_core::{BrainClassSpec, BrainScaleTier};

    #[test]
    fn schema_version_mismatch_is_rejected() {
        let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
        let mut asset = GeneratedInitialWeightAsset::procedural_fallback(
            &spec,
            GeneratedWeightTemplate::NeutralControl,
            42,
        )
        .unwrap();
        asset.schema_version = 99;
        asset.validation_digest = super::compute_validation_digest(&asset);
        assert!(matches!(
            asset.validate_against_spec(&spec),
            Err(GeneratedWeightAssetError::WrongSchemaVersion {
                expected: 1,
                actual: 99
            })
        ));
    }

    #[test]
    fn procedural_fallback_is_deterministic() {
        let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
        let left = GeneratedInitialWeightAsset::procedural_fallback(
            &spec,
            GeneratedWeightTemplate::CuriousExplorer,
            1234,
        )
        .unwrap();
        let right = GeneratedInitialWeightAsset::procedural_fallback(
            &spec,
            GeneratedWeightTemplate::CuriousExplorer,
            1234,
        )
        .unwrap();
        assert_eq!(left.validation_digest, right.validation_digest);
        assert_eq!(left.w_genetic_fixed.entries, right.w_genetic_fixed.entries);
    }

    #[test]
    fn alpha_bounds_are_rejected() {
        let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
        let mut asset = GeneratedInitialWeightAsset::procedural_fallback(
            &spec,
            GeneratedWeightTemplate::SurvivalBaseline,
            55,
        )
        .unwrap();
        asset.alpha_mask.default_alpha = 1.25;
        asset.validation_digest = super::compute_validation_digest(&asset);
        assert!(matches!(
            asset.validate_against_spec(&spec),
            Err(GeneratedWeightAssetError::Contract(_))
        ));
    }

    #[test]
    fn density_bounds_are_rejected() {
        let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
        let mut asset = GeneratedInitialWeightAsset::procedural_fallback(
            &spec,
            GeneratedWeightTemplate::SurvivalBaseline,
            55,
        )
        .unwrap();
        asset.density.active_synapses = spec.max_active_synapses + 1;
        asset.validation_digest = super::compute_validation_digest(&asset);
        assert!(matches!(
            asset.validate_against_spec(&spec),
            Err(GeneratedWeightAssetError::DensityMetadataMismatch)
        ));
    }

    #[test]
    fn import_rejects_mismatched_brain_class() {
        let nano = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
        let small = BrainClassSpec::for_tier(BrainScaleTier::Small1024);
        let asset = GeneratedInitialWeightAsset::procedural_fallback(
            &nano,
            GeneratedWeightTemplate::NeutralControl,
            77,
        )
        .unwrap();
        assert!(matches!(
            asset.validate_against_spec(&small),
            Err(GeneratedWeightAssetError::BrainClassMismatch)
        ));
    }

    #[test]
    fn imported_schema_contains_only_genetic_and_alpha_layers() {
        let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
        let asset = GeneratedInitialWeightAsset::procedural_fallback(
            &spec,
            GeneratedWeightTemplate::SocialLearner,
            99,
        )
        .unwrap();
        let schema = asset.import_to_projection_schema().unwrap();
        let decoded = schema.projections[0].tiles[0].decode_synapses().unwrap();
        assert_eq!(decoded[0].weights.lifetime_consolidated, 0.0);
        assert_eq!(decoded[0].weights.h_operational, 0.0);
        assert_eq!(decoded[0].weights.h_shadow, 0.0);
        assert!((0.0..=1.0).contains(&decoded[0].weights.alpha));
    }

    #[test]
    fn export_rejects_lifetime_payload() {
        let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
        let asset = GeneratedInitialWeightAsset::procedural_fallback(
            &spec,
            GeneratedWeightTemplate::SurvivalBaseline,
            101,
        )
        .unwrap();
        let mut schema = asset.import_to_projection_schema().unwrap();
        if let alife_core::SparseTilePayload::Coo(coo) = &mut schema.projections[0].tiles[0].payload
        {
            coo.entries[0].weights.lifetime_consolidated = 0.1;
        }
        assert!(matches!(
            GeneratedInitialWeightAsset::export_from_projection_schema(
                &spec,
                &schema,
                GeneratedWeightTemplate::SurvivalBaseline,
                101,
            ),
            Err(GeneratedWeightAssetError::LifetimePayloadPresent)
        ));
    }

    #[test]
    fn external_hook_is_documented_but_not_required_by_runtime() {
        let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
        let asset = GeneratedInitialWeightAsset::procedural_fallback(
            &spec,
            GeneratedWeightTemplate::LanguageBiasedLexicon,
            303,
        )
        .unwrap();
        let hook = asset.provenance.external_hook.unwrap();
        assert!(!hook.python_or_ml_required_by_rust_runtime);
        assert_eq!(
            asset.provenance.kind,
            GeneratedWeightProvenanceKind::ProceduralFallback
        );
    }

    #[test]
    fn committed_tiny_fixture_imports() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/p32_tiny_initial_weights.json");
        let asset = GeneratedInitialWeightAsset::from_json_file(path).unwrap();
        assert_eq!(asset.brain_class, BrainScaleTier::Nano512);
        assert_eq!(asset.w_genetic_fixed.entries.len(), 8);
        assert!(asset.import_to_projection_schema().is_ok());
    }
}
