use alife_world::{CreaturePartFamilyId, CreaturePartSlotKey, CreaturePartSources};

use crate::{CreaturePartCatalog, CreaturePartCatalogError, CreaturePartSlot};

pub const RARE_PART_MUTATION_THRESHOLD: u16 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreaturePartMutationResult {
    pub sources: CreaturePartSources,
    pub changed_slot: Option<CreaturePartSlot>,
    pub rare_cross_family: bool,
    pub incompatible_slot_count: u8,
}

pub fn mutate_creature_part_sources(
    inherited: CreaturePartSources,
    mutation_count: u16,
    mutation_seed: u64,
    catalog: &CreaturePartCatalog,
) -> Result<CreaturePartMutationResult, CreaturePartCatalogError> {
    let torso = inherited.torso;
    if catalog.family(torso).is_none() {
        return Err(CreaturePartCatalogError::UnknownFamily(torso));
    }

    let rare_cross_family = mutation_count >= RARE_PART_MUTATION_THRESHOLD
        && ((mutation_seed ^ u64::from(mutation_count)) & 0x7) == 0x5;
    let slots = if rare_cross_family {
        [
            CreaturePartSlotKey::Head,
            CreaturePartSlotKey::Arms,
            CreaturePartSlotKey::Legs,
            CreaturePartSlotKey::Tail,
        ]
        .as_slice()
    } else {
        [
            CreaturePartSlotKey::Head,
            CreaturePartSlotKey::Arms,
            CreaturePartSlotKey::Legs,
            CreaturePartSlotKey::Tail,
        ]
        .as_slice()
    };
    let slot = slots[deterministic_index(mutation_seed, 0x51A7, slots.len())];
    let current = family_for_slot(inherited, slot);
    let mut candidates = catalog
        .families
        .iter()
        .map(|family| family.id)
        .filter(|candidate| *candidate != current)
        .filter(|candidate| {
            if rare_cross_family {
                !slot_is_ordinary_compatible(torso, slot, *candidate, catalog)
            } else {
                slot_is_ordinary_compatible(torso, slot, *candidate, catalog)
            }
        })
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
    })
}

pub fn part_sources_are_ordinary_compatible(
    sources: &CreaturePartSources,
    catalog: &CreaturePartCatalog,
) -> bool {
    incompatible_slot_count(sources, catalog) == 0
}

fn incompatible_slot_count(sources: &CreaturePartSources, catalog: &CreaturePartCatalog) -> u8 {
    sources
        .iter_slots()
        .into_iter()
        .filter(|(slot, family)| {
            !slot_is_ordinary_compatible(sources.torso, *slot, *family, catalog)
        })
        .count() as u8
}

fn slot_is_ordinary_compatible(
    torso: CreaturePartFamilyId,
    slot: CreaturePartSlotKey,
    candidate: CreaturePartFamilyId,
    catalog: &CreaturePartCatalog,
) -> bool {
    match slot {
        CreaturePartSlotKey::Head => {
            catalog.ordinarily_compatible(torso, CreaturePartSlot::Head, candidate)
        }
        CreaturePartSlotKey::Torso => {
            catalog.ordinarily_compatible(torso, CreaturePartSlot::Torso, candidate)
        }
        CreaturePartSlotKey::Arms => {
            catalog.ordinarily_compatible(torso, CreaturePartSlot::LeftArm, candidate)
                && catalog.ordinarily_compatible(torso, CreaturePartSlot::RightArm, candidate)
        }
        CreaturePartSlotKey::Legs => {
            catalog.ordinarily_compatible(torso, CreaturePartSlot::LeftLeg, candidate)
                && catalog.ordinarily_compatible(torso, CreaturePartSlot::RightLeg, candidate)
        }
        CreaturePartSlotKey::Tail => {
            catalog.ordinarily_compatible(torso, CreaturePartSlot::TailBack, candidate)
        }
    }
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
    use alife_world::{CreaturePartFamilyId, CreaturePartSources};

    use super::*;
    use crate::load_production_creature_part_catalog;

    #[test]
    fn ordinary_mutation_never_violates_catalog_compatibility() {
        let catalog = load_production_creature_part_catalog().unwrap();
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
    fn rare_mutation_replaces_at_most_one_incompatible_non_torso_slot() {
        let catalog = load_production_creature_part_catalog().unwrap();
        let inherited = CreaturePartSources::coherent(CreaturePartFamilyId(0));
        let result =
            mutate_creature_part_sources(inherited, RARE_PART_MUTATION_THRESHOLD, 0xfeed, &catalog)
                .unwrap();

        assert!(result.rare_cross_family);
        assert!(result.incompatible_slot_count <= 1);
        assert_eq!(result.sources.torso, inherited.torso);
        assert_ne!(result.changed_slot, Some(crate::CreaturePartSlot::Torso));
    }

    #[test]
    fn mutation_is_deterministic_for_identical_inputs() {
        let catalog = load_production_creature_part_catalog().unwrap();
        let inherited = CreaturePartSources::coherent(CreaturePartFamilyId(4));

        let left = mutate_creature_part_sources(inherited, 11, 44, &catalog).unwrap();
        let right = mutate_creature_part_sources(inherited, 11, 44, &catalog).unwrap();

        assert_eq!(left, right);
    }
}
