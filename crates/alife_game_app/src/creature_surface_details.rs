use std::collections::BTreeMap;

use alife_world::CreatureAppearanceGenome;
use thiserror::Error;

use crate::{
    CreaturePartSlot, CreatureVisualBounds, GeneForgeCanonicalBounds, GeneForgeLandmarkId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureDetailMeshKind {
    CheekPatch,
    BrowBand,
    Muzzle,
    Ear,
    Fin,
    Tuft,
    LimbBand,
    Hand,
    Foot,
    TailAccent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureDetailMaterialRole {
    Belly,
    Accent,
    Dark,
    Keratin,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureSurfaceDetailSpec {
    pub role: &'static str,
    pub anchor_slot: CreaturePartSlot,
    pub local_offset: [f32; 3],
    pub local_scale: [f32; 3],
    pub mesh: CreatureDetailMeshKind,
    pub material: CreatureDetailMaterialRole,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureFaceStyle {
    pub eye_spacing: f32,
    pub eye_height: f32,
    pub eye_forward: f32,
    pub sclera_scale: [f32; 3],
    pub iris_scale: [f32; 3],
    pub pupil_scale: [f32; 3],
    pub sclera_rgba: [f32; 4],
    pub pupil_rgba: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CreatureSurfaceDetailError {
    #[error("head asset is missing required landmark {0:?}")]
    MissingLandmark(GeneForgeLandmarkId),
    #[error("head asset contains a non-finite eye landmark")]
    NonFiniteLandmark,
    #[error("head asset contains invalid canonical or emitted bounds")]
    InvalidBounds,
}

pub fn remap_creature_face_landmarks(
    canonical_bounds: GeneForgeCanonicalBounds,
    emitted_bounds: CreatureVisualBounds,
    landmarks: &BTreeMap<GeneForgeLandmarkId, [f32; 3]>,
) -> Result<BTreeMap<GeneForgeLandmarkId, [f32; 3]>, CreatureSurfaceDetailError> {
    if !emitted_bounds.is_valid()
        || !(0..3).all(|axis| {
            canonical_bounds.min[axis].is_finite()
                && canonical_bounds.max[axis].is_finite()
                && canonical_bounds.min[axis] < canonical_bounds.max[axis]
                && emitted_bounds.min[axis] < emitted_bounds.max[axis]
        })
    {
        return Err(CreatureSurfaceDetailError::InvalidBounds);
    }
    landmarks
        .iter()
        .map(|(landmark, point)| {
            if !point.iter().copied().all(f32::is_finite) {
                return Err(CreatureSurfaceDetailError::NonFiniteLandmark);
            }
            let remapped = std::array::from_fn(|axis| {
                let normalized = (point[axis] - canonical_bounds.min[axis])
                    / (canonical_bounds.max[axis] - canonical_bounds.min[axis]);
                emitted_bounds.min[axis]
                    + normalized * (emitted_bounds.max[axis] - emitted_bounds.min[axis])
            });
            Ok((*landmark, remapped))
        })
        .collect()
}

/// Renderer-only fallback until Task 7 passes source landmarks into eye spawning.
#[deprecated(note = "Task 7 must use creature_face_style_from_landmarks")]
pub fn creature_face_style(appearance: CreatureAppearanceGenome) -> CreatureFaceStyle {
    let inherited = f32::from(appearance.ear_muzzle_trait) / 255.0;
    let species = f32::from(appearance.species_archetype)
        / f32::from(alife_world::CREATURE_APPEARANCE_SPECIES_COUNT.saturating_sub(1));
    let eye_size = 0.70 + inherited * 0.10;
    CreatureFaceStyle {
        eye_spacing: 0.080 + inherited * 0.012 + species * 0.006,
        eye_height: 0.225 + species * 0.035 + inherited * 0.012,
        eye_forward: 0.255 + inherited * 0.025,
        sclera_scale: [eye_size, eye_size * 1.08, 0.42],
        iris_scale: [eye_size * 0.62, eye_size * 0.68, 0.25],
        pupil_scale: [eye_size * 0.26, eye_size * 0.36, 0.13],
        sclera_rgba: [0.91, 0.86, 0.74, 1.0],
        pupil_rgba: [0.08, 0.045, 0.03, 1.0],
    }
}

pub fn creature_face_style_from_landmarks(
    appearance: CreatureAppearanceGenome,
    landmarks: &BTreeMap<GeneForgeLandmarkId, [f32; 3]>,
) -> Result<CreatureFaceStyle, CreatureSurfaceDetailError> {
    let left = landmarks
        .get(&GeneForgeLandmarkId::LeftEye)
        .copied()
        .ok_or(CreatureSurfaceDetailError::MissingLandmark(
            GeneForgeLandmarkId::LeftEye,
        ))?;
    let right = landmarks
        .get(&GeneForgeLandmarkId::RightEye)
        .copied()
        .ok_or(CreatureSurfaceDetailError::MissingLandmark(
            GeneForgeLandmarkId::RightEye,
        ))?;
    if !left.into_iter().chain(right).all(f32::is_finite) {
        return Err(CreatureSurfaceDetailError::NonFiniteLandmark);
    }

    let inherited = f32::from(appearance.ear_muzzle_trait) / 255.0;
    let eye_spacing = (right[0] - left[0]).abs() * 0.5;
    let eye_radius = (eye_spacing * (0.52 + inherited * 0.08)).clamp(0.10, 0.17);
    let eye_size = eye_radius / 0.085;
    Ok(CreatureFaceStyle {
        eye_spacing,
        eye_height: (left[1] + right[1]) * 0.5,
        eye_forward: (left[2] + right[2]) * 0.5,
        sclera_scale: [eye_size, eye_size * 1.08, 0.42],
        iris_scale: [eye_size * 0.62, eye_size * 0.68, 0.25],
        pupil_scale: [eye_size * 0.30, eye_size * 0.42, eye_size * 0.16],
        sclera_rgba: [0.91, 0.86, 0.74, 1.0],
        pupil_rgba: [0.08, 0.045, 0.03, 1.0],
    })
}

/// Temporary renderer entry point. Source-authored heads and limbs already own
/// face, ear, hand, foot, and silhouette geometry, so only coat markings remain.
pub fn creature_surface_detail_recipe(
    appearance: CreatureAppearanceGenome,
    lod_scale: f32,
) -> Vec<CreatureSurfaceDetailSpec> {
    let lod_scale = lod_scale.clamp(0.5, 1.25);
    let marking = f32::from(appearance.marking_density) / 255.0;
    let body_mass = f32::from(appearance.body_mass_trait) / 255.0;
    let width = (0.90 + marking * 0.48 + body_mass * 0.08) * lod_scale;
    let height = (1.02 + body_mass * 0.18) * lod_scale;
    let depth = (0.62 + marking * 0.10) * lod_scale;
    let offset_y = (0.04 + body_mass * 0.04) * lod_scale;

    vec![CreatureSurfaceDetailSpec {
        role: "belly-coat-marking",
        anchor_slot: CreaturePartSlot::Torso,
        local_offset: [0.0, offset_y, 0.20 * lod_scale],
        local_scale: [width, height, depth],
        mesh: CreatureDetailMeshKind::CheekPatch,
        material: CreatureDetailMaterialRole::Belly,
    }]
}

#[cfg(test)]
mod tests {
    use alife_world::CreatureAppearanceGenome;

    use crate::{
        load_geneforge_creature_part_catalog, CreaturePartAssetId, CreaturePartSlot,
        GeneForgeLandmarkId,
    };

    use super::*;

    #[test]
    fn face_placement_uses_exact_catalog_head_landmarks_and_continuous_traits() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        let head = catalog
            .asset(&CreaturePartAssetId("grendel-head".into()))
            .unwrap();
        let appearance = CreatureAppearanceGenome::founder_for_species(11, 0xFACE_6001);
        let style = creature_face_style_from_landmarks(appearance, &head.landmarks).unwrap();
        let left = head.landmarks[&GeneForgeLandmarkId::LeftEye];
        let right = head.landmarks[&GeneForgeLandmarkId::RightEye];

        assert!((style.eye_spacing - (right[0] - left[0]).abs() * 0.5).abs() <= 1.0e-6);
        assert!((style.eye_height - (left[1] + right[1]) * 0.5).abs() <= 1.0e-6);
        assert!((style.eye_forward - (left[2] + right[2]) * 0.5).abs() <= 1.0e-6);

        let mut shifted = head.landmarks.clone();
        shifted.get_mut(&GeneForgeLandmarkId::LeftEye).unwrap()[1] += 0.05;
        assert_ne!(
            creature_face_style_from_landmarks(appearance, &shifted).unwrap(),
            style
        );
    }

    #[test]
    fn canonical_face_landmarks_remap_into_emitted_head_bounds_and_avoid_bead_eyes() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        let head = catalog
            .asset(&CreaturePartAssetId("norn-head".into()))
            .unwrap();
        let emitted = CreatureVisualBounds::new(
            [-0.82063305, 0.9410908, -0.10534345],
            [0.82063305, 1.6487956, 0.7203621],
        );
        let remapped =
            remap_creature_face_landmarks(head.canonical_bounds, emitted, &head.landmarks).unwrap();
        let left = remapped[&GeneForgeLandmarkId::LeftEye];
        let right = remapped[&GeneForgeLandmarkId::RightEye];
        assert!(left[1] > emitted.min[1] && left[1] < emitted.max[1]);
        assert!(left[2] >= emitted.min[2] && left[2] < emitted.min[2] + 0.10);
        assert!(right[0] > left[0]);

        let appearance = CreatureAppearanceGenome::founder_for_species(0, 0xFACE_7001);
        let style = creature_face_style_from_landmarks(appearance, &remapped).unwrap();
        assert!(style.sclera_scale[0] >= 1.15);
        assert!(style.pupil_scale[0] >= 0.28);
    }

    #[test]
    fn source_authored_anatomy_removes_generic_face_hat_and_limb_recipes() {
        for species in 0..alife_world::CREATURE_APPEARANCE_SPECIES_COUNT {
            let appearance =
                CreatureAppearanceGenome::founder_for_species(species, 0x6002 + u64::from(species));
            let details = creature_surface_detail_recipe(appearance, 1.0);
            assert!(details.iter().all(|detail| {
                detail.anchor_slot == CreaturePartSlot::Torso
                    && !matches!(
                        detail.mesh,
                        CreatureDetailMeshKind::Muzzle
                            | CreatureDetailMeshKind::Ear
                            | CreatureDetailMeshKind::Fin
                            | CreatureDetailMeshKind::Tuft
                            | CreatureDetailMeshKind::Hand
                            | CreatureDetailMeshKind::Foot
                    )
            }));
        }
    }

    #[test]
    fn continuous_marking_traits_change_torso_surface_without_family_tables() {
        let base = CreatureAppearanceGenome::founder_for_species(0, 0x6003);
        let mut changed = base;
        changed.marking_density = changed.marking_density.saturating_add(1).min(15);
        assert_ne!(
            creature_surface_detail_recipe(base, 1.0),
            creature_surface_detail_recipe(changed, 1.0)
        );
    }

    #[test]
    fn missing_required_eye_landmark_is_rejected() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        let head = catalog
            .asset(&CreaturePartAssetId("norn-head".into()))
            .unwrap();
        let mut landmarks = head.landmarks.clone();
        landmarks.remove(&GeneForgeLandmarkId::RightEye);
        assert!(matches!(
            creature_face_style_from_landmarks(CreatureAppearanceGenome::default(), &landmarks),
            Err(CreatureSurfaceDetailError::MissingLandmark(
                GeneForgeLandmarkId::RightEye
            ))
        ));
    }
}
