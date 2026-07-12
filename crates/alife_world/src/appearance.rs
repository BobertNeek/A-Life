//! Renderer-neutral creature appearance genes for saved state and lineage.
//!
//! The production renderer may project these small integer genes into meshes,
//! colors, and markings, but this module stays Bevy- and renderer-free.

use alife_core::{GenomeId, OrganismId, ScaffoldContractError};
use serde::{Deserialize, Serialize};

pub const CREATURE_APPEARANCE_SCHEMA_VERSION: u16 = 1;
pub const CREATURE_APPEARANCE_SPECIES_COUNT: u8 = 16;
pub const CREATURE_APPEARANCE_GENE_BUCKETS: u8 = 16;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatureAppearanceGenome {
    pub schema_version: u16,
    pub species_archetype: u8,
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
        Ok(())
    }

    pub fn inherited_from(self, parent_a: Self, parent_b: Self) -> bool {
        let species_inherited = self.species_archetype == parent_a.species_archetype
            || self.species_archetype == parent_b.species_archetype;
        let palette_inherited_or_mutated = self.palette_family < CREATURE_APPEARANCE_GENE_BUCKETS;
        species_inherited
            && palette_inherited_or_mutated
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
            "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.species_archetype,
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
