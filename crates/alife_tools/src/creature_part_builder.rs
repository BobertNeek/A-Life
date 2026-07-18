use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};

use alife_game_app::{
    CreaturePartFamilyDefinition, CreaturePartLodId, CreaturePartSlot, CutPlane, CutVolume,
    GeneForgeAnatomyChannel, GeneForgeCreaturePartCatalog, GeneForgeDetailRole, GeneForgeDonorId,
    GeneForgePartAssetDefinition, SocketFrame,
};
use alife_world::CreaturePartSlotKey;
use flate2::bufread::ZlibDecoder;
use image::ImageDecoder;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const SLOT_PRIORITY: [CreaturePartSlot; 7] = [
    CreaturePartSlot::Head,
    CreaturePartSlot::LeftArm,
    CreaturePartSlot::RightArm,
    CreaturePartSlot::LeftLeg,
    CreaturePartSlot::RightLeg,
    CreaturePartSlot::TailBack,
    CreaturePartSlot::Torso,
];
const OUTPUT_SLOT_ORDER: [CreaturePartSlot; 7] = [
    CreaturePartSlot::Head,
    CreaturePartSlot::Torso,
    CreaturePartSlot::LeftArm,
    CreaturePartSlot::RightArm,
    CreaturePartSlot::LeftLeg,
    CreaturePartSlot::RightLeg,
    CreaturePartSlot::TailBack,
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjVertex {
    pub position: [f64; 3],
    pub uv: [f64; 2],
    pub normal: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjTriangle {
    pub vertices: [ObjVertex; 3],
    pub source_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceObjMesh {
    pub triangles: Vec<ObjTriangle>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GeneratedPartMesh {
    pub vertices: Vec<ObjVertex>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlicedCreaturePartPack {
    pub family_id: alife_world::CreaturePartFamilyId,
    pub lod: CreaturePartLodId,
    pub parts: BTreeMap<CreaturePartSlot, GeneratedPartMesh>,
    pub source_triangle_count: usize,
    pub source_triangle_owners: BTreeMap<usize, BTreeSet<CreaturePartSlot>>,
    pub source_triangle_fragment_slots: BTreeMap<usize, BTreeSet<CreaturePartSlot>>,
    pub sockets: BTreeMap<String, SocketFrame>,
    pub canonical_source_bounds: [[f64; 3]; 2],
    pub minimum_join_overlap: f32,
    pub obj_bytes: Vec<u8>,
    pub socket_json_bytes: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum CreaturePartBuilderError {
    #[error("OBJ line {line}: {message}")]
    Obj { line: usize, message: String },
    #[error("OBJ contains no triangles")]
    EmptyObj,
    #[error("family {family:?} LOD {lod:?} triangle {triangle} has no cut owner")]
    UnownedTriangle {
        family: alife_world::CreaturePartFamilyId,
        lod: CreaturePartLodId,
        triangle: usize,
    },
    #[error("family {family:?} is missing cut volume {slot:?}")]
    MissingCutVolume {
        family: alife_world::CreaturePartFamilyId,
        slot: CreaturePartSlot,
    },
    #[error("family {family:?} is missing attachment socket {socket}")]
    MissingSocket {
        family: alife_world::CreaturePartFamilyId,
        socket: &'static str,
    },
    #[error("generated part pack is invalid: {0}")]
    InvalidPack(&'static str),
    #[error("generated part {0:?} is invalid: {1}")]
    InvalidPart(CreaturePartSlot, &'static str),
    #[error(
        "family {family:?} LOD {lod:?} generated files exceed 512 KiB: OBJ {obj_bytes} bytes, sockets {socket_bytes} bytes"
    )]
    GeneratedFileTooLarge {
        family: alife_world::CreaturePartFamilyId,
        lod: CreaturePartLodId,
        obj_bytes: usize,
        socket_bytes: usize,
    },
    #[error("socket manifest serialization failed: {0}")]
    SocketJson(#[from] serde_json::Error),
    #[error("GeneForge staging validation failed: {0}")]
    Staging(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneForgeStagingValidation {
    pub donor_count: usize,
    pub asset_count: usize,
    pub lod_count: usize,
    pub obj_count: usize,
    pub mask_count: usize,
    pub anatomy_mask_count: usize,
    pub total_bytes: u64,
}

#[derive(Debug, Deserialize)]
struct GeneForgeBuildReceipt {
    schema: String,
    blender_version: String,
    importer_version: String,
    recipe_sha256: String,
    source_sha256: BTreeMap<String, String>,
    donor_count: usize,
    asset_count: usize,
    lods: Vec<String>,
    worker_execution: GeneForgeWorkerExecution,
    sources: Vec<GeneForgeSourceBuildReceipt>,
    outputs: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct GeneForgeSourceBuildReceipt {
    donor: String,
    asset_count: usize,
    output_count: usize,
    outputs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GeneForgeWorkerExecution {
    strategy: String,
    max_workers: usize,
}

#[derive(Debug, Deserialize)]
struct StagedBounds {
    min: [f64; 3],
    max: [f64; 3],
}

#[derive(Debug, Deserialize)]
struct StagedSocket {
    translation: [f64; 3],
    rotation_xyzw: [f64; 4],
    scale: [f64; 3],
}

#[derive(Debug, Deserialize)]
struct StagedSocketManifest {
    schema: String,
    asset_id: String,
    logical_slot: String,
    donor: String,
    lod: String,
    bounds: StagedBounds,
    sockets: BTreeMap<String, StagedSocket>,
    landmarks: BTreeMap<String, [f64; 3]>,
    ground_contacts: Vec<[f64; 3]>,
    semantic_mask: String,
    anatomy_mask: String,
    lod_topology: StagedObjTopology,
    expected_groups: BTreeSet<String>,
    microdetail: StagedMicrodetail,
    assembly_preparation_schema: String,
    bridge_geometry: Vec<StagedGeometryPreparation>,
    assembly_preparations: Vec<StagedAssemblyPreparation>,
}

#[derive(Debug)]
struct StagedObjSummary {
    bounds: StagedBounds,
    positions: Vec<[f64; 3]>,
    groups: BTreeSet<String>,
    topology: StagedObjTopology,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct StagedObjTopology {
    triangle_count: usize,
    connected_components: usize,
    boundary_edges: usize,
    non_manifold_edges: usize,
    component_ids: BTreeSet<String>,
    component_triangle_counts: BTreeMap<String, usize>,
    component_connected_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Deserialize)]
struct StagedMicrodetail {
    source_files: Vec<String>,
    uvless_fallback: String,
}

#[derive(Debug, Deserialize)]
struct StagedAssemblyPreparation {
    family_id: u16,
    family_label: String,
    logical_slot: String,
    asset_id: String,
    fit: StagedSocket,
    seam_offset: [f64; 3],
    prepared_translation: [f64; 3],
    prepared_matrix: [f64; 16],
    bridge_sockets: Vec<String>,
    bridge_kind: String,
    join_cover_kind: String,
    transform_mode: String,
    target_torso_asset_id: String,
    overlap_depth: f64,
    attachment_error_bound: f64,
    predicted_attachment_error: f64,
    bridge_geometry: Vec<StagedAssemblyBridgeEvidence>,
}

#[derive(Debug, Deserialize)]
struct StagedGeometryPreparation {
    socket: String,
    prepared_vertex_count: usize,
    applied_overlap_depth: f64,
    original_anchor: [f64; 3],
    prepared_anchor: [f64; 3],
}

#[derive(Debug, Deserialize)]
struct StagedAssemblyBridgeEvidence {
    socket: String,
    runtime_group: String,
    prepared_vertex_count: usize,
    applied_overlap_depth: f64,
    original_anchor: [f64; 3],
    prepared_anchor: [f64; 3],
    source_anchor: [f64; 3],
    target_anchor: [f64; 3],
    transformed_source_anchor: [f64; 3],
    prepared_matrix: [f64; 16],
    residual: f64,
}

impl SourceObjMesh {
    pub fn parse(text: &str) -> Result<Self, CreaturePartBuilderError> {
        let mut positions = Vec::<[f64; 3]>::new();
        let mut uvs = Vec::<[f64; 2]>::new();
        let mut normals = Vec::<[f64; 3]>::new();
        let mut triangles = Vec::new();

        for (line_index, raw_line) in text.lines().enumerate() {
            let line_number = line_index + 1;
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split_whitespace();
            match fields.next().unwrap_or_default() {
                "v" => positions.push(parse_vector::<3>(fields, line_number, "position")?),
                "vt" => uvs.push(parse_vector::<2>(fields, line_number, "UV")?),
                "vn" => {
                    let normal = parse_vector::<3>(fields, line_number, "normal")?;
                    normals.push(normalize(normal).ok_or_else(|| {
                        CreaturePartBuilderError::Obj {
                            line: line_number,
                            message: "normal must be nonzero".to_string(),
                        }
                    })?);
                }
                "f" => {
                    let refs = fields
                        .map(|field| {
                            parse_face_ref(
                                field,
                                positions.len(),
                                uvs.len(),
                                normals.len(),
                                line_number,
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if refs.len() < 3 {
                        return Err(CreaturePartBuilderError::Obj {
                            line: line_number,
                            message: "face requires at least three vertices".to_string(),
                        });
                    }
                    for index in 1..refs.len() - 1 {
                        let triangle_refs = [refs[0], refs[index], refs[index + 1]];
                        let vertices = triangle_refs.map(|(position, uv, normal)| ObjVertex {
                            position: positions[position],
                            uv: uvs[uv],
                            normal: normals[normal],
                        });
                        triangles.push(ObjTriangle {
                            vertices,
                            source_index: triangles.len(),
                        });
                    }
                }
                _ => {}
            }
        }
        if triangles.is_empty() {
            return Err(CreaturePartBuilderError::EmptyObj);
        }
        Ok(Self { triangles })
    }
}

pub fn validate_geneforge_staging(
    staging_root: &Path,
    recipe_path: &Path,
) -> Result<GeneForgeStagingValidation, CreaturePartBuilderError> {
    let fail = |message: String| CreaturePartBuilderError::Staging(message);
    let canonical_staging_root = fs::canonicalize(staging_root).map_err(|error| {
        fail(format!(
            "failed canonicalizing staging root {}: {error}",
            staging_root.display()
        ))
    })?;
    if !canonical_staging_root.is_dir() {
        return Err(fail(format!(
            "staging root is not a directory: {}",
            staging_root.display()
        )));
    }
    let recipe_text = fs::read_to_string(recipe_path).map_err(|error| {
        fail(format!(
            "failed reading external GeneForge recipe {}: {error}",
            recipe_path.display()
        ))
    })?;
    let catalog = GeneForgeCreaturePartCatalog::from_json_str(&recipe_text)
        .map_err(|error| fail(format!("invalid external GeneForge recipe: {error}")))?;
    let canonical_digest = canonical_recipe_sha256(&recipe_text)?;
    if !catalog
        .recipe_sha256
        .eq_ignore_ascii_case(&canonical_digest)
    {
        return Err(fail(format!(
            "external recipe digest mismatch: expected {}, calculated {canonical_digest}",
            catalog.recipe_sha256
        )));
    }
    if catalog.schema != "alife.geneforge_creature_part_catalog.v2"
        || catalog.schema_version != 2
        || catalog.blender_version != "5.1.0"
        || catalog.importer_version != "alife.geneforge_importer.v2"
        || catalog.sources.len() != 3
        || catalog.part_assets.len() != 14
        || catalog.families.len() != 12
        || catalog.assembly_contract.schema != "alife.geneforge_family_assembly.v1"
    {
        return Err(fail(
            "external recipe does not match the pinned Task 4 catalog contract".to_string(),
        ));
    }

    let mut expected_outputs = BTreeSet::new();
    let mut obj_contracts = BTreeMap::new();
    let mut socket_contracts = BTreeMap::new();
    let mut mask_contracts = BTreeMap::new();
    let mut anatomy_contracts = BTreeMap::new();
    let mut output_digest_contracts = BTreeMap::new();
    let mut source_output_contracts = BTreeMap::<String, BTreeSet<String>>::new();
    let mut source_asset_counts = BTreeMap::<String, usize>::new();
    for (asset_index, asset) in catalog.part_assets.iter().enumerate() {
        let donor = donor_name(asset.donor).to_string();
        *source_asset_counts.entry(donor.clone()).or_default() += 1;
        if asset.lods.len() != 3 {
            return Err(fail(format!(
                "asset {} does not declare three LODs",
                asset.id.0
            )));
        }
        for (lod_index, lod) in asset.lods.iter().enumerate() {
            for relative in [
                &lod.generated_obj,
                &lod.socket_manifest,
                &lod.semantic_mask,
                &lod.anatomy_mask,
            ] {
                validate_relative_staging_path(relative)?;
                if !expected_outputs.insert(relative.clone()) {
                    return Err(fail(format!("duplicate recipe output path {relative}")));
                }
                source_output_contracts
                    .entry(donor.clone())
                    .or_default()
                    .insert(relative.clone());
            }
            obj_contracts.insert(lod.generated_obj.clone(), (asset_index, lod_index));
            socket_contracts.insert(lod.socket_manifest.clone(), (asset_index, lod_index));
            mask_contracts.insert(lod.semantic_mask.clone(), (asset_index, lod_index));
            anatomy_contracts.insert(lod.anatomy_mask.clone(), (asset_index, lod_index));
            output_digest_contracts
                .insert(lod.generated_obj.clone(), lod.generated_obj_sha256.clone());
            output_digest_contracts.insert(
                lod.socket_manifest.clone(),
                lod.socket_manifest_sha256.clone(),
            );
            output_digest_contracts
                .insert(lod.semantic_mask.clone(), lod.semantic_mask_sha256.clone());
            output_digest_contracts
                .insert(lod.anatomy_mask.clone(), lod.anatomy_mask_sha256.clone());
        }
    }
    if expected_outputs.len() != 14 * 3 * 4
        || obj_contracts.len() != 42
        || socket_contracts.len() != 42
        || mask_contracts.len() != 42
        || anatomy_contracts.len() != 42
    {
        return Err(fail(
            "recipe must declare 14 shared assets with three unique OBJ/socket/semantic/anatomy LOD outputs"
                .to_string(),
        ));
    }

    let receipt_path =
        confined_existing_staged_path(&canonical_staging_root, "build_receipt.json")?;
    let receipt_bytes = fs::read(&receipt_path).map_err(|error| {
        fail(format!(
            "missing build receipt {}: {error}",
            receipt_path.display()
        ))
    })?;
    let receipt: GeneForgeBuildReceipt = serde_json::from_slice(&receipt_bytes)
        .map_err(|error| fail(format!("invalid build receipt: {error}")))?;
    let expected_sources = catalog
        .sources
        .iter()
        .map(|source| (donor_name(source.donor).to_string(), source.sha256.clone()))
        .collect::<BTreeMap<_, _>>();
    if receipt.schema != "alife.geneforge_build_receipt.v2"
        || receipt.blender_version != "5.1.0"
        || receipt.importer_version != catalog.importer_version
        || !receipt
            .recipe_sha256
            .eq_ignore_ascii_case(&catalog.recipe_sha256)
        || receipt.source_sha256 != expected_sources
        || receipt.donor_count != 3
        || receipt.asset_count != 14
        || receipt.lods != ["full", "compact", "impostor"]
        || receipt.worker_execution.strategy != "bounded-parallel-donor-workers"
        || receipt.worker_execution.max_workers != catalog.sources.len().min(3)
    {
        return Err(fail(
            "build receipt recipe digest, importer version, source digest, or asset metadata drift"
                .to_string(),
        ));
    }
    if receipt.outputs.keys().cloned().collect::<BTreeSet<_>>() != expected_outputs {
        return Err(fail(
            "build receipt outputs do not exactly match the external recipe paths".to_string(),
        ));
    }
    validate_receipt_source_accounting(
        &receipt,
        &source_output_contracts,
        &source_asset_counts,
        &expected_outputs,
    )?;

    let mut total_bytes = receipt_bytes.len() as u64;
    for relative in &expected_outputs {
        let kind = if relative.ends_with("_anatomy.png") {
            "anatomy mask"
        } else if relative.ends_with(".png") {
            "semantic mask"
        } else {
            "output"
        };
        let path = confined_existing_staged_path(&canonical_staging_root, relative)
            .map_err(|error| fail(format!("missing {kind} {relative}: {error}")))?;
        let metadata = fs::metadata(&path)
            .map_err(|error| fail(format!("missing {kind} {relative}: {error}")))?;
        if metadata.len() > 512 * 1024 {
            return Err(fail(format!(
                "output {relative} exceeds the 512 KiB per-file budget"
            )));
        }
        total_bytes += metadata.len();
    }
    if total_bytes > 8 * 1024 * 1024 {
        return Err(fail(format!(
            "staged pack exceeds the 8 MiB budget: {total_bytes} bytes"
        )));
    }

    for (relative, (asset_index, _)) in &mask_contracts {
        let path = confined_existing_staged_path(&canonical_staging_root, relative)?;
        let bytes = fs::read(path)
            .map_err(|error| fail(format!("failed reading semantic mask {relative}: {error}")))?;
        let expected_colors = expected_asset_semantic_colors(&catalog.part_assets[*asset_index])?;
        validate_semantic_mask_png_bytes(relative, &bytes, &expected_colors)?;
    }
    for (relative, (asset_index, lod_index)) in &anatomy_contracts {
        let asset = &catalog.part_assets[*asset_index];
        let semantic_relative = &asset.lods[*lod_index].semantic_mask;
        let semantic_path =
            confined_existing_staged_path(&canonical_staging_root, semantic_relative)?;
        let semantic = fs::read(semantic_path).map_err(|error| {
            fail(format!(
                "failed reading semantic mask {semantic_relative}: {error}"
            ))
        })?;
        let anatomy_path = confined_existing_staged_path(&canonical_staging_root, relative)?;
        let anatomy = fs::read(anatomy_path)
            .map_err(|error| fail(format!("failed reading anatomy mask {relative}: {error}")))?;
        validate_anatomy_mask_png_bytes(relative, &semantic, &anatomy, asset.logical_slot)?;
    }

    let mut obj_summaries = BTreeMap::new();
    for (relative, (asset_index, lod_index)) in &obj_contracts {
        let asset = &catalog.part_assets[*asset_index];
        let expected_groups = expected_asset_groups(asset);
        let path = confined_existing_staged_path(&canonical_staging_root, relative)?;
        let summary = validate_staged_obj(&path, &expected_groups)?;
        obj_summaries.insert((*asset_index, *lod_index), summary);
    }

    let mut full_preparations = BTreeSet::new();
    for (relative, (asset_index, lod_index)) in &socket_contracts {
        let asset = &catalog.part_assets[*asset_index];
        let lod = &asset.lods[*lod_index];
        let summary = obj_summaries
            .get(&(*asset_index, *lod_index))
            .ok_or_else(|| fail(format!("socket manifest has no matching OBJ: {relative}")))?;
        validate_staged_socket_manifest(
            &canonical_staging_root,
            &confined_existing_staged_path(&canonical_staging_root, relative)?,
            &catalog,
            asset,
            lod,
            summary,
            &mut full_preparations,
        )?;
    }
    if full_preparations.len() != 12 * 5 {
        return Err(fail(format!(
            "assembly preparation union must contain 60 family-slot references; found {}",
            full_preparations.len()
        )));
    }

    for (asset_index, asset) in catalog.part_assets.iter().enumerate() {
        let by_lod = |lod: CreaturePartLodId| {
            asset
                .lods
                .iter()
                .position(|entry| entry.lod == lod)
                .and_then(|index| obj_summaries.get(&(asset_index, index)))
        };
        let full = by_lod(CreaturePartLodId::Full)
            .ok_or_else(|| fail(format!("asset {} is missing Full LOD", asset.id.0)))?;
        let compact = by_lod(CreaturePartLodId::Compact)
            .ok_or_else(|| fail(format!("asset {} is missing Compact LOD", asset.id.0)))?;
        let impostor = by_lod(CreaturePartLodId::Impostor)
            .ok_or_else(|| fail(format!("asset {} is missing Impostor LOD", asset.id.0)))?;
        validate_topology_preserving_lods(
            &asset.id.0,
            &full.topology,
            &compact.topology,
            &impostor.topology,
        )?;
    }

    for (relative, expected) in &output_digest_contracts {
        let path = confined_existing_staged_path(&canonical_staging_root, relative)?;
        let bytes = fs::read(path).map_err(|error| {
            fail(format!(
                "failed reading {relative} for external catalog digest: {error}"
            ))
        })?;
        let actual = sha256_hex(&bytes);
        if !expected.eq_ignore_ascii_case(&actual) {
            return Err(fail(format!(
                "external catalog digest drift for {relative}: expected {expected}, found {actual}"
            )));
        }
    }

    for (relative, expected) in &receipt.outputs {
        let path = confined_existing_staged_path(&canonical_staging_root, relative)?;
        let bytes = fs::read(path)
            .map_err(|error| fail(format!("failed reading {relative} for digest: {error}")))?;
        let actual = sha256_hex(&bytes);
        if !expected.eq_ignore_ascii_case(&actual) {
            return Err(fail(format!(
                "digest drift for {relative}: expected {expected}, found {actual}"
            )));
        }
    }

    Ok(GeneForgeStagingValidation {
        donor_count: receipt.donor_count,
        asset_count: receipt.asset_count,
        lod_count: obj_contracts.len(),
        obj_count: obj_contracts.len(),
        mask_count: mask_contracts.len(),
        anatomy_mask_count: anatomy_contracts.len(),
        total_bytes,
    })
}

fn validate_relative_staging_path(relative: &str) -> Result<(), CreaturePartBuilderError> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(CreaturePartBuilderError::Staging(format!(
            "output path escapes staging: {relative}"
        )));
    }
    Ok(())
}

fn validate_receipt_source_accounting(
    receipt: &GeneForgeBuildReceipt,
    expected_by_donor: &BTreeMap<String, BTreeSet<String>>,
    expected_asset_counts: &BTreeMap<String, usize>,
    expected_outputs: &BTreeSet<String>,
) -> Result<(), CreaturePartBuilderError> {
    let fail = |reason: &str| {
        CreaturePartBuilderError::Staging(format!("receipt source accounting {reason}"))
    };
    if receipt.sources.len() != expected_by_donor.len() {
        return Err(fail("has an invalid donor count"));
    }
    let mut donors = BTreeSet::new();
    let mut union = BTreeSet::new();
    for source in &receipt.sources {
        if !donors.insert(source.donor.clone()) {
            return Err(fail("contains a duplicate donor"));
        }
        let source_outputs = source.outputs.iter().cloned().collect::<BTreeSet<_>>();
        if source_outputs.len() != source.outputs.len() {
            return Err(fail("contains a duplicate donor-owned output path"));
        }
        if source.asset_count
            != expected_asset_counts
                .get(&source.donor)
                .copied()
                .unwrap_or(0)
            || source.output_count != source.outputs.len()
            || expected_by_donor.get(&source.donor) != Some(&source_outputs)
        {
            return Err(fail("does not match exact donor-owned outputs"));
        }
        for relative in source_outputs {
            if !union.insert(relative) {
                return Err(fail("duplicates an output across donors"));
            }
        }
    }
    if donors != expected_by_donor.keys().cloned().collect::<BTreeSet<_>>()
        || union != *expected_outputs
    {
        return Err(fail("union does not equal the top-level output set"));
    }
    Ok(())
}

pub fn canonical_path_is_within(canonical_root: &Path, canonical_candidate: &Path) -> bool {
    canonical_candidate.starts_with(canonical_root)
}

fn confined_existing_staged_path(
    canonical_staging_root: &Path,
    relative: &str,
) -> Result<PathBuf, CreaturePartBuilderError> {
    // Confinement assumes no concurrent mutator swaps components after this
    // check. Every ordinary access rechecks symlink and Windows reparse state.
    validate_relative_staging_path(relative)?;
    let mut candidate = canonical_staging_root.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::CurDir => continue,
            Component::Normal(component) => candidate.push(component),
            _ => {
                return Err(CreaturePartBuilderError::Staging(format!(
                    "output path escapes staging: {relative}"
                )))
            }
        }
        let metadata = fs::symlink_metadata(&candidate).map_err(|error| {
            CreaturePartBuilderError::Staging(format!("missing staged output {relative}: {error}"))
        })?;
        if metadata_is_symlink_or_reparse(&metadata) {
            return Err(CreaturePartBuilderError::Staging(format!(
                "staged output contains a symlink or reparse point: {relative}"
            )));
        }
    }
    let canonical = fs::canonicalize(&candidate).map_err(|error| {
        CreaturePartBuilderError::Staging(format!(
            "failed canonicalizing staged output {relative}: {error}"
        ))
    })?;
    if !canonical_path_is_within(canonical_staging_root, &canonical) {
        return Err(CreaturePartBuilderError::Staging(format!(
            "staged output escapes canonical staging root: {relative}"
        )));
    }
    Ok(canonical)
}

fn metadata_is_symlink_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

fn expected_asset_groups(asset: &GeneForgePartAssetDefinition) -> BTreeSet<String> {
    let mut groups = asset.groups.values().cloned().collect::<BTreeSet<_>>();
    for (role, objects) in &asset.detail_groups {
        if !objects.is_empty() {
            groups.insert(format!("head.{}", detail_role_name(*role)));
        }
    }
    groups
}

fn expected_asset_semantic_colors(
    asset: &GeneForgePartAssetDefinition,
) -> Result<BTreeSet<[u8; 3]>, CreaturePartBuilderError> {
    expected_asset_groups(asset)
        .iter()
        .map(|group| {
            semantic_group_color(group).ok_or_else(|| {
                CreaturePartBuilderError::Staging(format!(
                    "asset {} has no semantic mask color for group {group}",
                    asset.id.0
                ))
            })
        })
        .collect()
}

fn semantic_group_color(group: &str) -> Option<[u8; 3]> {
    match group {
        "head" => Some([230, 92, 88]),
        "torso" => Some([64, 166, 184]),
        "left-arm" | "right-arm" => Some([244, 177, 76]),
        "left-leg" | "right-leg" => Some([95, 177, 104]),
        "tail-back" => Some([154, 108, 180]),
        "head.eyes" => Some([238, 238, 224]),
        "head.lids" => Some([184, 80, 96]),
        "head.hair" => Some([114, 84, 145]),
        "head.teeth" => Some([235, 222, 188]),
        "head.tongue" => Some([213, 92, 126]),
        _ => None,
    }
}

fn detail_role_name(role: GeneForgeDetailRole) -> &'static str {
    match role {
        GeneForgeDetailRole::Eyes => "eyes",
        GeneForgeDetailRole::Lids => "lids",
        GeneForgeDetailRole::Hair => "hair",
        GeneForgeDetailRole::Teeth => "teeth",
        GeneForgeDetailRole::Tongue => "tongue",
    }
}

fn donor_name(donor: GeneForgeDonorId) -> &'static str {
    match donor {
        GeneForgeDonorId::Norn => "norn",
        GeneForgeDonorId::Ettin => "ettin",
        GeneForgeDonorId::Grendel => "grendel",
    }
}

fn lod_name(lod: CreaturePartLodId) -> &'static str {
    match lod {
        CreaturePartLodId::Full => "full",
        CreaturePartLodId::Compact => "compact",
        CreaturePartLodId::Impostor => "impostor",
    }
}

fn slot_name(slot: CreaturePartSlotKey) -> &'static str {
    match slot {
        CreaturePartSlotKey::Head => "head",
        CreaturePartSlotKey::Torso => "torso",
        CreaturePartSlotKey::Arms => "arms",
        CreaturePartSlotKey::Legs => "legs",
        CreaturePartSlotKey::Tail => "tail",
    }
}

fn serialized_kebab_name<T: Serialize>(value: &T) -> Result<String, CreaturePartBuilderError> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| {
            CreaturePartBuilderError::Staging(
                "recipe enum could not be represented as a stable string".to_string(),
            )
        })
}

fn expected_landmarks(
    catalog: &GeneForgeCreaturePartCatalog,
    asset: &GeneForgePartAssetDefinition,
) -> Result<BTreeSet<String>, CreaturePartBuilderError> {
    let last_marker = if asset.donor == GeneForgeDonorId::Ettin {
        12
    } else {
        14
    };
    let mut expected = BTreeSet::new();
    for marker_id in 1..=last_marker {
        let semantic = catalog.marker_map.get(&marker_id).ok_or_else(|| {
            CreaturePartBuilderError::Staging(format!(
                "recipe marker map is missing ID {marker_id}"
            ))
        })?;
        expected.insert(serialized_kebab_name(semantic)?);
    }
    for landmark in asset.landmarks.keys() {
        expected.insert(serialized_kebab_name(landmark)?);
    }
    Ok(expected)
}

fn validate_staged_obj(
    path: &Path,
    expected_groups: &BTreeSet<String>,
) -> Result<StagedObjSummary, CreaturePartBuilderError> {
    let text = fs::read_to_string(path).map_err(|error| {
        CreaturePartBuilderError::Staging(format!("failed reading OBJ {}: {error}", path.display()))
    })?;
    let mut positions = Vec::<[f64; 3]>::new();
    let mut uvs = Vec::<[f64; 2]>::new();
    let mut normals = Vec::<[f64; 3]>::new();
    let mut groups = BTreeSet::new();
    let mut current_group = None::<String>;
    let mut current_component = None::<String>;
    let mut face_count = 0_usize;
    let mut position_normals = BTreeMap::new();
    let mut bounds = StagedBounds {
        min: [f64::INFINITY; 3],
        max: [f64::NEG_INFINITY; 3],
    };
    for (index, raw) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw.trim();
        let mut fields = line.split_whitespace();
        match fields.next().unwrap_or_default() {
            "v" => {
                let value = parse_vector::<3>(fields, line_number, "position")?;
                extend_bounds(&mut [bounds.min, bounds.max], value);
                for axis in 0..3 {
                    bounds.min[axis] = bounds.min[axis].min(value[axis]);
                    bounds.max[axis] = bounds.max[axis].max(value[axis]);
                }
                positions.push(value);
            }
            "vt" => {
                let value = parse_vector::<2>(fields, line_number, "UV")?;
                if !value
                    .into_iter()
                    .all(|coordinate| (-0.001..=1.001).contains(&coordinate))
                {
                    return Err(CreaturePartBuilderError::Staging(format!(
                        "OBJ UV is outside the semantic atlas at {}:{line_number}",
                        path.display()
                    )));
                }
                uvs.push(value);
            }
            "vn" => {
                let value = parse_vector::<3>(fields, line_number, "normal")?;
                let length = dot(value, value).sqrt();
                if (length - 1.0).abs() > 2.0e-3 {
                    return Err(CreaturePartBuilderError::Staging(format!(
                        "OBJ normal is non-unit at {}:{line_number}",
                        path.display()
                    )));
                }
                normals.push(value);
            }
            "g" => {
                let group = fields.next().unwrap_or_default().to_string();
                if group.is_empty() {
                    return Err(CreaturePartBuilderError::Staging(format!(
                        "OBJ has an empty group at {}:{line_number}",
                        path.display()
                    )));
                }
                groups.insert(group.clone());
                current_group = Some(group);
            }
            "o" => {
                let component = fields.next().unwrap_or_default().to_string();
                if component.is_empty() || fields.next().is_some() {
                    return Err(CreaturePartBuilderError::Staging(format!(
                        "OBJ has an invalid component ID at {}:{line_number}",
                        path.display()
                    )));
                }
                current_component = Some(component);
            }
            "f" => {
                if current_group.is_none() {
                    return Err(CreaturePartBuilderError::Staging(format!(
                        "OBJ face has no semantic group at {}:{line_number}",
                        path.display()
                    )));
                }
                if current_component.is_none() {
                    return Err(CreaturePartBuilderError::Staging(format!(
                        "OBJ face has no semantic component at {}:{line_number}",
                        path.display()
                    )));
                }
                let refs = fields.collect::<Vec<_>>();
                if refs.len() != 3 {
                    return Err(CreaturePartBuilderError::Staging(format!(
                        "OBJ face is not triangular at {}:{line_number}",
                        path.display()
                    )));
                }
                for reference in refs {
                    let parts = reference.split('/').collect::<Vec<_>>();
                    if parts.len() != 3
                        || !valid_positive_obj_index(parts[0], positions.len())
                        || !valid_positive_obj_index(parts[1], uvs.len())
                        || !valid_positive_obj_index(parts[2], normals.len())
                    {
                        return Err(CreaturePartBuilderError::Staging(format!(
                            "OBJ index is invalid at {}:{line_number}",
                            path.display()
                        )));
                    }
                    let position_index = parts[0].parse::<usize>().unwrap() - 1;
                    let normal_index = parts[2].parse::<usize>().unwrap() - 1;
                    if position_normals
                        .insert(position_index, normal_index)
                        .is_some_and(|previous| previous != normal_index)
                    {
                        return Err(CreaturePartBuilderError::Staging(format!(
                            "OBJ shared position has split rather than smooth normals at {}:{line_number}",
                            path.display()
                        )));
                    }
                }
                face_count += 1;
            }
            _ => {}
        }
    }
    if positions.is_empty() || uvs.is_empty() || normals.is_empty() || face_count == 0 {
        return Err(CreaturePartBuilderError::Staging(format!(
            "OBJ is empty or incomplete: {}",
            path.display()
        )));
    }
    if &groups != expected_groups {
        return Err(CreaturePartBuilderError::Staging(format!(
            "OBJ semantic groups do not match its shared asset contract: {}",
            path.display()
        )));
    }
    let topology = analyze_obj_topology(&text)?;
    if topology.non_manifold_edges != 0 {
        return Err(CreaturePartBuilderError::Staging(format!(
            "OBJ contains non-manifold edges after repair: {}",
            path.display()
        )));
    }
    Ok(StagedObjSummary {
        bounds,
        positions,
        groups,
        topology,
    })
}

fn valid_positive_obj_index(value: &str, count: usize) -> bool {
    value
        .parse::<usize>()
        .is_ok_and(|index| index > 0 && index <= count)
}

fn validate_staged_socket_manifest(
    staging_root: &Path,
    path: &Path,
    catalog: &GeneForgeCreaturePartCatalog,
    asset: &GeneForgePartAssetDefinition,
    lod: &alife_game_app::GeneForgeGeneratedPartLod,
    obj: &StagedObjSummary,
    full_preparations: &mut BTreeSet<(u16, String)>,
) -> Result<(), CreaturePartBuilderError> {
    let fail = |message: String| CreaturePartBuilderError::Staging(message);
    let bytes = fs::read(path).map_err(|error| {
        fail(format!(
            "failed reading socket manifest {}: {error}",
            path.display()
        ))
    })?;
    let manifest: StagedSocketManifest = serde_json::from_slice(&bytes).map_err(|error| {
        fail(format!(
            "invalid socket manifest {}: {error}",
            path.display()
        ))
    })?;
    if manifest.schema != "alife.creature_part_sockets.v2"
        || manifest.asset_id != asset.id.0
        || manifest.logical_slot != slot_name(asset.logical_slot)
        || manifest.donor != donor_name(asset.donor)
        || manifest.lod != lod_name(lod.lod)
    {
        return Err(fail(format!(
            "socket manifest metadata is invalid: {}",
            path.display()
        )));
    }
    if !manifest
        .bounds
        .min
        .into_iter()
        .chain(manifest.bounds.max)
        .all(f64::is_finite)
        || !(0..3).all(|axis| manifest.bounds.min[axis] < manifest.bounds.max[axis])
    {
        return Err(fail(format!(
            "socket bounds are invalid: {}",
            path.display()
        )));
    }
    for axis in 0..3 {
        if obj.bounds.min[axis] < manifest.bounds.min[axis] - 0.01
            || obj.bounds.max[axis] > manifest.bounds.max[axis] + 0.01
        {
            return Err(fail(format!(
                "OBJ bounds exceed declared socket bounds: {}",
                path.display()
            )));
        }
    }
    let required_sockets = &catalog.assembly_contract.slot_sockets[&asset.logical_slot];
    for required in required_sockets {
        if !manifest.sockets.contains_key(required) {
            return Err(fail(format!(
                "socket {required} is missing: {}",
                path.display()
            )));
        }
    }
    for (name, socket) in &manifest.sockets {
        if !socket
            .translation
            .into_iter()
            .chain(socket.rotation_xyzw)
            .chain(socket.scale)
            .all(f64::is_finite)
            || socket.scale.into_iter().any(|value| value <= 0.0)
        {
            return Err(fail(format!("socket {name} has invalid finite values")));
        }
        let quaternion_length = socket
            .rotation_xyzw
            .into_iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        if (quaternion_length - 1.0).abs() > 1.0e-4 {
            return Err(fail(format!("socket {name} has a non-unit quaternion")));
        }
        if (0..3).any(|axis| {
            socket.translation[axis] < manifest.bounds.min[axis] - 2.0
                || socket.translation[axis] > manifest.bounds.max[axis] + 2.0
        }) {
            return Err(fail(format!(
                "socket {name} is detached from declared bounds"
            )));
        }
    }
    if manifest.landmarks.keys().cloned().collect::<BTreeSet<_>>()
        != expected_landmarks(catalog, asset)?
    {
        return Err(fail(
            "required marker and face landmark set does not match the recipe".to_string(),
        ));
    }
    if manifest
        .landmarks
        .values()
        .flatten()
        .chain(manifest.ground_contacts.iter().flatten())
        .any(|value| !value.is_finite())
    {
        return Err(fail("landmark or ground contact is non-finite".to_string()));
    }
    if asset.logical_slot == CreaturePartSlotKey::Legs {
        if manifest.ground_contacts.len() != 2
            || manifest
                .ground_contacts
                .iter()
                .any(|contact| contact[1] > manifest.bounds.min[1] + 0.25)
        {
            return Err(fail(
                "ground contacts are not planted within tolerance".to_string(),
            ));
        }
    } else if !manifest.ground_contacts.is_empty() {
        return Err(fail(
            "non-leg shared asset unexpectedly declares ground contacts".to_string(),
        ));
    }
    if manifest.semantic_mask != lod.semantic_mask || manifest.anatomy_mask != lod.anatomy_mask {
        return Err(fail(format!(
            "semantic/anatomy mask reference is missing or unsafe: {}, {}",
            manifest.semantic_mask, manifest.anatomy_mask
        )));
    }
    confined_existing_staged_path(staging_root, &manifest.semantic_mask)?;
    confined_existing_staged_path(staging_root, &manifest.anatomy_mask)?;
    if manifest.expected_groups != obj.groups {
        return Err(fail(
            "socket manifest semantic groups do not match the OBJ".to_string(),
        ));
    }
    if manifest.lod_topology != obj.topology {
        return Err(fail(
            "socket manifest topology does not match the OBJ".to_string(),
        ));
    }
    if manifest.microdetail.source_files.is_empty()
        || manifest
            .microdetail
            .source_files
            .iter()
            .any(|name| Path::new(name).file_name().and_then(|value| value.to_str()) != Some(name))
        || manifest.microdetail.uvless_fallback != "evaluated-normal-curvature-material-output"
    {
        return Err(fail(
            "semantic mask lacks source-derived microdetail provenance".to_string(),
        ));
    }
    validate_assembly_preparations(
        staging_root,
        &manifest,
        catalog,
        asset,
        lod.lod,
        obj,
        full_preparations,
    )?;
    Ok(())
}

fn validate_assembly_preparations(
    staging_root: &Path,
    manifest: &StagedSocketManifest,
    catalog: &GeneForgeCreaturePartCatalog,
    asset: &GeneForgePartAssetDefinition,
    lod: CreaturePartLodId,
    obj: &StagedObjSummary,
    full_preparations: &mut BTreeSet<(u16, String)>,
) -> Result<(), CreaturePartBuilderError> {
    let fail = |message: String| CreaturePartBuilderError::Staging(message);
    if manifest.assembly_preparation_schema != catalog.assembly_contract.schema {
        return Err(fail("assembly preparation schema drift".to_string()));
    }
    let required_sockets = &catalog.assembly_contract.slot_sockets[&asset.logical_slot];
    let mut observed_geometry = BTreeMap::new();
    for geometry in &manifest.bridge_geometry {
        if observed_geometry
            .insert(geometry.socket.clone(), geometry)
            .is_some()
            || !required_sockets.contains(&geometry.socket)
            || geometry.prepared_vertex_count == 0
            || !geometry.applied_overlap_depth.is_finite()
            || geometry.applied_overlap_depth <= 0.0
            || geometry.applied_overlap_depth
                > f64::from(catalog.assembly_contract.default_overlap_depth) + 1.0e-9
            || !geometry
                .original_anchor
                .into_iter()
                .chain(geometry.prepared_anchor)
                .all(f64::is_finite)
            || vector_distance(geometry.original_anchor, geometry.prepared_anchor) <= 1.0e-9
            || !obj
                .positions
                .iter()
                .any(|position| vector_distance(*position, geometry.prepared_anchor) <= 1.0e-6)
        {
            return Err(fail(
                "assembly bridge geometry is not bound to prepared OBJ vertices".to_string(),
            ));
        }
    }
    if observed_geometry.keys().cloned().collect::<BTreeSet<_>>()
        != required_sockets.iter().cloned().collect::<BTreeSet<_>>()
    {
        return Err(fail(
            "assembly bridge geometry socket set drift".to_string(),
        ));
    }
    let expected = catalog
        .families
        .iter()
        .filter_map(|family| {
            family
                .parts
                .get(&asset.logical_slot)
                .filter(|part| part.asset_id == asset.id)
                .map(|part| (family, part))
        })
        .collect::<Vec<_>>();
    if manifest.assembly_preparations.len() != expected.len() {
        return Err(fail(format!(
            "assembly preparation count drift for asset {}",
            asset.id.0
        )));
    }
    let mut observed = BTreeSet::new();
    for prepared in &manifest.assembly_preparations {
        let Some((family, part)) = expected
            .iter()
            .find(|(family, _)| family.id.0 == prepared.family_id)
            .copied()
        else {
            return Err(fail(
                "assembly preparation references an unexpected family".to_string(),
            ));
        };
        let slot = slot_name(asset.logical_slot);
        let expected_transform_mode = if matches!(
            asset.logical_slot,
            CreaturePartSlotKey::Arms | CreaturePartSlotKey::Legs
        ) {
            "per-group-socket-transforms"
        } else {
            "slot-transform"
        };
        if !observed.insert((prepared.family_id, prepared.logical_slot.clone()))
            || prepared.family_label != family.label
            || prepared.logical_slot != slot
            || prepared.asset_id != asset.id.0
            || prepared.bridge_sockets
                != catalog.assembly_contract.slot_sockets[&asset.logical_slot]
            || prepared.bridge_kind != format!("{slot}-join-cover")
            || prepared.join_cover_kind != part.join_cover_kind
            || prepared.transform_mode != expected_transform_mode
            || !near(
                prepared.overlap_depth,
                f64::from(catalog.assembly_contract.default_overlap_depth),
            )
            || !near(
                prepared.attachment_error_bound,
                f64::from(catalog.assembly_contract.attachment_error_limit),
            )
        {
            return Err(fail("assembly preparation metadata drift".to_string()));
        }
        let authored_offset: [f64; 3] = std::array::from_fn(|axis| {
            f64::from(part.fit.translation[axis] + part.seam_offset[axis])
        });
        let torso_part = &family.parts[&CreaturePartSlotKey::Torso];
        if prepared.target_torso_asset_id != torso_part.asset_id.0 {
            return Err(fail(
                "assembly bridge geometry target torso drift".to_string(),
            ));
        }
        let target_manifest = if asset.logical_slot == CreaturePartSlotKey::Torso {
            None
        } else {
            let torso_asset = catalog.asset(&torso_part.asset_id).ok_or_else(|| {
                fail("assembly bridge geometry references an unknown torso asset".to_string())
            })?;
            let torso_lod = torso_asset
                .lods
                .iter()
                .find(|entry| entry.lod == lod)
                .ok_or_else(|| {
                    fail("assembly bridge geometry target torso LOD is missing".to_string())
                })?;
            let torso_manifest_path =
                confined_existing_staged_path(staging_root, &torso_lod.socket_manifest)?;
            let bytes = fs::read(torso_manifest_path).map_err(|error| {
                fail(format!(
                    "assembly bridge geometry target torso manifest is missing: {error}"
                ))
            })?;
            Some(
                serde_json::from_slice::<StagedSocketManifest>(&bytes).map_err(|error| {
                    fail(format!(
                        "assembly bridge geometry target torso manifest is invalid: {error}"
                    ))
                })?,
            )
        };
        let source_centroid = socket_centroid(manifest, &prepared.bridge_sockets)?;
        let expected_translation = if let Some(target) = &target_manifest {
            let target_centroid = socket_centroid(target, &prepared.bridge_sockets)?;
            let linear = prepared_matrix(&prepared.fit, [0.0; 3]);
            let transformed_source = transform_matrix_point(linear, source_centroid);
            std::array::from_fn(|axis| {
                target_centroid[axis] + authored_offset[axis] - transformed_source[axis]
            })
        } else {
            authored_offset
        };
        let expected_matrix = prepared_matrix(&prepared.fit, expected_translation);
        if !socket_matches(&prepared.fit, part.fit)
            || !(0..3).all(|axis| {
                near(
                    prepared.seam_offset[axis],
                    f64::from(part.seam_offset[axis]),
                )
            })
            || !(0..3).all(|axis| {
                near(
                    prepared.prepared_translation[axis],
                    expected_translation[axis],
                )
            })
        {
            return Err(fail(
                "assembly preparation fit or seam transform drift".to_string(),
            ));
        }
        if !prepared.prepared_matrix.into_iter().all(f64::is_finite)
            || !prepared
                .prepared_matrix
                .into_iter()
                .zip(expected_matrix)
                .all(|(actual, expected)| near(actual, expected))
        {
            return Err(fail(
                "assembly preparation prepared matrix drift".to_string(),
            ));
        }
        if prepared.bridge_geometry.len() != prepared.bridge_sockets.len() {
            return Err(fail(
                "assembly bridge geometry evidence count drift".to_string(),
            ));
        }
        let mut bridge_sockets = BTreeSet::new();
        let mut predicted = 0.0_f64;
        for bridge in &prepared.bridge_geometry {
            let source_socket = manifest.sockets.get(&bridge.socket).ok_or_else(|| {
                fail("assembly bridge geometry source socket is missing".to_string())
            })?;
            let prepared_geometry = observed_geometry.get(&bridge.socket).ok_or_else(|| {
                fail("assembly bridge geometry has no prepared vertex evidence".to_string())
            })?;
            let expected_runtime_group =
                runtime_group_for_socket(asset.logical_slot, &bridge.socket).ok_or_else(|| {
                    fail("assembly bridge geometry socket has no runtime OBJ group".to_string())
                })?;
            let expected_target = if let Some(target) = &target_manifest {
                let target_socket = target.sockets.get(&bridge.socket).ok_or_else(|| {
                    fail("assembly bridge geometry target socket is missing".to_string())
                })?;
                std::array::from_fn(|axis| target_socket.translation[axis] + authored_offset[axis])
            } else {
                transform_matrix_point(prepared.prepared_matrix, bridge.source_anchor)
            };
            let expected_bridge_matrix = if target_manifest.is_some() {
                let linear = prepared_matrix(&prepared.fit, [0.0; 3]);
                let linear_source = transform_matrix_point(linear, bridge.source_anchor);
                let translation =
                    std::array::from_fn(|axis| expected_target[axis] - linear_source[axis]);
                prepared_matrix(&prepared.fit, translation)
            } else {
                prepared.prepared_matrix
            };
            let transformed = transform_matrix_point(expected_bridge_matrix, bridge.source_anchor);
            let residual = vector_distance(transformed, expected_target);
            if !bridge_sockets.insert(bridge.socket.clone())
                || !prepared.bridge_sockets.contains(&bridge.socket)
                || bridge.runtime_group != expected_runtime_group
                || !manifest.expected_groups.contains(expected_runtime_group)
                || bridge.prepared_vertex_count != prepared_geometry.prepared_vertex_count
                || !near(
                    bridge.applied_overlap_depth,
                    prepared_geometry.applied_overlap_depth,
                )
                || !vectors_near(bridge.original_anchor, prepared_geometry.original_anchor)
                || !vectors_near(bridge.prepared_anchor, prepared_geometry.prepared_anchor)
                || !vectors_near(bridge.source_anchor, source_socket.translation)
                || !vectors_near(bridge.target_anchor, expected_target)
                || !vectors_near(bridge.transformed_source_anchor, transformed)
                || !bridge.prepared_matrix.into_iter().all(f64::is_finite)
                || !bridge
                    .prepared_matrix
                    .into_iter()
                    .zip(expected_bridge_matrix)
                    .all(|(actual, expected)| near(actual, expected))
                || !near(bridge.residual, residual)
                || !bridge
                    .source_anchor
                    .into_iter()
                    .chain(bridge.target_anchor)
                    .chain(bridge.transformed_source_anchor)
                    .all(f64::is_finite)
            {
                return Err(fail(
                    "assembly bridge geometry transformed socket evidence drift".to_string(),
                ));
            }
            predicted = predicted.max(residual);
        }
        if bridge_sockets
            != prepared
                .bridge_sockets
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
        {
            return Err(fail(
                "assembly bridge geometry socket evidence drift".to_string(),
            ));
        }
        if !near(prepared.predicted_attachment_error, predicted)
            || prepared.predicted_attachment_error > prepared.attachment_error_bound + 1.0e-9
        {
            return Err(fail(
                "assembly preparation attachment-error bound is invalid".to_string(),
            ));
        }
        if lod == CreaturePartLodId::Full {
            full_preparations.insert((prepared.family_id, prepared.logical_slot.clone()));
        }
    }
    Ok(())
}

fn socket_matches(actual: &StagedSocket, expected: SocketFrame) -> bool {
    (0..3).all(|axis| {
        near(
            actual.translation[axis],
            f64::from(expected.translation[axis]),
        )
    }) && (0..4).all(|axis| {
        near(
            actual.rotation_xyzw[axis],
            f64::from(expected.rotation_xyzw[axis]),
        )
    }) && (0..3).all(|axis| near(actual.scale[axis], f64::from(expected.scale[axis])))
}

fn prepared_matrix(fit: &StagedSocket, translation: [f64; 3]) -> [f64; 16] {
    let [x, y, z, w] = fit.rotation_xyzw;
    let [sx, sy, sz] = fit.scale;
    [
        (1.0 - 2.0 * (y * y + z * z)) * sx,
        (2.0 * (x * y - z * w)) * sy,
        (2.0 * (x * z + y * w)) * sz,
        translation[0],
        (2.0 * (x * y + z * w)) * sx,
        (1.0 - 2.0 * (x * x + z * z)) * sy,
        (2.0 * (y * z - x * w)) * sz,
        translation[1],
        (2.0 * (x * z - y * w)) * sx,
        (2.0 * (y * z + x * w)) * sy,
        (1.0 - 2.0 * (x * x + y * y)) * sz,
        translation[2],
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}

fn transform_matrix_point(matrix: [f64; 16], point: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|row| {
        (0..3)
            .map(|axis| matrix[row * 4 + axis] * point[axis])
            .sum::<f64>()
            + matrix[row * 4 + 3]
    })
}

fn socket_centroid(
    manifest: &StagedSocketManifest,
    socket_names: &[String],
) -> Result<[f64; 3], CreaturePartBuilderError> {
    if socket_names.is_empty() {
        return Err(CreaturePartBuilderError::Staging(
            "assembly bridge geometry has no sockets".to_string(),
        ));
    }
    let mut centroid = [0.0; 3];
    for name in socket_names {
        let socket = manifest.sockets.get(name).ok_or_else(|| {
            CreaturePartBuilderError::Staging(format!(
                "assembly bridge geometry socket {name} is missing"
            ))
        })?;
        for (axis, value) in centroid.iter_mut().enumerate() {
            *value += socket.translation[axis];
        }
    }
    for value in &mut centroid {
        *value /= socket_names.len() as f64;
    }
    Ok(centroid)
}

fn vector_distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    (0..3)
        .map(|axis| (left[axis] - right[axis]).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn vectors_near(left: [f64; 3], right: [f64; 3]) -> bool {
    (0..3).all(|axis| near(left[axis], right[axis]))
}

fn runtime_group_for_socket(
    logical_slot: CreaturePartSlotKey,
    socket: &str,
) -> Option<&'static str> {
    match (logical_slot, socket) {
        (CreaturePartSlotKey::Head, "neck") => Some("head"),
        (CreaturePartSlotKey::Torso, _) => Some("torso"),
        (CreaturePartSlotKey::Arms, "left-shoulder") => Some("left-arm"),
        (CreaturePartSlotKey::Arms, "right-shoulder") => Some("right-arm"),
        (CreaturePartSlotKey::Legs, "left-hip") => Some("left-leg"),
        (CreaturePartSlotKey::Legs, "right-hip") => Some("right-leg"),
        (CreaturePartSlotKey::Tail, "tail-base") => Some("tail-back"),
        _ => None,
    }
}

fn near(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1.0e-6
}

fn decode_deterministic_rgba8_png(
    label: &str,
    kind: &str,
    bytes: &[u8],
) -> Result<Vec<u8>, CreaturePartBuilderError> {
    const WIDTH: usize = 64;
    const HEIGHT: usize = 64;
    const PIXEL_BYTES: usize = WIDTH * HEIGHT * 4;
    const FILTERED_BYTES: usize = HEIGHT * (1 + WIDTH * 4);
    let fail =
        |reason: String| CreaturePartBuilderError::Staging(format!("{kind} {label} {reason}"));
    let compressed = deterministic_png_idat(bytes).map_err(&fail)?;
    let filtered = inflate_zlib_bounded(&compressed, FILTERED_BYTES).map_err(&fail)?;
    if filtered.len() != FILTERED_BYTES {
        return Err(fail(format!(
            "has invalid decoded row length: expected {FILTERED_BYTES}, found {}",
            filtered.len()
        )));
    }
    for row in 0..HEIGHT {
        if filtered[row * (1 + WIDTH * 4)] != 0 {
            return Err(fail("must use deterministic filter zero rows".to_string()));
        }
    }

    let decoder = image::codecs::png::PngDecoder::new(Cursor::new(bytes))
        .map_err(|error| fail(format!("is not a valid PNG: {error}")))?;
    if decoder.dimensions() != (WIDTH as u32, HEIGHT as u32)
        || decoder.color_type() != image::ColorType::Rgba8
        || decoder.total_bytes() != PIXEL_BYTES as u64
    {
        return Err(fail(
            "must be exactly 64x64 native deterministic RGBA8".to_string(),
        ));
    }
    let mut pixels = vec![0; PIXEL_BYTES];
    decoder
        .read_image(&mut pixels)
        .map_err(|error| fail(format!("failed native RGBA8 decode: {error}")))?;
    if pixels.len() != PIXEL_BYTES {
        return Err(fail("has an invalid decoded pixel length".to_string()));
    }
    Ok(pixels)
}

fn deterministic_png_idat(bytes: &[u8]) -> Result<Vec<u8>, String> {
    const SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < SIGNATURE.len() || &bytes[..8] != SIGNATURE {
        return Err("is not a PNG".to_string());
    }
    let mut offset = 8_usize;
    let mut saw_ihdr = false;
    let mut saw_iend = false;
    let mut compressed = Vec::new();
    while offset < bytes.len() {
        if bytes.len() - offset < 12 {
            return Err("has a truncated PNG chunk".to_string());
        }
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let chunk_end = offset
            .checked_add(12)
            .and_then(|value| value.checked_add(length))
            .ok_or_else(|| "has an overflowing PNG chunk".to_string())?;
        if chunk_end > bytes.len() {
            return Err("has a truncated PNG chunk payload".to_string());
        }
        let chunk_kind = &bytes[offset + 4..offset + 8];
        let payload = &bytes[offset + 8..offset + 8 + length];
        let expected_crc = u32::from_be_bytes(
            bytes[offset + 8 + length..offset + 12 + length]
                .try_into()
                .unwrap(),
        );
        if png_chunk_crc32(chunk_kind, payload) != expected_crc {
            return Err("has an invalid PNG chunk checksum".to_string());
        }
        if !saw_ihdr && chunk_kind != b"IHDR" {
            return Err("must begin with IHDR".to_string());
        }
        match chunk_kind {
            b"IHDR" => {
                if saw_ihdr || length != 13 {
                    return Err("has invalid IHDR structure".to_string());
                }
                saw_ihdr = true;
                let width = u32::from_be_bytes(payload[0..4].try_into().unwrap());
                let height = u32::from_be_bytes(payload[4..8].try_into().unwrap());
                if width != 64 || height != 64 || payload[8..] != [8, 6, 0, 0, 0] {
                    return Err("must be exactly 64x64 native deterministic RGBA8".to_string());
                }
            }
            b"IDAT" => {
                if !saw_ihdr {
                    return Err("has IDAT before IHDR".to_string());
                }
                if compressed.len() + payload.len() > 512 * 1024 {
                    return Err("has oversized compressed PNG data".to_string());
                }
                compressed.extend_from_slice(payload);
            }
            b"IEND" => {
                if length != 0 {
                    return Err("has invalid IEND structure".to_string());
                }
                saw_iend = true;
                offset = chunk_end;
                break;
            }
            _ => {}
        }
        offset = chunk_end;
    }
    if !saw_ihdr || !saw_iend || compressed.is_empty() {
        return Err("is missing required PNG chunks".to_string());
    }
    if offset != bytes.len() {
        return Err("has trailing bytes after IEND".to_string());
    }
    Ok(compressed)
}

fn png_chunk_crc32(kind: &[u8], payload: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in kind.iter().chain(payload) {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

fn inflate_zlib_bounded(data: &[u8], output_limit: usize) -> Result<Vec<u8>, String> {
    let decoder = ZlibDecoder::new(Cursor::new(data));
    let mut bounded = decoder.take((output_limit + 1) as u64);
    let mut output = Vec::with_capacity(output_limit);
    bounded
        .read_to_end(&mut output)
        .map_err(|error| format!("has invalid zlib/DEFLATE data: {error}"))?;
    let decoder = bounded.into_inner();
    if output.len() > output_limit {
        return Err("exceeds the bounded PNG decode size".to_string());
    }
    if decoder.total_in() != data.len() as u64 {
        return Err("has a truncated stream or trailing zlib data".to_string());
    }
    Ok(output)
}

fn validate_semantic_mask_png_bytes(
    label: &str,
    bytes: &[u8],
    expected_colors: &BTreeSet<[u8; 3]>,
) -> Result<(), CreaturePartBuilderError> {
    let fail = |message: String| CreaturePartBuilderError::Staging(message);
    let pixels = decode_deterministic_rgba8_png(label, "semantic mask", bytes)?;
    let mut semantic_colors = BTreeSet::new();
    let mut microdetail = BTreeSet::new();
    for pixel in pixels.chunks_exact(4) {
        if pixel[3] == 0 {
            continue;
        }
        semantic_colors.insert([pixel[0], pixel[1], pixel[2]]);
        microdetail.insert(pixel[3]);
    }
    if semantic_colors != *expected_colors {
        return Err(fail(format!(
            "semantic mask {label} occupied semantic colors do not match its asset groups"
        )));
    }
    if microdetail.len() <= 8 {
        return Err(fail(format!(
            "semantic mask {label} lacks nonuniform source-derived microdetail"
        )));
    }
    Ok(())
}

fn validate_anatomy_mask_png_bytes(
    label: &str,
    semantic_bytes: &[u8],
    anatomy_bytes: &[u8],
    logical_slot: CreaturePartSlotKey,
) -> Result<(), CreaturePartBuilderError> {
    let fail = |message: String| CreaturePartBuilderError::Staging(message);
    let semantic = decode_deterministic_rgba8_png(
        label,
        "semantic mask paired with anatomy mask",
        semantic_bytes,
    )?;
    let anatomy = decode_deterministic_rgba8_png(label, "anatomy mask", anatomy_bytes)?;
    let (required, allowed) = anatomy_channels_for_slot(logical_slot);
    let mut used = BTreeSet::new();
    for (semantic_pixel, anatomy_pixel) in semantic.chunks_exact(4).zip(anatomy.chunks_exact(4)) {
        if (semantic_pixel[3] > 0) != (anatomy_pixel[3] > 0) {
            return Err(fail(format!(
                "anatomy mask {label} occupancy does not match its semantic mask"
            )));
        }
        if anatomy_pixel[3] == 0 {
            if anatomy_pixel != [0, 0, 0, 0] {
                return Err(fail(format!(
                    "anatomy mask {label} has a nonzero transparent pixel"
                )));
            }
            continue;
        }
        let channel =
            anatomy_channel_from_rgb([anatomy_pixel[0], anatomy_pixel[1], anatomy_pixel[2]])
                .ok_or_else(|| {
                    fail(format!(
                        "anatomy mask {label} contains an unknown channel color"
                    ))
                })?;
        if !allowed.contains(&channel) {
            return Err(fail(format!(
                "anatomy mask {label} violates source channel ownership"
            )));
        }
        used.insert(channel);
    }
    if !required.is_subset(&used) {
        return Err(fail(format!(
            "anatomy mask {label} lacks required source channel coverage"
        )));
    }
    Ok(())
}

fn anatomy_channel_from_rgb(rgb: [u8; 3]) -> Option<GeneForgeAnatomyChannel> {
    match rgb {
        [248, 248, 248] => Some(GeneForgeAnatomyChannel::Primary),
        [232, 176, 72] => Some(GeneForgeAnatomyChannel::Belly),
        [226, 112, 128] => Some(GeneForgeAnatomyChannel::Muzzle),
        [238, 86, 154] => Some(GeneForgeAnatomyChannel::InnerEar),
        [72, 174, 218] => Some(GeneForgeAnatomyChannel::HandsFeet),
        [64, 52, 72] => Some(GeneForgeAnatomyChannel::KeratinSkin),
        [84, 92, 214] => Some(GeneForgeAnatomyChannel::SecondaryMarking),
        _ => None,
    }
}

fn anatomy_channels_for_slot(
    slot: CreaturePartSlotKey,
) -> (
    BTreeSet<GeneForgeAnatomyChannel>,
    BTreeSet<GeneForgeAnatomyChannel>,
) {
    let primary = GeneForgeAnatomyChannel::Primary;
    let secondary = GeneForgeAnatomyChannel::SecondaryMarking;
    match slot {
        CreaturePartSlotKey::Head => {
            let channels = BTreeSet::from([
                primary,
                GeneForgeAnatomyChannel::Muzzle,
                GeneForgeAnatomyChannel::InnerEar,
                GeneForgeAnatomyChannel::KeratinSkin,
                secondary,
            ]);
            (channels.clone(), channels)
        }
        CreaturePartSlotKey::Torso => (
            BTreeSet::from([primary, GeneForgeAnatomyChannel::Belly, secondary]),
            BTreeSet::from([
                primary,
                GeneForgeAnatomyChannel::Belly,
                GeneForgeAnatomyChannel::KeratinSkin,
                secondary,
            ]),
        ),
        CreaturePartSlotKey::Arms | CreaturePartSlotKey::Legs => (
            BTreeSet::from([primary, GeneForgeAnatomyChannel::HandsFeet, secondary]),
            BTreeSet::from([
                primary,
                GeneForgeAnatomyChannel::HandsFeet,
                GeneForgeAnatomyChannel::KeratinSkin,
                secondary,
            ]),
        ),
        CreaturePartSlotKey::Tail => {
            let channels =
                BTreeSet::from([primary, GeneForgeAnatomyChannel::KeratinSkin, secondary]);
            (channels.clone(), channels)
        }
    }
}

fn analyze_obj_topology(text: &str) -> Result<StagedObjTopology, CreaturePartBuilderError> {
    let fail = |message: String| CreaturePartBuilderError::Staging(message);
    let mut position_count = 0_usize;
    let mut triangles = Vec::new();
    let mut triangle_components = Vec::new();
    let mut current_component = None::<String>;
    for (line_index, raw) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let mut fields = raw.split_whitespace();
        match fields.next().unwrap_or_default() {
            "v" => position_count += 1,
            "o" => {
                let component = fields.next().unwrap_or_default().to_string();
                if component.is_empty() || fields.next().is_some() {
                    return Err(fail(format!(
                        "invalid OBJ component ID at line {line_number}"
                    )));
                }
                current_component = Some(component);
            }
            "f" => {
                let component = current_component.clone().ok_or_else(|| {
                    fail(format!(
                        "OBJ topology face has no semantic component at line {line_number}"
                    ))
                })?;
                let refs = fields
                    .map(|field| {
                        let value = field.split('/').next().unwrap_or_default();
                        let index = value.parse::<usize>().map_err(|_| {
                            fail(format!("invalid OBJ topology index at line {line_number}"))
                        })?;
                        if index == 0 || index > position_count {
                            return Err(fail(format!(
                                "invalid OBJ topology index at line {line_number}"
                            )));
                        }
                        Ok(index - 1)
                    })
                    .collect::<Result<Vec<_>, CreaturePartBuilderError>>()?;
                if refs.len() < 3 {
                    return Err(fail(format!(
                        "OBJ topology face has fewer than three vertices at line {line_number}"
                    )));
                }
                for index in 1..refs.len() - 1 {
                    triangles.push([refs[0], refs[index], refs[index + 1]]);
                    triangle_components.push(component.clone());
                }
            }
            _ => {}
        }
    }
    if triangles.is_empty() {
        return Err(fail("OBJ topology contains no triangles".to_string()));
    }
    let mut edge_faces = BTreeMap::<(usize, usize), Vec<usize>>::new();
    for (face_index, triangle) in triangles.iter().enumerate() {
        for (first, second) in [(0, 1), (1, 2), (2, 0)] {
            let edge = if triangle[first] < triangle[second] {
                (triangle[first], triangle[second])
            } else {
                (triangle[second], triangle[first])
            };
            edge_faces.entry(edge).or_default().push(face_index);
        }
    }
    let mut adjacency = vec![BTreeSet::new(); triangles.len()];
    for linked in edge_faces.values() {
        for face in linked {
            adjacency[*face].extend(linked.iter().copied().filter(|other| other != face));
        }
    }
    let mut unseen = BTreeSet::from_iter(0..triangles.len());
    let mut connected_components = 0;
    let mut component_connected_counts = BTreeMap::<String, usize>::new();
    while let Some(first) = unseen.pop_first() {
        connected_components += 1;
        let mut pending = vec![first];
        let mut connected_faces = Vec::new();
        while let Some(face) = pending.pop() {
            connected_faces.push(face);
            for neighbor in &adjacency[face] {
                if unseen.remove(neighbor) {
                    pending.push(*neighbor);
                }
            }
        }
        let declared = connected_faces
            .iter()
            .map(|face| triangle_components[*face].clone())
            .collect::<BTreeSet<_>>();
        if declared.len() != 1 {
            return Err(fail(
                "geometrically connected OBJ faces cross semantic component IDs".to_string(),
            ));
        }
        *component_connected_counts
            .entry(declared.into_iter().next().unwrap())
            .or_default() += 1;
    }
    let mut component_triangle_counts = BTreeMap::<String, usize>::new();
    for component in &triangle_components {
        *component_triangle_counts
            .entry(component.clone())
            .or_default() += 1;
    }
    let component_ids = component_triangle_counts
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if component_connected_counts
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>()
        != component_ids
        || component_connected_counts.values().any(|count| *count == 0)
        || component_connected_counts.values().sum::<usize>() != connected_components
    {
        return Err(fail(
            "declared OBJ source-object component islands are inconsistent".to_string(),
        ));
    }
    Ok(StagedObjTopology {
        triangle_count: triangles.len(),
        connected_components,
        boundary_edges: edge_faces.values().filter(|faces| faces.len() == 1).count(),
        non_manifold_edges: edge_faces.values().filter(|faces| faces.len() > 2).count(),
        component_ids,
        component_triangle_counts,
        component_connected_counts,
    })
}

fn validate_topology_preserving_lods(
    asset_id: &str,
    full: &StagedObjTopology,
    compact: &StagedObjTopology,
    impostor: &StagedObjTopology,
) -> Result<(), CreaturePartBuilderError> {
    let fail = |message: &str| {
        CreaturePartBuilderError::Staging(format!(
            "asset {asset_id} has invalid topology-preserving LOD reduction: {message}"
        ))
    };
    let summaries = [full, compact, impostor];
    if !(full.triangle_count > compact.triangle_count
        && compact.triangle_count > impostor.triangle_count)
    {
        return Err(fail("triangle counts do not decrease strictly"));
    }
    if summaries
        .iter()
        .any(|summary| summary.non_manifold_edges != 0)
    {
        return Err(fail("non-manifold geometry remains"));
    }
    for summary in summaries {
        if summary.component_ids.is_empty()
            || summary
                .component_triangle_counts
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>()
                != summary.component_ids
            || summary
                .component_connected_counts
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>()
                != summary.component_ids
            || summary
                .component_triangle_counts
                .values()
                .any(|count| *count == 0)
            || summary
                .component_connected_counts
                .values()
                .any(|count| *count == 0)
            || summary.component_triangle_counts.values().sum::<usize>() != summary.triangle_count
            || summary.component_connected_counts.values().sum::<usize>()
                != summary.connected_components
        {
            return Err(fail("semantic component identity/count is inconsistent"));
        }
    }
    if compact.component_ids != full.component_ids || impostor.component_ids != full.component_ids {
        return Err(fail(
            "semantic source-object component identity differs between LODs",
        ));
    }
    for component in &full.component_ids {
        let full_islands = full.component_connected_counts[component];
        let compact_islands = compact.component_connected_counts[component];
        let impostor_islands = impostor.component_connected_counts[component];
        if compact_islands > full_islands || impostor_islands > compact_islands {
            return Err(fail(
                "LOD multiplied connected islands within a source-object component",
            ));
        }
    }
    if compact.boundary_edges > full.boundary_edges {
        return Err(fail(
            "Full->Compact LOD introduced open component boundaries",
        ));
    }
    if impostor.boundary_edges > compact.boundary_edges {
        return Err(fail(
            "Compact->Impostor LOD introduced open component boundaries",
        ));
    }
    Ok(())
}

fn canonical_recipe_sha256(text: &str) -> Result<String, CreaturePartBuilderError> {
    let mut value: serde_json::Value = serde_json::from_str(text).map_err(|error| {
        CreaturePartBuilderError::Staging(format!("invalid recipe JSON for digest: {error}"))
    })?;
    let object = value.as_object_mut().ok_or_else(|| {
        CreaturePartBuilderError::Staging("recipe JSON must be an object".to_string())
    })?;
    if !object.contains_key("recipe_sha256") {
        return Err(CreaturePartBuilderError::Staging(
            "recipe JSON is missing recipe_sha256".to_string(),
        ));
    }
    object.insert(
        "recipe_sha256".to_string(),
        serde_json::Value::String("0".repeat(64)),
    );
    let canonical = serde_json::to_vec(&value).map_err(|error| {
        CreaturePartBuilderError::Staging(format!("failed canonicalizing recipe JSON: {error}"))
    })?;
    Ok(sha256_hex(&canonical))
}

pub fn sha256_hex(input: &[u8]) -> String {
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
    let bit_len = (input.len() as u64) * 8;
    let mut padded = input.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    let mut hash = INITIAL;
    for chunk in padded.chunks_exact(64) {
        let mut words = [0_u32; 64];
        for (index, bytes) in chunk.chunks_exact(4).enumerate() {
            words[index] = u32::from_be_bytes(bytes.try_into().unwrap());
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
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = hash;
        for index in 0..64 {
            let sum1 = h
                .wrapping_add(e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25))
                .wrapping_add((e & f) ^ (!e & g))
                .wrapping_add(K[index])
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
        for (state, value) in hash.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *state = state.wrapping_add(value);
        }
    }
    hash.into_iter()
        .map(|value| format!("{value:08x}"))
        .collect()
}

pub fn slice_creature_mesh(
    source: &SourceObjMesh,
    family: &CreaturePartFamilyDefinition,
    lod: CreaturePartLodId,
) -> Result<SlicedCreaturePartPack, CreaturePartBuilderError> {
    let mut parts = OUTPUT_SLOT_ORDER
        .into_iter()
        .map(|slot| (slot, GeneratedPartMesh::default()))
        .collect::<BTreeMap<_, _>>();
    let mut vertex_indices = OUTPUT_SLOT_ORDER
        .into_iter()
        .map(|slot| (slot, BTreeMap::<VertexKey, u32>::new()))
        .collect::<BTreeMap<_, _>>();
    let mut owners = BTreeMap::<usize, BTreeSet<CreaturePartSlot>>::new();
    let mut fragment_slots = BTreeMap::<usize, BTreeSet<CreaturePartSlot>>::new();
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let partition_planes = unique_partition_planes(family);

    for triangle in &source.triangles {
        let transformed = triangle.vertices.map(|vertex| {
            let transformed = transform_source_vertex(vertex, family.source_to_canonical);
            extend_bounds(&mut bounds, transformed.position);
            transformed
        });
        let centroid = [
            transformed.iter().map(|v| v.position[0]).sum::<f64>() / 3.0,
            transformed.iter().map(|v| v.position[1]).sum::<f64>() / 3.0,
            transformed.iter().map(|v| v.position[2]).sum::<f64>() / 3.0,
        ];
        let slot = SLOT_PRIORITY
            .into_iter()
            .find(|slot| {
                family
                    .cuts
                    .get(slot)
                    .is_some_and(|volume| point_in_volume(centroid, volume))
            })
            .ok_or(CreaturePartBuilderError::UnownedTriangle {
                family: family.id,
                lod,
                triangle: triangle.source_index,
            })?;
        owners
            .entry(triangle.source_index)
            .or_default()
            .insert(slot);
        for polygon in partition_triangle(transformed, &partition_planes) {
            let fragment_centroid = polygon_centroid(&polygon);
            let fragment_slot = SLOT_PRIORITY
                .into_iter()
                .find(|slot| {
                    family
                        .cuts
                        .get(slot)
                        .is_some_and(|volume| point_in_volume(fragment_centroid, volume))
                })
                .ok_or(CreaturePartBuilderError::UnownedTriangle {
                    family: family.id,
                    lod,
                    triangle: triangle.source_index,
                })?;
            fragment_slots
                .entry(triangle.source_index)
                .or_default()
                .insert(fragment_slot);
            for index in 1..polygon.len() - 1 {
                for vertex in [polygon[0], polygon[index], polygon[index + 1]] {
                    let local = to_socket_local(vertex, fragment_slot, family)?;
                    let key = VertexKey::from(local);
                    let part = parts
                        .get_mut(&fragment_slot)
                        .expect("all output slots initialized");
                    let index = if let Some(index) = vertex_indices[&fragment_slot].get(&key) {
                        *index
                    } else {
                        let index = part.vertices.len() as u32;
                        part.vertices.push(local);
                        vertex_indices
                            .get_mut(&fragment_slot)
                            .expect("all output slots initialized")
                            .insert(key, index);
                        index
                    };
                    part.indices.push(index);
                }
            }
        }
    }

    let minimum_join_overlap = family
        .join_covers
        .iter()
        .map(|cover| cover.overlap_depth)
        .fold(f32::INFINITY, f32::min);
    let mut pack = SlicedCreaturePartPack {
        family_id: family.id,
        lod,
        parts,
        source_triangle_count: source.triangles.len(),
        source_triangle_owners: owners,
        source_triangle_fragment_slots: fragment_slots,
        sockets: family.sockets.clone(),
        canonical_source_bounds: bounds,
        minimum_join_overlap,
        obj_bytes: Vec::new(),
        socket_json_bytes: Vec::new(),
    };
    pack.obj_bytes = emit_named_obj(&pack);
    pack.socket_json_bytes = emit_socket_json(&pack)?;
    validate_sliced_pack(&pack)?;
    Ok(pack)
}

pub fn validate_sliced_pack(pack: &SlicedCreaturePartPack) -> Result<(), CreaturePartBuilderError> {
    if pack.source_triangle_count == 0
        || pack.source_triangle_owners.len() != pack.source_triangle_count
        || pack.source_triangle_fragment_slots.len() != pack.source_triangle_count
        || pack
            .source_triangle_owners
            .values()
            .any(|owner| owner.len() != 1)
        || pack.source_triangle_owners.iter().any(|(source, owner)| {
            pack.source_triangle_fragment_slots
                .get(source)
                .is_none_or(|slots| slots.is_empty() || !owner.is_subset(slots))
        })
    {
        return Err(CreaturePartBuilderError::InvalidPack(
            "every source triangle must have one primary owner represented in its fragment slots",
        ));
    }
    for slot in CreaturePartSlot::REQUIRED_RUNTIME_SLOTS {
        let part = pack
            .parts
            .get(&slot)
            .ok_or(CreaturePartBuilderError::InvalidPack(
                "required part group is missing",
            ))?;
        if part.vertices.is_empty() || part.indices.is_empty() || part.indices.len() % 3 != 0 {
            return Err(CreaturePartBuilderError::InvalidPart(
                slot,
                "required group is empty or non-triangular",
            ));
        }
    }
    for part in pack.parts.values() {
        if part
            .indices
            .iter()
            .any(|index| *index as usize >= part.vertices.len())
        {
            return Err(CreaturePartBuilderError::InvalidPack(
                "part index is outside vertex bounds",
            ));
        }
        for vertex in &part.vertices {
            if !vertex
                .position
                .into_iter()
                .chain(vertex.uv)
                .chain(vertex.normal)
                .all(f64::is_finite)
                || !vertex
                    .uv
                    .into_iter()
                    .all(|value| (-0.001..=1.001).contains(&value))
            {
                return Err(CreaturePartBuilderError::InvalidPack(
                    "part contains invalid vertex values",
                ));
            }
            let normal_length = dot(vertex.normal, vertex.normal).sqrt();
            if (normal_length - 1.0).abs() > 1.0e-4 {
                return Err(CreaturePartBuilderError::InvalidPack(
                    "part contains a non-unit normal",
                ));
            }
        }
    }
    for socket in pack.sockets.values() {
        if socket
            .translation
            .into_iter()
            .enumerate()
            .any(|(axis, value)| {
                f64::from(value) < pack.canonical_source_bounds[0][axis] - 0.25
                    || f64::from(value) > pack.canonical_source_bounds[1][axis] + 0.25
            })
        {
            return Err(CreaturePartBuilderError::InvalidPack(
                "socket lies outside canonical source bounds",
            ));
        }
    }
    let left_foot = lowest_axis(
        pack.parts
            .get(&CreaturePartSlot::LeftLeg)
            .expect("required part checked"),
        2,
    );
    let right_foot = lowest_axis(
        pack.parts
            .get(&CreaturePartSlot::RightLeg)
            .expect("required part checked"),
        2,
    );
    if (left_foot - right_foot).abs() > 0.04 {
        return Err(CreaturePartBuilderError::InvalidPack(
            "paired feet have incompatible ground heights",
        ));
    }
    if !pack.minimum_join_overlap.is_finite() || pack.minimum_join_overlap < 0.015 {
        return Err(CreaturePartBuilderError::InvalidPack(
            "join overlap is below the hidden-seam minimum",
        ));
    }
    if pack.obj_bytes.len() > 512 * 1024 || pack.socket_json_bytes.len() > 512 * 1024 {
        return Err(CreaturePartBuilderError::GeneratedFileTooLarge {
            family: pack.family_id,
            lod: pack.lod,
            obj_bytes: pack.obj_bytes.len(),
            socket_bytes: pack.socket_json_bytes.len(),
        });
    }
    Ok(())
}

fn parse_vector<const N: usize>(
    fields: impl Iterator<Item = impl AsRef<str>>,
    line: usize,
    label: &str,
) -> Result<[f64; N], CreaturePartBuilderError> {
    let values = fields
        .map(|field| field.as_ref().parse::<f64>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CreaturePartBuilderError::Obj {
            line,
            message: format!("invalid {label} scalar"),
        })?;
    if values.len() != N || !values.iter().all(|value| value.is_finite()) {
        return Err(CreaturePartBuilderError::Obj {
            line,
            message: format!("{label} requires {N} finite scalars"),
        });
    }
    Ok(std::array::from_fn(|index| values[index]))
}

fn parse_face_ref(
    field: &str,
    position_count: usize,
    uv_count: usize,
    normal_count: usize,
    line: usize,
) -> Result<(usize, usize, usize), CreaturePartBuilderError> {
    let indices = field.split('/').collect::<Vec<_>>();
    if indices.len() != 3 || indices.iter().any(|index| index.is_empty()) {
        return Err(CreaturePartBuilderError::Obj {
            line,
            message: "face vertices must use v/vt/vn tuples".to_string(),
        });
    }
    Ok((
        resolve_obj_index(indices[0], position_count, line)?,
        resolve_obj_index(indices[1], uv_count, line)?,
        resolve_obj_index(indices[2], normal_count, line)?,
    ))
}

fn resolve_obj_index(
    value: &str,
    count: usize,
    line: usize,
) -> Result<usize, CreaturePartBuilderError> {
    let parsed = value
        .parse::<isize>()
        .map_err(|_| CreaturePartBuilderError::Obj {
            line,
            message: "invalid OBJ index".to_string(),
        })?;
    if parsed == 0 {
        return Err(CreaturePartBuilderError::Obj {
            line,
            message: "OBJ indices are one-based and may not be zero".to_string(),
        });
    }
    let resolved = if parsed > 0 {
        parsed - 1
    } else {
        count as isize + parsed
    };
    if resolved < 0 || resolved as usize >= count {
        return Err(CreaturePartBuilderError::Obj {
            line,
            message: "OBJ index is outside the available attribute array".to_string(),
        });
    }
    Ok(resolved as usize)
}

fn transform_source_vertex(vertex: ObjVertex, frame: SocketFrame) -> ObjVertex {
    let scaled = [
        vertex.position[0] * f64::from(frame.scale[0]),
        vertex.position[1] * f64::from(frame.scale[1]),
        vertex.position[2] * f64::from(frame.scale[2]),
    ];
    ObjVertex {
        position: add(
            rotate(scaled, frame.rotation_xyzw),
            frame.translation.map(f64::from),
        ),
        uv: vertex.uv,
        normal: normalize(rotate(vertex.normal, frame.rotation_xyzw)).unwrap_or(vertex.normal),
    }
}

fn to_socket_local(
    vertex: ObjVertex,
    slot: CreaturePartSlot,
    family: &CreaturePartFamilyDefinition,
) -> Result<ObjVertex, CreaturePartBuilderError> {
    let Some(socket_name) = socket_name_for_slot(slot) else {
        return Ok(vertex);
    };
    let socket =
        family
            .sockets
            .get(socket_name)
            .ok_or(CreaturePartBuilderError::MissingSocket {
                family: family.id,
                socket: socket_name,
            })?;
    let inverse_rotation = [
        -socket.rotation_xyzw[0],
        -socket.rotation_xyzw[1],
        -socket.rotation_xyzw[2],
        socket.rotation_xyzw[3],
    ];
    let relative = sub(vertex.position, socket.translation.map(f64::from));
    let rotated = rotate(relative, inverse_rotation);
    Ok(ObjVertex {
        position: [
            rotated[0] / f64::from(socket.scale[0]),
            rotated[1] / f64::from(socket.scale[1]),
            rotated[2] / f64::from(socket.scale[2]),
        ],
        uv: vertex.uv,
        normal: normalize(rotate(vertex.normal, inverse_rotation)).unwrap_or(vertex.normal),
    })
}

const fn socket_name_for_slot(slot: CreaturePartSlot) -> Option<&'static str> {
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

fn point_in_volume(point: [f64; 3], volume: &CutVolume) -> bool {
    volume
        .planes
        .iter()
        .all(|plane| signed_plane_distance(point, *plane) <= 1.0e-9)
}

fn unique_partition_planes(family: &CreaturePartFamilyDefinition) -> Vec<CutPlane> {
    let mut keys = BTreeSet::new();
    let mut planes = Vec::new();
    for plane in family.cuts.values().flat_map(|volume| &volume.planes) {
        let key = [
            plane.normal[0].to_bits(),
            plane.normal[1].to_bits(),
            plane.normal[2].to_bits(),
            plane.offset.to_bits(),
        ];
        if keys.insert(key) {
            planes.push(*plane);
        }
    }
    planes
}

fn partition_triangle(triangle: [ObjVertex; 3], planes: &[CutPlane]) -> Vec<Vec<ObjVertex>> {
    let mut polygons = vec![triangle.to_vec()];
    for plane in planes {
        let mut next = Vec::with_capacity(polygons.len() + 1);
        for polygon in polygons {
            let (inside, outside) = split_polygon(&polygon, *plane);
            if inside.len() >= 3 {
                next.push(inside);
            }
            if outside.len() >= 3 {
                next.push(outside);
            }
        }
        polygons = next;
    }
    polygons
}

fn split_polygon(polygon: &[ObjVertex], plane: CutPlane) -> (Vec<ObjVertex>, Vec<ObjVertex>) {
    let mut inside = Vec::new();
    let mut outside = Vec::new();
    for index in 0..polygon.len() {
        let current = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        let current_distance = signed_plane_distance(current.position, plane);
        let next_distance = signed_plane_distance(next.position, plane);
        let current_inside = current_distance <= 1.0e-9;
        if current_inside {
            inside.push(current);
        } else {
            outside.push(current);
        }
        if current_inside != (next_distance <= 1.0e-9) {
            let denominator = current_distance - next_distance;
            if denominator.abs() > 1.0e-12 {
                let intersection =
                    interpolate_vertex(current, next, current_distance / denominator);
                inside.push(intersection);
                outside.push(intersection);
            }
        }
    }
    (inside, outside)
}

fn interpolate_vertex(start: ObjVertex, end: ObjVertex, t: f64) -> ObjVertex {
    let lerp = |a: f64, b: f64| a + (b - a) * t;
    let normal = std::array::from_fn(|axis| lerp(start.normal[axis], end.normal[axis]));
    ObjVertex {
        position: std::array::from_fn(|axis| lerp(start.position[axis], end.position[axis])),
        uv: std::array::from_fn(|axis| lerp(start.uv[axis], end.uv[axis])),
        normal: normalize(normal).unwrap_or(start.normal),
    }
}

fn polygon_centroid(polygon: &[ObjVertex]) -> [f64; 3] {
    std::array::from_fn(|axis| {
        polygon
            .iter()
            .map(|vertex| vertex.position[axis])
            .sum::<f64>()
            / polygon.len() as f64
    })
}

fn signed_plane_distance(point: [f64; 3], plane: CutPlane) -> f64 {
    dot(point, plane.normal.map(f64::from)) - f64::from(plane.offset)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct VertexKey([u64; 8]);

impl From<ObjVertex> for VertexKey {
    fn from(vertex: ObjVertex) -> Self {
        Self([
            vertex.position[0].to_bits(),
            vertex.position[1].to_bits(),
            vertex.position[2].to_bits(),
            vertex.uv[0].to_bits(),
            vertex.uv[1].to_bits(),
            vertex.normal[0].to_bits(),
            vertex.normal[1].to_bits(),
            vertex.normal[2].to_bits(),
        ])
    }
}

fn emit_named_obj(pack: &SlicedCreaturePartPack) -> Vec<u8> {
    let mut output = String::from("# A-Life deterministic creature part pack v1\n");
    let mut vertex_offset = 1_u32;
    for slot in OUTPUT_SLOT_ORDER {
        let part = &pack.parts[&slot];
        output.push_str("o ");
        output.push_str(part_group_name(slot));
        output.push('\n');
        for vertex in &part.vertices {
            output.push_str(&format!(
                "v {:.6} {:.6} {:.6}\n",
                vertex.position[0] as f32, vertex.position[1] as f32, vertex.position[2] as f32
            ));
        }
        for vertex in &part.vertices {
            output.push_str(&format!(
                "vt {:.6} {:.6}\n",
                vertex.uv[0] as f32, vertex.uv[1] as f32
            ));
        }
        for vertex in &part.vertices {
            output.push_str(&format!(
                "vn {:.6} {:.6} {:.6}\n",
                vertex.normal[0] as f32, vertex.normal[1] as f32, vertex.normal[2] as f32
            ));
        }
        for triangle in part.indices.chunks_exact(3) {
            output.push_str("f");
            for index in triangle {
                let global = vertex_offset + index;
                output.push_str(&format!(" {global}/{global}/{global}"));
            }
            output.push('\n');
        }
        vertex_offset += part.vertices.len() as u32;
    }
    output.into_bytes()
}

#[derive(Serialize)]
struct SocketManifest<'a> {
    schema: &'static str,
    schema_version: u16,
    family_id: u16,
    lod: CreaturePartLodId,
    sockets: &'a BTreeMap<String, SocketFrame>,
}

fn emit_socket_json(pack: &SlicedCreaturePartPack) -> Result<Vec<u8>, serde_json::Error> {
    let manifest = SocketManifest {
        schema: "alife.creature_part_sockets.v1",
        schema_version: 1,
        family_id: pack.family_id.0,
        lod: pack.lod,
        sockets: &pack.sockets,
    };
    let mut bytes = serde_json::to_vec_pretty(&manifest)?;
    bytes.push(b'\n');
    Ok(bytes)
}

const fn part_group_name(slot: CreaturePartSlot) -> &'static str {
    match slot {
        CreaturePartSlot::Head => "part_head",
        CreaturePartSlot::Torso => "part_torso",
        CreaturePartSlot::LeftArm => "part_left_arm",
        CreaturePartSlot::RightArm => "part_right_arm",
        CreaturePartSlot::LeftLeg => "part_left_leg",
        CreaturePartSlot::RightLeg => "part_right_leg",
        CreaturePartSlot::TailBack => "part_tail_back",
    }
}

fn extend_bounds(bounds: &mut [[f64; 3]; 2], point: [f64; 3]) {
    for axis in 0..3 {
        bounds[0][axis] = bounds[0][axis].min(point[axis]);
        bounds[1][axis] = bounds[1][axis].max(point[axis]);
    }
}

fn lowest_axis(part: &GeneratedPartMesh, axis: usize) -> f64 {
    part.vertices
        .iter()
        .map(|vertex| vertex.position[axis])
        .fold(f64::INFINITY, f64::min)
}

fn rotate(vector: [f64; 3], quaternion: [f32; 4]) -> [f64; 3] {
    let q = quaternion.map(f64::from);
    let u = [q[0], q[1], q[2]];
    let scalar = q[3];
    let uv = cross(u, vector);
    let uuv = cross(u, uv);
    add(vector, add(scale(uv, 2.0 * scalar), scale(uuv, 2.0)))
}

fn normalize(vector: [f64; 3]) -> Option<[f64; 3]> {
    let length = dot(vector, vector).sqrt();
    (length.is_finite() && length > 1.0e-12).then(|| scale(vector, 1.0 / length))
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|index| a[index] + b[index])
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|index| a[index] - b[index])
}

fn scale(vector: [f64; 3], scalar: f64) -> [f64; 3] {
    vector.map(|value| value * scalar)
}

#[cfg(test)]
mod tests {
    use alife_game_app::{
        load_geneforge_creature_part_catalog, load_production_creature_part_catalog,
        CreaturePartLodId,
    };
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    use image::{ExtendedColorType, ImageEncoder, Rgba, RgbaImage};

    use super::*;

    const TEST_BIPED_OBJ: &str = r#"
v -0.05 0.00 0.90
v 0.05 0.00 0.90
v 0.00 0.05 1.00
v -0.05 0.00 0.35
v 0.05 0.00 0.35
v 0.00 0.05 0.45
v -0.70 0.00 0.35
v -0.55 0.00 0.35
v -0.62 0.05 0.44
v 0.55 0.00 0.35
v 0.70 0.00 0.35
v 0.62 0.05 0.44
v -0.40 0.00 -0.50
v -0.20 0.00 -0.50
v -0.30 0.05 -0.40
v 0.20 0.00 -0.50
v 0.40 0.00 -0.50
v 0.30 0.05 -0.40
v -0.05 0.75 0.25
v 0.05 0.75 0.25
v 0.00 0.90 0.35
v -0.08 0.00 0.90
v 0.08 0.00 0.90
v 0.00 0.05 0.00
vt 0.0 0.0
vt 1.0 0.0
vt 0.5 1.0
vn 0.0 1.0 0.0
f 1/1/1 2/2/1 3/3/1
f 4/1/1 5/2/1 6/3/1
f 7/1/1 8/2/1 9/3/1
f 10/1/1 11/2/1 12/3/1
f 13/1/1 14/2/1 15/3/1
f 16/1/1 17/2/1 18/3/1
f 19/1/1 20/2/1 21/3/1
f 22/1/1 23/2/1 24/3/1
"#;

    fn canonical_test_family() -> CreaturePartFamilyDefinition {
        let catalog = load_production_creature_part_catalog().unwrap();
        let mut family = catalog.families[0].clone();
        family.source_to_canonical = alife_game_app::SocketFrame::IDENTITY;
        family
    }

    fn build_test_pack() -> SlicedCreaturePartPack {
        let source = SourceObjMesh::parse(TEST_BIPED_OBJ).unwrap();
        let family = canonical_test_family();
        slice_creature_mesh(&source, &family, CreaturePartLodId::Compact).unwrap()
    }

    #[test]
    fn structured_obj_parser_triangulates_and_preserves_attributes() {
        let source = SourceObjMesh::parse(TEST_BIPED_OBJ).unwrap();
        assert_eq!(source.triangles.len(), 8);
        assert_eq!(source.triangles[0].vertices[1].uv, [1.0, 0.0]);
        assert_eq!(source.triangles[0].vertices[2].normal, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn slicing_assigns_every_source_triangle_exactly_once() {
        let source = SourceObjMesh::parse(TEST_BIPED_OBJ).unwrap();
        let pack = build_test_pack();
        assert_eq!(pack.source_triangle_count, source.triangles.len());
        assert_eq!(pack.source_triangle_owners.len(), source.triangles.len());
        assert_eq!(
            pack.source_triangle_owners
                .keys()
                .copied()
                .collect::<BTreeSet<_>>(),
            source
                .triangles
                .iter()
                .map(|triangle| triangle.source_index)
                .collect::<BTreeSet<_>>()
        );
        assert!(pack
            .source_triangle_owners
            .values()
            .all(|owners| owners.len() == 1));
        assert!(pack.parts.values().all(|part| part.indices.len() % 3 == 0));
        validate_sliced_pack(&pack).unwrap();
    }

    #[test]
    fn slicing_partitions_crossing_triangles_without_surface_loss() {
        let source = SourceObjMesh::parse(TEST_BIPED_OBJ).unwrap();
        let family = canonical_test_family();
        let pack = slice_creature_mesh(&source, &family, CreaturePartLodId::Compact).unwrap();
        assert_eq!(
            pack.source_triangle_owners[&7],
            BTreeSet::from([CreaturePartSlot::Head])
        );
        let source_area = source
            .triangles
            .iter()
            .map(|triangle| {
                triangle_area(triangle.vertices.map(|vertex| {
                    transform_source_vertex(vertex, family.source_to_canonical).position
                }))
            })
            .sum::<f64>();
        let output_triangle_count = pack
            .parts
            .values()
            .map(|part| part.indices.len() / 3)
            .sum::<usize>();
        let output_area = pack
            .parts
            .iter()
            .flat_map(|(slot, part)| {
                part.indices.chunks_exact(3).map(|triangle| {
                    triangle_area(std::array::from_fn(|corner| {
                        from_socket_local_position(
                            *slot,
                            part.vertices[triangle[corner] as usize].position,
                            &family,
                        )
                    }))
                })
            })
            .sum::<f64>();

        assert!(output_triangle_count > source.triangles.len());
        assert!((source_area - output_area).abs() <= source_area * 1.0e-8);
        assert!(pack
            .source_triangle_fragment_slots
            .values()
            .any(|slots| slots.len() > 1));
        assert!(pack.source_triangle_owners.iter().all(|(source, owner)| {
            owner.is_subset(&pack.source_triangle_fragment_slots[source])
        }));
    }

    fn from_socket_local_position(
        slot: CreaturePartSlot,
        position: [f64; 3],
        family: &CreaturePartFamilyDefinition,
    ) -> [f64; 3] {
        let Some(socket_name) = socket_name_for_slot(slot) else {
            return position;
        };
        let socket = family.sockets[socket_name];
        add(
            rotate(
                std::array::from_fn(|axis| position[axis] * f64::from(socket.scale[axis])),
                socket.rotation_xyzw,
            ),
            socket.translation.map(f64::from),
        )
    }

    fn triangle_area([a, b, c]: [[f64; 3]; 3]) -> f64 {
        let ab = sub(b, a);
        let ac = sub(c, a);
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        dot(cross, cross).sqrt() * 0.5
    }

    #[test]
    fn generated_bytes_are_deterministic() {
        let first = build_test_pack();
        let second = build_test_pack();
        assert_eq!(first.obj_bytes, second.obj_bytes);
        assert_eq!(first.socket_json_bytes, second.socket_json_bytes);
    }

    #[test]
    fn malformed_obj_indices_are_rejected() {
        let malformed = "v 0 0 0\nvt 0 0\nvn 0 1 0\nf 0/1/1 1/1/1 1/1/1\n";
        assert!(SourceObjMesh::parse(malformed).is_err());
    }

    #[test]
    fn creature_part_builder_mask_contract_rejects_uniform_microdetail() {
        let image = RgbaImage::from_pixel(64, 64, Rgba([230, 92, 88, 127]));
        let bytes = deterministic_test_png(&image);

        let expected_colors = BTreeSet::from([[230, 92, 88]]);
        let error =
            validate_semantic_mask_png_bytes("uniform.png", &bytes, &expected_colors).unwrap_err();
        assert!(error.to_string().contains("microdetail"));
    }

    #[test]
    fn creature_part_builder_mask_contract_rejects_asset_independent_stripes() {
        let image = RgbaImage::from_fn(64, 64, |x, y| {
            let colors = [
                [230, 92, 88],
                [64, 166, 184],
                [244, 177, 76],
                [95, 177, 104],
            ];
            let color = colors[(y as usize / 16).min(colors.len() - 1)];
            Rgba([color[0], color[1], color[2], 32 + ((x + y) % 192) as u8])
        });
        let bytes = deterministic_test_png(&image);

        let expected_colors = BTreeSet::from([[64, 166, 184]]);
        let error = validate_semantic_mask_png_bytes("torso-stripes.png", &bytes, &expected_colors)
            .unwrap_err();
        assert!(error.to_string().contains("semantic colors"));
    }

    fn deterministic_test_png(image: &RgbaImage) -> Vec<u8> {
        let mut bytes = Vec::new();
        PngEncoder::new_with_quality(&mut bytes, CompressionType::Best, FilterType::NoFilter)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgba8,
            )
            .unwrap();
        bytes
    }

    #[test]
    fn creature_part_builder_topology_analysis_counts_boundaries_and_components() {
        let closed_tetrahedron = concat!(
            "o shell\n",
            "v 0 0 0\n",
            "v 1 0 0\n",
            "v 0 1 0\n",
            "v 0 0 1\n",
            "f 1 3 2\n",
            "f 1 2 4\n",
            "f 2 3 4\n",
            "f 3 1 4\n",
        );
        let closed = analyze_obj_topology(closed_tetrahedron).unwrap();
        assert_eq!(closed.connected_components, 1);
        assert_eq!(closed.component_ids, BTreeSet::from(["shell".to_string()]));
        assert_eq!(closed.boundary_edges, 0);
        assert_eq!(closed.non_manifold_edges, 0);

        let disconnected = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 3 0 0\nv 4 0 0\nv 3 1 0\no shell\nf 1 2 3\no eye\nf 4 5 6\n";
        let open = analyze_obj_topology(disconnected).unwrap();
        assert_eq!(open.connected_components, 2);
        assert_eq!(
            open.component_ids,
            BTreeSet::from(["eye".to_string(), "shell".to_string()])
        );
        assert_eq!(open.boundary_edges, 6);
    }

    #[test]
    fn creature_part_builder_topology_allows_multiple_islands_per_source_object() {
        let same_object_islands = concat!(
            "v 0 0 0\n",
            "v 1 0 0\n",
            "v 0 1 0\n",
            "v 3 0 0\n",
            "v 4 0 0\n",
            "v 3 1 0\n",
            "o source-hair-object\n",
            "f 1 2 3\n",
            "f 4 5 6\n",
        );

        let topology = analyze_obj_topology(same_object_islands).unwrap();

        assert_eq!(topology.connected_components, 2);
        assert_eq!(
            topology.component_ids,
            BTreeSet::from(["source-hair-object".to_string()])
        );
        assert_eq!(
            topology.component_connected_counts,
            BTreeMap::from([("source-hair-object".to_string(), 2)])
        );
    }

    #[test]
    fn creature_part_builder_allows_lod_to_merge_islands_without_losing_object_id() {
        let topology = |triangles, connected_components| StagedObjTopology {
            triangle_count: triangles,
            connected_components,
            boundary_edges: 0,
            non_manifold_edges: 0,
            component_ids: BTreeSet::from(["source-hair-object".to_string()]),
            component_triangle_counts: BTreeMap::from([(
                "source-hair-object".to_string(),
                triangles,
            )]),
            component_connected_counts: BTreeMap::from([(
                "source-hair-object".to_string(),
                connected_components,
            )]),
        };
        let full = topology(100, 3);
        let compact = topology(60, 2);
        let impostor = topology(30, 1);

        validate_topology_preserving_lods("fixture-head", &full, &compact, &impostor).unwrap();
    }

    #[test]
    fn creature_part_builder_rejects_boundary_growth_only_between_compact_and_impostor() {
        let topology = |triangles, boundaries| StagedObjTopology {
            triangle_count: triangles,
            connected_components: 1,
            boundary_edges: boundaries,
            non_manifold_edges: 0,
            component_ids: BTreeSet::from(["object-a".to_string()]),
            component_triangle_counts: BTreeMap::from([("object-a".to_string(), triangles)]),
            component_connected_counts: BTreeMap::from([("object-a".to_string(), 1)]),
        };
        let error = validate_topology_preserving_lods(
            "compact-impostor-boundary-regression",
            &topology(12, 8),
            &topology(8, 2),
            &topology(4, 6),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Compact->Impostor"), "{error}");
    }

    #[test]
    fn creature_part_builder_rejects_lod_component_loss() {
        let topology = |triangles, components, ids: &[&str]| {
            let component_ids = ids
                .iter()
                .map(|value| (*value).to_string())
                .collect::<BTreeSet<_>>();
            let base = triangles / ids.len();
            let remainder = triangles % ids.len();
            StagedObjTopology {
                triangle_count: triangles,
                connected_components: components,
                boundary_edges: 0,
                non_manifold_edges: 0,
                component_triangle_counts: ids
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        ((*value).to_string(), base + usize::from(index < remainder))
                    })
                    .collect(),
                component_connected_counts: ids
                    .iter()
                    .map(|value| ((*value).to_string(), 1))
                    .collect(),
                component_ids,
            }
        };
        let full = topology(100, 2, &["eye", "shell"]);
        let compact = topology(60, 1, &["shell"]);
        let impostor = topology(30, 1, &["shell"]);

        let error = validate_topology_preserving_lods("fixture-head", &full, &compact, &impostor)
            .unwrap_err();
        assert!(error.to_string().contains("component"));
    }

    #[test]
    fn creature_part_builder_recipe_digest_zeroes_only_the_digest_field() {
        let first = canonical_recipe_sha256(r#"{"recipe_sha256":"aaaa","schema":"x"}"#).unwrap();
        let second = canonical_recipe_sha256(r#"{"recipe_sha256":"bbbb","schema":"x"}"#).unwrap();
        let changed = canonical_recipe_sha256(r#"{"recipe_sha256":"aaaa","schema":"y"}"#).unwrap();
        assert_eq!(first, second);
        assert_ne!(first, changed);
    }

    #[test]
    fn creature_part_builder_catalog_types_bridge_and_seam_contract() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        assert_eq!(
            catalog.assembly_contract.schema,
            "alife.geneforge_family_assembly.v1"
        );
        assert_eq!(catalog.assembly_contract.attachment_error_limit, 0.025);
        assert_eq!(catalog.assembly_contract.slot_sockets.len(), 5);
        assert_eq!(
            catalog
                .sources
                .iter()
                .map(|source| (donor_name(source.donor), source.microdetail_root.as_str()))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                ("norn", "Norn/Alpha Textures"),
                ("ettin", "Ettin/Alpha Textures"),
                ("grendel", "Grendel/Alpha Textures"),
            ])
        );
    }
}
