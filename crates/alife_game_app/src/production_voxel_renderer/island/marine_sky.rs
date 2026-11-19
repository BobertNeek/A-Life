//! One art-directed island backdrop. This is a distant visual, not a new terrain.
use super::*;

const ASSET: &str = "landscape/island/marine-day.glb";

fn material(image: Handle<Image>) -> StandardMaterial {
    StandardMaterial {
        base_color_texture: Some(image),
        // The sky cannot illuminate, receive distance fog twice or acquire a
        // sun glint. Its one inward-facing envelope is behind the whole island.
        unlit: true,
        fog_enabled: false,
        ..default()
    }
}

pub(super) fn spawn(world: &mut World) {
    use bevy::gltf::GltfAssetLabel;
    let server = world.resource::<AssetServer>();
    let mesh: Handle<Mesh> = server.load(
        GltfAssetLabel::Primitive { mesh: 0, primitive: 0 }.from_asset(ASSET),
    );
    let image: Handle<Image> = server.load(GltfAssetLabel::Texture(0).from_asset(ASSET));
    let paint = world.resource_mut::<Assets<StandardMaterial>>().add(material(image));
    world.spawn((
        Name::new("Marine clear and cloud"),
        Mesh3d(mesh),
        MeshMaterial3d(paint),
        Transform::default(),
        NotShadowCaster,
        bevy::light::NotShadowReceiver,
        bevy::picking::Pickable::IGNORE,
        Fvr04ProductionRuntimeSceneRoot,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_sky_has_no_physical_or_weather_dependency() {
        let m = material(Handle::default());
        assert!(m.unlit && !m.fog_enabled);
        let raw = include_bytes!("../../../assets/landscape/island/marine-day.glb");
        let n = u32::from_le_bytes(raw[12..16].try_into().unwrap()) as usize;
        let g: serde_json::Value = serde_json::from_slice(&raw[20..20+n]).unwrap();
        assert_eq!(g["meshes"].as_array().unwrap().len(),1);
        let p = &g["meshes"][0]["primitives"][0];
        let a = &g["accessors"][p["attributes"]["POSITION"].as_u64().unwrap() as usize];
        assert!(a["min"][0].as_f64().unwrap() < -2700.0 && a["max"][0].as_f64().unwrap() > 2700.0);
        assert!(a["max"][1].as_f64().unwrap() < 3000.0);
        assert_eq!(g["materials"][0]["extensions"]["KHR_materials_unlit"],serde_json::json!({}));
        assert_eq!(g["images"][0]["mimeType"].as_str(),Some("image/png"));
        assert!(raw.len() < 1_500_000);
    }
}
