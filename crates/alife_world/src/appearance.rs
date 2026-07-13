//! Renderer-neutral creature appearance genes for saved state and lineage.
//!
//! The production renderer may project these small integer genes into meshes,
//! colors, and markings, but this module stays Bevy- and renderer-free.

use alife_core::{GenomeId, OrganismId, ScaffoldContractError};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

pub const CREATURE_APPEARANCE_SCHEMA_VERSION: u16 = 2;
pub const CREATURE_APPEARANCE_SPECIES_COUNT: u8 = 16;
pub const CREATURE_APPEARANCE_GENE_BUCKETS: u8 = 16;
pub const INITIAL_CREATURE_PART_FAMILY_COUNT: u16 = 8;

const SPECIES_LABELS: [&str; CREATURE_APPEARANCE_SPECIES_COUNT as usize] = [
    "roundling-cave-kid",
    "longtail-scout",
    "broadpaw-digger",
    "wide-ear-nightling",
    "river-muzzle-gatherer",
    "peak-ear-climber",
    "sleepy-longarm",
    "mask-face-forager",
    "desert-ear-skipper",
    "tiny-tusk-rooter",
    "woolly-fluff",
    "shaggy-bearcat",
    "plume-tail-climber",
    "tundra-muzzle",
    "yak-fluffball",
    "bigfoot-hopper",
];

const BODY_PLAN_SIGNATURES: [&str; CREATURE_APPEARANCE_SPECIES_COUNT as usize] = [
    "round-head-short-limbs",
    "long-tail-slim-scout",
    "wide-paws-low-shoulders",
    "giant-ears-small-body",
    "sleek-torso-river-muzzle",
    "upright-climber-peak-ears",
    "long-arms-sleepy-hunch",
    "masked-face-grabber-hands",
    "tall-ears-spring-feet",
    "small-tusks-stocky-rooter",
    "wool-dome-short-feet",
    "shaggy-pear-body",
    "plume-tail-high-cheeks",
    "seal-muzzle-round-belly",
    "heavy-fluff-wide-stance",
    "hopper-feet-compact-torso",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CreaturePartFamilyId(pub u16);

impl CreaturePartFamilyId {
    pub const INVALID: Self = Self(u16::MAX);

    pub const fn validate(self) -> Result<(), ScaffoldContractError> {
        if self.0 == Self::INVALID.0 {
            Err(ScaffoldContractError::InvalidId)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreaturePartSlotKey {
    Head,
    Torso,
    Arms,
    Legs,
    Tail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreaturePartSources {
    pub head: CreaturePartFamilyId,
    pub torso: CreaturePartFamilyId,
    pub arms: CreaturePartFamilyId,
    pub legs: CreaturePartFamilyId,
    pub tail: CreaturePartFamilyId,
}

impl CreaturePartSources {
    pub const fn coherent(family: CreaturePartFamilyId) -> Self {
        Self {
            head: family,
            torso: family,
            arms: family,
            legs: family,
            tail: family,
        }
    }

    pub fn distinct_family_count(self) -> usize {
        self.iter_slots()
            .into_iter()
            .map(|(_, family)| family)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    }

    pub const fn iter_slots(self) -> [(CreaturePartSlotKey, CreaturePartFamilyId); 5] {
        [
            (CreaturePartSlotKey::Head, self.head),
            (CreaturePartSlotKey::Torso, self.torso),
            (CreaturePartSlotKey::Arms, self.arms),
            (CreaturePartSlotKey::Legs, self.legs),
            (CreaturePartSlotKey::Tail, self.tail),
        ]
    }

    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        for (_, family) in self.iter_slots() {
            family.validate()?;
        }
        Ok(())
    }

    pub fn inherited_from(self, parent_a: Self, parent_b: Self) -> bool {
        self.iter_slots()
            .into_iter()
            .zip(parent_a.iter_slots())
            .zip(parent_b.iter_slots())
            .all(|(((slot, child), (slot_a, a)), (slot_b, b))| {
                slot == slot_a && slot == slot_b && (child == a || child == b)
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CreatureAppearanceGenome {
    pub schema_version: u16,
    pub species_archetype: u8,
    pub part_sources: CreaturePartSources,
    pub palette_family: u8,
    pub fur_pattern: u8,
    pub marking_density: u8,
    pub accessory_trait: u8,
    pub ear_muzzle_trait: u8,
    pub tail_trait: u8,
    pub body_mass_trait: u8,
    pub mutation_count: u16,
    pub bipedal_caveman_furry: bool,
}

#[derive(Deserialize)]
struct CreatureAppearanceGenomeWire {
    schema_version: u16,
    species_archetype: u8,
    #[serde(default)]
    part_sources: Option<CreaturePartSources>,
    palette_family: u8,
    fur_pattern: u8,
    marking_density: u8,
    accessory_trait: u8,
    ear_muzzle_trait: u8,
    tail_trait: u8,
    body_mass_trait: u8,
    mutation_count: u16,
    bipedal_caveman_furry: bool,
}

impl<'de> Deserialize<'de> for CreatureAppearanceGenome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CreatureAppearanceGenomeWire::deserialize(deserializer)?;
        let part_sources = match wire.schema_version {
            1 => migrate_v1_part_sources(wire.species_archetype),
            CREATURE_APPEARANCE_SCHEMA_VERSION => wire
                .part_sources
                .ok_or_else(|| D::Error::missing_field("part_sources"))?,
            version => {
                return Err(D::Error::custom(format!(
                    "unsupported creature appearance schema version {version}"
                )));
            }
        };
        let appearance = Self {
            schema_version: CREATURE_APPEARANCE_SCHEMA_VERSION,
            species_archetype: wire.species_archetype,
            part_sources,
            palette_family: wire.palette_family,
            fur_pattern: wire.fur_pattern,
            marking_density: wire.marking_density,
            accessory_trait: wire.accessory_trait,
            ear_muzzle_trait: wire.ear_muzzle_trait,
            tail_trait: wire.tail_trait,
            body_mass_trait: wire.body_mass_trait,
            mutation_count: wire.mutation_count,
            bipedal_caveman_furry: wire.bipedal_caveman_furry,
        };
        appearance
            .validate()
            .map_err(|error| D::Error::custom(format!("invalid creature appearance: {error}")))?;
        Ok(appearance)
    }
}

pub const fn migrate_v1_part_sources(species_archetype: u8) -> CreaturePartSources {
    CreaturePartSources::coherent(CreaturePartFamilyId(
        (species_archetype as u16) % INITIAL_CREATURE_PART_FAMILY_COUNT,
    ))
}

impl Default for CreatureAppearanceGenome {
    fn default() -> Self {
        Self::founder_for_species(0, 0xA11F_0001)
    }
}

impl CreatureAppearanceGenome {
    pub fn founder_for_species(species_archetype: u8, seed: u64) -> Self {
        let species_archetype = species_archetype % CREATURE_APPEARANCE_SPECIES_COUNT;
        Self {
            schema_version: CREATURE_APPEARANCE_SCHEMA_VERSION,
            species_archetype,
            part_sources: migrate_v1_part_sources(species_archetype),
            palette_family: gene(seed, 0x70A1_0001),
            fur_pattern: gene(seed, 0x70A1_0002),
            marking_density: 5 + gene(seed, 0x70A1_0003) % 9,
            accessory_trait: gene(seed, 0x70A1_0004),
            ear_muzzle_trait: gene(seed, 0x70A1_0005),
            tail_trait: gene(seed, 0x70A1_0006),
            body_mass_trait: gene(seed, 0x70A1_0007),
            mutation_count: 0,
            bipedal_caveman_furry: true,
        }
    }

    pub fn from_ids(
        organism_id: OrganismId,
        genome_id: GenomeId,
        population_slot: usize,
        world_seed: u64,
    ) -> Self {
        let species = (population_slot % usize::from(CREATURE_APPEARANCE_SPECIES_COUNT)) as u8;
        let seed = mix_seed(
            world_seed,
            organism_id.raw(),
            genome_id.raw(),
            population_slot as u64,
        );
        Self::founder_for_species(species, seed)
    }

    pub fn offspring_from_parents(parent_a: Self, parent_b: Self, mutation_seed: u64) -> Self {
        let mut child = Self {
            schema_version: CREATURE_APPEARANCE_SCHEMA_VERSION,
            species_archetype: choose_gene(
                parent_a.species_archetype,
                parent_b.species_archetype,
                mutation_seed,
                0x0F1,
                CREATURE_APPEARANCE_SPECIES_COUNT,
                false,
            ),
            part_sources: CreaturePartSources {
                head: choose_family(
                    parent_a.part_sources.head,
                    parent_b.part_sources.head,
                    mutation_seed,
                    0x1F1,
                ),
                torso: choose_family(
                    parent_a.part_sources.torso,
                    parent_b.part_sources.torso,
                    mutation_seed,
                    0x1F2,
                ),
                arms: choose_family(
                    parent_a.part_sources.arms,
                    parent_b.part_sources.arms,
                    mutation_seed,
                    0x1F3,
                ),
                legs: choose_family(
                    parent_a.part_sources.legs,
                    parent_b.part_sources.legs,
                    mutation_seed,
                    0x1F4,
                ),
                tail: choose_family(
                    parent_a.part_sources.tail,
                    parent_b.part_sources.tail,
                    mutation_seed,
                    0x1F5,
                ),
            },
            palette_family: choose_gene(
                parent_a.palette_family,
                parent_b.palette_family,
                mutation_seed,
                0x0F2,
                CREATURE_APPEARANCE_GENE_BUCKETS,
                true,
            ),
            fur_pattern: choose_gene(
                parent_a.fur_pattern,
                parent_b.fur_pattern,
                mutation_seed,
                0x0F3,
                CREATURE_APPEARANCE_GENE_BUCKETS,
                true,
            ),
            marking_density: choose_gene(
                parent_a.marking_density,
                parent_b.marking_density,
                mutation_seed,
                0x0F4,
                CREATURE_APPEARANCE_GENE_BUCKETS,
                true,
            )
            .clamp(2, 15),
            accessory_trait: choose_gene(
                parent_a.accessory_trait,
                parent_b.accessory_trait,
                mutation_seed,
                0x0F5,
                CREATURE_APPEARANCE_GENE_BUCKETS,
                true,
            ),
            ear_muzzle_trait: choose_gene(
                parent_a.ear_muzzle_trait,
                parent_b.ear_muzzle_trait,
                mutation_seed,
                0x0F6,
                CREATURE_APPEARANCE_GENE_BUCKETS,
                true,
            ),
            tail_trait: choose_gene(
                parent_a.tail_trait,
                parent_b.tail_trait,
                mutation_seed,
                0x0F7,
                CREATURE_APPEARANCE_GENE_BUCKETS,
                true,
            ),
            body_mass_trait: choose_gene(
                parent_a.body_mass_trait,
                parent_b.body_mass_trait,
                mutation_seed,
                0x0F8,
                CREATURE_APPEARANCE_GENE_BUCKETS,
                true,
            ),
            mutation_count: parent_a
                .mutation_count
                .max(parent_b.mutation_count)
                .saturating_add(1),
            bipedal_caveman_furry: parent_a.bipedal_caveman_furry && parent_b.bipedal_caveman_furry,
        };

        let forced_delta =
            1 + (gene(mutation_seed, 0x0F9) % (CREATURE_APPEARANCE_GENE_BUCKETS - 1));
        child.fur_pattern = (child.fur_pattern + forced_delta) % CREATURE_APPEARANCE_GENE_BUCKETS;
        child
    }

    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != CREATURE_APPEARANCE_SCHEMA_VERSION
            || self.species_archetype >= CREATURE_APPEARANCE_SPECIES_COUNT
            || self.palette_family >= CREATURE_APPEARANCE_GENE_BUCKETS
            || self.fur_pattern >= CREATURE_APPEARANCE_GENE_BUCKETS
            || self.marking_density >= CREATURE_APPEARANCE_GENE_BUCKETS
            || self.accessory_trait >= CREATURE_APPEARANCE_GENE_BUCKETS
            || self.ear_muzzle_trait >= CREATURE_APPEARANCE_GENE_BUCKETS
            || self.tail_trait >= CREATURE_APPEARANCE_GENE_BUCKETS
            || self.body_mass_trait >= CREATURE_APPEARANCE_GENE_BUCKETS
            || !self.bipedal_caveman_furry
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.part_sources.validate()?;
        Ok(())
    }

    pub fn inherited_from(self, parent_a: Self, parent_b: Self) -> bool {
        let species_inherited = self.species_archetype == parent_a.species_archetype
            || self.species_archetype == parent_b.species_archetype;
        let palette_inherited_or_mutated = self.palette_family < CREATURE_APPEARANCE_GENE_BUCKETS;
        species_inherited
            && palette_inherited_or_mutated
            && self
                .part_sources
                .inherited_from(parent_a.part_sources, parent_b.part_sources)
            && self.mutation_count > parent_a.mutation_count.max(parent_b.mutation_count)
    }

    pub fn species_label(self) -> &'static str {
        SPECIES_LABELS[self.species_archetype as usize]
    }

    pub fn body_plan_signature(self) -> &'static str {
        BODY_PLAN_SIGNATURES[self.species_archetype as usize]
    }

    pub fn signature_line(self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.species_archetype,
            self.part_sources.head.0,
            self.part_sources.torso.0,
            self.part_sources.arms.0,
            self.part_sources.legs.0,
            self.part_sources.tail.0,
            self.palette_family,
            self.fur_pattern,
            self.marking_density,
            self.accessory_trait,
            self.ear_muzzle_trait,
            self.tail_trait,
            self.body_mass_trait,
            self.mutation_count,
            self.bipedal_caveman_furry
        )
    }
}

fn choose_family(
    parent_a: CreaturePartFamilyId,
    parent_b: CreaturePartFamilyId,
    seed: u64,
    salt: u64,
) -> CreaturePartFamilyId {
    if mix_seed(seed, salt, u64::from(parent_a.0), u64::from(parent_b.0)) & 1 == 0 {
        parent_a
    } else {
        parent_b
    }
}

fn choose_gene(parent_a: u8, parent_b: u8, seed: u64, salt: u64, buckets: u8, mutate: bool) -> u8 {
    let mixed = mix_seed(seed, salt, u64::from(parent_a), u64::from(parent_b));
    let inherited = if mixed & 1 == 0 { parent_a } else { parent_b } % buckets;
    if mutate && (mixed >> 3) & 0x3 == 0 {
        (inherited + 1 + ((mixed >> 11) as u8 % (buckets - 1))) % buckets
    } else {
        inherited
    }
}

fn gene(seed: u64, salt: u64) -> u8 {
    (mix_seed(seed, salt, 0xA11F_705E, 0xC0DE_F00D) % u64::from(CREATURE_APPEARANCE_GENE_BUCKETS))
        as u8
}

fn mix_seed(a: u64, b: u64, c: u64, d: u64) -> u64 {
    let mut x = a ^ b.rotate_left(17) ^ c.rotate_left(31) ^ d.rotate_left(47);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_v1_appearance_migrates_to_coherent_schema_v2_parts() {
        let json = r#"{
            "schema_version":1,"species_archetype":5,"palette_family":3,
            "fur_pattern":4,"marking_density":8,"accessory_trait":2,
            "ear_muzzle_trait":6,"tail_trait":7,"body_mass_trait":9,
            "mutation_count":0,"bipedal_caveman_furry":true
        }"#;

        let appearance: CreatureAppearanceGenome = serde_json::from_str(json).unwrap();

        assert_eq!(
            appearance.schema_version,
            CREATURE_APPEARANCE_SCHEMA_VERSION
        );
        assert_eq!(
            appearance.part_sources,
            CreaturePartSources::coherent(CreaturePartFamilyId(5))
        );
        assert_eq!(appearance.palette_family, 3);
    }

    #[test]
    fn schema_v2_mixed_parts_roundtrip_without_renderer_types() {
        let mut appearance = CreatureAppearanceGenome::founder_for_species(2, 99);
        appearance.part_sources = CreaturePartSources {
            head: CreaturePartFamilyId(2),
            torso: CreaturePartFamilyId(2),
            arms: CreaturePartFamilyId(6),
            legs: CreaturePartFamilyId(2),
            tail: CreaturePartFamilyId(7),
        };

        let json = serde_json::to_string(&appearance).unwrap();
        let roundtrip: CreatureAppearanceGenome = serde_json::from_str(&json).unwrap();

        assert_eq!(roundtrip, appearance);
        assert!(!json.to_ascii_lowercase().contains("bevy"));
        assert!(!json.to_ascii_lowercase().contains("mesh"));
    }
}
