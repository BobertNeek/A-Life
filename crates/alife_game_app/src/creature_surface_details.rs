use alife_world::CreatureAppearanceGenome;

use crate::CreaturePartSlot;

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

pub fn creature_face_style(appearance: CreatureAppearanceGenome) -> CreatureFaceStyle {
    let inherited = f32::from(appearance.ear_muzzle_trait) / 255.0;
    let species = f32::from(appearance.species_archetype % 8) / 7.0;
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

pub fn creature_surface_detail_recipe(
    appearance: CreatureAppearanceGenome,
    lod_scale: f32,
) -> Vec<CreatureSurfaceDetailSpec> {
    let lod_scale = lod_scale.clamp(0.5, 1.25);
    let marking_scale = 0.78 + f32::from(appearance.marking_density) / 255.0 * 0.48;
    let muzzle_scale = 0.78 + f32::from(appearance.ear_muzzle_trait) / 255.0 * 0.46;
    let tail_scale = 0.72 + f32::from(appearance.tail_trait) / 255.0 * 0.62;
    let species = appearance.species_archetype % 8;
    let arm_lengths = [0.70, 0.61, 0.58, 0.72, 0.58, 0.60, 0.60, 0.72];
    let leg_lengths = [0.58, 0.64, 0.57, 0.53, 0.56, 0.72, 0.54, 0.68];
    let tail_lengths = [0.55, 0.76, 0.38, 0.45, 0.68, 0.30, 0.48, 0.84];
    let arm_length = arm_lengths[usize::from(appearance.part_sources.arms.0 % 8)];
    let leg_length = leg_lengths[usize::from(appearance.part_sources.legs.0 % 8)];
    let tail_length = tail_lengths[usize::from(appearance.part_sources.tail.0 % 8)];
    let mut details = Vec::with_capacity(16);
    let mut push =
        |role, anchor_slot, local_offset: [f32; 3], local_scale: [f32; 3], mesh, material| {
            details.push(CreatureSurfaceDetailSpec {
                role,
                anchor_slot,
                local_offset: local_offset.map(|axis| axis * lod_scale),
                local_scale: local_scale.map(|axis| axis * lod_scale),
                mesh,
                material,
            });
        };

    push(
        "left-cheek-patch",
        CreaturePartSlot::Head,
        [-0.20, 0.18, 0.24],
        [marking_scale, 0.82 * marking_scale, 0.72],
        CreatureDetailMeshKind::CheekPatch,
        CreatureDetailMaterialRole::Accent,
    );
    push(
        "right-cheek-patch",
        CreaturePartSlot::Head,
        [0.20, 0.18, 0.24],
        [marking_scale, 0.82 * marking_scale, 0.72],
        CreatureDetailMeshKind::CheekPatch,
        CreatureDetailMaterialRole::Accent,
    );
    push(
        "brow-mask",
        CreaturePartSlot::Head,
        [0.0, 0.30, 0.22],
        [0.86 + marking_scale * 0.18, 0.78, 0.68],
        CreatureDetailMeshKind::BrowBand,
        CreatureDetailMaterialRole::Dark,
    );
    push(
        "muzzle",
        CreaturePartSlot::Head,
        [0.0, 0.10 - (muzzle_scale - 1.0) * 0.03, 0.27],
        [
            muzzle_scale,
            0.82 + muzzle_scale * 0.12,
            0.84 + muzzle_scale * 0.18,
        ],
        CreatureDetailMeshKind::Muzzle,
        CreatureDetailMaterialRole::Belly,
    );
    push(
        "belly-patch",
        CreaturePartSlot::Torso,
        [0.0, 0.06, 0.20],
        [0.92 + marking_scale * 0.14, 1.12, 0.72],
        CreatureDetailMeshKind::Muzzle,
        CreatureDetailMaterialRole::Belly,
    );
    push(
        "left-hand",
        CreaturePartSlot::LeftArm,
        [0.0, -arm_length * 0.90, 0.03],
        [0.92, 0.90, 0.92],
        CreatureDetailMeshKind::Hand,
        CreatureDetailMaterialRole::Belly,
    );
    push(
        "right-hand",
        CreaturePartSlot::RightArm,
        [0.0, -arm_length * 0.90, 0.03],
        [0.92, 0.90, 0.92],
        CreatureDetailMeshKind::Hand,
        CreatureDetailMaterialRole::Belly,
    );
    push(
        "left-foot",
        CreaturePartSlot::LeftLeg,
        [0.0, -leg_length + 0.045, 0.06],
        [1.08, 0.78, 1.24],
        CreatureDetailMeshKind::Foot,
        CreatureDetailMaterialRole::Dark,
    );
    push(
        "right-foot",
        CreaturePartSlot::RightLeg,
        [0.0, -leg_length + 0.045, 0.06],
        [1.08, 0.78, 1.24],
        CreatureDetailMeshKind::Foot,
        CreatureDetailMaterialRole::Dark,
    );

    let ear_mesh = if matches!(species, 2 | 4 | 7) {
        CreatureDetailMeshKind::Fin
    } else {
        CreatureDetailMeshKind::Ear
    };
    let ear_width = 0.82 + muzzle_scale * 0.30;
    push(
        "left-ear",
        CreaturePartSlot::Head,
        [-0.27 - (muzzle_scale - 1.0) * 0.035, 0.36, 0.015],
        [ear_width, 0.92 + muzzle_scale * 0.20, 0.86],
        ear_mesh,
        CreatureDetailMaterialRole::Accent,
    );
    push(
        "right-ear",
        CreaturePartSlot::Head,
        [0.27 + (muzzle_scale - 1.0) * 0.035, 0.36, 0.015],
        [ear_width, 0.92 + muzzle_scale * 0.20, 0.86],
        ear_mesh,
        CreatureDetailMaterialRole::Accent,
    );
    push(
        "tail-accent",
        CreaturePartSlot::TailBack,
        [0.0, 0.03, -tail_length * (0.42 + (tail_scale - 1.0) * 0.08)],
        [
            0.74 + tail_scale * 0.34,
            0.82 + tail_scale * 0.28,
            tail_scale,
        ],
        CreatureDetailMeshKind::TailAccent,
        CreatureDetailMaterialRole::Accent,
    );

    match species {
        0 => push(
            "roundling-crown-tuft",
            CreaturePartSlot::Head,
            [0.0, 0.35, 0.18],
            [0.74, 0.62, 0.64],
            CreatureDetailMeshKind::Tuft,
            CreatureDetailMaterialRole::Accent,
        ),
        1 => push(
            "longtail-wrist-band",
            CreaturePartSlot::LeftArm,
            [0.0, -arm_length * 0.68, 0.035],
            [0.90, 0.82, 0.92],
            CreatureDetailMeshKind::LimbBand,
            CreatureDetailMaterialRole::Dark,
        ),
        2 => push(
            "digger-forehead-crest",
            CreaturePartSlot::Head,
            [0.0, 0.37, 0.18],
            [0.84, 0.72, 0.70],
            CreatureDetailMeshKind::Tuft,
            CreatureDetailMaterialRole::Keratin,
        ),
        3 => push(
            "nightling-brow-horn",
            CreaturePartSlot::Head,
            [0.0, 0.36, 0.20],
            [0.64, 0.76, 0.66],
            CreatureDetailMeshKind::Tuft,
            CreatureDetailMaterialRole::Keratin,
        ),
        4 => push(
            "riverling-side-fin",
            CreaturePartSlot::Torso,
            [-0.32, 0.08, 0.01],
            [1.02, 1.10, 0.90],
            CreatureDetailMeshKind::Fin,
            CreatureDetailMaterialRole::Accent,
        ),
        5 => push(
            "leafling-chest-band",
            CreaturePartSlot::Torso,
            [0.0, 0.17, 0.20],
            [1.18, 0.86, 0.78],
            CreatureDetailMeshKind::LimbBand,
            CreatureDetailMaterialRole::Accent,
        ),
        6 => push(
            "longarm-shoulder-tuft",
            CreaturePartSlot::Torso,
            [0.0, 0.28, 0.03],
            [1.24, 0.94, 0.86],
            CreatureDetailMeshKind::Tuft,
            CreatureDetailMaterialRole::Accent,
        ),
        _ => push(
            "maskling-face-chevron",
            CreaturePartSlot::Head,
            [0.0, 0.23, 0.25],
            [1.12, 0.82, 0.72],
            CreatureDetailMeshKind::BrowBand,
            CreatureDetailMaterialRole::Accent,
        ),
    }

    if appearance.marking_density >= 128 {
        push(
            "left-flank-mark",
            CreaturePartSlot::Torso,
            [-0.27, 0.0, 0.17],
            [marking_scale, 0.88, 0.72],
            CreatureDetailMeshKind::CheekPatch,
            CreatureDetailMaterialRole::Accent,
        );
        push(
            "right-flank-mark",
            CreaturePartSlot::Torso,
            [0.27, 0.0, 0.17],
            [marking_scale, 0.88, 0.72],
            CreatureDetailMeshKind::CheekPatch,
            CreatureDetailMaterialRole::Accent,
        );
    }

    details
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::CreaturePartSlot;

    use super::*;

    #[test]
    fn every_species_recipe_has_integrated_face_hands_feet_and_bounded_geometry() {
        for species in 0..alife_world::CREATURE_APPEARANCE_SPECIES_COUNT {
            let appearance = CreatureAppearanceGenome::founder_for_species(
                species,
                0xC0DE_0000 + u64::from(species),
            );
            let recipe = creature_surface_detail_recipe(appearance, 1.0);
            let roles = recipe
                .iter()
                .map(|detail| detail.role)
                .collect::<BTreeSet<_>>();

            for required in [
                "left-cheek-patch",
                "right-cheek-patch",
                "brow-mask",
                "muzzle",
                "left-hand",
                "right-hand",
                "left-foot",
                "right-foot",
            ] {
                assert!(
                    roles.contains(required),
                    "species {species} is missing visible detail {required}"
                );
            }
            assert!(recipe.len() >= 10, "species {species} lacks surface detail");
            assert!(recipe.iter().all(|detail| {
                detail
                    .local_offset
                    .into_iter()
                    .chain(detail.local_scale)
                    .all(f32::is_finite)
                    && detail
                        .local_offset
                        .into_iter()
                        .all(|axis| axis.abs() <= 1.5)
                    && detail
                        .local_scale
                        .into_iter()
                        .all(|axis| (0.05..=2.0).contains(&axis))
            }));
        }
    }

    #[test]
    fn previously_unused_appearance_genes_change_visible_detail_recipes() {
        let base = CreatureAppearanceGenome::founder_for_species(4, 0xA11F_E001);
        let base_recipe = creature_surface_detail_recipe(base, 1.0);

        let mut denser_markings = base;
        denser_markings.marking_density = base.marking_density.wrapping_add(37);
        let mut different_muzzle = base;
        different_muzzle.ear_muzzle_trait = base.ear_muzzle_trait.wrapping_add(61);
        let mut different_tail = base;
        different_tail.tail_trait = base.tail_trait.wrapping_add(83);

        assert_ne!(
            creature_surface_detail_recipe(denser_markings, 1.0),
            base_recipe,
            "marking_density must have a visible renderer consumer"
        );
        assert_ne!(
            creature_surface_detail_recipe(different_muzzle, 1.0),
            base_recipe,
            "ear_muzzle_trait must have a visible renderer consumer"
        );
        assert_ne!(
            creature_surface_detail_recipe(different_tail, 1.0),
            base_recipe,
            "tail_trait must have a visible renderer consumer"
        );
    }

    #[test]
    fn lod_scale_changes_size_without_changing_identity_or_part_count() {
        let appearance = CreatureAppearanceGenome::founder_for_species(2, 0xA11F_E002);
        let full = creature_surface_detail_recipe(appearance, 1.0);
        let compact = creature_surface_detail_recipe(appearance, 0.82);

        assert_eq!(full.len(), compact.len());
        assert!(full.iter().zip(&compact).all(|(a, b)| {
            a.role == b.role
                && a.mesh == b.mesh
                && a.material == b.material
                && a.local_offset != b.local_offset
                && a.local_scale != b.local_scale
        }));
    }

    #[test]
    fn face_style_uses_integrated_warm_eyes_and_small_nonblack_pupils() {
        for species in 0..alife_world::CREATURE_APPEARANCE_SPECIES_COUNT {
            let appearance = CreatureAppearanceGenome::founder_for_species(
                species,
                0xFACE_0000 + u64::from(species),
            );
            let style = creature_face_style(appearance);
            assert!(style
                .sclera_scale
                .into_iter()
                .chain(style.iris_scale)
                .chain(style.pupil_scale)
                .chain(style.sclera_rgba)
                .chain(style.pupil_rgba)
                .chain([style.eye_spacing, style.eye_height, style.eye_forward])
                .all(f32::is_finite));
            assert!((0.075..=0.105).contains(&style.eye_spacing));
            assert!((0.22..=0.285).contains(&style.eye_height));
            assert!((0.24..=0.33).contains(&style.eye_forward));
            assert!((0.66..=0.86).contains(&style.sclera_scale[0]));
            assert!((0.70..=0.94).contains(&style.sclera_scale[1]));
            assert!((0.34..=0.48).contains(&style.sclera_scale[2]));
            assert!(style
                .sclera_rgba
                .into_iter()
                .take(3)
                .all(|channel| (0.68..=0.94).contains(&channel)));
            assert!(style
                .pupil_rgba
                .into_iter()
                .take(3)
                .all(|channel| (0.025..=0.22).contains(&channel)));
            assert!(style
                .iris_scale
                .iter()
                .zip(style.sclera_scale)
                .all(|(iris, sclera)| *iris < sclera));
            assert!(style
                .pupil_scale
                .iter()
                .zip(style.iris_scale)
                .all(|(pupil, iris)| *pupil < iris));
        }
    }

    #[test]
    fn runtime_head_ornaments_extend_the_brow_instead_of_stacking_above_the_skull() {
        for species in 0..alife_world::CREATURE_APPEARANCE_SPECIES_COUNT {
            let appearance = CreatureAppearanceGenome::founder_for_species(
                species,
                0xC0DE_0000 + u64::from(species),
            );
            let recipe = creature_surface_detail_recipe(appearance, 1.0);
            for detail in recipe.iter().filter(|detail| {
                matches!(
                    detail.role,
                    "roundling-crown-tuft" | "digger-forehead-crest" | "nightling-brow-horn"
                )
            }) {
                assert!(
                    detail.local_offset[1] <= 0.40,
                    "{} stacks above the canonical head at y {}",
                    detail.role,
                    detail.local_offset[1]
                );
                assert!(
                    detail.local_offset[2] >= 0.14,
                    "{} must merge into the forward brow silhouette",
                    detail.role
                );
            }
        }
    }

    #[test]
    fn inherited_ear_muzzle_trait_changes_face_proportions() {
        let base = CreatureAppearanceGenome::founder_for_species(5, 0xFACE_A11F);
        let mut changed = base;
        changed.ear_muzzle_trait = changed.ear_muzzle_trait.wrapping_add(97);
        assert_ne!(creature_face_style(base), creature_face_style(changed));
    }

    #[test]
    fn every_surface_detail_declares_the_anatomical_part_it_must_follow() {
        for species in 0..alife_world::CREATURE_APPEARANCE_SPECIES_COUNT {
            let appearance = CreatureAppearanceGenome::founder_for_species(
                species,
                0xA11C_0000 + u64::from(species),
            );
            let recipe = creature_surface_detail_recipe(appearance, 1.0);
            assert!(recipe.iter().all(|detail| {
                match detail.role {
                    "left-hand" | "longtail-wrist-band" => {
                        detail.anchor_slot == CreaturePartSlot::LeftArm
                    }
                    "right-hand" => detail.anchor_slot == CreaturePartSlot::RightArm,
                    "left-foot" => detail.anchor_slot == CreaturePartSlot::LeftLeg,
                    "right-foot" => detail.anchor_slot == CreaturePartSlot::RightLeg,
                    "tail-accent" => detail.anchor_slot == CreaturePartSlot::TailBack,
                    "belly-patch"
                    | "left-flank-mark"
                    | "right-flank-mark"
                    | "riverling-side-fin"
                    | "leafling-chest-band"
                    | "longarm-shoulder-tuft" => detail.anchor_slot == CreaturePartSlot::Torso,
                    _ => detail.anchor_slot == CreaturePartSlot::Head,
                }
            }));
        }
    }
}
