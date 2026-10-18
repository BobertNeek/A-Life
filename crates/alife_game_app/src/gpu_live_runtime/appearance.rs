//! Small cosmetic state shared by live presentation and checkpoint summaries.
//!
//! This cache owns no genotype or learned state. Keep retired parents until new
//! admissions have inherited their saved cosmetics, then retain the live set.

use std::collections::{BTreeMap, BTreeSet};

use alife_core::{GenomeId, OrganismId, ScaffoldContractError};
use alife_world::{persistence::CreatureSaveState, CreatureAppearanceGenome, WorldOrganismRecord};

#[derive(Clone, Copy, PartialEq, Eq)]
struct CachedAppearance {
    genome_id: GenomeId,
    appearance: CreatureAppearanceGenome,
}

#[derive(Default)]
pub(super) struct CreatureAppearanceCache {
    entries: BTreeMap<u64, CachedAppearance>,
}

impl CreatureAppearanceCache {
    /// Install externally validated save values exactly. A matching identity may
    /// replace cosmetics generated before the raw runtime restore was seeded.
    pub(super) fn seed_saved(
        &mut self,
        creatures: &[CreatureSaveState],
    ) -> Result<(), ScaffoldContractError> {
        let mut seeded = BTreeMap::new();
        for creature in creatures {
            let raw = creature.organism_id.raw();
            let entry = CachedAppearance {
                genome_id: creature.genome_id,
                appearance: creature.appearance,
            };
            if self
                .entries
                .get(&raw)
                .is_some_and(|existing| existing.genome_id != creature.genome_id)
                || seeded
                    .insert(raw, entry)
                    .is_some_and(|existing| existing != entry)
            {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
        }
        // Validate the entire batch before replacing any current value.
        self.entries.extend(seeded);
        Ok(())
    }

    pub(super) fn get(
        &self,
        organism_id: OrganismId,
        genome_id: GenomeId,
    ) -> Option<CreatureAppearanceGenome> {
        self.entries
            .get(&organism_id.raw())
            .filter(|entry| entry.genome_id == genome_id)
            .map(|entry| entry.appearance)
    }

    /// Derive cosmetics once, using the existing checkpoint inheritance rules.
    pub(super) fn admit(
        &mut self,
        record: &WorldOrganismRecord,
    ) -> Result<(), ScaffoldContractError> {
        let genome = record.genome();
        let raw = record.organism_id().raw();
        if let Some(existing) = self.entries.get(&raw) {
            return if existing.genome_id == genome.id {
                Ok(())
            } else {
                Err(ScaffoldContractError::BrainOwnershipMismatch)
            };
        }
        let parent_appearance = |parent: Option<&GenomeId>| {
            parent.and_then(|genome_id| {
                // Genome IDs can be shared by cloned individuals. Existing
                // checkpoint lookup picks the first match; use stable organism
                // order so save vector/admission order cannot change that match.
                self.entries
                    .values()
                    .find(|entry| entry.genome_id == *genome_id)
                    .map(|entry| entry.appearance)
            })
        };
        let appearance = match (
            parent_appearance(genome.parent_genome_ids.first()),
            parent_appearance(genome.parent_genome_ids.get(1)),
        ) {
            (Some(parent_a), Some(parent_b)) => CreatureAppearanceGenome::offspring_from_parents(
                parent_a,
                parent_b,
                genome.conception_seed,
            ),
            _ => CreatureAppearanceGenome::default(),
        }
        .with_body_phenotype(&record.phenotype().body);
        self.entries.insert(
            raw,
            CachedAppearance {
                genome_id: genome.id,
                appearance,
            },
        );
        Ok(())
    }

    pub(super) fn retain_organisms(&mut self, live_ids: &BTreeSet<u64>) {
        self.entries.retain(|raw, _| live_ids.contains(raw));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{
        BrainCapacityClass, BrainScaleTier, CreatureGenome, FoundationGeneticIdentity,
        HomeostaticSnapshot, Tick, WorldEntityId,
    };
    use alife_world::persistence::{
        CreatureMindSaveSummary, LearningTraceSaveSummary, WeightLayerSaveSummary,
    };

    fn record(raw: u64, genome: CreatureGenome) -> WorldOrganismRecord {
        let phenotype = genome.express().unwrap();
        WorldOrganismRecord::newborn(
            OrganismId(raw),
            WorldEntityId(raw + 100),
            genome,
            phenotype,
            Tick::ZERO,
        )
        .unwrap()
    }

    fn founder(raw: u64) -> WorldOrganismRecord {
        record(
            raw,
            CreatureGenome::early_mammal_founder(
                0xE10_3100 + raw,
                FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
            )
            .unwrap(),
        )
    }

    fn child(
        raw: u64,
        parent_a: &WorldOrganismRecord,
        parent_b: &WorldOrganismRecord,
    ) -> WorldOrganismRecord {
        record(
            raw,
            CreatureGenome::reproduce(parent_a.genome(), parent_b.genome(), raw + 1_000).unwrap(),
        )
    }

    fn saved(record: &WorldOrganismRecord, seed: u64) -> CreatureSaveState {
        CreatureSaveState {
            organism_id: record.organism_id(),
            genome_id: record.genome().id,
            brain_class: BrainScaleTier::Nano512,
            development_tick: Tick(3),
            appearance: CreatureAppearanceGenome::founder_for_species(2, seed),
            mind: CreatureMindSaveSummary {
                tick: Tick(3),
                homeostasis: HomeostaticSnapshot::baseline(Tick(3)),
                memory_record_count: 2,
                memory_source_ids: Vec::new(),
                concept_count: 1,
                edge_count: 0,
                simplex_count: 0,
                unresolved_gap_count: 0,
                sleep_state_label: "awake".to_string(),
                diagnostics: vec!["retained learning sentinel".to_string()],
            },
            weights: WeightLayerSaveSummary {
                generated_weight_asset_id: None,
                genetic_fixed_digest: "fnv1a64:0000000000000001".to_string(),
                genetic_layer_mutable: false,
                lifetime_consolidated_entries: 3,
                h_operational_entries: 1,
                h_shadow_entries: 1,
            },
            learning: LearningTraceSaveSummary {
                lifetime_learning_enabled: true,
                lamarckian_mode_enabled: false,
                last_consolidated_tick: Some(Tick(2)),
            },
            composite_genetics: None,
            lifetime_state_asset: None,
            gpu_brain: None,
        }
    }

    fn appearance(
        cache: &CreatureAppearanceCache,
        record: &WorldOrganismRecord,
    ) -> CreatureAppearanceGenome {
        cache.get(record.organism_id(), record.genome().id).unwrap()
    }

    #[test]
    fn saved_values_replace_generated_defaults_exactly_without_mutating_sources() {
        let record = founder(1);
        let mut save = saved(&record, 17);
        let projected = save
            .appearance
            .with_body_phenotype(&record.phenotype().body);
        save.appearance.palette_family = (projected.palette_family + 1) % 16;
        save.appearance.body_mass_trait = (projected.body_mass_trait + 1) % 16;
        let before_save = serde_json::to_vec(&save).unwrap();
        let before_record = serde_json::to_vec(&record).unwrap();
        let mut cache = CreatureAppearanceCache::default();
        cache.admit(&record).unwrap();
        assert_ne!(appearance(&cache, &record), save.appearance);

        cache.seed_saved(std::slice::from_ref(&save)).unwrap();
        cache.admit(&record).unwrap();

        assert_eq!(appearance(&cache, &record), save.appearance);
        assert_eq!(serde_json::to_vec(&save).unwrap(), before_save);
        assert_eq!(serde_json::to_vec(&record).unwrap(), before_record);
    }

    #[test]
    fn file_reload_keeps_exact_appearance_and_all_saved_learning_summaries() {
        let parent_a = founder(1);
        let parent_b = founder(2);
        let child = child(3, &parent_a, &parent_b);
        let records = [&parent_a, &parent_b, &child];
        let before_records = serde_json::to_vec(&records).unwrap();
        let mut saves = vec![saved(&parent_a, 10), saved(&parent_b, 20)];
        let mut cache = CreatureAppearanceCache::default();
        cache.seed_saved(&saves).unwrap();
        cache.admit(&child).unwrap();
        let mut child_save = saved(&child, 30);
        child_save.appearance = appearance(&cache, &child);
        saves.push(child_save);
        let before = serde_json::to_vec(&saves).unwrap();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "alife-appearance-learning-{}-{nonce}.json",
            std::process::id()
        ));
        std::fs::write(&path, &before).unwrap();
        let reopened_bytes = std::fs::read(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        let reopened: Vec<CreatureSaveState> = serde_json::from_slice(&reopened_bytes).unwrap();
        let mut restored_cache = CreatureAppearanceCache::default();
        restored_cache.seed_saved(&reopened).unwrap();
        for (record, expected) in records.into_iter().zip(&saves) {
            restored_cache.admit(record).unwrap();
            assert_eq!(appearance(&restored_cache, record), expected.appearance);
        }

        // These are CPU save summaries, not a GPU acquired-weight capture.
        assert_eq!(reopened[2].mind.memory_record_count, 2);
        assert_eq!(reopened[2].weights.lifetime_consolidated_entries, 3);
        assert_eq!(reopened[2].learning.last_consolidated_tick, Some(Tick(2)));
        assert_eq!(serde_json::to_vec(&reopened).unwrap(), before);
        assert_eq!(serde_json::to_vec(&records).unwrap(), before_records);
    }

    #[test]
    fn identity_rebound_is_rejected_and_saved_batch_failure_is_atomic() {
        let original = founder(1);
        let replacement = record(1, founder(2).genome().clone());
        let other = founder(3);
        let save = saved(&original, 11);
        let mut cache = CreatureAppearanceCache::default();
        cache.seed_saved(std::slice::from_ref(&save)).unwrap();

        assert_eq!(
            cache.get(original.organism_id(), replacement.genome().id),
            None
        );
        assert!(matches!(
            cache.admit(&replacement),
            Err(ScaffoldContractError::BrainOwnershipMismatch)
        ));
        assert!(matches!(
            cache.seed_saved(&[saved(&other, 13), saved(&replacement, 12)]),
            Err(ScaffoldContractError::BrainOwnershipMismatch)
        ));
        assert_eq!(appearance(&cache, &original), save.appearance);
        assert_eq!(cache.get(other.organism_id(), other.genome().id), None);
    }

    #[test]
    fn conflicting_duplicate_saved_values_do_not_depend_on_vector_order() {
        let record = founder(1);
        let first = saved(&record, 20);
        let mut second = first.clone();
        second.appearance.tail_trait = (first.appearance.tail_trait + 1) % 16;
        for saves in [[first.clone(), second.clone()], [second, first]] {
            let mut cache = CreatureAppearanceCache::default();
            assert!(matches!(
                cache.seed_saved(&saves),
                Err(ScaffoldContractError::BrainOwnershipMismatch)
            ));
            assert!(cache.entries.is_empty());
        }
    }

    #[test]
    fn offspring_survive_parent_retirement_and_pass_cosmetics_to_the_next_generation() {
        let parent_a = founder(1);
        let parent_b = founder(2);
        let mate = founder(3);
        let first_child = child(4, &parent_a, &parent_b);
        let grandchild = child(5, &first_child, &mate);
        let parents = [saved(&parent_a, 10), saved(&parent_b, 20), saved(&mate, 30)];
        let before = serde_json::to_vec(&parents).unwrap();
        let before_child = serde_json::to_vec(&first_child).unwrap();
        let before_grandchild = serde_json::to_vec(&grandchild).unwrap();
        let expected_child = CreatureAppearanceGenome::offspring_from_parents(
            parents[0].appearance,
            parents[1].appearance,
            first_child.genome().conception_seed,
        )
        .with_body_phenotype(&first_child.phenotype().body);
        let expected_grandchild = CreatureAppearanceGenome::offspring_from_parents(
            expected_child,
            parents[2].appearance,
            grandchild.genome().conception_seed,
        )
        .with_body_phenotype(&grandchild.phenotype().body);
        let mut cache = CreatureAppearanceCache::default();
        cache.seed_saved(&parents).unwrap();
        cache.admit(&first_child).unwrap();
        cache.retain_organisms(&BTreeSet::from([3, 4]));
        cache.admit(&first_child).unwrap();
        assert_eq!(appearance(&cache, &first_child), expected_child);
        assert_eq!(cache.entries.len(), 2);

        cache.admit(&grandchild).unwrap();
        cache.retain_organisms(&BTreeSet::from([5]));
        assert_eq!(appearance(&cache, &grandchild), expected_grandchild);
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(serde_json::to_vec(&parents).unwrap(), before);
        assert_eq!(serde_json::to_vec(&first_child).unwrap(), before_child);
        assert_eq!(serde_json::to_vec(&grandchild).unwrap(), before_grandchild);
    }

    #[test]
    fn known_parents_produce_the_same_offspring_in_any_sibling_admission_order() {
        let parent_a = founder(1);
        let parent_b = founder(2);
        let children = [
            child(3, &parent_a, &parent_b),
            child(4, &parent_a, &parent_b),
        ];
        let parents = [saved(&parent_a, 10), saved(&parent_b, 20)];
        let mut forward = CreatureAppearanceCache::default();
        let mut reverse = CreatureAppearanceCache::default();
        forward.seed_saved(&parents).unwrap();
        reverse
            .seed_saved(&[parents[1].clone(), parents[0].clone()])
            .unwrap();
        for child in &children {
            forward.admit(child).unwrap();
        }
        for child in children.iter().rev() {
            reverse.admit(child).unwrap();
        }
        for child in children {
            assert_eq!(appearance(&forward, &child), appearance(&reverse, &child));
        }
    }

    #[test]
    fn cloned_parent_genome_lookup_uses_stable_organism_order() {
        let parent_a = founder(1);
        let parent_b = record(2, parent_a.genome().clone());
        let child = child(3, &parent_a, &parent_b);
        let parents = [saved(&parent_a, 10), saved(&parent_b, 20)];
        let expected = CreatureAppearanceGenome::offspring_from_parents(
            parents[0].appearance,
            parents[0].appearance,
            child.genome().conception_seed,
        )
        .with_body_phenotype(&child.phenotype().body);
        let mut cache = CreatureAppearanceCache::default();
        cache
            .seed_saved(&[parents[1].clone(), parents[0].clone()])
            .unwrap();
        cache.admit(&child).unwrap();
        assert_eq!(appearance(&cache, &child), expected);
    }

    #[test]
    fn missing_parent_keeps_the_existing_default_fallback_and_is_not_rederived() {
        let parent_a = founder(1);
        let parent_b = founder(2);
        let child = child(3, &parent_a, &parent_b);
        let mut cache = CreatureAppearanceCache::default();
        cache.seed_saved(&[saved(&parent_a, 10)]).unwrap();
        cache.admit(&child).unwrap();
        let expected =
            CreatureAppearanceGenome::default().with_body_phenotype(&child.phenotype().body);
        assert_eq!(appearance(&cache, &child), expected);

        cache.seed_saved(&[saved(&parent_b, 20)]).unwrap();
        cache.admit(&child).unwrap();
        assert_eq!(appearance(&cache, &child), expected);
    }
}
