use std::collections::{BTreeMap, BTreeSet};

use alife_game_app::{
    CreaturePartFamilyDefinition, CreaturePartLodId, CreaturePartSlot, CutPlane, CutVolume,
    SocketFrame,
};
use serde::Serialize;
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
