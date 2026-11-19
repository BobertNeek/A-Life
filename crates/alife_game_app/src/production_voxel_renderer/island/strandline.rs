//! View-only tidal strand and sheltered woodland floor on the approved terrain.
use super::*;

const STRAND: &str = "landscape/island/strandline.glb";
const COPSE: &str = "landscape/island/copse-floor.glb";

#[derive(Clone, Copy)]
enum Dressing {
    Tidal,
    Woodland,
}

#[derive(Component)]
pub(in crate::production_voxel_renderer) struct GroundDetailPatch {
    center: Vec2,
    dressing: Dressing,
}

pub(in crate::production_voxel_renderer) type PatchViews<'w, 's> =
    Query<'w, 's, (&'static GroundDetailPatch, &'static mut Visibility), Without<PropRange>>;

fn center(name: &str, translation: Vec3) -> Option<Vec2> {
    ((name.starts_with("Strand_") || name.starts_with("Copse_"))
        && (translation.x != 0.0 || translation.z != 0.0))
        .then_some(Vec2::new(translation.x, translation.z))
}

pub(super) fn spawn(world: &mut World) {
    for (name, asset) in [("Tidal strandline", STRAND), ("Sheltered copse floor", COPSE)] {
        let scene = world
            .resource::<AssetServer>()
            .load(bevy::gltf::GltfAssetLabel::Scene(0).from_asset(asset));
        world
            .spawn((
                Name::new(name),
                SceneRoot(scene),
                Transform::default(),
                Visibility::Hidden,
                Fvr04ProductionRuntimeSceneRoot,
                bevy::picking::Pickable::IGNORE,
            ))
            .observe(bind);
    }
}

fn bind(
    ready: On<bevy::scene::SceneInstanceReady>,
    children: Query<&Children>,
    nodes: Query<(&Name, &Transform)>,
    mut commands: Commands,
) {
    for entity in children.iter_descendants(ready.entity) {
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert(bevy::picking::Pickable::IGNORE);
        if let Ok((name, transform)) = nodes.get(entity) {
            // The GLB places every actual patch node at its tile centre. Its
            // primitive children have local identity and inherit visibility.
            if let Some(xz) = center(name.as_str(), transform.translation) {
                let dressing = if name.as_str().starts_with("Copse_") {
                    Dressing::Woodland
                } else {
                    Dressing::Tidal
                };
                entity_commands.insert((
                    GroundDetailPatch { center: xz, dressing },
                    Visibility::Hidden,
                ));
                // Only substantial rooted coastal thickets cast. Tiny stones,
                // lace, leaves and fitted ground films should not self-shadow.
                if !name.as_str().ends_with("_headland") {
                    entity_commands.insert(NotShadowCaster);
                    for mesh in children.iter_descendants(entity) {
                        commands.entity(mesh).insert(NotShadowCaster);
                    }
                }
            }
        }
    }
    // Reveal the group only after all its tile gates start hidden. A large
    // overview must not flash every patch for one scene-load frame.
    commands.entity(ready.entity).insert(Visibility::Inherited);
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
        let shown = match patch.dressing {
            Dressing::Tidal => visible(patch.center, xz, span),
            // A 50 m tile centre is at most 36 m from its 100 m parent
            // terrain centre. Both gates consequently retain exact near ground.
            Dressing::Woodland => span < 60.0 && patch.center.distance_squared(xz) < 65.0 * 65.0,
        };
        visibility.set_if_neq(if shown {
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
