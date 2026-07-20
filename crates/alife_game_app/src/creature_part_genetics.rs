use alife_world::{CreaturePartFamilyId, CreaturePartSlotKey, CreaturePartSources};

use crate::{CreaturePartSlot, GeneForgeCatalogError, GeneForgeCreaturePartCatalog};

pub const RARE_PART_MUTATION_THRESHOLD: u16 = 8;
const MUTABLE_PART_SLOTS: [CreaturePartSlotKey; 4] = [
    CreaturePartSlotKey::Head,
    CreaturePartSlotKey::Arms,
    CreaturePartSlotKey::Legs,
    CreaturePartSlotKey::Tail,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreaturePartMutationWarning {
    UnknownFamilyFallback {
        requested: CreaturePartFamilyId,
        fallback: CreaturePartFamilyId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreaturePartMutationResult {
    pub sources: CreaturePartSources,
    pub changed_slot: Option<CreaturePartSlot>,
    pub rare_cross_family: bool,
    pub incompatible_slot_count: u8,
    pub warning: Option<CreaturePartMutationWarning>,
}

pub fn mutate_creature_part_sources(
    inherited: CreaturePartSources,
    mutation_count: u16,
    mutation_seed: u64,
    catalog: &GeneForgeCreaturePartCatalog,
) -> Result<CreaturePartMutationResult, GeneForgeCatalogError> {
    if catalog.families.is_empty() {
        return Err(GeneForgeCatalogError::InvalidCatalogMetadata {
            reason: "mutation requires at least one GeneForge family",
        });
    }

    let rare_cross_family = mutation_count >= RARE_PART_MUTATION_THRESHOLD
        && ((mutation_seed ^ u64::from(mutation_count)) & 0x7) == 0x5;
    let slot =
        MUTABLE_PART_SLOTS[deterministic_index(mutation_seed, 0x51A7, MUTABLE_PART_SLOTS.len())];
    let current = family_for_slot(inherited, slot);
    let mut candidates = catalog
        .families
        .iter()
        .map(|family| family.id)
        .filter(|candidate| *candidate != current)
        .collect::<Vec<_>>();
    candidates.sort_unstable();

    let (sources, changed_slot) = if candidates.is_empty() {
        (inherited, None)
    } else {
        let candidate = candidates[deterministic_index(mutation_seed, 0xFA11, candidates.len())];
        (
            with_family_for_slot(inherited, slot, candidate),
            Some(runtime_slot(slot)),
        )
    };
    let incompatible_slot_count = incompatible_slot_count(&sources, catalog);
    Ok(CreaturePartMutationResult {
        sources,
        changed_slot,
        rare_cross_family,
        incompatible_slot_count,
        warning: None,
    })
}

pub fn part_sources_are_ordinary_compatible(
    sources: &CreaturePartSources,
    catalog: &GeneForgeCreaturePartCatalog,
) -> bool {
    incompatible_slot_count(sources, catalog) == 0
}

fn incompatible_slot_count(
    sources: &CreaturePartSources,
    catalog: &GeneForgeCreaturePartCatalog,
) -> u8 {
    sources
        .iter_slots()
        .into_iter()
        .filter(|(slot, family)| {
            !slot_is_ordinary_compatible(sources.torso, *slot, *family, catalog)
        })
        .count() as u8
}

fn slot_is_ordinary_compatible(
    _torso: CreaturePartFamilyId,
    _slot: CreaturePartSlotKey,
    candidate: CreaturePartFamilyId,
    catalog: &GeneForgeCreaturePartCatalog,
) -> bool {
    catalog.families.iter().any(|family| family.id == candidate)
}

const fn family_for_slot(
    sources: CreaturePartSources,
    slot: CreaturePartSlotKey,
) -> CreaturePartFamilyId {
    match slot {
        CreaturePartSlotKey::Head => sources.head,
        CreaturePartSlotKey::Torso => sources.torso,
        CreaturePartSlotKey::Arms => sources.arms,
        CreaturePartSlotKey::Legs => sources.legs,
        CreaturePartSlotKey::Tail => sources.tail,
    }
}

const fn with_family_for_slot(
    mut sources: CreaturePartSources,
    slot: CreaturePartSlotKey,
    family: CreaturePartFamilyId,
) -> CreaturePartSources {
    match slot {
        CreaturePartSlotKey::Head => sources.head = family,
        CreaturePartSlotKey::Torso => sources.torso = family,
        CreaturePartSlotKey::Arms => sources.arms = family,
        CreaturePartSlotKey::Legs => sources.legs = family,
        CreaturePartSlotKey::Tail => sources.tail = family,
    }
    sources
}

const fn runtime_slot(slot: CreaturePartSlotKey) -> CreaturePartSlot {
    match slot {
        CreaturePartSlotKey::Head => CreaturePartSlot::Head,
        CreaturePartSlotKey::Torso => CreaturePartSlot::Torso,
        CreaturePartSlotKey::Arms => CreaturePartSlot::LeftArm,
        CreaturePartSlotKey::Legs => CreaturePartSlot::LeftLeg,
        CreaturePartSlotKey::Tail => CreaturePartSlot::TailBack,
    }
}

fn deterministic_index(seed: u64, salt: u64, len: usize) -> usize {
    let mut value = seed ^ salt.rotate_left(23);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    ((value ^ (value >> 31)) % len as u64) as usize
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use alife_world::{CreaturePartFamilyId, CreaturePartSources};

    use super::*;
    use crate::load_geneforge_creature_part_catalog;

    #[test]
    fn ordinary_mutation_never_violates_catalog_compatibility() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        let inherited = CreaturePartSources::coherent(CreaturePartFamilyId(0));
        for seed in 0..10_000 {
            let result = mutate_creature_part_sources(inherited, 3, seed, &catalog).unwrap();
            if !result.rare_cross_family {
                assert!(part_sources_are_ordinary_compatible(
                    &result.sources,
                    &catalog
                ));
            }
        }
    }

    #[test]
    fn rare_mutation_replaces_at_most_one_non_torso_slot() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        let inherited = CreaturePartSources::coherent(CreaturePartFamilyId(0));
        let result =
            mutate_creature_part_sources(inherited, RARE_PART_MUTATION_THRESHOLD, 0xfeed, &catalog)
                .unwrap();

        assert!(result.rare_cross_family);
        assert_eq!(result.incompatible_slot_count, 0);
        assert_eq!(result.sources.torso, inherited.torso);
        assert_ne!(result.changed_slot, Some(crate::CreaturePartSlot::Torso));
    }

    #[test]
    fn mutation_is_deterministic_for_identical_inputs() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        let inherited = CreaturePartSources::coherent(CreaturePartFamilyId(4));

        let left = mutate_creature_part_sources(inherited, 11, 44, &catalog).unwrap();
        let right = mutate_creature_part_sources(inherited, 11, 44, &catalog).unwrap();

        assert_eq!(left, right);
    }

    #[test]
    fn valid_ids_eight_through_eleven_are_accepted_with_every_torso_frame() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        for torso in 0..12 {
            for inherited in 8..=11 {
                let sources = CreaturePartSources {
                    head: CreaturePartFamilyId(inherited),
                    torso: CreaturePartFamilyId(torso),
                    arms: CreaturePartFamilyId(inherited),
                    legs: CreaturePartFamilyId(inherited),
                    tail: CreaturePartFamilyId(inherited),
                };
                assert!(part_sources_are_ordinary_compatible(&sources, &catalog));
                let result =
                    mutate_creature_part_sources(sources, 3, 0x8000 + inherited as u64, &catalog)
                        .unwrap();
                assert_eq!(result.sources.torso, sources.torso);
                assert_eq!(result.warning, None);
                assert_eq!(result.incompatible_slot_count, 0);
            }
        }
    }

    #[test]
    fn rare_mutation_can_select_every_high_family_id() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        for torso in 0..12 {
            let inherited = CreaturePartSources::coherent(CreaturePartFamilyId(torso));
            let mut selected = BTreeSet::new();
            for seed in 0..100_000 {
                let result = mutate_creature_part_sources(
                    inherited,
                    RARE_PART_MUTATION_THRESHOLD,
                    seed,
                    &catalog,
                )
                .unwrap();
                if result.rare_cross_family {
                    selected.extend(
                        result
                            .sources
                            .iter_slots()
                            .into_iter()
                            .map(|(_, family)| family.0)
                            .filter(|family| (8..=11).contains(family)),
                    );
                }
                if selected == BTreeSet::from([8, 9, 10, 11]) {
                    break;
                }
            }
            assert_eq!(selected, BTreeSet::from([8, 9, 10, 11]), "torso {torso}");
        }
    }

    #[test]
    fn unknown_attached_part_is_not_normalized_when_another_slot_mutates() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        let torso = CreaturePartFamilyId(4);
        let inherited = CreaturePartSources {
            head: CreaturePartFamilyId(999),
            torso,
            arms: torso,
            legs: torso,
            tail: torso,
        };

        let result = (0..10_000)
            .map(|seed| mutate_creature_part_sources(inherited, 3, seed, &catalog).unwrap())
            .find(|result| result.changed_slot != Some(CreaturePartSlot::Head))
            .unwrap();

        assert_eq!(result.sources.head, CreaturePartFamilyId(999));
        assert_eq!(result.warning, None);
    }

    #[test]
    fn unknown_torso_is_not_rewritten_by_mutation_preparation() {
        let catalog = load_geneforge_creature_part_catalog().unwrap();
        let inherited = CreaturePartSources::coherent(CreaturePartFamilyId(999));
        let result = mutate_creature_part_sources(inherited, 0, 1, &catalog).unwrap();

        assert_eq!(result.sources.torso, CreaturePartFamilyId(999));
        assert_eq!(result.warning, None);
    }
}
