use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

use alife_game_app::{
    CreaturePartFamilyDefinition, CreaturePartLodId, CreaturePartSlot, CutPlane, CutVolume,
    SocketFrame,
};
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
    pub lod_count: usize,
    pub obj_count: usize,
    pub mask_count: usize,
    pub total_bytes: u64,
}

#[derive(Debug, Deserialize)]
struct GeneForgeBuildReceipt {
    donor_count: usize,
    lods: Vec<String>,
    outputs: BTreeMap<String, String>,
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
    donor: String,
    lod: String,
    bounds: StagedBounds,
    sockets: BTreeMap<String, StagedSocket>,
    landmarks: BTreeMap<String, [f64; 3]>,
    ground_contacts: Vec<[f64; 3]>,
    semantic_mask: String,
}

#[derive(Debug)]
struct StagedObjSummary {
    bounds: StagedBounds,
    groups: BTreeSet<String>,
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
) -> Result<GeneForgeStagingValidation, CreaturePartBuilderError> {
    let fail = |message: String| CreaturePartBuilderError::Staging(message);
    let receipt_path = staging_root.join("build_receipt.json");
    let receipt_bytes = fs::read(&receipt_path).map_err(|error| {
        fail(format!(
            "missing build receipt {}: {error}",
            receipt_path.display()
        ))
    })?;
    let receipt: GeneForgeBuildReceipt = serde_json::from_slice(&receipt_bytes)
        .map_err(|error| fail(format!("invalid build receipt: {error}")))?;
    if receipt.donor_count != 3
        || receipt.lods != ["full", "compact", "impostor"]
        || receipt.outputs.len() != 27
    {
        return Err(fail(
            "build receipt must contain three donors, three ordered LODs, and 27 outputs"
                .to_string(),
        ));
    }

    let mut total_bytes = receipt_bytes.len() as u64;
    let mut obj_paths = Vec::new();
    let mut socket_paths = Vec::new();
    let mut mask_paths = Vec::new();
    for relative in receipt.outputs.keys() {
        let relative_path = Path::new(relative);
        if relative_path.is_absolute()
            || relative_path
                .components()
                .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
        {
            return Err(fail(format!("output path escapes staging: {relative}")));
        }
        let path = staging_root.join(relative_path);
        let metadata = fs::metadata(&path).map_err(|error| {
            let kind = if relative.ends_with(".png") {
                "semantic mask"
            } else {
                "output"
            };
            fail(format!("missing {kind} {relative}: {error}"))
        })?;
        if metadata.len() > 512 * 1024 {
            return Err(fail(format!(
                "output {relative} exceeds the 512 KiB per-file budget"
            )));
        }
        total_bytes += metadata.len();
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("obj") => obj_paths.push(path),
            Some("json") => socket_paths.push(path),
            Some("png") => mask_paths.push(path),
            _ => return Err(fail(format!("unsupported staged output {relative}"))),
        }
    }
    if total_bytes > 8 * 1024 * 1024 {
        return Err(fail(format!(
            "staged pack exceeds the 8 MiB budget: {total_bytes} bytes"
        )));
    }
    if obj_paths.len() != 9 || socket_paths.len() != 9 || mask_paths.len() != 9 {
        return Err(fail(
            "staged pack must contain nine OBJs, socket manifests, and semantic masks".to_string(),
        ));
    }

    let mut obj_summaries = BTreeMap::new();
    for path in &obj_paths {
        obj_summaries.insert(staged_stem(path)?, validate_staged_obj(path)?);
    }
    for path in &mask_paths {
        let bytes = fs::read(path).map_err(|error| {
            fail(format!(
                "failed reading semantic mask {}: {error}",
                path.display()
            ))
        })?;
        if bytes.len() < 32 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err(fail(format!(
                "semantic mask {} is not a valid bounded PNG",
                path.display()
            )));
        }
    }
    for path in &socket_paths {
        let stem = staged_stem(path)?.replace("_sockets", "_parts");
        let summary = obj_summaries
            .get(&stem)
            .ok_or_else(|| fail(format!("socket manifest has no matching OBJ: {stem}")))?;
        validate_staged_socket_manifest(staging_root, path, summary)?;
    }

    for (relative, expected) in &receipt.outputs {
        let bytes = fs::read(staging_root.join(relative))
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
        lod_count: obj_paths.len(),
        obj_count: obj_paths.len(),
        mask_count: mask_paths.len(),
        total_bytes,
    })
}

fn staged_stem(path: &Path) -> Result<String, CreaturePartBuilderError> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_owned)
        .ok_or_else(|| {
            CreaturePartBuilderError::Staging(format!(
                "staged output has a non-UTF-8 file name: {}",
                path.display()
            ))
        })
}

fn validate_staged_obj(path: &Path) -> Result<StagedObjSummary, CreaturePartBuilderError> {
    let text = fs::read_to_string(path).map_err(|error| {
        CreaturePartBuilderError::Staging(format!("failed reading OBJ {}: {error}", path.display()))
    })?;
    let mut positions = Vec::<[f64; 3]>::new();
    let mut uvs = Vec::<[f64; 2]>::new();
    let mut normals = Vec::<[f64; 3]>::new();
    let mut groups = BTreeSet::new();
    let mut current_group = None::<String>;
    let mut face_count = 0_usize;
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
            "f" => {
                if current_group.is_none() {
                    return Err(CreaturePartBuilderError::Staging(format!(
                        "OBJ face has no semantic group at {}:{line_number}",
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
    for required in [
        "head",
        "torso",
        "left-arm",
        "right-arm",
        "left-leg",
        "right-leg",
    ] {
        if !groups
            .iter()
            .any(|group| group == required || group.starts_with(&format!("{required}.")))
        {
            return Err(CreaturePartBuilderError::Staging(format!(
                "OBJ is missing semantic group {required}: {}",
                path.display()
            )));
        }
    }
    Ok(StagedObjSummary { bounds, groups })
}

fn valid_positive_obj_index(value: &str, count: usize) -> bool {
    value
        .parse::<usize>()
        .is_ok_and(|index| index > 0 && index <= count)
}

fn validate_staged_socket_manifest(
    staging_root: &Path,
    path: &Path,
    obj: &StagedObjSummary,
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
        || manifest.donor.trim().is_empty()
        || !["full", "compact", "impostor"].contains(&manifest.lod.as_str())
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
    for required in [
        "neck",
        "left-shoulder",
        "right-shoulder",
        "left-hip",
        "right-hip",
    ] {
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
            socket.translation[axis] < manifest.bounds.min[axis] - 0.25
                || socket.translation[axis] > manifest.bounds.max[axis] + 0.25
        }) {
            return Err(fail(format!(
                "socket {name} is detached from declared bounds"
            )));
        }
    }
    for required in [
        "head",
        "torso",
        "left-foot",
        "right-foot",
        "left-upper-arm",
        "right-upper-arm",
    ] {
        if !manifest.landmarks.contains_key(required) {
            return Err(fail(format!("required landmark {required} is missing")));
        }
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
    let mask_relative = Path::new(&manifest.semantic_mask);
    if mask_relative.is_absolute()
        || mask_relative
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
        || !staging_root.join(mask_relative).is_file()
    {
        return Err(fail(format!(
            "semantic mask reference is missing or unsafe: {}",
            manifest.semantic_mask
        )));
    }
    let _ = &obj.groups;
    Ok(())
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
    use alife_game_app::{load_production_creature_part_catalog, CreaturePartLodId};

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
}
