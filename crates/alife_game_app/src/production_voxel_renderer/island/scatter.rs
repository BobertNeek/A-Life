//! Signed Starter L0 garden-floor dressing. One non-pickable indexed mesh.
use super::*;
use bevy::{asset::{AssetEvent, AssetId}, ecs::message::MessageCursor,
    gltf::GltfAssetLabel, mesh::VertexAttributeValues};

const ROOT: &str = "landscape/island/review/starter_scatter";
const FORMAT: &str = "qualia_starter_scatter_v1";
const RELEASE: &str = "b1652cf4aa186e284fd90b16fd417e1d2ad57176";
const ROWS: usize = 192;
const FIRST_ID: u32 = 1_398_013_952;
const NAMES: [&str; 3] = ["GrassTuft", "Rosette", "Sedge"];
const LAYOUT: &str = include_str!("../../../assets/landscape/island/review/starter_scatter/layout-v1.json");
const AUTH: &str = include_str!("../../../assets/landscape/island/review/starter_scatter/mesh-signatures-v1.json");

#[derive(Clone, Copy, serde::Deserialize)]
enum Species { GrassTuft, Rosette, Sedge }
impl Species {
    fn index(self) -> usize {
        match self { Self::GrassTuft => 0, Self::Rosette => 1, Self::Sedge => 2 }
    }
}
#[derive(serde::Deserialize)]
struct Layout { format: String, signed_release: String, tile: [u16; 2], rows: Vec<Row> }
#[derive(serde::Deserialize)]
struct Row { id: u32, rank: u16, species: Species, xyz: [f32; 3], yaw: f32, scale: f32 }
#[derive(Clone, Copy, Eq, PartialEq, serde::Deserialize)]
struct Fingerprint { positions: usize, indices: usize, position_words: u64, index_words: u64 }
#[derive(Clone, Copy, serde::Deserialize)]
struct SpeciesFingerprint {
    #[serde(flatten)]
    geometry: Fingerprint,
    normal_words: u64,
    rgba_words: u64,
}
#[derive(serde::Deserialize)]
struct Inputs {
    terrain_sha256: String,
    surface_sha256: String,
    tile: Fingerprint,
    species: [SpeciesFingerprint; 3],
}

#[derive(Component)]
pub(in crate::production_voxel_renderer) struct StarterScatter;
struct Ready {
    count: usize,
    terrain: AssetId<Mesh>,
    source: [AssetId<Mesh>; 3],
    target: Entity,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}
#[derive(Default)]
pub(in crate::production_voxel_renderer) struct ScatterState {
    authored: Option<(Layout, Inputs)>,
    sources: Option<[Handle<Mesh>; 3]>,
    checked_tile: Option<(AssetId<Mesh>, bool)>,
    changes: MessageCursor<AssetEvent<Mesh>>,
    ready: Option<Ready>,
}

fn authored(surface: &alife_world::TerrainSurface) -> Option<(Layout, Inputs)> {
    let layout: Layout = serde_json::from_str(LAYOUT).ok()?;
    let inputs: Inputs = serde_json::from_str(AUTH).ok()?;
    if layout.format != FORMAT || layout.signed_release != RELEASE || layout.tile != [5, 8]
        || layout.rows.len() != ROWS
        || inputs.terrain_sha256 != "65bed5720b89b6c4462e55d15e24e7d860d5fd4109844d4376fafc50d914928c"
        || inputs.surface_sha256 != "8d0e9cf6343ffe8ea5e82bcb7ad3e4ef77eef7207042cf0274de5d91f2457c38"
    {
        return None;
    }
    for (rank, row) in layout.rows.iter().enumerate() {
        if row.id != FIRST_ID + rank as u32 || usize::from(row.rank) != rank
            || !row.xyz.into_iter().all(f32::is_finite)
            || !(76.0..=92.0).contains(&row.xyz[0]) || !(207.0..=227.0).contains(&row.xyz[2])
            || !(0.0..=std::f32::consts::TAU).contains(&row.yaw)
            || !(0.1..=1.12).contains(&row.scale)
            || !surface.height(row.xyz[0], row.xyz[2])
                .is_some_and(|y| (row.xyz[1] - y).abs() <= 0.001)
        {
            return None;
        }
    }
    Some((layout, inputs))
}

fn wanted(level: u8) -> usize {
    match level { 4 => 96, 5 => ROWS, _ => 0 }
}

fn fnv(words: impl Iterator<Item = u32>) -> u64 {
    words.flat_map(u32::to_le_bytes).fold(0xcbf29ce484222325, |h, byte| {
        (h ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}
fn fingerprint(mesh: &Mesh) -> Option<Fingerprint> {
    let VertexAttributeValues::Float32x3(p) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)? else {
        return None;
    };
    let indices = mesh.indices()?;
    Some(Fingerprint {
        positions: p.len(), indices: indices.len(),
        position_words: fnv(p.iter().flatten().copied().map(f32::to_bits)),
        index_words: fnv(indices.iter().map(|value| value as u32)),
    })
}

struct Primitive<'a> {
    p: &'a [[f32; 3]], n: &'a [[f32; 3]], c: &'a [[f32; 4]], i: &'a Indices,
}

fn primitive(mesh: &Mesh, expected: SpeciesFingerprint) -> Option<Primitive<'_>> {
    let VertexAttributeValues::Float32x3(p) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)? else { return None; };
    let VertexAttributeValues::Float32x3(n) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)? else { return None; };
    let VertexAttributeValues::Float32x4(c) = mesh.attribute(Mesh::ATTRIBUTE_COLOR)? else { return None; };
    let i = mesh.indices()?;
    if p.is_empty() || p.len() > 512 || p.len() != n.len() || p.len() != c.len()
        || i.is_empty() || i.len() > 3_072 || i.len() % 3 != 0
        || mesh.primitive_topology() != PrimitiveTopology::TriangleList
        || fingerprint(mesh) != Some(expected.geometry)
        || p.iter().flatten().any(|x| !x.is_finite()) || n.iter().flatten().any(|x| !x.is_finite())
        || c.iter().flatten().any(|x| !x.is_finite() || !(0.0..=1.0).contains(x))
        || i.iter().any(|x| x >= p.len())
        || fnv(n.iter().flatten().copied().map(f32::to_bits)) != expected.normal_words
        || fnv(c.iter().flatten().copied().map(f32::to_bits)) != expected.rgba_words
    {
        return None;
    }
    Some(Primitive { p, n, c, i })
}

fn batch(rows: &[Row], sources: [Primitive<'_>; 3]) -> Mesh {
    let (mut positions, mut normals, mut colors, mut indexes) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for row in rows {
        let shape = &sources[row.species.index()];
        let at = Vec3::from_array(row.xyz);
        let turn = Quat::from_rotation_y(row.yaw);
        let low_base = positions.len() as u32;
        positions.extend(shape.p.iter().map(|p| (turn * (Vec3::from_array(*p) * row.scale) + at).to_array()));
        normals.extend(shape.n.iter().map(|n| (turn * Vec3::from_array(*n)).to_array()));
        colors.extend_from_slice(shape.c);
        indexes.extend(shape.i.iter().map(|i| low_base + i as u32));
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indexes));
    mesh
}

fn changed_id(event: &AssetEvent<Mesh>) -> Option<AssetId<Mesh>> {
    match event {
        AssetEvent::Added { id } | AssetEvent::Modified { id } | AssetEvent::Removed { id } => Some(*id),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments, reason = "Independent optional renderer assets and exact current L0 ownership")]
pub(in crate::production_voxel_renderer) fn sync(
    mut commands: Commands,
    settings: Option<Res<Fvr05ProductionUxStateResource>>,
    island: Option<Res<IslandAssets>>,
    terrain: Option<Res<creature_grounding::SelectedTerrain>>,
    server: Option<Res<AssetServer>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    events: Option<Res<bevy::ecs::message::Messages<AssetEvent<Mesh>>>>,
    chunks: Query<(&Mesh3d, &TerrainLod)>,
    cameras: Query<(&Transform, &Projection), With<Fvr03ProductionVoxelCamera>>,
    current: Query<Entity, With<StarterScatter>>,
    mut state: Local<ScatterState>,
) {
    let active = (|| {
        let count = wanted(settings.as_deref()?.settings.foliage_density);
        if count == 0 { return None; }
        let (island, terrain) = (island.as_deref()?, terrain.as_deref()?);
        if !island.spawned || !has_baked_art(Some(&terrain.0))
            || !std::sync::Arc::ptr_eq(&island.surface, terrain.0.surface())
        {
            return None;
        }
        let (camera, projection) = cameras.single().ok()?;
        if !(0.0..=36.0).contains(&view_height(projection)) { return None; }
        let focus = focus_on_surface(camera, &island.surface);
        if !(Vec2::new(focus.x - 84., focus.z - 217.).length() < 42.0) { return None; }
        let mut tile = chunks.iter().filter(|(_, lod)| lod.center.x == 50. && lod.center.z == 250.);
        let (shown, lod) = tile.next()?;
        if tile.next().is_some() || lod.current != 0 || shown.0.id() != lod.meshes[0].id() {
            return None;
        }
        Some((count, shown.0.id(), &island.surface))
    })();
    let (Some((count, shown, surface)), Some(server), Some(mut meshes), Some(mut materials)) =
        (active, server, meshes, materials)
    else {
        for id in &current { commands.entity(id).despawn(); }
        *state = ScatterState::default(); // Off and out-of-field own no draw or mesh handles.
        return;
    };
    if state.authored.is_none() { state.authored = authored(surface); }
    let Some(tile_proof) = state.authored.as_ref().map(|(_, proof)| proof.tile) else { return; };
    let source_keys = state.sources.as_ref().map(|sources| std::array::from_fn::<_, 3, _>(|i| sources[i].id()));
    if let Some(events) = events.as_deref() {
        let (mut recheck_tile, mut recheck_art) = (false, false);
        for id in state.changes.read(events).filter_map(changed_id) {
            recheck_tile |= id == shown;
            recheck_art |= source_keys.is_some_and(|ids| ids.contains(&id));
        }
        if recheck_tile { state.checked_tile = None; }
        if recheck_art { state.ready = None; }
    }
    if state.checked_tile.is_none_or(|(key, _)| key != shown) {
        if let Some(mesh) = meshes.get(shown) {
            state.checked_tile = Some((shown, fingerprint(mesh) == Some(tile_proof)));
        }
    }
    if state.checked_tile != Some((shown, true)) || meshes.get(shown).is_none() {
        for id in &current { commands.entity(id).despawn(); }
        state.ready = None;
        return;
    }
    let sources = state.sources.get_or_insert_with(|| NAMES.map(|name| {
        server.load(GltfAssetLabel::Primitive { mesh: 0, primitive: 0 }.from_asset(format!("{ROOT}/{name}.glb")))
    }));
    let keys = std::array::from_fn(|i| sources[i].id());
    if let Some(old) = state.ready.as_ref().filter(|old| old.terrain == shown && old.source == keys
        && old.count == count && current.get(old.target).is_ok()
        && keys.iter().all(|id| meshes.get(*id).is_some())
        && meshes.get(&old.mesh).is_some() && materials.get(&old.material).is_some())
    {
        for id in &current {
            if id != old.target { commands.entity(id).despawn(); }
        }
        return;
    }
    for id in &current { commands.entity(id).despawn(); }
    state.ready = None;
    let Some((layout, proofs)) = state.authored.as_ref() else { return; };
    let Some(a) = meshes.get(keys[0]).and_then(|m| primitive(m, proofs.species[0])) else { return; };
    let Some(b) = meshes.get(keys[1]).and_then(|m| primitive(m, proofs.species[1])) else { return; };
    let Some(c) = meshes.get(keys[2]).and_then(|m| primitive(m, proofs.species[2])) else { return; };
    let built = batch(&layout.rows[..count], [a, b, c]);
    let mesh = meshes.add(built);
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE, perceptual_roughness: 0.88, double_sided: true, cull_mode: None,
        ..default()
    });
    let target = commands.spawn((
        Name::new("Starter signed decorative ground cover"), StarterScatter,
        Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), Transform::default(),
        NotShadowCaster, bevy::picking::Pickable::IGNORE, Fvr04ProductionRuntimeSceneRoot,
    )).id();
    state.ready = Some(Ready { count, terrain: shown, source: keys, target, mesh, material });
}

#[cfg(test)]
#[test]
fn supplemental_settings_have_exact_nested_prefixes_from_signed_ground() {
    let terrain = alife_world::island_terrain();
    let (layout, _) = authored(terrain.surface()).expect("signed layout");
    assert_eq!(layout.rows.len(), ROWS);
    assert_eq!((0..=5).map(wanted).collect::<Vec<_>>(), vec![0, 0, 0, 0, 96, 192]);
}

#[cfg(test)]
#[test]
fn altered_optional_normals_and_rgba_fail_even_if_topology_stays_signed() {
    let normals = vec![[0.0_f32, 1.0, 0.0]; 3];
    let colors = vec![[0.25_f32, 0.5, 0.125, 1.0]; 3];
    let mut signed = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    signed.insert_attribute(Mesh::ATTRIBUTE_POSITION,
        vec![[0.0_f32, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]]);
    signed.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());
    signed.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors.clone());
    signed.insert_indices(Indices::U32(vec![0, 1, 2]));
    let proof = SpeciesFingerprint {
        geometry: fingerprint(&signed).expect("signed topology"),
        normal_words: fnv(normals.iter().flatten().copied().map(f32::to_bits)),
        rgba_words: fnv(colors.iter().flatten().copied().map(f32::to_bits)),
    };
    assert!(primitive(&signed, proof).is_some());

    let mut changed = signed.clone();
    let mut altered_normals = normals;
    altered_normals[0] = [1.0, 0.0, 0.0]; // Still finite, same len, still unit.
    changed.insert_attribute(Mesh::ATTRIBUTE_NORMAL, altered_normals);
    assert!(fingerprint(&changed) == Some(proof.geometry));
    assert!(primitive(&changed, proof).is_none());

    let mut changed = signed;
    let mut altered_colors = colors;
    altered_colors[0][0] = 0.75; // Still finite and in [0,1].
    changed.insert_attribute(Mesh::ATTRIBUTE_COLOR, altered_colors);
    assert!(fingerprint(&changed) == Some(proof.geometry));
    assert!(primitive(&changed, proof).is_none());
}
