//! OBJ input used by the editable creature-part asset builder.

use crate::CreaturePartLodId;
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PartMeshData {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Error)]
pub enum CreaturePartAssetError {
    #[error("generated part OBJ line {line}: {message}")]
    Obj { line: usize, message: String },
    #[error("GeneForge mesh is missing runtime group {runtime_group} for {asset_id:?} {lod:?}")]
    MissingRuntimeGroup {
        asset_id: crate::CreaturePartAssetId,
        lod: CreaturePartLodId,
        runtime_group: String,
    },
}

pub fn parse_geneforge_runtime_groups(
    text: &str,
    runtime_groups: &std::collections::BTreeSet<String>,
    asset_id: &crate::CreaturePartAssetId,
    lod: CreaturePartLodId,
) -> Result<BTreeMap<String, PartMeshData>, CreaturePartAssetError> {
    let mut source_positions = Vec::<[f32; 3]>::new();
    let mut source_uvs = Vec::<[f32; 2]>::new();
    let mut source_normals = Vec::<[f32; 3]>::new();
    let mut parts = BTreeMap::<String, PartMeshData>::new();
    let mut vertex_maps = BTreeMap::<String, BTreeMap<(usize, usize, usize), u32>>::new();
    let mut current_group = None::<String>;

    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        match fields.next().unwrap_or_default() {
            "g" => {
                let name = fields.next().ok_or_else(|| CreaturePartAssetError::Obj {
                    line: line_number,
                    message: "group requires a name".into(),
                })?;
                current_group = runtime_groups
                    .iter()
                    .any(|group| name == group.as_str() || name.starts_with(&format!("{group}.")))
                    .then(|| name.to_string());
                if let Some(group) = &current_group {
                    parts.entry(group.clone()).or_default();
                    vertex_maps.entry(group.clone()).or_default();
                }
            }
            "v" => source_positions.push(parse_vector::<3>(fields, line_number, "position")?),
            "vt" => source_uvs.push(parse_vector::<2>(fields, line_number, "UV")?),
            "vn" => source_normals.push(parse_vector::<3>(fields, line_number, "normal")?),
            "f" => {
                let group = current_group
                    .as_ref()
                    .ok_or_else(|| CreaturePartAssetError::Obj {
                        line: line_number,
                        message: "face appears before a recognized runtime group".into(),
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
                        message: "face requires at least three vertices".into(),
                    });
                }
                for corner in 1..refs.len() - 1 {
                    for key in [refs[0], refs[corner], refs[corner + 1]] {
                        let index = if let Some(index) = vertex_maps[group].get(&key).copied() {
                            index
                        } else {
                            let part = parts.get_mut(group).expect("runtime group initialized");
                            let index = part.positions.len() as u32;
                            part.positions.push(source_positions[key.0]);
                            part.uvs.push(source_uvs[key.1]);
                            part.normals.push(source_normals[key.2]);
                            vertex_maps
                                .get_mut(group)
                                .expect("runtime group initialized")
                                .insert(key, index);
                            index
                        };
                        parts
                            .get_mut(group)
                            .expect("runtime group initialized")
                            .indices
                            .push(index);
                    }
                }
            }
            _ => {}
        }
    }

    for runtime_group in runtime_groups {
        if parts
            .get(runtime_group)
            .is_none_or(|part| part.positions.is_empty() || part.indices.is_empty())
        {
            return Err(CreaturePartAssetError::MissingRuntimeGroup {
                asset_id: asset_id.clone(),
                lod,
                runtime_group: runtime_group.clone(),
            });
        }
    }
    Ok(parts)
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
