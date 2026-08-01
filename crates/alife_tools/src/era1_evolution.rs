//! Bounded, deterministic reproduction receipts for the Era 1 evolution program.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use alife_archive::{CompositeGeneticArchiveInput, LineageLibrary, LineageLibraryConfig};
use alife_core::{
    BrainCapacityClass, BrainScaleTier, CreatureGenome, EnvironmentalRegime, Era1Ability,
    Era1Control, Era1EvidencePartition, Era1TrialReceipt, FoundationWeightAsset, GenomeId,
    HomeostaticSnapshot, LanguageTokenId, LineageId, MetricReading, OrganismId, PassiveLifeEvent,
    PassiveLifeStatistics, PassiveMetricKind, PhenotypeCompiler, PolicyBackend,
    ScaffoldContractError, SensorProfile, Tick, Validate,
};
use alife_training::Era1TrialRunEvidence;
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
use crate::{
    ei0_exit_gate::{validate_committed_ei0_exit_gate_report, Ei0ExitGateReport},
    era1_promotion::canonical_world_family_id,
};

pub const ERA1_EVOLUTION_SCHEMA_VERSION: u16 = 2;
pub const ERA1_SELECTION_EVIDENCE_SCHEMA_VERSION: u16 = 1;
pub const ERA1_ECOLOGY_RECEIPT_SCHEMA_VERSION: u16 = 1;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1SelectionCandidateIdentity {
    pub organism_id: OrganismId,
    pub genome_id: GenomeId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub lineage_id: LineageId,
    pub generation: u32,
}

impl Era1SelectionCandidateIdentity {
    fn validate_contract(&self) -> Result<(), Era1EvolutionError> {
        self.organism_id.validate()?;
        self.genome_id.validate()?;
        self.lineage_id.validate()?;
        match self.generation {
            0 if self.parent_genome_ids.is_empty() => Ok(()),
            0 => Err(Era1EvolutionError::InvalidEvidence(
                "founder selection identity has parents",
            )),
            _ if self.parent_genome_ids.len() == 2
                && self.parent_genome_ids[0] != self.parent_genome_ids[1] =>
            {
                self.parent_genome_ids[0].validate()?;
                self.parent_genome_ids[1].validate()?;
                Ok(())
            }
            _ => Err(Era1EvolutionError::InvalidEvidence(
                "offspring selection identity has invalid parents",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1EcologyReceipt {
    pub schema_version: u16,
    pub identity: Era1SelectionCandidateIdentity,
    pub evaluation_seed: u64,
    pub world_variant_id: u64,
    pub statistics: PassiveLifeStatistics,
    pub trial_evidence_digest: String,
    pub reproduction_partner: CreatureGenome,
    pub reproduction_seed: u64,
    pub reproduction_offspring_genome_id: GenomeId,
    pub source_commit: String,
    pub source_tree: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Era1CandidateSelectionEvidence {
    pub schema_version: u16,
    pub identity: Era1SelectionCandidateIdentity,
    pub trial_evidence: Vec<Era1TrialRunEvidence>,
    pub ecology_receipts: Vec<Era1EcologyReceipt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1CandidateSelectionReceipt {
    pub schema_version: u16,
    pub identity: Era1SelectionCandidateIdentity,
    pub trial_receipts: Vec<Era1TrialReceipt>,
    pub ecology_receipts: Vec<Era1EcologyReceipt>,
}

pub fn derive_era1_ecology_receipt(
    identity: &Era1SelectionCandidateIdentity,
    candidate: &CreatureGenome,
    reproduction_partner: &CreatureGenome,
    reproduction_seed: u64,
    evaluation_seed: u64,
    world_variant_id: u64,
    trial_evidence: &[Era1TrialRunEvidence],
) -> Result<Era1EcologyReceipt, Era1EvolutionError> {
    identity.validate_contract()?;
    candidate.validate_contract()?;
    reproduction_partner.validate_contract()?;
    if identity.genome_id != candidate.id
        || identity.parent_genome_ids != candidate.parent_genome_ids
        || identity.lineage_id != candidate.lineage_id
        || reproduction_partner.id == candidate.id
        || reproduction_seed == 0
    {
        return Err(Era1EvolutionError::InvalidEvidence(
            "ecology reproduction assay identity is invalid",
        ));
    }

    let mut intact = trial_evidence
        .iter()
        .filter(|trial| {
            trial.receipt.identity.seed == evaluation_seed
                && trial.receipt.identity.world_variant_id == world_variant_id
                && trial.receipt.control == Era1Control::Intact
        })
        .collect::<Vec<_>>();
    intact.sort_by_key(|trial| trial.receipt.ability);
    if intact.len() != Era1Ability::ALL.len()
        || intact
            .iter()
            .zip(Era1Ability::ALL)
            .any(|(trial, ability)| trial.receipt.ability != ability)
    {
        return Err(Era1EvolutionError::InvalidEvidence(
            "ecology collector is missing intact ability evidence",
        ));
    }

    let mut source_binding: Option<(&str, &str)> = None;
    let mut statistics = PassiveLifeStatistics::new(identity.organism_id, Tick::ZERO)?;
    let mut ecology_tick = 0_u64;
    let mut imitation_score = None;
    let mut recognition_score = None;
    for trial in &intact {
        trial.validate_contract()?;
        if trial.receipt.identity.organism_id != identity.organism_id
            || trial.receipt.identity.genome_id != identity.genome_id
            || trial.receipt.identity.parent_genome_ids != identity.parent_genome_ids
            || trial.receipt.identity.lineage_id != identity.lineage_id
            || trial.receipt.identity.generation != identity.generation
        {
            return Err(Era1EvolutionError::InvalidEvidence(
                "ecology trial identity does not match candidate",
            ));
        }
        match source_binding {
            None => {
                source_binding = Some((&trial.receipt.source_commit, &trial.receipt.source_tree))
            }
            Some((commit, tree))
                if commit == trial.receipt.source_commit && tree == trial.receipt.source_tree => {}
            Some(_) => {
                return Err(Era1EvolutionError::InvalidEvidence(
                    "ecology trial source binding changed",
                ))
            }
        }

        for step in &trial.steps {
            statistics.observe_sealed_patch(&step.sealed_patch)?;
            ecology_tick = ecology_tick.saturating_add(1);
            let homeostasis = step.sealed_patch.pre_action().homeostasis();
            let displacement = step.sealed_patch.outcome().physical.displacement;
            let movement = (displacement.x * displacement.x
                + displacement.y * displacement.y
                + displacement.z * displacement.z)
                .sqrt()
                .clamp(0.0, 1.0);
            statistics.observe(PassiveLifeEvent::SurvivalTick {
                tick: Tick::new(ecology_tick),
                regime: ecology_regime(trial.receipt.ability),
                energy_q16: unit_f32_to_q16(homeostasis.drives.brain_atp),
                movement_distance_q16: unit_f32_to_q16(movement),
                gpu_dispatched: true,
                gpu_throttled: false,
            })?;
        }

        let demonstrated = trial.learning_assessment.demonstrated;
        statistics.observe(PassiveLifeEvent::LearningProbe {
            improvement_q16: u32::try_from(
                trial.learning_assessment.acquisition_improvement_q16.max(0),
            )
            .unwrap_or(u32::MAX)
            .min(65_535),
        })?;
        match trial.receipt.ability {
            Era1Ability::FlexibleForaging => {
                statistics.observe(PassiveLifeEvent::FoodOutcome {
                    beneficial: demonstrated,
                })?;
            }
            Era1Ability::HazardAvoidance => {
                statistics.observe(PassiveLifeEvent::PoisonEncounter {
                    avoided: demonstrated,
                })?;
                statistics.observe(PassiveLifeEvent::HazardEncounter {
                    avoided: demonstrated,
                })?;
            }
            Era1Ability::RewardReversal => {
                let ticks_to_recover = trial
                    .learning_assessment
                    .causal_proof
                    .successful_behavior_ticks
                    .first()
                    .map(|tick| u32::try_from(tick.raw()).unwrap_or(u32::MAX))
                    .unwrap_or(u32::MAX);
                statistics.observe(PassiveLifeEvent::ReversalRecovery { ticks_to_recover })?;
            }
            Era1Ability::IndividualRecognition => {
                recognition_score = trial.receipt.score.value_q16();
                statistics.observe(PassiveLifeEvent::PeerCommunication {
                    successful: demonstrated,
                })?;
            }
            Era1Ability::Imitation => {
                imitation_score = trial.receipt.score.value_q16();
                statistics.observe(PassiveLifeEvent::DialectTransfer {
                    successful: demonstrated,
                })?;
            }
            Era1Ability::GroundedLanguage => {
                let grounding_proven =
                    demonstrated && !trial.learning_assessment.grounding_receipts.is_empty();
                statistics.observe(PassiveLifeEvent::VocabularyGrounding {
                    correct: grounding_proven,
                })?;
                statistics.observe(PassiveLifeEvent::Comprehension {
                    assisted: false,
                    correct: grounding_proven,
                })?;
            }
            Era1Ability::PostSleepRetention => {
                statistics.observe(PassiveLifeEvent::SleepRetention {
                    retained: demonstrated,
                })?;
            }
            Era1Ability::SpatialMemory
            | Era1Ability::DelayedChoice
            | Era1Ability::ObjectTransfer
            | Era1Ability::MultiStepProblem => {}
        }
    }

    let offspring = CreatureGenome::reproduce(candidate, reproduction_partner, reproduction_seed)?;
    offspring.validate_contract()?;
    statistics.observe(PassiveLifeEvent::Reproduction { successful: true })?;
    let dialect_distance = imitation_score
        .zip(recognition_score)
        .map(|(imitation, recognition)| imitation.abs_diff(recognition))
        .ok_or(Era1EvolutionError::UnknownSelectionObjective(candidate.id))?;
    statistics.observe(PassiveLifeEvent::DialectDivergence {
        distance_q16: dialect_distance,
    })?;
    statistics.finalize(
        Tick::new(ecology_tick.saturating_add(1)),
        "completed authoritative Era 1 ecology evaluation",
    )?;
    let (source_commit, source_tree) = source_binding.ok_or(
        Era1EvolutionError::InvalidEvidence("ecology evidence has no source binding"),
    )?;
    Ok(Era1EcologyReceipt {
        schema_version: ERA1_ECOLOGY_RECEIPT_SCHEMA_VERSION,
        identity: identity.clone(),
        evaluation_seed,
        world_variant_id,
        statistics,
        trial_evidence_digest: digest_bytes(&serde_json::to_vec(&intact)?),
        reproduction_partner: reproduction_partner.clone(),
        reproduction_seed,
        reproduction_offspring_genome_id: offspring.id,
        source_commit: source_commit.to_string(),
        source_tree: source_tree.to_string(),
    })
}

fn ecology_regime(ability: Era1Ability) -> EnvironmentalRegime {
    match ability {
        Era1Ability::FlexibleForaging => EnvironmentalRegime::Scarcity,
        Era1Ability::HazardAvoidance | Era1Ability::RewardReversal => {
            EnvironmentalRegime::Hazardous
        }
        Era1Ability::IndividualRecognition
        | Era1Ability::Imitation
        | Era1Ability::GroundedLanguage => EnvironmentalRegime::Social,
        Era1Ability::ObjectTransfer | Era1Ability::MultiStepProblem => EnvironmentalRegime::Novel,
        Era1Ability::SpatialMemory | Era1Ability::DelayedChoice => EnvironmentalRegime::Temperate,
        Era1Ability::PostSleepRetention => EnvironmentalRegime::Abundance,
    }
}

fn unit_f32_to_q16(value: f32) -> u32 {
    (value.clamp(0.0, 1.0) * 65_535.0).round() as u32
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1SelectionProfile {
    pub identity: Era1SelectionCandidateIdentity,
    pub objectives: ObjectiveVector,
    pub known_ancestor_genome_ids: Vec<GenomeId>,
    pub population_share: f32,
    pub specialist_roles: Vec<SpecialistRole>,
    pub evidence_digest: String,
    pub source_commit: String,
    pub source_tree: String,
}

impl Era1SelectionProfile {
    pub fn validate_contract(&self) -> Result<(), Era1EvolutionError> {
        self.identity.validate_contract()?;
        if !self.objectives.all_known()
            || !self.population_share.is_finite()
            || !(0.0..=1.0).contains(&self.population_share)
            || !valid_digest_text(&self.evidence_digest)
            || !valid_git_object_id(&self.source_commit)
            || !valid_git_object_id(&self.source_tree)
        {
            return Err(Era1EvolutionError::UnknownSelectionObjective(
                self.identity.genome_id,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1SelectionRoundReceipt {
    pub parent_generation: u32,
    pub evidence: Vec<Era1CandidateSelectionReceipt>,
    pub derived_profiles: Vec<Era1SelectionProfile>,
}

pub fn derive_era1_selection_profile(
    config: &Era1EvolutionConfig,
    genome: &CreatureGenome,
    evidence: &Era1CandidateSelectionEvidence,
    population_size: usize,
    known_ancestor_genome_ids: &[GenomeId],
) -> Result<Era1SelectionProfile, Era1EvolutionError> {
    let receipt = validate_completed_selection_evidence(config, genome, evidence)?;
    recompute_era1_selection_profile_from_receipt(
        config,
        genome,
        &receipt,
        population_size,
        known_ancestor_genome_ids,
    )
}

fn validate_completed_selection_evidence(
    config: &Era1EvolutionConfig,
    genome: &CreatureGenome,
    evidence: &Era1CandidateSelectionEvidence,
) -> Result<Era1CandidateSelectionReceipt, Era1EvolutionError> {
    config.validate_contract()?;
    genome.validate_contract()?;
    if evidence.schema_version != ERA1_SELECTION_EVIDENCE_SCHEMA_VERSION
        || evidence.identity.genome_id != genome.id
        || evidence.identity.parent_genome_ids != genome.parent_genome_ids
        || evidence.identity.lineage_id != genome.lineage_id
    {
        return Err(Era1EvolutionError::InvalidEvidence(
            "completed selection evidence identity is invalid",
        ));
    }
    let mut trial_receipts = Vec::with_capacity(evidence.trial_evidence.len());
    for trial in &evidence.trial_evidence {
        trial.validate_contract()?;
        trial_receipts.push(trial.receipt.clone());
    }
    let expected_trial_count = config
        .evaluation_seeds
        .len()
        .checked_mul(config.held_out_world_transforms.len())
        .and_then(|count| count.checked_mul(Era1Ability::ALL.len()))
        .and_then(|count| count.checked_mul(config.controls.len()))
        .ok_or(Era1EvolutionError::InvalidEvidence(
            "selection trial coverage overflow",
        ))?;
    if trial_receipts.len() != expected_trial_count {
        return Err(Era1EvolutionError::InvalidEvidence(
            "selection trial coverage is incomplete",
        ));
    }
    for ecology in &evidence.ecology_receipts {
        let expected = derive_era1_ecology_receipt(
            &evidence.identity,
            genome,
            &ecology.reproduction_partner,
            ecology.reproduction_seed,
            ecology.evaluation_seed,
            ecology.world_variant_id,
            &evidence.trial_evidence,
        )?;
        if ecology != &expected {
            return Err(Era1EvolutionError::InvalidEvidence(
                "ecology receipt does not recompute from causal runtime evidence",
            ));
        }
    }
    Ok(Era1CandidateSelectionReceipt {
        schema_version: evidence.schema_version,
        identity: evidence.identity.clone(),
        trial_receipts,
        ecology_receipts: evidence.ecology_receipts.clone(),
    })
}

pub fn recompute_era1_selection_profile_from_receipt(
    config: &Era1EvolutionConfig,
    genome: &CreatureGenome,
    evidence: &Era1CandidateSelectionReceipt,
    population_size: usize,
    known_ancestor_genome_ids: &[GenomeId],
) -> Result<Era1SelectionProfile, Era1EvolutionError> {
    config.validate_contract()?;
    genome.validate_contract()?;
    evidence.identity.validate_contract()?;
    if evidence.schema_version != ERA1_SELECTION_EVIDENCE_SCHEMA_VERSION
        || population_size == 0
        || evidence.identity.genome_id != genome.id
        || evidence.identity.parent_genome_ids != genome.parent_genome_ids
        || evidence.identity.lineage_id != genome.lineage_id
    {
        return Err(Era1EvolutionError::InvalidEvidence(
            "selection evidence identity does not match candidate genome",
        ));
    }

    let expected_trial_count = config
        .evaluation_seeds
        .len()
        .checked_mul(config.held_out_world_transforms.len())
        .and_then(|count| count.checked_mul(Era1Ability::ALL.len()))
        .and_then(|count| count.checked_mul(config.controls.len()))
        .ok_or(Era1EvolutionError::InvalidEvidence(
            "selection trial coverage overflow",
        ))?;
    if evidence.trial_receipts.len() != expected_trial_count {
        return Err(Era1EvolutionError::InvalidEvidence(
            "selection trial coverage is incomplete",
        ));
    }
    let mut trial_keys = BTreeSet::new();
    let mut source_binding: Option<(&str, &str)> = None;
    for receipt in &evidence.trial_receipts {
        receipt.validate_contract()?;
        let identity = &receipt.identity;
        if identity.organism_id != evidence.identity.organism_id
            || identity.genome_id != evidence.identity.genome_id
            || identity.parent_genome_ids != evidence.identity.parent_genome_ids
            || identity.lineage_id != evidence.identity.lineage_id
            || identity.generation != evidence.identity.generation
            || identity.brain_class_id != genome.foundation.brain_class_id
            || receipt.foundation_id != genome.foundation.foundation_id
            || receipt.foundation_version != u32::from(genome.foundation.version)
            || !config.controls.contains(&receipt.control)
            || receipt.partition != Era1EvidencePartition::HeldOutTransfer
            || !receipt.assistance.is_empty()
            || identity.world_family_id != canonical_world_family_id(receipt.ability)
            || !config.evaluation_seeds.contains(&identity.seed)
            || !config
                .held_out_world_transforms
                .contains(&identity.world_variant_id)
            || !matches!(receipt.score, MetricReading::Measured { .. })
        {
            return Err(Era1EvolutionError::InvalidEvidence(
                "selection trial receipt has mismatched or incomplete provenance",
            ));
        }
        if !trial_keys.insert((
            identity.seed,
            identity.world_variant_id,
            receipt.ability,
            receipt.control,
        )) {
            return Err(Era1EvolutionError::InvalidEvidence(
                "selection trial coverage contains duplicates",
            ));
        }
        match source_binding {
            None => source_binding = Some((&receipt.source_commit, &receipt.source_tree)),
            Some((commit, tree))
                if commit == receipt.source_commit && tree == receipt.source_tree => {}
            Some(_) => {
                return Err(Era1EvolutionError::InvalidEvidence(
                    "selection trial source binding changed",
                ))
            }
        }
    }

    let expected_ecology_count = config
        .evaluation_seeds
        .len()
        .checked_mul(config.held_out_world_transforms.len())
        .ok_or(Era1EvolutionError::InvalidEvidence(
            "ecology coverage overflow",
        ))?;
    if evidence.ecology_receipts.len() != expected_ecology_count {
        return Err(Era1EvolutionError::InvalidEvidence(
            "ecology coverage is incomplete",
        ));
    }
    let mut ecology_keys = BTreeSet::new();
    for receipt in &evidence.ecology_receipts {
        receipt.identity.validate_contract()?;
        receipt.statistics.validate_contract()?;
        receipt.reproduction_partner.validate_contract()?;
        let expected_offspring = CreatureGenome::reproduce(
            genome,
            &receipt.reproduction_partner,
            receipt.reproduction_seed,
        )?;
        let Some((commit, tree)) = source_binding else {
            return Err(Era1EvolutionError::InvalidEvidence(
                "selection evidence has no source binding",
            ));
        };
        if receipt.schema_version != ERA1_ECOLOGY_RECEIPT_SCHEMA_VERSION
            || receipt.identity != evidence.identity
            || receipt.statistics.organism_id() != evidence.identity.organism_id
            || receipt.statistics.death_tick().is_none()
            || !valid_digest_text(&receipt.trial_evidence_digest)
            || receipt.reproduction_seed == 0
            || receipt.reproduction_partner.id == genome.id
            || receipt.reproduction_offspring_genome_id != expected_offspring.id
            || !config.evaluation_seeds.contains(&receipt.evaluation_seed)
            || !config
                .held_out_world_transforms
                .contains(&receipt.world_variant_id)
            || receipt.source_commit != commit
            || receipt.source_tree != tree
            || !ecology_keys.insert((receipt.evaluation_seed, receipt.world_variant_id))
        {
            return Err(Era1EvolutionError::InvalidEvidence(
                "ecology receipt has mismatched or incomplete provenance",
            ));
        }
    }

    let trial_score = |ability| {
        aggregate_readings(evidence.trial_receipts.iter().filter_map(|receipt| {
            (receipt.ability == ability && receipt.control == Era1Control::Intact)
                .then_some(receipt.score)
        }))
    };
    let ecology_score = |kind| {
        aggregate_readings(
            evidence
                .ecology_receipts
                .iter()
                .map(|receipt| receipt.statistics.metric(kind)),
        )
    };

    let objectives = ObjectiveVector {
        ecological: average_estimates(&[
            aggregate_survival_readings(
                evidence
                    .ecology_receipts
                    .iter()
                    .map(|receipt| receipt.statistics.metric(PassiveMetricKind::SurvivalTicks)),
            ),
            ecology_score(PassiveMetricKind::FoodSuccess),
            ecology_score(PassiveMetricKind::PoisonAvoidance),
            ecology_score(PassiveMetricKind::HazardAvoidance),
            ecology_score(PassiveMetricKind::EnergyStability),
            ecology_score(PassiveMetricKind::Reproduction),
        ]),
        cognitive: average_estimates(&[
            trial_score(Era1Ability::FlexibleForaging),
            trial_score(Era1Ability::HazardAvoidance),
            trial_score(Era1Ability::SpatialMemory),
            trial_score(Era1Ability::DelayedChoice),
            trial_score(Era1Ability::RewardReversal),
            trial_score(Era1Ability::ObjectTransfer),
            trial_score(Era1Ability::MultiStepProblem),
            trial_score(Era1Ability::PostSleepRetention),
            ecology_score(PassiveMetricKind::LearningSlope),
        ]),
        social: average_estimates(&[
            trial_score(Era1Ability::IndividualRecognition),
            trial_score(Era1Ability::Imitation),
            trial_score(Era1Ability::GroundedLanguage),
            ecology_score(PassiveMetricKind::VocabularyGrounding),
            ecology_score(PassiveMetricKind::UnaidedComprehension),
            ecology_score(PassiveMetricKind::PeerCommunication),
        ]),
        group: average_estimates(&[
            trial_score(Era1Ability::Imitation),
            trial_score(Era1Ability::IndividualRecognition),
            ecology_score(PassiveMetricKind::PeerCommunication),
            ecology_score(PassiveMetricKind::DialectTransfer),
        ]),
        stability: average_estimates(&[
            trial_score(Era1Ability::RewardReversal),
            trial_score(Era1Ability::PostSleepRetention),
            ecology_score(PassiveMetricKind::EnergyStability),
            ecology_score(PassiveMetricKind::SleepRetention),
            ecology_score(PassiveMetricKind::ReversalRecovery),
        ]),
        efficiency: average_estimates(&[
            trial_score(Era1Ability::FlexibleForaging),
            trial_score(Era1Ability::MultiStepProblem),
            ecology_score(PassiveMetricKind::FoodSuccess),
            ecology_score(PassiveMetricKind::Movement),
            ecology_score(PassiveMetricKind::GpuThrottleAvoidance),
        ]),
        diversity: average_estimates(&[
            trial_score(Era1Ability::ObjectTransfer),
            ecology_score(PassiveMetricKind::DialectTransfer),
            ecology_score(PassiveMetricKind::DialectDivergence),
        ]),
    };

    let mut ancestors = known_ancestor_genome_ids.to_vec();
    ancestors.sort_by_key(|id| id.0);
    ancestors.dedup();
    let mut specialist_roles = Vec::new();
    if estimate_at_least(objectives.ecological, 0.75) {
        specialist_roles.push(SpecialistRole::EcologicalSurvivor);
    }
    if estimate_at_least(objectives.cognitive, 0.75) {
        specialist_roles.push(SpecialistRole::TransferSpecialist);
    }
    if estimate_at_least(objectives.social, 0.75) {
        specialist_roles.push(SpecialistRole::Teacher);
    }
    if estimate_at_least(objectives.group, 0.75) {
        specialist_roles.push(SpecialistRole::Coordinator);
    }
    let (source_commit, source_tree) = source_binding.ok_or(
        Era1EvolutionError::InvalidEvidence("selection evidence has no source binding"),
    )?;
    let profile = Era1SelectionProfile {
        identity: evidence.identity.clone(),
        objectives,
        known_ancestor_genome_ids: ancestors,
        population_share: 1.0 / population_size as f32,
        specialist_roles,
        evidence_digest: digest_bytes(&serde_json::to_vec(evidence)?),
        source_commit: source_commit.to_string(),
        source_tree: source_tree.to_string(),
    };
    profile.validate_contract()?;
    Ok(profile)
}

fn aggregate_readings(readings: impl Iterator<Item = MetricReading>) -> ScoreEstimate {
    let mut value_sum = 0_u128;
    let mut reading_count = 0_u64;
    let mut samples = 0_u64;
    for reading in readings {
        let MetricReading::Measured {
            value_q16,
            exposures,
        } = reading
        else {
            return ScoreEstimate::UNKNOWN;
        };
        value_sum = value_sum.saturating_add(u128::from(value_q16));
        reading_count = reading_count.saturating_add(1);
        samples = samples.saturating_add(exposures);
    }
    if reading_count == 0 {
        return ScoreEstimate::UNKNOWN;
    }
    let mean_q16 = (value_sum + u128::from(reading_count / 2)) / u128::from(reading_count);
    ScoreEstimate::known(
        mean_q16 as f32 / 65_535.0,
        u32::try_from(samples).unwrap_or(u32::MAX),
    )
}

fn aggregate_survival_readings(readings: impl Iterator<Item = MetricReading>) -> ScoreEstimate {
    const COMPLETE_WINDOW_TICKS: u32 =
        Era1Ability::ALL.len() as u32 * alife_world::ERA1_TRIAL_END_TICK as u32;
    let mut normalized_sum = 0.0_f64;
    let mut reading_count = 0_u64;
    let mut samples = 0_u64;
    for reading in readings {
        let MetricReading::Measured {
            value_q16: survival_ticks,
            exposures,
        } = reading
        else {
            return ScoreEstimate::UNKNOWN;
        };
        normalized_sum +=
            f64::from(survival_ticks.min(COMPLETE_WINDOW_TICKS)) / f64::from(COMPLETE_WINDOW_TICKS);
        reading_count = reading_count.saturating_add(1);
        samples = samples.saturating_add(exposures);
    }
    if reading_count == 0 {
        return ScoreEstimate::UNKNOWN;
    }
    ScoreEstimate::known(
        (normalized_sum / reading_count as f64) as f32,
        u32::try_from(samples).unwrap_or(u32::MAX),
    )
}

fn average_estimates(estimates: &[ScoreEstimate]) -> ScoreEstimate {
    let Some(values) = estimates
        .iter()
        .map(|estimate| estimate.value)
        .collect::<Option<Vec<_>>>()
    else {
        return ScoreEstimate::UNKNOWN;
    };
    if values.is_empty() {
        return ScoreEstimate::UNKNOWN;
    }
    ScoreEstimate::known(
        values.iter().copied().sum::<f32>() / values.len() as f32,
        estimates
            .iter()
            .fold(0_u32, |sum, estimate| sum.saturating_add(estimate.samples)),
    )
}

fn estimate_at_least(estimate: ScoreEstimate, threshold: f32) -> bool {
    estimate.value.is_some_and(|value| value >= threshold)
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
    pub baseline_ei0_exit_gate: Ei0ExitGateReport,
    pub wild_reservoir: Vec<CreatureGenome>,
    pub selection_rounds: Vec<Era1SelectionRoundReceipt>,
    pub generations: Vec<Era1GenerationReceipt>,
    pub lineages: Vec<Era1LineageReceipt>,
}

impl Era1EvolutionReceipt {
    pub fn validate_contract(&self) -> Result<(), Era1EvolutionError> {
        self.config.validate_contract()?;
        validate_committed_ei0_exit_gate_report(&self.baseline_ei0_exit_gate)
            .map_err(|error| Era1EvolutionError::Ei0Gate(error.to_string()))?;
        if self.schema_version != ERA1_EVOLUTION_SCHEMA_VERSION
            || self.wild_reservoir.len() != self.config.lineage_count
            || self.selection_rounds.len()
                != usize::try_from(self.config.ordinary_birth_generations)
                    .map_err(|_| Era1EvolutionError::InvalidEvidence("generation overflow"))?
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
        let wild_ids = self
            .wild_reservoir
            .iter()
            .map(|genome| genome.id)
            .collect::<Vec<_>>();
        let mut selection_source: Option<(String, String)> = None;

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
                let parent_generation = u32::try_from(generation_index - 1)
                    .map_err(|_| Era1EvolutionError::InvalidEvidence("generation overflow"))?;
                let round = &self.selection_rounds[generation_index - 1];
                let parents = &self.generations[generation_index - 1].births;
                let expected_profiles = derive_selection_round_profiles_from_receipts(
                    &self.config,
                    parent_generation,
                    parents,
                    &round.evidence,
                    &self.generations[..generation_index],
                )?;
                if round.parent_generation != parent_generation
                    || round.derived_profiles != expected_profiles
                {
                    return Err(Era1EvolutionError::InvalidEvidence(
                        "selection profiles do not recompute from included receipts",
                    ));
                }
                for profile in &expected_profiles {
                    match selection_source.as_ref() {
                        None => {
                            selection_source =
                                Some((profile.source_commit.clone(), profile.source_tree.clone()))
                        }
                        Some((commit, tree))
                            if commit == &profile.source_commit && tree == &profile.source_tree => {
                        }
                        Some(_) => {
                            return Err(Era1EvolutionError::InvalidEvidence(
                                "selection source binding changed between generations",
                            ))
                        }
                    }
                }
                let expected_candidates =
                    selection_candidates(&self.wild_reservoir, parents, &expected_profiles)?;
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
    #[error("committed EI0 exit gate precondition failed: {0}")]
    Ei0Gate(String),
    #[error("authoritative Era 1 trial evidence failed: {0}")]
    TrialEvidence(String),
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

fn derive_selection_round_profiles_from_receipts(
    config: &Era1EvolutionConfig,
    parent_generation: u32,
    parents: &[Era1BirthReceipt],
    evidence: &[Era1CandidateSelectionReceipt],
    prior_generations: &[Era1GenerationReceipt],
) -> Result<Vec<Era1SelectionProfile>, Era1EvolutionError> {
    if parents.len() != config.lineage_count || evidence.len() != parents.len() {
        return Err(Era1EvolutionError::InvalidEvidence(
            "selection evidence candidate count changed",
        ));
    }
    parents
        .iter()
        .zip(evidence)
        .map(|(birth, evidence)| {
            let expected_identity = Era1SelectionCandidateIdentity {
                organism_id: birth.organism_id,
                genome_id: birth.genome.id,
                parent_genome_ids: birth.genome.parent_genome_ids.clone(),
                lineage_id: birth.genome.lineage_id,
                generation: parent_generation,
            };
            if birth.generation != parent_generation || evidence.identity != expected_identity {
                return Err(Era1EvolutionError::InvalidEvidence(
                    "selection evidence does not match the stable parent identity",
                ));
            }
            let ancestors = known_ancestors(&birth.genome, prior_generations);
            recompute_era1_selection_profile_from_receipt(
                config,
                &birth.genome,
                evidence,
                parents.len(),
                &ancestors,
            )
        })
        .collect()
}

fn known_ancestors(
    genome: &CreatureGenome,
    generations: &[Era1GenerationReceipt],
) -> Vec<GenomeId> {
    let by_id = generations
        .iter()
        .flat_map(|generation| &generation.births)
        .map(|birth| (birth.genome.id.0, &birth.genome))
        .collect::<BTreeMap<_, _>>();
    let mut pending = genome.parent_genome_ids.clone();
    let mut ancestors = BTreeSet::new();
    while let Some(parent_id) = pending.pop() {
        if ancestors.insert(parent_id.0) {
            if let Some(parent) = by_id.get(&parent_id.0) {
                pending.extend(parent.parent_genome_ids.iter().copied());
            }
        }
    }
    ancestors.into_iter().map(GenomeId).collect()
}

pub fn run_era1_evolution<F>(
    config: &Era1EvolutionConfig,
    committed_ei0_exit_gate: Option<&Ei0ExitGateReport>,
    founders: &[CreatureGenome],
    artifact_root: impl AsRef<Path>,
    mut evidence_provider: F,
) -> Result<Era1EvolutionReceipt, Era1EvolutionError>
where
    F: FnMut(
        u32,
        usize,
        &[Era1BirthReceipt],
    ) -> Result<Era1CandidateSelectionEvidence, Era1EvolutionError>,
{
    let committed_ei0_exit_gate = committed_ei0_exit_gate.ok_or_else(|| {
        Era1EvolutionError::Ei0Gate("committed EI0 exit gate is missing".to_string())
    })?;
    validate_committed_ei0_exit_gate_report(committed_ei0_exit_gate)
        .map_err(|error| Era1EvolutionError::Ei0Gate(error.to_string()))?;
    config.validate_contract()?;
    validate_founders(founders)?;
    if founders.len() != config.lineage_count {
        return Err(Era1EvolutionError::InvalidEvidence(
            "founder count does not match bounded lineages",
        ));
    }

    let artifact_root = artifact_root.as_ref();
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
    let mut generations = vec![Era1GenerationReceipt {
        generation: 0,
        births: founder_births,
        preserved_wild_genome_ids: wild_ids.clone(),
        selection_plan: None,
        habitat_breeding: Vec::new(),
        archives: Vec::new(),
        portable_save: pending_portable_save(0),
    }];
    let mut habitats = habitats;
    let mut selection_rounds = Vec::with_capacity(config.ordinary_birth_generations as usize);

    for generation in 1..=config.ordinary_birth_generations {
        let parents = &generations
            .last()
            .expect("founder generation is always present")
            .births;
        let parent_generation = generation - 1;
        let mut evidence_receipts = Vec::with_capacity(parents.len());
        for candidate_index in 0..parents.len() {
            let evidence = evidence_provider(parent_generation, candidate_index, parents)?;
            let parent = &parents[candidate_index];
            let expected_identity = Era1SelectionCandidateIdentity {
                organism_id: parent.organism_id,
                genome_id: parent.genome.id,
                parent_genome_ids: parent.genome.parent_genome_ids.clone(),
                lineage_id: parent.genome.lineage_id,
                generation: parent_generation,
            };
            if evidence.identity != expected_identity {
                return Err(Era1EvolutionError::InvalidEvidence(
                    "selection evidence does not match the stable parent identity",
                ));
            }
            evidence_receipts.push(validate_completed_selection_evidence(
                config,
                &parent.genome,
                &evidence,
            )?);
        }
        let profiles = derive_selection_round_profiles_from_receipts(
            config,
            parent_generation,
            parents,
            &evidence_receipts,
            &generations,
        )?;
        let candidates = selection_candidates(&wild_reservoir, parents, &profiles)?;
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
        births.sort_by_key(|birth| birth.lineage_slot);
        generations.push(Era1GenerationReceipt {
            generation,
            births,
            preserved_wild_genome_ids: wild_ids.clone(),
            selection_plan: Some(selection_plan),
            habitat_breeding,
            archives: Vec::new(),
            portable_save: pending_portable_save(generation),
        });
        selection_rounds.push(Era1SelectionRoundReceipt {
            parent_generation,
            evidence: evidence_receipts,
            derived_profiles: profiles,
        });
    }

    // Evidence for every selection round is complete before the first archive or save write.
    // A missing, invalid, or UNKNOWN receipt therefore cannot leave evolution artifacts behind.
    std::fs::create_dir_all(artifact_root)?;
    let mut library = LineageLibrary::open(LineageLibraryConfig::profile_default(
        artifact_root.join("lineage-library"),
    ))?;
    for generation in &mut generations {
        generation.archives =
            archive_births(&mut library, generation.generation, &generation.births)?;
        generation.portable_save = persist_generation_save(
            artifact_root,
            config.evolution_seed,
            generation.generation,
            &generation.births,
            &habitats,
        )?;
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
        baseline_ei0_exit_gate: committed_ei0_exit_gate.clone(),
        wild_reservoir,
        selection_rounds,
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

fn pending_portable_save(generation: u32) -> Era1PortableSaveReceipt {
    Era1PortableSaveReceipt {
        generation,
        relative_path: String::new(),
        digest_hex: String::new(),
        organism_ids: Vec::new(),
        genome_ids: Vec::new(),
    }
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
        minority_lineage_share_max: 0.25,
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
        if profile.identity.organism_id != birth.organism_id
            || profile.identity.genome_id != birth.genome.id
            || profile.identity.parent_genome_ids != birth.genome.parent_genome_ids
            || profile.identity.lineage_id != birth.genome.lineage_id
            || profile.identity.generation != birth.generation
        {
            return Err(Era1EvolutionError::InvalidEvidence(
                "derived selection profile does not match managed birth",
            ));
        }
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

fn valid_git_object_id(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_selection_keeps_enough_distinct_parents_for_two_pairs() {
        let foundation = alife_core::FoundationGeneticIdentity::new(
            0x4E32_3034_385F_5631,
            1,
            0x4E32_3034_385F_FA11,
            BrainCapacityClass::N2048_ID,
        )
        .unwrap();
        let candidates = [71_001, 71_002, 71_003, 71_004]
            .into_iter()
            .enumerate()
            .map(|(index, seed)| {
                let ecological = if index == 1 { 0.424 } else { 0.419 };
                let stability = if index == 1 { 0.109 } else { 0.103 };
                SelectionCandidate {
                    genome: CreatureGenome::early_mammal_founder(seed, foundation).unwrap(),
                    objectives: ObjectiveVector {
                        ecological: ScoreEstimate::known(ecological, 6),
                        cognitive: ScoreEstimate::known(0.0, 6),
                        social: ScoreEstimate::known(0.0, 6),
                        group: ScoreEstimate::known(0.0, 6),
                        stability: ScoreEstimate::known(stability, 6),
                        efficiency: ScoreEstimate::known(0.20, 6),
                        diversity: ScoreEstimate::known(0.0, 6),
                    },
                    known_ancestor_genome_ids: Vec::new(),
                    population_share: 0.25,
                    lane: PopulationLane::Managed,
                    specialist_roles: Vec::new(),
                }
            })
            .collect::<Vec<_>>();
        let config = Era1EvolutionConfig::bounded_default(0xE1_6001).unwrap();

        let plan = run_managed_selection(&candidates, &selection_config(&config, 1)).unwrap();

        assert_eq!(plan.pairings.len(), 2);
        let parent_ids = plan
            .pairings
            .iter()
            .flat_map(|pairing| [pairing.maternal_genome_id.0, pairing.paternal_genome_id.0])
            .collect::<BTreeSet<_>>();
        assert_eq!(parent_ids.len(), 4);
    }
}
