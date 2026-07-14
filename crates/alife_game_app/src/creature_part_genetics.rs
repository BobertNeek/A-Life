use alife_world::{CreaturePartFamilyId, CreaturePartSlotKey, CreaturePartSources};

use crate::{CreaturePartCatalog, CreaturePartCatalogError, CreaturePartSlot};

pub const RARE_PART_MUTATION_THRESHOLD: u16 = 8;

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
    catalog: &CreaturePartCatalog,
) -> Result<CreaturePartMutationResult, CreaturePartCatalogError> {
    let (normalized, warning) = normalize_sources(inherited, mutation_seed, catalog)?;
    let torso = normalized.torso;

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
    let current = family_for_slot(normalized, slot);
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
        (normalized, None)
    } else {
        let candidate = candidates[deterministic_index(mutation_seed, 0xFA11, candidates.len())];
        (
            with_family_for_slot(normalized, slot, candidate),
            Some(runtime_slot(slot)),
        )
    };
    let incompatible_slot_count = incompatible_slot_count(&sources, catalog);
    Ok(CreaturePartMutationResult {
        sources,
        changed_slot,
        rare_cross_family,
        incompatible_slot_count,
        warning,
    })
}

fn normalize_sources(
    inherited: CreaturePartSources,
    mutation_seed: u64,
    catalog: &CreaturePartCatalog,
) -> Result<(CreaturePartSources, Option<CreaturePartMutationWarning>), CreaturePartCatalogError> {
    let mut normalized = inherited;
    let mut warning = None;
    if catalog.family(normalized.torso).is_none() {
        let fallback = catalog
            .families
            .iter()
            .min_by_key(|family| family.id)
            .map(|family| family.id)
            .ok_or(CreaturePartCatalogError::Empty)?;
        normalized = CreaturePartSources::coherent(fallback);
        warning = Some(CreaturePartMutationWarning::UnknownFamilyFallback {
            requested: inherited.torso,
            fallback,
        });
    } else {
        for slot in [
            CreaturePartSlotKey::Head,
            CreaturePartSlotKey::Arms,
            CreaturePartSlotKey::Legs,
            CreaturePartSlotKey::Tail,
        ] {
            let current = family_for_slot(normalized, slot);
            if catalog.family(current).is_some() {
                continue;
            }
            normalized = with_family_for_slot(normalized, slot, normalized.torso);
            if warning.is_none() {
                warning = Some(CreaturePartMutationWarning::UnknownFamilyFallback {
                    requested: current,
                    fallback: normalized.torso,
                });
            }
        }
    }
    Ok((
        normalize_ordinary_compatibility(normalized, mutation_seed, catalog)?,
        warning,
    ))
}

fn normalize_ordinary_compatibility(
    inherited: CreaturePartSources,
    mutation_seed: u64,
    catalog: &CreaturePartCatalog,
) -> Result<CreaturePartSources, CreaturePartCatalogError> {
    let mut normalized = inherited;
    for (index, slot) in [
        CreaturePartSlotKey::Head,
        CreaturePartSlotKey::Arms,
        CreaturePartSlotKey::Legs,
        CreaturePartSlotKey::Tail,
    ]
    .into_iter()
    .enumerate()
    {
        let current = family_for_slot(normalized, slot);
        if slot_is_ordinary_compatible(normalized.torso, slot, current, catalog) {
            continue;
        }
        let mut candidates = catalog
            .families
            .iter()
            .map(|family| family.id)
            .filter(|candidate| {
                slot_is_ordinary_compatible(normalized.torso, slot, *candidate, catalog)
            })
            .collect::<Vec<_>>();
        candidates.sort_unstable();
        if candidates.is_empty() {
            return Err(CreaturePartCatalogError::NoCompatibleFamily {
                torso: normalized.torso,
                slot: runtime_slot(slot),
            });
        }
        let salt = 0xC011_u64.wrapping_add(index as u64);
        let candidate = candidates[deterministic_index(mutation_seed, salt, candidates.len())];
        normalized = with_family_for_slot(normalized, slot, candidate);
    }
    Ok(normalized)
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

    #[test]
    fn unknown_attached_part_falls_back_to_saved_torso_before_mutation() {
        let catalog = load_production_creature_part_catalog().unwrap();
        let torso = CreaturePartFamilyId(4);
        let inherited = CreaturePartSources {
            head: CreaturePartFamilyId(999),
            torso,
            arms: torso,
            legs: torso,
            tail: torso,
        };

        let result = mutate_creature_part_sources(inherited, 0, 1, &catalog).unwrap();

        assert_eq!(result.sources.head, torso);
        assert_eq!(
            result.warning,
            Some(CreaturePartMutationWarning::UnknownFamilyFallback {
                requested: CreaturePartFamilyId(999),
                fallback: torso,
            })
        );
    }

    #[test]
    fn unknown_torso_falls_back_to_catalog_minimum_before_mutation() {
        let catalog = load_production_creature_part_catalog().unwrap();
        let result = mutate_creature_part_sources(
            CreaturePartSources::coherent(CreaturePartFamilyId(999)),
            0,
            1,
            &catalog,
        )
        .unwrap();

        assert_eq!(result.sources.torso, CreaturePartFamilyId(0));
        assert_eq!(
            result.warning,
            Some(CreaturePartMutationWarning::UnknownFamilyFallback {
                requested: CreaturePartFamilyId(999),
                fallback: CreaturePartFamilyId(0),
            })
        );
    }
}
