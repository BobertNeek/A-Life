//! Bounded, deterministic reproduction receipts for the Era 1 evolution program.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use alife_archive::{CompositeGeneticArchiveInput, LineageLibrary, LineageLibraryConfig};
use alife_core::{
    BrainCapacityClass, BrainScaleTier, CreatureGenome, Era1Control, FoundationWeightAsset,
    GenomeId, HomeostaticSnapshot, LanguageTokenId, OrganismId, PhenotypeCompiler, PolicyBackend,
    ScaffoldContractError, SensorProfile, Tick, Validate,
};
use alife_world::{
    persist_composite_genetic_birth_assets, AssetManifest, CreatureAppearanceGenome,
    CreatureMindSaveSummary, CreatureSaveState, Habitat, HabitatActor, HabitatAuthority,
    HabitatBreedingKind, HabitatBreedingReceipt, HabitatBreedingRequest, HabitatId, HabitatMode,
    HeadlessScenarioBuilder, LearningTraceSaveSummary, PortableSaveFile, RuntimeConfig,
    WeightLayerSaveSummary, P34_ASSET_MANIFEST_SCHEMA, P34_ASSET_MANIFEST_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::p33_evaluation::{ObjectiveVector, ScoreEstimate};
use crate::p33_selection::{
    run_managed_selection, ManagedBreedingPlan, ManagedSelectionConfig, PopulationLane,
    SelectionCandidate, SpecialistRole,
};

pub const ERA1_EVOLUTION_SCHEMA_VERSION: u16 = 1;
const BOUNDED_LINEAGES: usize = 4;
const BOUNDED_EVALUATION_SEEDS: usize = 3;
const BOUNDED_HELD_OUT_TRANSFORMS: usize = 2;
const BOUNDED_ORDINARY_GENERATIONS: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1EvolutionConfig {
    pub schema_version: u16,
    pub evolution_seed: u64,
    pub lineage_count: usize,
    pub evaluation_seeds: Vec<u64>,
    pub held_out_world_transforms: Vec<u64>,
    pub controls: Vec<Era1Control>,
    pub ordinary_birth_generations: u32,
}

impl Era1EvolutionConfig {
    pub fn bounded_default(evolution_seed: u64) -> Result<Self, Era1EvolutionError> {
        if evolution_seed == 0 {
            return Err(Era1EvolutionError::InvalidConfig("evolution seed is zero"));
        }
        let config = Self {
            schema_version: ERA1_EVOLUTION_SCHEMA_VERSION,
            evolution_seed,
            lineage_count: BOUNDED_LINEAGES,
            evaluation_seeds: (0..BOUNDED_EVALUATION_SEEDS)
                .map(|index| derived_seed(evolution_seed, 0xE1A1_0000, index as u64))
                .collect(),
            held_out_world_transforms: (0..BOUNDED_HELD_OUT_TRANSFORMS)
                .map(|index| derived_seed(evolution_seed, 0xE1A1_1000, index as u64))
                .collect(),
            controls: Era1Control::ALL.to_vec(),
            ordinary_birth_generations: BOUNDED_ORDINARY_GENERATIONS,
        };
        config.validate_contract()?;
        Ok(config)
    }

    pub fn validate_contract(&self) -> Result<(), Era1EvolutionError> {
        if self.schema_version != ERA1_EVOLUTION_SCHEMA_VERSION
            || self.evolution_seed == 0
            || self.lineage_count != BOUNDED_LINEAGES
            || self.evaluation_seeds.len() != BOUNDED_EVALUATION_SEEDS
            || self.held_out_world_transforms.len() != BOUNDED_HELD_OUT_TRANSFORMS
            || self.controls != Era1Control::ALL
            || self.ordinary_birth_generations != BOUNDED_ORDINARY_GENERATIONS
            || !all_unique_nonzero(&self.evaluation_seeds)
            || !all_unique_nonzero(&self.held_out_world_transforms)
        {
            return Err(Era1EvolutionError::InvalidConfig(
                "bounded Era 1 matrix changed",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1AcquiredStateEvidence {
    pub lifetime_weight_digest: Option<[u64; 4]>,
    pub memory_digests: Vec<[u64; 4]>,
    pub learned_vocabulary: Vec<LanguageTokenId>,
    pub pending_eligibility_digest: Option<[u64; 4]>,
    pub transient_state_digest: Option<[u64; 4]>,
}

impl Era1AcquiredStateEvidence {
    pub fn is_empty(&self) -> bool {
        self.lifetime_weight_digest.is_none()
            && self.memory_digests.is_empty()
            && self.learned_vocabulary.is_empty()
            && self.pending_eligibility_digest.is_none()
            && self.transient_state_digest.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1SelectionProfile {
    pub founder_genome_id: GenomeId,
    pub objectives: ObjectiveVector,
    pub known_ancestor_genome_ids: Vec<GenomeId>,
    pub population_share: f32,
    pub specialist_roles: Vec<SpecialistRole>,
}

impl Era1SelectionProfile {
    pub fn validate_contract(&self) -> Result<(), Era1EvolutionError> {
        self.founder_genome_id.validate()?;
        if !self.objectives.all_known()
            || !self.population_share.is_finite()
            || !(0.0..=1.0).contains(&self.population_share)
        {
            return Err(Era1EvolutionError::UnknownSelectionObjective(
                self.founder_genome_id,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1ArchiveReceipt {
    pub generation: u32,
    pub organism_id: OrganismId,
    pub genome_id: GenomeId,
    pub manifest_digest_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1PortableSaveReceipt {
    pub generation: u32,
    pub relative_path: String,
    pub digest_hex: String,
    pub organism_ids: Vec<OrganismId>,
    pub genome_ids: Vec<GenomeId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1BirthReceipt {
    pub generation: u32,
    pub lineage_slot: usize,
    pub organism_id: OrganismId,
    pub genome: CreatureGenome,
    pub inherited_starter_tokens: Vec<LanguageTokenId>,
    pub acquired_state: Era1AcquiredStateEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1GenerationReceipt {
    pub generation: u32,
    pub births: Vec<Era1BirthReceipt>,
    pub preserved_wild_genome_ids: Vec<GenomeId>,
    pub selection_plan: Option<ManagedBreedingPlan>,
    pub habitat_breeding: Vec<HabitatBreedingReceipt>,
    pub archives: Vec<Era1ArchiveReceipt>,
    pub portable_save: Era1PortableSaveReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1LineageReceipt {
    pub lineage_slot: usize,
    pub founder_genome_id: GenomeId,
    pub genome_ids: Vec<GenomeId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1EvolutionReceipt {
    pub schema_version: u16,
    pub config: Era1EvolutionConfig,
    pub wild_reservoir: Vec<CreatureGenome>,
    pub selection_profiles: Vec<Era1SelectionProfile>,
    pub generations: Vec<Era1GenerationReceipt>,
    pub lineages: Vec<Era1LineageReceipt>,
}

impl Era1EvolutionReceipt {
    pub fn validate_contract(&self) -> Result<(), Era1EvolutionError> {
        self.config.validate_contract()?;
        if self.schema_version != ERA1_EVOLUTION_SCHEMA_VERSION
            || self.wild_reservoir.len() != self.config.lineage_count
            || self.selection_profiles.len() != self.config.lineage_count
            || self.generations.len()
                != usize::try_from(self.config.ordinary_birth_generations + 1)
                    .map_err(|_| Era1EvolutionError::InvalidEvidence("generation overflow"))?
            || self.lineages.len() != self.config.lineage_count
        {
            return Err(Era1EvolutionError::InvalidEvidence(
                "evolution receipt shape changed",
            ));
        }

        validate_founders(&self.wild_reservoir)?;
        for (founder, profile) in self.wild_reservoir.iter().zip(&self.selection_profiles) {
            profile.validate_contract()?;
            if profile.founder_genome_id != founder.id {
                return Err(Era1EvolutionError::InvalidEvidence(
                    "selection profile does not match wild founder",
                ));
            }
        }
        let wild_ids = self
            .wild_reservoir
            .iter()
            .map(|genome| genome.id)
            .collect::<Vec<_>>();

        for (generation_index, generation) in self.generations.iter().enumerate() {
            let expected_generation = u32::try_from(generation_index)
                .map_err(|_| Era1EvolutionError::InvalidEvidence("generation overflow"))?;
            if generation.generation != expected_generation
                || generation.births.len() != self.config.lineage_count
                || generation.preserved_wild_genome_ids != wild_ids
                || generation.archives.len() != generation.births.len()
                || generation.portable_save.generation != expected_generation
                || generation.portable_save.organism_ids
                    != generation
                        .births
                        .iter()
                        .map(|birth| birth.organism_id)
                        .collect::<Vec<_>>()
                || generation.portable_save.genome_ids
                    != generation
                        .births
                        .iter()
                        .map(|birth| birth.genome.id)
                        .collect::<Vec<_>>()
                || !valid_digest_text(&generation.portable_save.digest_hex)
            {
                return Err(Era1EvolutionError::InvalidEvidence(
                    "generation receipt shape changed",
                ));
            }

            for (slot, birth) in generation.births.iter().enumerate() {
                validate_birth(birth, expected_generation, slot)?;
                if generation_index == 0 {
                    if birth.genome != self.wild_reservoir[slot]
                        || birth.genome.provenance.ordinary_birth
                        || !birth.genome.parent_genome_ids.is_empty()
                        || generation.selection_plan.is_some()
                        || !generation.habitat_breeding.is_empty()
                    {
                        return Err(Era1EvolutionError::InvalidEvidence(
                            "founder birth receipt changed",
                        ));
                    }
                } else {
                    let parents = &self.generations[generation_index - 1].births;
                    let plan = generation.selection_plan.as_ref().ok_or(
                        Era1EvolutionError::InvalidEvidence("managed selection plan is missing"),
                    )?;
                    let [maternal_id, paternal_id] = birth.genome.parent_genome_ids.as_slice()
                    else {
                        return Err(Era1EvolutionError::InvalidEvidence(
                            "offspring parent count changed",
                        ));
                    };
                    let maternal = parents
                        .iter()
                        .find(|candidate| candidate.genome.id == *maternal_id)
                        .ok_or(Era1EvolutionError::InvalidEvidence(
                            "selected maternal genome is missing",
                        ))?;
                    let paternal = parents
                        .iter()
                        .find(|candidate| candidate.genome.id == *paternal_id)
                        .ok_or(Era1EvolutionError::InvalidEvidence(
                            "selected paternal genome is missing",
                        ))?;
                    if !plan.pairings.iter().any(|pairing| {
                        pairing.maternal_genome_id == *maternal_id
                            && pairing.paternal_genome_id == *paternal_id
                    }) || birth.genome
                        != CreatureGenome::reproduce(
                            &maternal.genome,
                            &paternal.genome,
                            birth.genome.conception_seed,
                        )?
                    {
                        return Err(Era1EvolutionError::InvalidEvidence(
                            "offspring does not match authoritative reproduction",
                        ));
                    }
                }
                let archive = &generation.archives[slot];
                if archive.generation != expected_generation
                    || archive.organism_id != birth.organism_id
                    || archive.genome_id != birth.genome.id
                    || !valid_digest_text(&archive.manifest_digest_hex)
                {
                    return Err(Era1EvolutionError::InvalidEvidence(
                        "archive receipt does not match birth",
                    ));
                }
            }
            if generation_index > 0 {
                let expected_candidates = selection_candidates(
                    &self.wild_reservoir,
                    &self.generations[generation_index - 1].births,
                    &self.selection_profiles,
                )?;
                let expected_plan = run_managed_selection(
                    &expected_candidates,
                    &selection_config(&self.config, expected_generation),
                )?;
                if generation.selection_plan.as_ref() != Some(&expected_plan)
                    || generation.habitat_breeding.len() != expected_plan.pairings.len()
                    || generation.habitat_breeding.iter().any(|receipt| {
                        receipt.mode != HabitatMode::Managed
                            || receipt.kind != HabitatBreedingKind::Explicit
                            || receipt.actor != HabitatActor::WorldAuthority
                            || receipt.cognition_policy != PolicyBackend::NeuralClosedLoopGpu
                    })
                {
                    return Err(Era1EvolutionError::InvalidEvidence(
                        "managed selection or habitat authority receipt changed",
                    ));
                }
            }
        }

        for (slot, lineage) in self.lineages.iter().enumerate() {
            let expected = self
                .generations
                .iter()
                .map(|generation| generation.births[slot].genome.id)
                .collect::<Vec<_>>();
            if lineage.lineage_slot != slot
                || lineage.founder_genome_id != self.wild_reservoir[slot].id
                || lineage.genome_ids != expected
            {
                return Err(Era1EvolutionError::InvalidEvidence(
                    "lineage receipt does not match generations",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Era1EvolutionError {
    #[error("invalid Era 1 evolution configuration: {0}")]
    InvalidConfig(&'static str),
    #[error("invalid Era 1 evolution evidence: {0}")]
    InvalidEvidence(&'static str),
    #[error("selection objectives for {0:?} contain UNKNOWN evidence")]
    UnknownSelectionObjective(GenomeId),
    #[error("authoritative genome operation failed: {0}")]
    Genome(#[from] ScaffoldContractError),
    #[error("managed selection failed: {0}")]
    Selection(#[from] crate::p33_selection::SelectionError),
    #[error("habitat authority failed: {0}")]
    Habitat(#[from] alife_world::HabitatAuthorityError),
    #[error("lineage archive failed: {0}")]
    Archive(#[from] alife_archive::ArchiveError),
    #[error("portable save failed: {0}")]
    Persistence(#[from] alife_world::PersistenceError),
    #[error("evolution artifact I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("evolution artifact JSON failed: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn run_era1_evolution(
    config: &Era1EvolutionConfig,
    founders: &[CreatureGenome],
    selection_profiles: &[Era1SelectionProfile],
    artifact_root: impl AsRef<Path>,
) -> Result<Era1EvolutionReceipt, Era1EvolutionError> {
    config.validate_contract()?;
    validate_founders(founders)?;
    if founders.len() != config.lineage_count || selection_profiles.len() != founders.len() {
        return Err(Era1EvolutionError::InvalidEvidence(
            "founder/profile count does not match bounded lineages",
        ));
    }
    for (founder, profile) in founders.iter().zip(selection_profiles) {
        profile.validate_contract()?;
        if profile.founder_genome_id != founder.id {
            return Err(Era1EvolutionError::InvalidEvidence(
                "selection profile does not match founder",
            ));
        }
    }

    let artifact_root = artifact_root.as_ref();
    std::fs::create_dir_all(artifact_root)?;
    let wild_reservoir = founders.to_vec();
    let wild_ids = founders.iter().map(|genome| genome.id).collect::<Vec<_>>();
    let founder_births = founders
        .iter()
        .cloned()
        .enumerate()
        .map(|(lineage_slot, genome)| {
            birth_receipt(
                0,
                lineage_slot,
                managed_organism_id(0, lineage_slot)?,
                genome,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let habitats = evolution_habitat_authority(founders.len(), &founder_births)?;
    let mut library = LineageLibrary::open(LineageLibraryConfig::profile_default(
        artifact_root.join("lineage-library"),
    ))?;
    let founder_archives = archive_births(&mut library, 0, &founder_births)?;
    let founder_save = persist_generation_save(
        artifact_root,
        config.evolution_seed,
        0,
        &founder_births,
        &habitats,
    )?;
    let mut generations = vec![Era1GenerationReceipt {
        generation: 0,
        births: founder_births,
        preserved_wild_genome_ids: wild_ids.clone(),
        selection_plan: None,
        habitat_breeding: Vec::new(),
        archives: founder_archives,
        portable_save: founder_save,
    }];
    let mut habitats = habitats;

    for generation in 1..=config.ordinary_birth_generations {
        let parents = &generations
            .last()
            .expect("founder generation is always present")
            .births;
        let candidates = selection_candidates(&wild_reservoir, parents, selection_profiles)?;
        let selection_config = selection_config(config, generation);
        let selection_plan = run_managed_selection(&candidates, &selection_config)?;
        let (mut births, habitat_breeding) =
            materialize_selected_births(config, generation, parents, &selection_plan, &habitats)?;
        for birth in &births {
            habitats.register_creature(
                birth.organism_id,
                managed_habitat_id(),
                Tick::new(u64::from(generation)),
            )?;
        }
        let archives = archive_births(&mut library, generation, &births)?;
        let portable_save = persist_generation_save(
            artifact_root,
            config.evolution_seed,
            generation,
            &births,
            &habitats,
        )?;
        births.sort_by_key(|birth| birth.lineage_slot);
        generations.push(Era1GenerationReceipt {
            generation,
            births,
            preserved_wild_genome_ids: wild_ids.clone(),
            selection_plan: Some(selection_plan),
            habitat_breeding,
            archives,
            portable_save,
        });
    }

    let lineages = (0..config.lineage_count)
        .map(|lineage_slot| Era1LineageReceipt {
            lineage_slot,
            founder_genome_id: founders[lineage_slot].id,
            genome_ids: generations
                .iter()
                .map(|generation| generation.births[lineage_slot].genome.id)
                .collect(),
        })
        .collect();
    let receipt = Era1EvolutionReceipt {
        schema_version: ERA1_EVOLUTION_SCHEMA_VERSION,
        config: config.clone(),
        wild_reservoir,
        selection_profiles: selection_profiles.to_vec(),
        generations,
        lineages,
    };
    receipt.validate_contract()?;
    Ok(receipt)
}

fn birth_receipt(
    generation: u32,
    lineage_slot: usize,
    organism_id: OrganismId,
    genome: CreatureGenome,
) -> Result<Era1BirthReceipt, Era1EvolutionError> {
    let inherited_starter_tokens = genome.express()?.predisposition.starter_tokens;
    let receipt = Era1BirthReceipt {
        generation,
        lineage_slot,
        organism_id,
        genome,
        inherited_starter_tokens,
        acquired_state: Era1AcquiredStateEvidence::default(),
    };
    validate_birth(&receipt, generation, lineage_slot)?;
    Ok(receipt)
}

fn validate_birth(
    birth: &Era1BirthReceipt,
    generation: u32,
    lineage_slot: usize,
) -> Result<(), Era1EvolutionError> {
    birth.genome.validate_contract()?;
    birth.organism_id.validate()?;
    let expressed = birth.genome.express()?;
    if birth.generation != generation
        || birth.lineage_slot != lineage_slot
        || birth.organism_id != managed_organism_id(generation, lineage_slot)?
        || !birth.acquired_state.is_empty()
        || birth.inherited_starter_tokens.is_empty()
        || birth.inherited_starter_tokens != expressed.predisposition.starter_tokens
        || birth
            .inherited_starter_tokens
            .iter()
            .any(|token| token.raw() == 0)
    {
        return Err(Era1EvolutionError::InvalidEvidence(
            "birth inherited copied or fabricated state",
        ));
    }
    Ok(())
}

fn managed_habitat_id() -> HabitatId {
    HabitatId::new(2).expect("managed habitat id is nonzero")
}

fn managed_organism_id(
    generation: u32,
    lineage_slot: usize,
) -> Result<OrganismId, Era1EvolutionError> {
    let slot = u64::try_from(lineage_slot)
        .map_err(|_| Era1EvolutionError::InvalidEvidence("lineage slot overflow"))?;
    let raw = 20_000_u64
        .checked_add(u64::from(generation).saturating_mul(100))
        .and_then(|base| base.checked_add(slot + 1))
        .ok_or(Era1EvolutionError::InvalidEvidence(
            "managed organism id overflow",
        ))?;
    Ok(OrganismId(raw))
}

fn evolution_habitat_authority(
    founder_count: usize,
    managed_births: &[Era1BirthReceipt],
) -> Result<HabitatAuthority, Era1EvolutionError> {
    let mut authority = HabitatAuthority::new(vec![
        Habitat::new(HabitatId::DEFAULT_WILD, "Wild", HabitatMode::Wild)?,
        Habitat::new(managed_habitat_id(), "Managed", HabitatMode::Managed)?,
    ])?;
    for index in 0..founder_count {
        let wild_id = OrganismId(10_001_u64.checked_add(index as u64).ok_or(
            Era1EvolutionError::InvalidEvidence("wild organism id overflow"),
        )?);
        authority.register_creature(wild_id, HabitatId::DEFAULT_WILD, Tick::ZERO)?;
    }
    for birth in managed_births {
        authority.register_creature(birth.organism_id, managed_habitat_id(), Tick::ZERO)?;
    }
    Ok(authority)
}

fn selection_config(config: &Era1EvolutionConfig, generation: u32) -> ManagedSelectionConfig {
    ManagedSelectionConfig {
        selection_seed: derived_seed(config.evolution_seed, 0xE1A1_5000, u64::from(generation)),
        max_pairings: config.lineage_count,
        minority_lineage_share_max: 0.20,
        fragile_ecology_max: 0.30,
        high_cognition_min: 0.75,
        robust_ecology_min: 0.65,
        introgression_sibling_count: 2,
    }
}

fn selection_candidates(
    wild_reservoir: &[CreatureGenome],
    managed_births: &[Era1BirthReceipt],
    profiles: &[Era1SelectionProfile],
) -> Result<Vec<SelectionCandidate>, Era1EvolutionError> {
    if managed_births.len() != profiles.len() || wild_reservoir.len() != profiles.len() {
        return Err(Era1EvolutionError::InvalidEvidence(
            "selection candidate shape changed",
        ));
    }
    let mut candidates = wild_reservoir
        .iter()
        .cloned()
        .map(|genome| SelectionCandidate {
            genome,
            objectives: unknown_objectives(),
            known_ancestor_genome_ids: Vec::new(),
            population_share: 1.0,
            lane: PopulationLane::Wild,
            specialist_roles: Vec::new(),
        })
        .collect::<Vec<_>>();
    for (birth, profile) in managed_births.iter().zip(profiles) {
        profile.validate_contract()?;
        let mut ancestors = profile.known_ancestor_genome_ids.clone();
        ancestors.extend(birth.genome.parent_genome_ids.iter().copied());
        ancestors.sort_by_key(|id| id.0);
        ancestors.dedup();
        candidates.push(SelectionCandidate {
            genome: birth.genome.clone(),
            objectives: profile.objectives.clone(),
            known_ancestor_genome_ids: ancestors,
            population_share: profile.population_share,
            lane: PopulationLane::Managed,
            specialist_roles: profile.specialist_roles.clone(),
        });
    }
    Ok(candidates)
}

fn unknown_objectives() -> ObjectiveVector {
    ObjectiveVector {
        ecological: ScoreEstimate::UNKNOWN,
        cognitive: ScoreEstimate::UNKNOWN,
        social: ScoreEstimate::UNKNOWN,
        group: ScoreEstimate::UNKNOWN,
        stability: ScoreEstimate::UNKNOWN,
        efficiency: ScoreEstimate::UNKNOWN,
        diversity: ScoreEstimate::UNKNOWN,
    }
}

fn materialize_selected_births(
    config: &Era1EvolutionConfig,
    generation: u32,
    parents: &[Era1BirthReceipt],
    plan: &ManagedBreedingPlan,
    habitats: &HabitatAuthority,
) -> Result<(Vec<Era1BirthReceipt>, Vec<HabitatBreedingReceipt>), Era1EvolutionError> {
    if plan.pairings.len() < 2 || plan.offspring.is_empty() {
        return Err(Era1EvolutionError::InvalidEvidence(
            "managed selection produced too few legal lineages",
        ));
    }
    let parent_by_genome = parents
        .iter()
        .map(|birth| (birth.genome.id.0, birth))
        .collect::<BTreeMap<_, _>>();
    let mut habitat_breeding = Vec::with_capacity(plan.pairings.len());
    for pairing in &plan.pairings {
        let maternal = parent_by_genome.get(&pairing.maternal_genome_id.0).ok_or(
            Era1EvolutionError::InvalidEvidence("selected maternal genome is absent"),
        )?;
        let paternal = parent_by_genome.get(&pairing.paternal_genome_id.0).ok_or(
            Era1EvolutionError::InvalidEvidence("selected paternal genome is absent"),
        )?;
        habitat_breeding.push(habitats.authorize_breeding(HabitatBreedingRequest {
            habitat_id: managed_habitat_id(),
            first_parent: maternal.organism_id,
            second_parent: paternal.organism_id,
            kind: HabitatBreedingKind::Explicit,
            actor: HabitatActor::WorldAuthority,
            tick: Tick::new(u64::from(generation)),
        })?);
    }

    let mut genomes = plan
        .offspring
        .iter()
        .map(|offspring| offspring.genome.clone())
        .collect::<Vec<_>>();
    let mut sibling_round = 1_u64;
    while genomes.len() < config.lineage_count {
        let mut pairing_order = (0..plan.pairings.len()).collect::<Vec<_>>();
        pairing_order
            .sort_by_key(|index| (plan.pairings[*index].offspring_genome_ids.len(), *index));
        for pairing_index in pairing_order {
            if genomes.len() == config.lineage_count {
                break;
            }
            let pairing = &plan.pairings[pairing_index];
            let maternal = &parent_by_genome[&pairing.maternal_genome_id.0].genome;
            let paternal = &parent_by_genome[&pairing.paternal_genome_id.0].genome;
            let seed = derived_seed(
                config.evolution_seed
                    ^ maternal.id.0
                    ^ paternal.id.0.rotate_left(23)
                    ^ sibling_round.rotate_left(7),
                u64::from(generation),
                pairing_index as u64,
            );
            let child = CreatureGenome::reproduce(maternal, paternal, seed)?;
            if !genomes.iter().any(|genome| genome.id == child.id) {
                genomes.push(child);
            }
        }
        sibling_round = sibling_round.saturating_add(1);
        if sibling_round > config.lineage_count as u64 + 2 {
            return Err(Era1EvolutionError::InvalidEvidence(
                "managed sibling expansion stalled",
            ));
        }
    }
    genomes.truncate(config.lineage_count);
    let births = genomes
        .into_iter()
        .enumerate()
        .map(|(slot, genome)| {
            birth_receipt(
                generation,
                slot,
                managed_organism_id(generation, slot)?,
                genome,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((births, habitat_breeding))
}

fn archive_births(
    library: &mut LineageLibrary,
    generation: u32,
    births: &[Era1BirthReceipt],
) -> Result<Vec<Era1ArchiveReceipt>, Era1EvolutionError> {
    let foundation = FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)?;
    let foundation_bytes = foundation.encode_canonical()?;
    births
        .iter()
        .map(|birth| {
            let phenotype = compile_genome(&birth.genome, &foundation)?;
            let digest = library.archive_composite_birth(CompositeGeneticArchiveInput {
                source_run_id: "era1-bounded-evolution",
                organism_id: birth.organism_id,
                birth_tick: Tick::new(u64::from(generation)),
                creature_genome: &birth.genome,
                phenotype: &phenotype,
                foundation_asset_bytes: &foundation_bytes,
            })?;
            Ok(Era1ArchiveReceipt {
                generation,
                organism_id: birth.organism_id,
                genome_id: birth.genome.id,
                manifest_digest_hex: format_blake3(digest),
            })
        })
        .collect()
}

fn compile_genome(
    genome: &CreatureGenome,
    foundation: &FoundationWeightAsset,
) -> Result<alife_core::BrainPhenotype, Era1EvolutionError> {
    let expressed = genome.express()?;
    let development = expressed.development_state_at(Tick::new(u64::from(
        expressed.development.maturation_duration_ticks,
    )))?;
    Ok(PhenotypeCompiler::compile_from_foundation_asset(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        foundation,
    )?)
}

fn persist_generation_save(
    artifact_root: &Path,
    evolution_seed: u64,
    generation: u32,
    births: &[Era1BirthReceipt],
    habitats: &HabitatAuthority,
) -> Result<Era1PortableSaveReceipt, Era1EvolutionError> {
    let save_root = artifact_root.join(format!("generation-{generation}"));
    std::fs::create_dir_all(&save_root)?;
    let foundation = FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)?;
    let mut builder = HeadlessScenarioBuilder::new(derived_seed(
        evolution_seed,
        0xE1A1_7000,
        u64::from(generation),
    ));
    for birth in births {
        builder = builder.agent(
            &format!("era1-managed-{}-{}", generation, birth.lineage_slot),
            birth.organism_id,
            alife_core::Vec3f::new(birth.lineage_slot as f32 * 2.0, 0.0, 0.0),
        );
    }
    let mut world = builder.build()?;
    for _ in 0..generation {
        world.advance_tick();
    }
    let managed_only = HabitatAuthority::restore(
        alife_world::HabitatAuthoritySnapshot {
            next_transfer_sequence: 1,
            next_tag_sequence: 1,
            habitats: habitats.habitats().to_vec(),
            memberships: births
                .iter()
                .map(|birth| {
                    habitats.membership(birth.organism_id).cloned().ok_or(
                        Era1EvolutionError::InvalidEvidence(
                            "saved birth is missing managed habitat membership",
                        ),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
            tags: Vec::new(),
            transfers: Vec::new(),
        },
        &births
            .iter()
            .map(|birth| birth.organism_id)
            .collect::<Vec<_>>(),
    )?;
    world.replace_habitat_authority(managed_only)?;

    let mut entries = Vec::new();
    let mut creatures = Vec::new();
    for birth in births {
        let phenotype = compile_genome(&birth.genome, &foundation)?;
        let (composite, additions) = persist_composite_genetic_birth_assets(
            &save_root,
            &birth.genome,
            &foundation,
            phenotype.phenotype_hash(),
        )?;
        for entry in additions {
            if !entries
                .iter()
                .any(|present: &alife_world::AssetManifestEntry| present.asset_id == entry.asset_id)
            {
                entries.push(entry);
            }
        }
        creatures.push(CreatureSaveState {
            organism_id: birth.organism_id,
            genome_id: birth.genome.id,
            brain_class: BrainScaleTier::Standard2048,
            development_tick: Tick::ZERO,
            appearance: CreatureAppearanceGenome::default(),
            mind: CreatureMindSaveSummary {
                tick: Tick::ZERO,
                homeostasis: HomeostaticSnapshot::baseline(Tick::ZERO),
                memory_record_count: 0,
                memory_source_ids: Vec::new(),
                concept_count: 0,
                edge_count: 0,
                simplex_count: 0,
                unresolved_gap_count: 0,
                sleep_state_label: "awake".to_string(),
                diagnostics: vec!["Era 1 ordinary-birth checkpoint".to_string()],
            },
            weights: WeightLayerSaveSummary {
                generated_weight_asset_id: None,
                genetic_fixed_digest: format!("fnv1a64:{:016x}", birth.genome.id.0),
                genetic_layer_mutable: false,
                lifetime_consolidated_entries: 0,
                h_operational_entries: 1,
                h_shadow_entries: 0,
            },
            learning: LearningTraceSaveSummary {
                lifetime_learning_enabled: true,
                lamarckian_mode_enabled: false,
                last_consolidated_tick: None,
            },
            composite_genetics: Some(composite),
            lifetime_state_asset: None,
            gpu_brain: None,
        });
    }
    let world_seed = world.seed();
    let save = PortableSaveFile::from_headless_world(
        format!("era1-generation-{generation}"),
        &world,
        RuntimeConfig::deterministic_default(world_seed, BrainScaleTier::Standard2048),
        AssetManifest {
            schema: P34_ASSET_MANIFEST_SCHEMA.to_string(),
            schema_version: P34_ASSET_MANIFEST_SCHEMA_VERSION,
            entries,
        },
        creatures,
    )?;
    let relative_path = format!("generation-{generation}/population.alife.json");
    let path = artifact_root.join(&relative_path);
    save.to_json_file(&path)?;
    let restored = PortableSaveFile::from_json_file(&path)?;
    restored.validate_with_asset_root(&save_root)?;
    for birth in births {
        let loaded = restored.load_composite_genetic_birth(birth.organism_id, &save_root)?;
        if loaded.creature_genome != birth.genome {
            return Err(Era1EvolutionError::InvalidEvidence(
                "portable composite save changed a genome",
            ));
        }
    }
    let bytes = std::fs::read(&path)?;
    Ok(Era1PortableSaveReceipt {
        generation,
        relative_path,
        digest_hex: digest_bytes(&bytes),
        organism_ids: births.iter().map(|birth| birth.organism_id).collect(),
        genome_ids: births.iter().map(|birth| birth.genome.id).collect(),
    })
}

fn valid_digest_text(value: &str) -> bool {
    value
        .strip_prefix("blake3-256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("blake3-256:{}", blake3::hash(bytes).to_hex())
}

fn format_blake3(digest: alife_core::Blake3Digest) -> String {
    let mut hex = String::with_capacity(64);
    for byte in digest.bytes() {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    format!("blake3-256:{hex}")
}

fn validate_founders(founders: &[CreatureGenome]) -> Result<(), Era1EvolutionError> {
    let mut genome_ids = BTreeSet::new();
    let mut lineage_ids = BTreeSet::new();
    for founder in founders {
        founder.validate_contract()?;
        let phenotype = founder.express()?;
        if founder.foundation.brain_class_id != BrainCapacityClass::N2048_ID
            || founder.provenance.ordinary_birth
            || !founder.parent_genome_ids.is_empty()
            || !genome_ids.insert(founder.id.0)
            || !lineage_ids.insert(founder.lineage_id.0)
            || phenotype.brain_genome.brain_class_id != BrainCapacityClass::N2048_ID
        {
            return Err(Era1EvolutionError::InvalidEvidence(
                "founders must be distinct viable N2048 lineages",
            ));
        }
    }
    Ok(())
}

fn all_unique_nonzero(values: &[u64]) -> bool {
    let unique = values.iter().copied().collect::<BTreeSet<_>>();
    !unique.contains(&0) && unique.len() == values.len()
}

fn derived_seed(root: u64, domain: u64, index: u64) -> u64 {
    let mut value = root ^ domain.rotate_left(17) ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    if value == 0 {
        1
    } else {
        value
    }
}
