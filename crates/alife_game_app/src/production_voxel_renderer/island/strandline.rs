//! View-only wrack, shingle and salt reeds on the approved coastal terrain.
use super::*;

const PATCH_ASSET: &str = "landscape/island/strandline.glb";

#[derive(Component)]
pub(in crate::production_voxel_renderer) struct StrandPatch(Vec2);

pub(in crate::production_voxel_renderer) type PatchViews<'w, 's> =
    Query<'w, 's, (&'static StrandPatch, &'static mut Visibility), Without<PropRange>>;

fn center(name: &str, translation: Vec3) -> Option<Vec2> {
    (name.starts_with("Strand_") && (translation.x != 0.0 || translation.z != 0.0))
        .then_some(Vec2::new(translation.x, translation.z))
}

pub(super) fn spawn(world: &mut World) {
    let scene = world
        .resource::<AssetServer>()
        .load(bevy::gltf::GltfAssetLabel::Scene(0).from_asset(PATCH_ASSET));
    world
        .spawn((
            Name::new("Tidal strandline"),
            SceneRoot(scene),
            Transform::default(),
            Fvr04ProductionRuntimeSceneRoot,
            bevy::picking::Pickable::IGNORE,
        ))
        .observe(bind);
}

fn bind(
    ready: On<bevy::scene::SceneInstanceReady>,
    children: Query<&Children>,
    nodes: Query<(&Name, &Transform)>,
    mut commands: Commands,
) {
    for entity in children.iter_descendants(ready.entity) {
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((bevy::picking::Pickable::IGNORE, NotShadowCaster));
        if let Ok((name, transform)) = nodes.get(entity) {
            // The GLB places every actual patch node at its tile centre. Its
            // primitive children have local identity and inherit visibility.
            if let Some(xz) = center(name.as_str(), transform.translation) {
                entity_commands.insert((StrandPatch(xz), Visibility::Hidden));
            }
        }
    }
}

fn visible(patch: Vec2, focus: Vec2, span: f32) -> bool {
    // 50 m decoration tiles nest in the original 100 m terrain chunks.
    // Film only appears against the exact near triangles: a coarse LOD could
    // otherwise sit several centimetres above or below a 3 mm film.
    let terrain_center = (patch / 100.0).floor() * 100.0 + Vec2::splat(50.0);
    span < 100.0 && terrain_center.distance_squared(focus) < 150.0 * 150.0
}

pub(super) fn update(patches: &mut PatchViews, focus: Vec3, span: f32) {
    let xz = Vec2::new(focus.x, focus.z);
    for (patch, mut visibility) in patches.iter_mut() {
        visibility.set_if_neq(if visible(patch.0, xz, span) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_can_be_seen_off_centre_but_not_across_island_or_in_overview() {
        let patch = Vec2::new(325.0, 125.0);
        assert!(visible(patch, patch + Vec2::X * 34.0, 6.0));
        assert!(!visible(patch, patch + Vec2::X * 195.0, 60.0));
        assert!(!visible(patch, patch, 100.0));
    }

    #[test]
    fn shipping_scene_has_distinct_nonidentity_range_centers() {
        let raw = include_bytes!("../../../assets/landscape/island/strandline.glb");
        assert_eq!(&raw[..4], b"glTF");
        let count = u32::from_le_bytes(raw[12..16].try_into().unwrap()) as usize;
        let gltf: serde_json::Value = serde_json::from_slice(&raw[20..20 + count]).unwrap();
        let nodes = gltf["nodes"].as_array().unwrap();
        assert!(!nodes.is_empty());
        for node in nodes {
            let unit: [f32; 3] = serde_json::from_value(node["translation"].clone()).unwrap();
            assert!(center(node["name"].as_str().unwrap(), Vec3::from_array(unit)).is_some());
        }
        assert!(center("Strand_6_2_film", Vec3::ZERO).is_none());
    }
}
