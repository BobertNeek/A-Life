use std::{collections::BTreeMap, fs, path::Path};

use thiserror::Error;

use crate::{
    CreaturePartCatalog, CreaturePartFamilyDefinition, CreaturePartLodId, CreaturePartSlot,
};

#[cfg(feature = "bevy-app")]
use bevy::{
    asset::RenderAssetUsages,
    mesh::Indices,
    prelude::{Assets, Handle, Mesh, Resource, StandardMaterial},
    render::render_resource::PrimitiveTopology,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PartMeshData {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedPartObjPack {
    pub parts: BTreeMap<CreaturePartSlot, PartMeshData>,
}

#[cfg(feature = "bevy-app")]
#[derive(Debug, Default, Resource)]
pub struct CreaturePartAssetLibrary {
    meshes: BTreeMap<crate::CreaturePartMeshKey, Handle<Mesh>>,
    materials: BTreeMap<crate::CreaturePartMaterialKey, Handle<StandardMaterial>>,
}

#[cfg(feature = "bevy-app")]
impl CreaturePartAssetLibrary {
    pub fn load(
        assets_root: &Path,
        catalog: &CreaturePartCatalog,
        mesh_assets: &mut Assets<Mesh>,
    ) -> Result<Self, CreaturePartAssetError> {
        let mut library = Self::default();
        for family in &catalog.families {
            for lod in &family.lods {
                let pack = load_generated_part_pack(assets_root, family, lod.lod)?;
                for (slot, part) in pack.parts {
                    let key = crate::CreaturePartMeshKey {
                        family: family.id,
                        lod: lod.lod,
                        slot,
                    };
                    library
                        .meshes
                        .insert(key, mesh_assets.add(part.into_mesh()));
                }
            }
        }
        Ok(library)
    }

    pub fn mesh(&self, key: crate::CreaturePartMeshKey) -> Option<Handle<Mesh>> {
        self.meshes.get(&key).cloned()
    }

    pub fn material(
        &self,
        key: crate::CreaturePartMaterialKey,
    ) -> Option<Handle<StandardMaterial>> {
        self.materials.get(&key).cloned()
    }

    pub fn cache_material(
        &mut self,
        key: crate::CreaturePartMaterialKey,
        material: Handle<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        self.materials.entry(key).or_insert(material).clone()
    }

    pub fn mesh_handle_count(&self) -> usize {
        self.meshes.len()
    }
}

#[cfg(feature = "bevy-app")]
impl PartMeshData {
    fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

#[derive(Debug, Error)]
pub enum CreaturePartAssetError {
    #[error("generated part OBJ line {line}: {message}")]
    Obj { line: usize, message: String },
    #[error("generated part OBJ is missing required group {0:?}")]
    MissingGroup(CreaturePartSlot),
    #[error("generated part OBJ contains duplicate group {0:?}")]
    DuplicateGroup(CreaturePartSlot),
    #[error("unknown creature part family")]
    UnknownFamily,
    #[error("missing creature part LOD")]
    MissingLod,
    #[error("creature part asset IO failed: {0}")]
    Io(#[from] std::io::Error),
}

pub fn parse_generated_part_obj(
    text: &str,
) -> Result<GeneratedPartObjPack, CreaturePartAssetError> {
    let mut source_positions = Vec::<[f32; 3]>::new();
    let mut source_uvs = Vec::<[f32; 2]>::new();
    let mut source_normals = Vec::<[f32; 3]>::new();
    let mut parts = BTreeMap::<CreaturePartSlot, PartMeshData>::new();
    let mut vertex_maps = BTreeMap::<CreaturePartSlot, BTreeMap<(usize, usize, usize), u32>>::new();
    let mut current_group = None;

    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        match fields.next().unwrap_or_default() {
            "o" => {
                let name = fields.next().ok_or_else(|| CreaturePartAssetError::Obj {
                    line: line_number,
                    message: "object group requires a name".to_string(),
                })?;
                if fields.next().is_some() {
                    return Err(CreaturePartAssetError::Obj {
                        line: line_number,
                        message: "object group name may not contain spaces".to_string(),
                    });
                }
                let slot =
                    slot_for_group_name(name).ok_or_else(|| CreaturePartAssetError::Obj {
                        line: line_number,
                        message: format!("unknown object group {name}"),
                    })?;
                if parts.insert(slot, PartMeshData::default()).is_some() {
                    return Err(CreaturePartAssetError::DuplicateGroup(slot));
                }
                vertex_maps.insert(slot, BTreeMap::new());
                current_group = Some(slot);
            }
            "v" => source_positions.push(parse_vector::<3>(fields, line_number, "position")?),
            "vt" => source_uvs.push(parse_vector::<2>(fields, line_number, "UV")?),
            "vn" => source_normals.push(parse_vector::<3>(fields, line_number, "normal")?),
            "f" => {
                let slot = current_group.ok_or_else(|| CreaturePartAssetError::Obj {
                    line: line_number,
                    message: "face appears before a named part group".to_string(),
                })?;
                let refs = fields
                    .map(|field| {
                        parse_face_ref(
                            field,
                            source_positions.len(),
                            source_uvs.len(),
                            source_normals.len(),
                            line_number,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if refs.len() < 3 {
                    return Err(CreaturePartAssetError::Obj {
                        line: line_number,
                        message: "face requires at least three vertices".to_string(),
                    });
                }
                for corner in 1..refs.len() - 1 {
                    for key in [refs[0], refs[corner], refs[corner + 1]] {
                        let existing = vertex_maps[&slot].get(&key).copied();
                        let index = if let Some(index) = existing {
                            index
                        } else {
                            let part = parts.get_mut(&slot).expect("named group initialized");
                            let index = part.positions.len() as u32;
                            part.positions.push(source_positions[key.0]);
                            part.uvs.push(source_uvs[key.1]);
                            part.normals.push(source_normals[key.2]);
                            vertex_maps
                                .get_mut(&slot)
                                .expect("named group initialized")
                                .insert(key, index);
                            index
                        };
                        parts
                            .get_mut(&slot)
                            .expect("named group initialized")
                            .indices
                            .push(index);
                    }
                }
            }
            _ => {}
        }
    }

    for slot in CreaturePartSlot::REQUIRED_RUNTIME_SLOTS {
        let part = parts
            .get(&slot)
            .ok_or(CreaturePartAssetError::MissingGroup(slot))?;
        if part.positions.is_empty() || part.indices.is_empty() {
            return Err(CreaturePartAssetError::MissingGroup(slot));
        }
    }
    Ok(GeneratedPartObjPack { parts })
}

pub fn load_generated_part_pack(
    assets_root: &Path,
    family: &CreaturePartFamilyDefinition,
    lod: CreaturePartLodId,
) -> Result<GeneratedPartObjPack, CreaturePartAssetError> {
    let lod = family
        .lods
        .iter()
        .find(|entry| entry.lod == lod)
        .ok_or(CreaturePartAssetError::MissingLod)?;
    parse_generated_part_obj(&fs::read_to_string(assets_root.join(&lod.generated_obj))?)
}

pub fn load_catalog_part_pack(
    assets_root: &Path,
    catalog: &CreaturePartCatalog,
    family: alife_world::CreaturePartFamilyId,
    lod: CreaturePartLodId,
) -> Result<GeneratedPartObjPack, CreaturePartAssetError> {
    let family = catalog
        .family(family)
        .ok_or(CreaturePartAssetError::UnknownFamily)?;
    load_generated_part_pack(assets_root, family, lod)
}

fn slot_for_group_name(name: &str) -> Option<CreaturePartSlot> {
    match name {
        "part_head" => Some(CreaturePartSlot::Head),
        "part_torso" => Some(CreaturePartSlot::Torso),
        "part_left_arm" => Some(CreaturePartSlot::LeftArm),
        "part_right_arm" => Some(CreaturePartSlot::RightArm),
        "part_left_leg" => Some(CreaturePartSlot::LeftLeg),
        "part_right_leg" => Some(CreaturePartSlot::RightLeg),
        "part_tail_back" => Some(CreaturePartSlot::TailBack),
        _ => None,
    }
}

fn parse_vector<const N: usize>(
    fields: impl Iterator<Item = impl AsRef<str>>,
    line: usize,
    label: &str,
) -> Result<[f32; N], CreaturePartAssetError> {
    let values = fields
        .map(|field| field.as_ref().parse::<f32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CreaturePartAssetError::Obj {
            line,
            message: format!("invalid {label} scalar"),
        })?;
    if values.len() != N || !values.iter().all(|value| value.is_finite()) {
        return Err(CreaturePartAssetError::Obj {
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
) -> Result<(usize, usize, usize), CreaturePartAssetError> {
    let indices = field.split('/').collect::<Vec<_>>();
    if indices.len() != 3 || indices.iter().any(|index| index.is_empty()) {
        return Err(CreaturePartAssetError::Obj {
            line,
            message: "face vertices must use v/vt/vn tuples".to_string(),
        });
    }
    Ok((
        resolve_index(indices[0], position_count, line)?,
        resolve_index(indices[1], uv_count, line)?,
        resolve_index(indices[2], normal_count, line)?,
    ))
}

fn resolve_index(value: &str, count: usize, line: usize) -> Result<usize, CreaturePartAssetError> {
    let value = value
        .parse::<isize>()
        .map_err(|_| CreaturePartAssetError::Obj {
            line,
            message: "invalid OBJ index".to_string(),
        })?;
    if value == 0 {
        return Err(CreaturePartAssetError::Obj {
            line,
            message: "OBJ index may not be zero".to_string(),
        });
    }
    let resolved = if value > 0 {
        value - 1
    } else {
        count as isize + value
    };
    if resolved < 0 || resolved as usize >= count {
        return Err(CreaturePartAssetError::Obj {
            line,
            message: "OBJ index is outside its attribute array".to_string(),
        });
    }
    Ok(resolved as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CreaturePartSlot;

    const TEST_NAMED_PART_OBJ: &str = r#"
o part_head
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
vn 0 0 1
f 1/1/1 2/2/1 3/3/1
o part_torso
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
vn 0 0 1
f 4/4/2 5/5/2 6/6/2
o part_left_arm
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
vn 0 0 1
f 7/7/3 8/8/3 9/9/3
o part_right_arm
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
vn 0 0 1
f 10/10/4 11/11/4 12/12/4
o part_left_leg
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
vn 0 0 1
f 13/13/5 14/14/5 15/15/5
o part_right_leg
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
vn 0 0 1
f 16/16/6 17/17/6 18/18/6
o part_tail_back
"#;

    #[test]
    fn generated_obj_loader_returns_all_required_named_parts() {
        let pack = parse_generated_part_obj(TEST_NAMED_PART_OBJ).unwrap();
        for slot in CreaturePartSlot::REQUIRED_RUNTIME_SLOTS {
            assert!(pack.parts.contains_key(&slot), "missing {slot:?}");
        }
        assert!(pack
            .parts
            .iter()
            .filter(|(slot, _)| CreaturePartSlot::REQUIRED_RUNTIME_SLOTS.contains(slot))
            .all(|(_, mesh)| !mesh.positions.is_empty()));
    }

    #[test]
    fn generated_obj_loader_rejects_faces_before_named_groups() {
        assert!(
            parse_generated_part_obj("v 0 0 0\nvt 0 0\nvn 0 0 1\nf 1/1/1 1/1/1 1/1/1").is_err()
        );
    }

    #[test]
    fn every_production_generated_pack_parses_with_required_parts() {
        let catalog = crate::load_production_creature_part_catalog().unwrap();
        let assets_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let mut pack_count = 0;
        for family in &catalog.families {
            for lod in &family.lods {
                let pack = load_generated_part_pack(&assets_root, family, lod.lod).unwrap();
                for slot in CreaturePartSlot::REQUIRED_RUNTIME_SLOTS {
                    assert!(pack.parts[&slot].indices.len() >= 3);
                }
                pack_count += 1;
            }
        }
        assert_eq!(pack_count, 24);
    }
}
