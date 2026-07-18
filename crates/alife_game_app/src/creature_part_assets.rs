use std::{collections::BTreeMap, fs, path::Path};

use thiserror::Error;

use crate::{
    CreaturePartCatalog, CreaturePartFamilyDefinition, CreaturePartLodId, CreaturePartSlot,
    CreatureVisualBounds,
};

#[cfg(feature = "bevy-app")]
use bevy::{
    asset::RenderAssetUsages,
    mesh::Indices,
    prelude::{Assets, Handle, Image, Mesh, Resource, StandardMaterial},
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
#[derive(Debug, Resource)]
pub struct CreaturePartAssetLibrary {
    meshes: BTreeMap<crate::CreaturePartMeshKey, Handle<Mesh>>,
    bounds: BTreeMap<crate::CreaturePartMeshKey, CreatureVisualBounds>,
    materials: BTreeMap<crate::CreaturePartMaterialKey, Handle<StandardMaterial>>,
    coat_cache: crate::CreatureCoatCache,
    coat_images: BTreeMap<u64, Handle<Image>>,
    coat_materials: BTreeMap<u64, Handle<StandardMaterial>>,
    next_coat_asset_id: u64,
}

#[cfg(feature = "bevy-app")]
impl CreaturePartAssetLibrary {
    pub fn load(
        assets_root: &Path,
        catalog: &CreaturePartCatalog,
        mesh_assets: &mut Assets<Mesh>,
    ) -> Result<Self, CreaturePartAssetError> {
        Self::load_for_profile(
            assets_root,
            catalog,
            mesh_assets,
            crate::ProductionFrontendProfileId::MinSpecComfort1080p,
        )
    }

    pub fn load_for_profile(
        assets_root: &Path,
        catalog: &CreaturePartCatalog,
        mesh_assets: &mut Assets<Mesh>,
        profile: crate::ProductionFrontendProfileId,
    ) -> Result<Self, CreaturePartAssetError> {
        let mut library =
            Self::with_coat_limits(crate::CreatureCoatCacheLimits::for_profile(profile));
        for family in &catalog.families {
            for lod in &family.lods {
                let pack = load_generated_part_pack(assets_root, family, lod.lod)?;
                for (slot, part) in pack.parts {
                    let key = crate::CreaturePartMeshKey {
                        family: family.id,
                        lod: lod.lod,
                        slot,
                    };
                    let bounds = part
                        .bevy_bounds()
                        .ok_or(CreaturePartAssetError::InvalidBounds(slot))?;
                    library.bounds.insert(key, bounds);
                    library
                        .meshes
                        .insert(key, mesh_assets.add(part.into_mesh()));
                }
            }
        }
        Ok(library)
    }

    fn with_coat_limits(limits: crate::CreatureCoatCacheLimits) -> Self {
        Self {
            meshes: BTreeMap::new(),
            bounds: BTreeMap::new(),
            materials: BTreeMap::new(),
            coat_cache: crate::CreatureCoatCache::new(limits),
            coat_images: BTreeMap::new(),
            coat_materials: BTreeMap::new(),
            next_coat_asset_id: 1,
        }
    }

    pub fn mesh(&self, key: crate::CreaturePartMeshKey) -> Option<Handle<Mesh>> {
        self.meshes.get(&key).cloned()
    }

    pub fn bounds(&self, key: crate::CreaturePartMeshKey) -> Option<CreatureVisualBounds> {
        self.bounds.get(&key).copied()
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

    pub fn material_handle_count(&self) -> usize {
        self.materials.len()
    }

    pub fn acquire_coat_assets(
        &mut self,
        key: crate::CreatureCoatKey,
        image: Handle<Image>,
        material: Handle<StandardMaterial>,
        image_assets: &mut Assets<Image>,
        material_assets: &mut Assets<StandardMaterial>,
    ) -> Result<CreatureCoatAssetHandles, CreaturePartAssetError> {
        let candidate = crate::CreatureCoatAssetPair::new(
            self.next_coat_asset_id,
            self.next_coat_asset_id.saturating_add(1),
        );
        self.next_coat_asset_id = self.next_coat_asset_id.saturating_add(2);
        let update = self.coat_cache.acquire(key, candidate);

        for evicted in update.evicted {
            if let Some(handle) = self.coat_images.remove(&evicted.image_id) {
                image_assets.remove(handle.id());
            }
            if let Some(handle) = self.coat_materials.remove(&evicted.material_id) {
                material_assets.remove(handle.id());
            }
        }

        if update.inserted {
            self.coat_images.insert(candidate.image_id, image);
            self.coat_materials.insert(candidate.material_id, material);
        } else {
            image_assets.remove(image.id());
            material_assets.remove(material.id());
        }

        let image = self
            .coat_images
            .get(&update.selected.image_id)
            .cloned()
            .ok_or(CreaturePartAssetError::MissingCoatAssetPair)?;
        let material = self
            .coat_materials
            .get(&update.selected.material_id)
            .cloned()
            .ok_or(CreaturePartAssetError::MissingCoatAssetPair)?;
        Ok(CreatureCoatAssetHandles {
            image,
            material,
            pair: update.selected,
            selected_key: update.selected_key,
            used_pinned_fallback: update.used_pinned_fallback,
        })
    }

    pub fn release_coat(
        &mut self,
        key: crate::CreatureCoatKey,
    ) -> Result<(), CreaturePartAssetError> {
        self.coat_cache.release(key)?;
        Ok(())
    }

    pub fn coat_handle_count(&self) -> usize {
        self.coat_cache.len()
    }

    pub const fn coat_rgba_bytes(&self) -> usize {
        self.coat_cache.rgba_bytes()
    }

    pub const fn coat_cache_limits(&self) -> crate::CreatureCoatCacheLimits {
        self.coat_cache.limits()
    }
}

#[cfg(feature = "bevy-app")]
impl Default for CreaturePartAssetLibrary {
    fn default() -> Self {
        Self::with_coat_limits(crate::CreatureCoatCacheLimits::comfort())
    }
}

#[cfg(feature = "bevy-app")]
#[derive(Debug, Clone)]
pub struct CreatureCoatAssetHandles {
    pub image: Handle<Image>,
    pub material: Handle<StandardMaterial>,
    pub pair: crate::CreatureCoatAssetPair,
    pub selected_key: crate::CreatureCoatKey,
    pub used_pinned_fallback: bool,
}

impl PartMeshData {
    pub fn bevy_bounds(&self) -> Option<CreatureVisualBounds> {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for &[x, depth, height] in &self.positions {
            let position = [x, height, -depth];
            if !position.into_iter().all(f32::is_finite) {
                return None;
            }
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(position[axis]);
            }
        }
        let bounds = CreatureVisualBounds::new(min, max);
        bounds.is_valid().then_some(bounds)
    }
}

#[cfg(all(test, feature = "bevy-app"))]
mod coat_asset_tests {
    use alife_world::{CreaturePartFamilyId, CreaturePartSources};
    use bevy::{
        asset::RenderAssetUsages,
        prelude::{Assets, Image, StandardMaterial},
        render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    };

    use super::*;

    fn key(offset: u16) -> crate::CreatureCoatKey {
        crate::CreatureCoatKey::new(
            CreaturePartSources {
                head: CreaturePartFamilyId(offset),
                torso: CreaturePartFamilyId(offset + 1),
                arms: CreaturePartFamilyId(offset + 2),
                legs: CreaturePartFamilyId(offset + 3),
                tail: CreaturePartFamilyId(offset + 4),
            },
            3,
            4,
            5,
        )
    }

    fn add_candidate(
        images: &mut Assets<Image>,
        materials: &mut Assets<StandardMaterial>,
    ) -> (Handle<Image>, Handle<StandardMaterial>) {
        let image = Image::new_fill(
            Extent3d {
                width: crate::CREATURE_COAT_ATLAS_SIZE,
                height: crate::CREATURE_COAT_ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[17, 29, 43, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        (
            images.add(image),
            materials.add(StandardMaterial::default()),
        )
    }

    #[test]
    fn duplicate_acquire_removes_unused_candidate_assets_and_reuses_identity() {
        let mut library =
            CreaturePartAssetLibrary::with_coat_limits(crate::CreatureCoatCacheLimits::minimum());
        let mut images = Assets::<Image>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let first_candidate = add_candidate(&mut images, &mut materials);
        let first = library
            .acquire_coat_assets(
                key(0),
                first_candidate.0,
                first_candidate.1,
                &mut images,
                &mut materials,
            )
            .unwrap();
        let duplicate_candidate = add_candidate(&mut images, &mut materials);
        let duplicate_image_id = duplicate_candidate.0.id();
        let duplicate_material_id = duplicate_candidate.1.id();
        let reused = library
            .acquire_coat_assets(
                key(0),
                duplicate_candidate.0,
                duplicate_candidate.1,
                &mut images,
                &mut materials,
            )
            .unwrap();

        assert_eq!(first.selected_key, key(0));
        assert_eq!(reused.selected_key, key(0));
        assert_eq!(first.image, reused.image);
        assert_eq!(first.material, reused.material);
        assert!(images.get(duplicate_image_id).is_none());
        assert!(materials.get(duplicate_material_id).is_none());
        assert_eq!(library.coat_rgba_bytes(), crate::CREATURE_COAT_RGBA_BYTES);
        library.release_coat(first.selected_key).unwrap();
        library.release_coat(reused.selected_key).unwrap();
    }

    #[test]
    fn eviction_removes_both_bevy_assets() {
        let mut library =
            CreaturePartAssetLibrary::with_coat_limits(crate::CreatureCoatCacheLimits {
                max_entries: 1,
                max_rgba_bytes: crate::CREATURE_COAT_RGBA_BYTES,
            });
        let mut images = Assets::<Image>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let first_candidate = add_candidate(&mut images, &mut materials);
        let first = library
            .acquire_coat_assets(
                key(0),
                first_candidate.0,
                first_candidate.1,
                &mut images,
                &mut materials,
            )
            .unwrap();
        library.release_coat(first.selected_key).unwrap();
        let first_image_id = first.image.id();
        let first_material_id = first.material.id();
        let second_candidate = add_candidate(&mut images, &mut materials);
        library
            .acquire_coat_assets(
                key(10),
                second_candidate.0,
                second_candidate.1,
                &mut images,
                &mut materials,
            )
            .unwrap();

        assert!(images.get(first_image_id).is_none());
        assert!(materials.get(first_material_id).is_none());
    }

    #[test]
    fn bevy_acquire_reports_pinned_fallback_and_release_underflow() {
        let mut library =
            CreaturePartAssetLibrary::with_coat_limits(crate::CreatureCoatCacheLimits {
                max_entries: 1,
                max_rgba_bytes: crate::CREATURE_COAT_RGBA_BYTES,
            });
        let mut images = Assets::<Image>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let resident_candidate = add_candidate(&mut images, &mut materials);
        let resident = library
            .acquire_coat_assets(
                key(0),
                resident_candidate.0,
                resident_candidate.1,
                &mut images,
                &mut materials,
            )
            .unwrap();
        let fallback_candidate = add_candidate(&mut images, &mut materials);
        let fallback = library
            .acquire_coat_assets(
                key(10),
                fallback_candidate.0,
                fallback_candidate.1,
                &mut images,
                &mut materials,
            )
            .unwrap();

        assert!(fallback.used_pinned_fallback);
        assert_eq!(fallback.selected_key, resident.selected_key);
        library.release_coat(resident.selected_key).unwrap();
        library.release_coat(fallback.selected_key).unwrap();
        assert!(matches!(
            library.release_coat(fallback.selected_key),
            Err(CreaturePartAssetError::CoatCache(
                crate::CreatureCoatCacheError::UnbalancedRelease
            ))
        ));
    }
}

#[cfg(feature = "bevy-app")]
impl PartMeshData {
    fn into_mesh(self) -> Mesh {
        let mut positions = self
            .positions
            .into_iter()
            .map(|[x, depth, height]| [x, height, -depth])
            .collect::<Vec<_>>();
        let fitted_scale = preserve_canonical_part_geometry(&mut positions);
        let normals = self
            .normals
            .into_iter()
            .map(|[x, depth, height]| {
                normalize3([
                    x / fitted_scale[0],
                    height / fitted_scale[1],
                    -depth / fitted_scale[2],
                ])
            })
            .collect::<Vec<_>>();
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

#[cfg(any(feature = "bevy-app", test))]
fn preserve_canonical_part_geometry(_positions: &mut [[f32; 3]]) -> [f32; 3] {
    [1.0; 3]
}

#[cfg(feature = "bevy-app")]
fn normalize3(value: [f32; 3]) -> [f32; 3] {
    let length = value
        .into_iter()
        .map(|axis| axis * axis)
        .sum::<f32>()
        .sqrt();
    if length > 1.0e-6 {
        value.map(|axis| axis / length)
    } else {
        [0.0, 1.0, 0.0]
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
    #[error("generated creature part has invalid bounds for {0:?}")]
    InvalidBounds(CreaturePartSlot),
    #[error("creature coat cache selected an asset pair that is not resident")]
    MissingCoatAssetPair,
    #[error(transparent)]
    CoatCache(#[from] crate::CreatureCoatCacheError),
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

    #[test]
    fn production_part_packs_are_canonical_biped_geometry_without_runtime_stretching() {
        let catalog = crate::load_production_creature_part_catalog().unwrap();
        let assets_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");

        for family in &catalog.families {
            for lod in &family.lods {
                let pack = load_generated_part_pack(&assets_root, family, lod.lod).unwrap();
                let bounds = |slot| {
                    let positions = &pack.parts[&slot].positions;
                    let min: [f32; 3] = std::array::from_fn(|axis| {
                        positions
                            .iter()
                            .map(|position| position[axis])
                            .fold(f32::INFINITY, f32::min)
                    });
                    let max: [f32; 3] = std::array::from_fn(|axis| {
                        positions
                            .iter()
                            .map(|position| position[axis])
                            .fold(f32::NEG_INFINITY, f32::max)
                    });
                    let span: [f32; 3] = std::array::from_fn(|axis| max[axis] - min[axis]);
                    (min, max, span)
                };

                let (head_min, head_max, head_span) = bounds(CreaturePartSlot::Head);
                let (torso_min, torso_max, torso_span) = bounds(CreaturePartSlot::Torso);
                assert!(
                    (0.38..=0.78).contains(&head_span[0])
                        && (0.34..=0.78).contains(&head_span[2])
                        && head_min[2] >= -0.12
                        && head_max[2] >= 0.30,
                    "family {} {:?} head is not canonical: min={head_min:?} max={head_max:?}",
                    family.label,
                    lod.lod
                );
                assert!(
                    (0.38..=0.90).contains(&torso_span[0])
                        && (0.58..=0.94).contains(&torso_span[2])
                        && torso_min[2] <= -0.26
                        && torso_max[2] >= 0.26,
                    "family {} {:?} torso is not a readable biped chest: min={torso_min:?} max={torso_max:?}",
                    family.label,
                    lod.lod
                );

                for slot in [CreaturePartSlot::LeftArm, CreaturePartSlot::RightArm] {
                    let (min, max, span) = bounds(slot);
                    assert!(
                        (0.10..=0.38).contains(&span[0])
                            && (0.48..=0.90).contains(&span[2])
                            && min[2] <= -0.42
                            && max[2] <= 0.18,
                        "family {} {:?} {slot:?} is not a bounded hanging arm: min={min:?} max={max:?}",
                        family.label,
                        lod.lod
                    );
                }
                for slot in [CreaturePartSlot::LeftLeg, CreaturePartSlot::RightLeg] {
                    let (min, max, span) = bounds(slot);
                    assert!(
                        (0.14..=0.40).contains(&span[0])
                            && (0.48..=0.82).contains(&span[2])
                            && min[2] <= -0.46
                            && max[2] <= 0.16,
                        "family {} {:?} {slot:?} is not a bounded grounded leg: min={min:?} max={max:?}",
                        family.label,
                        lod.lod
                    );
                }
            }
        }
    }

    #[test]
    fn production_creature_textures_have_readable_surface_detail_and_contrast() {
        let catalog = crate::load_production_creature_part_catalog().unwrap();
        let assets_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");

        for family in &catalog.families {
            let texture = image::open(assets_root.join(&family.texture_asset))
                .unwrap_or_else(|error| panic!("{} texture failed to load: {error}", family.label))
                .to_rgba8();
            assert!(
                texture.width() >= 128 && texture.height() >= 128,
                "{} texture is only {}x{}; palette strips are not production surface maps",
                family.label,
                texture.width(),
                texture.height()
            );
            let colors = texture
                .pixels()
                .map(|pixel| [pixel[0], pixel[1], pixel[2]])
                .collect::<std::collections::BTreeSet<_>>();
            let luminance = texture
                .pixels()
                .map(|pixel| {
                    (u16::from(pixel[0]) * 54
                        + u16::from(pixel[1]) * 183
                        + u16::from(pixel[2]) * 19)
                        / 256
                })
                .collect::<Vec<_>>();
            let range = luminance.iter().max().unwrap() - luminance.iter().min().unwrap();
            assert!(
                colors.len() >= 96 && range >= 64,
                "{} texture lacks bold readable detail: colors={} luminance_range={range}",
                family.label,
                colors.len()
            );
            assert!(texture.pixels().all(|pixel| pixel[3] == 255));
        }
    }

    #[test]
    fn canonical_part_geometry_preserves_authored_proportions() {
        let source = vec![[-2.0, -0.1, -4.0], [3.0, 0.2, 5.0], [0.0, 0.0, 0.0]];
        let mut fitted = source.clone();
        let scale = preserve_canonical_part_geometry(&mut fitted);
        assert_eq!(scale, [1.0; 3]);
        assert_eq!(
            fitted, source,
            "canonical generated parts must not be stretched per axis at runtime"
        );
    }

    #[test]
    fn part_mesh_bounds_use_the_same_canonical_to_bevy_axes_as_rendering() {
        let bounds = PartMeshData {
            positions: vec![[-0.4, -0.3, -0.7], [0.5, 0.2, 0.9]],
            ..Default::default()
        }
        .bevy_bounds()
        .expect("nonempty finite part has bounds");

        assert_eq!(bounds.min, [-0.4, -0.7, -0.2]);
        assert_eq!(bounds.max, [0.5, 0.9, 0.3]);
        assert!(bounds.is_valid());
    }

    #[cfg(feature = "bevy-app")]
    #[test]
    fn anatomical_mesh_fit_preserves_uvs_indices_and_unit_normals() {
        use bevy::mesh::VertexAttributeValues;

        let uvs = vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];
        let indices = vec![0, 1, 2];
        let mesh = PartMeshData {
            positions: vec![[-2.0, -0.1, -4.0], [3.0, 0.2, 5.0], [0.0, 0.0, 0.0]],
            uvs: uvs.clone(),
            normals: vec![[0.0, 0.0, 1.0]; 3],
            indices: indices.clone(),
        }
        .into_mesh();

        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_UV_0),
            Some(&VertexAttributeValues::Float32x2(uvs))
        );
        assert_eq!(mesh.indices(), Some(&Indices::U32(indices)));
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("mesh normals must be Float32x3");
        };
        assert!(normals.iter().all(|normal| {
            (normal.iter().map(|axis| axis * axis).sum::<f32>() - 1.0).abs() < 1.0e-5
        }));
    }
}
