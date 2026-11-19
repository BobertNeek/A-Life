//! Optional canopy geometry on existing conifers, never a second tree population.
use super::*;
use std::{num::NonZeroU8, sync::OnceLock};

pub(super) const FORMS: u8 = 12;
pub(super) const PACK: &str = "landscape/island/grove-crowns.glb";

#[derive(serde::Deserialize)]
struct Layout {
    form: u8,
    xz: [f32; 2],
}

#[derive(Clone, Copy)]
pub(in crate::production_voxel_renderer) struct GroveForm(NonZeroU8);

fn layout() -> &'static BTreeMap<usize, Layout> {
    static INDEX: OnceLock<BTreeMap<usize, Layout>> = OnceLock::new();
    INDEX.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../assets/landscape/island/grove-crowns.json"
        ))
        .expect("approved visual canopy layout")
    })
}

pub(super) fn at(index: usize, original: &Prop) -> Option<GroveForm> {
    if original.kind != "Conifer" {
        return None;
    }
    let entry = layout().get(&index)?;
    // Never put a previously baked upper crown on a relocated or different
    // trunk if another island revision changes placement order.
    let xz = Vec2::new(original.position[0], original.position[2]);
    if (1..=FORMS).contains(&entry.form)
        && xz.distance_squared(Vec2::from_array(entry.xz)) < 0.000004
    {
        NonZeroU8::new(entry.form).map(GroveForm)
    } else {
        None
    }
}

pub(super) struct CrownModels {
    meshes: Vec<[Handle<Mesh>; 3]>,
    pub(super) material: Handle<StandardMaterial>,
}

pub(super) fn models(pack: &Gltf, assets: &Assets<GltfMesh>) -> Option<CrownModels> {
    let [material] = pack.materials.as_slice() else {
        return None;
    };
    let mesh = |form, lod| {
        let handle = pack
            .named_meshes
            .get(format!("PineForm{form}_L{lod}").as_str())?;
        let [primitive] = assets.get(handle)?.primitives.as_slice() else {
            return None;
        };
        (primitive.material.as_ref()? == material).then(|| primitive.mesh.clone())
    };
    // Incomplete streaming or a mismatched material keeps the ordinary pines.
    // Bind the entire table once, never stampede the asset server for 421 trees.
    let meshes = (1..=FORMS)
        .map(|f| Some([mesh(f, 0)?, mesh(f, 1)?, mesh(f, 2)?]))
        .collect::<Option<_>>()?;
    Some(CrownModels {
        meshes,
        material: material.clone(),
    })
}

pub(super) fn levels<'a>(form: &GroveForm, models: &'a CrownModels) -> &'a [Handle<Mesh>; 3] {
    &models.meshes[form.0.get() as usize - 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alternate_forms_only_attach_to_the_same_original_pine() {
        let mut props = placements();
        let mut original = 0;
        let mut replaced = 0;
        for (idx, p) in props.iter().enumerate() {
            let found = at(idx, p);
            if p.kind == "Conifer" {
                if found.is_some() {
                    replaced += 1;
                } else {
                    original += 1;
                }
            } else {
                assert!(found.is_none());
            }
        }
        assert_eq!((original, replaced), (0, 421));
        let &idx = layout().keys().next().unwrap();
        props[idx].position[0] += 0.05;
        assert!(at(idx, &props[idx]).is_none());
    }

    #[test]
    fn shipping_pack_has_every_form_at_every_lod_and_one_draw_each() {
        let data = include_bytes!("../../../assets/landscape/island/grove-crowns.glb");
        let len = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
        let doc: serde_json::Value = serde_json::from_slice(&data[20..20 + len]).unwrap();
        let meshes = doc["meshes"].as_array().unwrap();
        assert_eq!(meshes.len(), usize::from(FORMS) * 3);
        let materials = doc["materials"].as_array().unwrap();
        assert_eq!(materials.len(), 1);
        let m = &materials[0];
        assert_eq!(m["pbrMetallicRoughness"]["baseColorTexture"]["index"], 0);
        assert_eq!(m["normalTexture"]["index"], 1);
        assert_eq!(
            m["pbrMetallicRoughness"]["metallicRoughnessTexture"]["index"],
            2
        );
        let images = doc["images"].as_array().unwrap();
        assert_eq!(images.len(), 3);
        for (image, uri) in images.iter().zip([
            "canopy/needles.png",
            "canopy/needles-normal.png",
            "canopy/needles-orm.png",
        ]) {
            assert_eq!(image["uri"].as_str(), Some(uri));
        }
        for f in 1..=FORMS {
            for l in 0..3 {
                let label = format!("PineForm{f}_L{l}");
                let m = meshes
                    .iter()
                    .find(|m| m["name"].as_str() == Some(label.as_str()))
                    .unwrap();
                assert_eq!(m["primitives"].as_array().unwrap().len(), 1);
                assert_eq!(m["primitives"][0]["material"], 0);
                assert!(m["primitives"][0]["attributes"]["TEXCOORD_0"].is_number());
            }
        }
    }
}
