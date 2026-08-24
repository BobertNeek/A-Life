//! Production-GPU evolutionary hardening for the trained N2048 foundation.

use std::cmp::Ordering;

use alife_core::{
    ActionCandidate, ActionKind, ActionTarget, BodySnapshot, BrainCapacityClass, BrainGenome,
    CandidateActionFamily, CandidateObservationRef, Confidence, DecisionSnapshot, DevelopmentState,
    DurationTicks, EndocrineDelta, ExperiencePatch, ExperiencePatchBuilder, ExperienceSequenceId,
    FoundationPromotionReceipt, FoundationWeightAsset, GroundedObjectSlotV1, HomeostaticDelta,
    HomeostaticSnapshot, NeuralActionSelection, NormalizedScalar, OrganismId, PerceptionFrame,
    PhenotypeCompiler, PhysicalActionOutcome, PhysicalContactKind, Pose, PostActionOutcome,
    PreActionSnapshot, ScaffoldContractError, SensorProfile, SensorProfileProvenance,
    SensoryAbiVersion, SensoryChannels, SensorySnapshot, SignedValence, Tick, TrackedObjectId,
    Vec3f, Velocity,
};
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};
use alife_runtime::{GpuAuthoritativeSession, GpuSessionConsumerKind};

use crate::{
    AdamWConfig, FoundationCurriculumStage, FoundationTrainer, N2048CurriculumV1,
    N2048FoundationProgram, StageTrainableMask, TrainingError, N2048_FOUNDATION_TRAINING_SEED,
};

pub const HARDENING_NEWBORNS_PER_GENOME: u32 = 4;
pub const HARDENING_WORLD_COUNT: u32 = 4;
pub const HARDENING_TICKS_PER_WORLD: u32 = 8;
pub const HARDENING_DESCENDANTS_PER_FINALIST: u32 = 8;
const MAX_FINALISTS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HardeningMutationKind {
    Baseline,
    SparseGeneticDelta,
    RouteDensity,
    PlasticityReceptor,
    AlphaPlasticity,
    BiasLeakResearch,
    BiochemicalSensitivity,
    DevelopmentalGate,
}

impl HardeningMutationKind {
    pub const ALL_MUTATIONS: [Self; 7] = [
        Self::SparseGeneticDelta,
        Self::RouteDensity,
        Self::PlasticityReceptor,
        Self::AlphaPlasticity,
        Self::BiasLeakResearch,
        Self::BiochemicalSensitivity,
        Self::DevelopmentalGate,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HardeningFitness {
    pub survival: f32,
    pub learning: f32,
    pub language_acquisition: f32,
    pub narration_fidelity: f32,
    pub mutation_robustness: f32,
    pub compute_efficiency: f32,
}

impl HardeningFitness {
    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        if [
            self.survival,
            self.learning,
            self.language_acquisition,
            self.narration_fidelity,
            self.mutation_robustness,
            self.compute_efficiency,
        ]
        .into_iter()
        .all(|value| value.is_finite() && (0.0..=1.0).contains(&value))
        {
            Ok(self)
        } else {
            Err(ScaffoldContractError::ScalarOutOfRange)
        }
    }

    pub fn dominates(self, other: Self) -> bool {
        let left = self.objectives();
        let right = other.objectives();
        left.iter().zip(right).all(|(a, b)| *a >= b) && left.iter().zip(right).any(|(a, b)| *a > b)
    }

    fn objectives(self) -> [f32; 6] {
        [
            self.survival,
            self.learning,
            self.language_acquisition,
            self.narration_fidelity,
            self.mutation_robustness,
            self.compute_efficiency,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HardeningEvaluation {
    pub genome: BrainGenome,
    pub mutation: HardeningMutationKind,
    pub viable: bool,
    pub nonviable_reason: Option<String>,
    pub newborn_count: u32,
    pub world_count: u32,
    pub neural_ticks: u32,
    pub fitness: Option<HardeningFitness>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FoundationHardeningReceipt {
    pub adapter_name: String,
    pub backend_api: String,
    pub evaluated_genomes: u32,
    pub nonviable_genomes: u32,
    pub pareto_finalists: Vec<alife_core::GenomeId>,
    pub descendants_per_finalist: u32,
    pub descendant_evaluations: u32,
    pub winner_genome_id: alife_core::GenomeId,
    pub winner_fitness: HardeningFitness,
    pub curated_regression_stage_count: u16,
}

#[derive(Debug, Clone)]
pub struct FoundationHardeningOutcome {
    pub promoted_foundation: FoundationWeightAsset,
    pub elite_genome: BrainGenome,
    pub receipt: FoundationHardeningReceipt,
}

pub fn pareto_front(evaluations: &[HardeningEvaluation]) -> Vec<usize> {
    evaluations
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.viable && candidate.fitness.is_some())
        .filter(|(index, candidate)| {
            let fitness = candidate.fitness.expect("filtered viable fitness");
            !evaluations.iter().enumerate().any(|(other_index, other)| {
                other_index != *index
                    && other.viable
                    && other
                        .fitness
                        .is_some_and(|other_fitness| other_fitness.dominates(fitness))
            })
        })
        .map(|(index, _)| index)
        .collect()
}

pub fn mutate_hardening_genome(
    parent: &BrainGenome,
    mutation: HardeningMutationKind,
    mutation_seed: u64,
) -> Result<BrainGenome, ScaffoldContractError> {
    if parent.brain_class_id != BrainCapacityClass::N2048_ID || mutation_seed == 0 {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    if mutation == HardeningMutationKind::Baseline {
        return Ok(parent.clone());
    }
    let species_seed = splitmix64(
        parent
            .species_seed
            .wrapping_add(splitmix64(mutation_seed))
            .wrapping_add(mutation_tag(mutation)),
    );
    let mut child = BrainGenome::scaffold(species_seed, parent.brain_class_id);
    child.parent_genome_ids = vec![parent.id];
    child.lineage_id = parent.lineage_id;
    match mutation {
        HardeningMutationKind::Baseline | HardeningMutationKind::SparseGeneticDelta => {}
        HardeningMutationKind::RouteDensity => {
            let row = child
                .sparse_density_priors
                .first_mut()
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            row.density = NormalizedScalar::new((row.density.raw() * 1.05).min(1.0))?;
        }
        HardeningMutationKind::PlasticityReceptor => {
            let source = *child.plasticity_parameters();
            let mut receptor_weights = *source.receptor_profile().weights();
            receptor_weights[6] = (receptor_weights[6] + 0.05).min(2.0);
            child = child.with_plasticity_parameters(
                alife_core::PlasticityGenomeParameters::try_new(
                    source.eligibility_decay(),
                    (source.base_learning_rate() * 1.1).min(1.0),
                    source.normalization_rate(),
                    source.sleep_replay_rate(),
                    alife_core::PlasticityReceptorProfile::try_new(receptor_weights)?,
                    source.fast_bounds().0,
                    source.fast_bounds().1,
                    source.sleep_staging_rate(),
                    source.sleep_weight_limit(),
                    source.sleep_fast_decay_rate(),
                )?,
            )?;
        }
        HardeningMutationKind::AlphaPlasticity => {
            child.alpha_mask.default_alpha =
                NormalizedScalar::new((child.alpha_mask.default_alpha.raw() + 0.025).min(1.0))?;
        }
        HardeningMutationKind::BiasLeakResearch => {}
        HardeningMutationKind::BiochemicalSensitivity => {
            // The brain genome may evolve neural receptor expression, but it
            // does not own endocrine production or biochemical state.
            let source = *child.plasticity_parameters();
            let mut receptor_weights = *source.receptor_profile().weights();
            receptor_weights[7] = (receptor_weights[7] - 0.05).max(-2.0);
            child = child.with_plasticity_parameters(
                alife_core::PlasticityGenomeParameters::try_new(
                    source.eligibility_decay(),
                    source.base_learning_rate(),
                    source.normalization_rate(),
                    source.sleep_replay_rate(),
                    alife_core::PlasticityReceptorProfile::try_new(receptor_weights)?,
                    source.fast_bounds().0,
                    source.fast_bounds().1,
                    source.sleep_staging_rate(),
                    source.sleep_weight_limit(),
                    source.sleep_fast_decay_rate(),
                )?,
            )?;
        }
        HardeningMutationKind::DevelopmentalGate => {
            child.developmental_schedule.sleep_pressure_maturation_gate =
                NormalizedScalar::new(0.30)?;
        }
    }
    alife_core::Validate::validate_contract(&child)?;
    Ok(child)
}

pub struct N2048EvolutionHardener {
    session: GpuAuthoritativeSession,
    source: FoundationWeightAsset,
    capacity: BrainCapacityClass,
    next_organism_id: u64,
    next_sequence_id: u64,
}

impl N2048EvolutionHardener {
    pub fn new_required(source: FoundationWeightAsset) -> Result<Self, TrainingError> {
        if source.manifest().training_stage().completed_stage_count()
            != FoundationCurriculumStage::ALL.len() as u16
        {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
        let session = GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Evolution);
        Ok(Self {
            session,
            source,
            capacity: BrainCapacityClass::n2048(),
            next_organism_id: 1,
            next_sequence_id: 1,
        })
    }

    pub const fn consumer_kind(&self) -> GpuSessionConsumerKind {
        self.session.authority().consumer()
    }

    pub fn evaluate_genome(
        &mut self,
        genome: BrainGenome,
        mutation: HardeningMutationKind,
    ) -> Result<HardeningEvaluation, TrainingError> {
        if mutation == HardeningMutationKind::BiasLeakResearch {
            return Ok(HardeningEvaluation {
                genome,
                mutation,
                viable: false,
                nonviable_reason: Some(
                    "the frozen N2048 genome has no heritable neuron bias/leak lane".to_owned(),
                ),
                newborn_count: 0,
                world_count: 0,
                neural_ticks: 0,
                fitness: None,
            });
        }
        let development = DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0)?);
        let phenotype = match PhenotypeCompiler::compile_from_foundation_asset(
            &genome,
            &self.capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
            &self.source,
        ) {
            Ok(phenotype) => phenotype,
            Err(error) => {
                return Ok(HardeningEvaluation {
                    genome,
                    mutation,
                    viable: false,
                    nonviable_reason: Some(error.to_string()),
                    newborn_count: 0,
                    world_count: 0,
                    neural_ticks: 0,
                    fitness: None,
                });
            }
        };

        let mut handles = Vec::with_capacity(HARDENING_NEWBORNS_PER_GENOME as usize);
        for _ in 0..HARDENING_NEWBORNS_PER_GENOME {
            let organism = OrganismId(self.next_organism_id);
            self.next_organism_id = self.next_organism_id.saturating_add(1);
            handles.push(self.session.insert_brain(organism, phenotype.clone())?);
        }
        let mut correct_by_tick = [0_u32; 2];
        let mut survival_correct = 0_u32;
        let mut survival_total = 0_u32;
        let mut language_correct = 0_u32;
        let mut narration_correct = 0_u32;
        let mut language_total = 0_u32;
        let mut total_cost_q24 = 0_u64;
        for tick_index in 0..HARDENING_TICKS_PER_WORLD {
            let frames = handles
                .iter()
                .enumerate()
                .map(|(world, handle)| {
                    challenge_frame(
                        handle.organism_id(),
                        Tick::new(u64::from(tick_index) + 1),
                        world as u32,
                        tick_index,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let batch = handles
                .iter()
                .copied()
                .zip(frames.iter().cloned())
                .collect::<Vec<_>>();
            let ticks = self.session.tick_batch(&batch)?;
            let mut patches = Vec::with_capacity(ticks.len());
            for (world, ((handle, frame), tick)) in handles
                .iter()
                .copied()
                .zip(frames.iter())
                .zip(ticks.iter())
                .enumerate()
            {
                let expected = expected_candidate_index(world as u32, tick_index);
                let correct = u16::from(tick.selection.candidate_index == expected);
                correct_by_tick[usize::from(tick_index >= HARDENING_TICKS_PER_WORLD / 2)] +=
                    u32::from(correct);
                if world < 3 {
                    survival_total += 1;
                    survival_correct += u32::from(correct);
                } else {
                    language_total += 1;
                    language_correct += u32::from(correct);
                    narration_correct += u32::from(
                        correct != 0
                            && tick
                                .speech_payload
                                .as_ref()
                                .is_some_and(|payload| !payload.tokens.is_empty()),
                    );
                }
                total_cost_q24 = total_cost_q24.saturating_add(tick.work.neural_cost_q24);
                let patch_development =
                    DevelopmentState::new(genome.id, frame.tick(), NormalizedScalar::new(1.0)?);
                patches.push(sealed_outcome(
                    handle,
                    &genome,
                    &patch_development,
                    frame,
                    tick,
                    ExperienceSequenceId(self.next_sequence_id),
                    correct != 0,
                )?);
                self.next_sequence_id = self.next_sequence_id.saturating_add(1);
            }
            for handle in &handles {
                let pending = self
                    .session
                    .pending_eligibility(*handle)?
                    .ok_or(ScaffoldContractError::MissingPhaseData)?;
                self.session
                    .discard_pending_eligibility(*handle, pending.identity())?;
            }
            require_canonical_chemistry_for_hardening()?;
        }
        for handle in handles {
            self.session.remove_brain(handle)?;
        }

        let half_total = HARDENING_NEWBORNS_PER_GENOME * (HARDENING_TICKS_PER_WORLD / 2);
        let first = correct_by_tick[0] as f32 / half_total as f32;
        let second = correct_by_tick[1] as f32 / half_total as f32;
        let average_cost = total_cost_q24 as f32
            / (HARDENING_NEWBORNS_PER_GENOME * HARDENING_TICKS_PER_WORLD) as f32;
        let fitness = HardeningFitness {
            survival: survival_correct as f32 / survival_total as f32,
            learning: (0.5 + (second - first) * 0.5).clamp(0.0, 1.0),
            language_acquisition: language_correct as f32 / language_total as f32,
            narration_fidelity: narration_correct as f32 / language_total as f32,
            mutation_robustness: 0.0,
            compute_efficiency: 1.0 / (1.0 + average_cost / 16_777_216.0),
        }
        .validate()?;
        Ok(HardeningEvaluation {
            genome,
            mutation,
            viable: true,
            nonviable_reason: None,
            newborn_count: HARDENING_NEWBORNS_PER_GENOME,
            world_count: HARDENING_WORLD_COUNT,
            neural_ticks: HARDENING_NEWBORNS_PER_GENOME * HARDENING_TICKS_PER_WORLD,
            fitness: Some(fitness),
        })
    }

    pub fn harden_one_generation(
        mut self,
        generation_seed: u64,
    ) -> Result<FoundationHardeningOutcome, TrainingError> {
        if generation_seed == 0 {
            return Err(ScaffoldContractError::InvalidId.into());
        }
        let parent = BrainGenome::scaffold(N2048_FOUNDATION_TRAINING_SEED, self.capacity.id());
        let mut genomes = vec![(parent.clone(), HardeningMutationKind::Baseline)];
        for (index, mutation) in HardeningMutationKind::ALL_MUTATIONS.into_iter().enumerate() {
            genomes.push((
                mutate_hardening_genome(
                    &parent,
                    mutation,
                    generation_seed.wrapping_add(index as u64 + 1),
                )?,
                mutation,
            ));
        }
        let mut evaluations = Vec::with_capacity(genomes.len());
        for (genome, mutation) in genomes {
            evaluations.push(self.evaluate_genome(genome, mutation)?);
        }
        let initial_nonviable = evaluations.iter().filter(|row| !row.viable).count() as u32;
        let mut finalists = pareto_front(&evaluations);
        finalists.sort_by(|a, b| compare_fitness(&evaluations[*b], &evaluations[*a]));
        finalists.truncate(MAX_FINALISTS);
        if finalists.is_empty() {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }

        let mut descendant_evaluations = 0_u32;
        let mut descendant_nonviable = 0_u32;
        for finalist_index in &finalists {
            let finalist_genome = evaluations[*finalist_index].genome.clone();
            let mut viable_descendants = 0_u32;
            let mut descendant_survival = 0.0_f32;
            for descendant in 0..HARDENING_DESCENDANTS_PER_FINALIST {
                let genome = mutate_hardening_genome(
                    &finalist_genome,
                    HardeningMutationKind::SparseGeneticDelta,
                    generation_seed
                        .wrapping_add(0xD35C_0000)
                        .wrapping_add(u64::from(descendant)),
                )?;
                let evaluation =
                    self.evaluate_genome(genome, HardeningMutationKind::SparseGeneticDelta)?;
                descendant_evaluations += 1;
                if let Some(fitness) = evaluation.fitness {
                    viable_descendants += 1;
                    descendant_survival += fitness.survival;
                } else {
                    descendant_nonviable += 1;
                }
            }
            let robustness = if viable_descendants == 0 {
                0.0
            } else {
                descendant_survival / viable_descendants as f32
            };
            evaluations[*finalist_index]
                .fitness
                .as_mut()
                .expect("Pareto finalist has fitness")
                .mutation_robustness = robustness;
        }
        finalists.sort_by(|a, b| compare_fitness(&evaluations[*b], &evaluations[*a]));
        let winner = evaluations[finalists[0]].clone();
        let winner_fitness = winner.fitness.expect("winner has fitness").validate()?;

        let winner_development =
            DevelopmentState::new(winner.genome.id, Tick::ZERO, NormalizedScalar::new(1.0)?);
        let winner_phenotype = PhenotypeCompiler::compile_from_foundation_asset(
            &winner.genome,
            &self.capacity,
            &winner_development,
            SensorProfile::GroundedObjectSlotsV1,
            &self.source,
        )?;
        let training_stage = self.source.manifest().training_stage();
        let evaluated_genetic_asset = FoundationWeightAsset::from_trained_weights(
            &winner_phenotype,
            winner_phenotype
                .synapses()
                .iter()
                .map(|synapse| synapse.genetic_weight())
                .collect(),
            training_stage,
        )?;
        let baseline_development =
            DevelopmentState::new(parent.id, Tick::ZERO, NormalizedScalar::new(1.0)?);
        let baseline_phenotype = PhenotypeCompiler::compile_from_foundation_asset(
            &parent,
            &self.capacity,
            &baseline_development,
            SensorProfile::GroundedObjectSlotsV1,
            &self.source,
        )?;
        let promotion = FoundationPromotionReceipt::promoted(
            training_stage.digest(),
            evaluated_genetic_asset.digest(),
            self.source.digest(),
        )?;
        let promoted_foundation = FoundationWeightAsset::from_promoted_weights(
            &baseline_phenotype,
            self.source.weights().to_vec(),
            training_stage,
            promotion,
        )?;

        let regression_phenotype = PhenotypeCompiler::compile_from_foundation_asset(
            &parent,
            &self.capacity,
            &baseline_development,
            SensorProfile::GroundedObjectSlotsV1,
            &promoted_foundation,
        )?;
        let curriculum = N2048CurriculumV1::new();
        let mask: StageTrainableMask = curriculum.stage_mask(
            &regression_phenotype,
            FoundationCurriculumStage::HeldOutGeneralization,
        )?;
        let trainer = FoundationTrainer::from_session(
            self.session,
            regression_phenotype,
            promoted_foundation.clone(),
            mask,
            AdamWConfig::default(),
        )?;
        let regression = N2048FoundationProgram::resume(trainer)?;
        let curated_regression_stage_count = regression.completed_stage_count();

        let hardware = regression.trainer().source_foundation();
        debug_assert_eq!(hardware.digest(), promoted_foundation.digest());
        let adapter = regression.trainer().hardware_receipt();
        let winner_genome_id = winner.genome.id;
        Ok(FoundationHardeningOutcome {
            promoted_foundation,
            elite_genome: winner.genome,
            receipt: FoundationHardeningReceipt {
                adapter_name: adapter.adapter_name.clone(),
                backend_api: adapter.backend_api.clone(),
                evaluated_genomes: evaluations.len() as u32,
                nonviable_genomes: initial_nonviable + descendant_nonviable,
                pareto_finalists: finalists
                    .iter()
                    .map(|index| evaluations[*index].genome.id)
                    .collect(),
                descendants_per_finalist: HARDENING_DESCENDANTS_PER_FINALIST,
                descendant_evaluations,
                winner_genome_id,
                winner_fitness,
                curated_regression_stage_count,
            },
        })
    }
}

fn require_canonical_chemistry_for_hardening() -> Result<(), TrainingError> {
    Err(ScaffoldContractError::MissingPhaseData.into())
}

fn compare_fitness(left: &HardeningEvaluation, right: &HardeningEvaluation) -> Ordering {
    let left = left.fitness.expect("finalist fitness").objectives();
    let right = right.fitness.expect("finalist fitness").objectives();
    left.into_iter()
        .zip(right)
        .find_map(|(a, b)| {
            a.partial_cmp(&b)
                .filter(|ordering| *ordering != Ordering::Equal)
        })
        .unwrap_or(Ordering::Equal)
}

fn expected_candidate_index(world: u32, tick: u32) -> u16 {
    ((tick / 2 + world) % 2) as u16
}

fn challenge_actions(
    world: u32,
) -> (
    (ActionKind, CandidateActionFamily),
    (ActionKind, CandidateActionFamily),
) {
    match world {
        0 => (
            (ActionKind::Interact, CandidateActionFamily::Ingest),
            (ActionKind::Move, CandidateActionFamily::Avoid),
        ),
        1 => (
            (ActionKind::Move, CandidateActionFamily::Avoid),
            (ActionKind::Move, CandidateActionFamily::Approach),
        ),
        2 => (
            (ActionKind::Rest, CandidateActionFamily::Rest),
            (ActionKind::Move, CandidateActionFamily::Approach),
        ),
        _ => (
            (ActionKind::Vocalize, CandidateActionFamily::Other),
            (ActionKind::Idle, CandidateActionFamily::Idle),
        ),
    }
}

fn challenge_frame(
    organism_id: OrganismId,
    tick: Tick,
    world: u32,
    tick_index: u32,
) -> Result<PerceptionFrame, ScaffoldContractError> {
    let expected = expected_candidate_index(world, tick_index);
    let (target, distractor) = challenge_actions(world);
    let mut channels = SensoryChannels::ZERO;
    channels.auditory_acoustic[0] = if world == 3 { 0.8 } else { 0.0 };
    channels.novelty_signal = NormalizedScalar::new(0.4)?;
    let sensory =
        SensorySnapshot::new(organism_id, tick, Vec3f::ZERO, channels, Default::default())?;
    let mut slots = Vec::with_capacity(2);
    let mut candidates = Vec::with_capacity(2);
    for index in 0..2_u16 {
        let is_target = index == expected;
        let slot = GroundedObjectSlotV1 {
            slot_index: index,
            tracked_object_id: TrackedObjectId(
                organism_id.raw() * 1_000 + tick.raw() * 2 + u64::from(index) + 1,
            ),
            bearing: [if is_target { 0.15 } else { -0.75 }, 0.0],
            distance: if is_target { 0.2 } else { 0.8 },
            relative_velocity: [0.0; 3],
            color: if is_target { [0.8, 0.4, 0.2] } else { [0.1; 3] },
            material: if is_target { [0.7, 0.2, 0.1] } else { [0.1; 3] },
            shape: if is_target { [0.6, 0.3, 0.2] } else { [0.1; 3] },
            chemical: if is_target {
                [0.75, 0.25, 0.1]
            } else {
                [0.05; 3]
            },
            contact: f32::from(is_target),
            proprioception: if is_target { [0.6, 0.2] } else { [0.1; 2] },
            temperature: if world == 1 && is_target { 0.8 } else { 0.2 },
            terrain: if is_target { [0.2, 0.8] } else { [0.8, 0.2] },
            confidence: Confidence::new(0.9)?,
        };
        let action = if is_target { target } else { distractor };
        let observation = if action.1 == CandidateActionFamily::Idle {
            CandidateObservationRef::None
        } else {
            CandidateObservationRef::ObjectSlot(index)
        };
        let features = if action.1 == CandidateActionFamily::Idle {
            alife_core::CandidateFeatureVector::zero()
        } else {
            slot.candidate_features()?
        };
        candidates.push(ActionCandidate::new(
            index,
            action.0.canonical_id(),
            action.0,
            action.1,
            observation,
            ActionTarget::NONE,
            features,
            Confidence::new(0.9)?,
            NormalizedScalar::new(0.1)?,
            DurationTicks::new(1),
            DurationTicks::new(1),
        )?);
        slots.push(slot);
    }
    PerceptionFrame::new(
        organism_id,
        tick,
        SensorProfile::GroundedObjectSlotsV1,
        sensory,
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        candidates,
        SensorProfileProvenance::new(
            SensorProfile::GroundedObjectSlotsV1,
            SensoryAbiVersion::CURRENT,
            tick,
        )?,
        slots,
    )
}

fn sealed_outcome(
    handle: alife_gpu_backend::GpuBrainHandle,
    genome: &BrainGenome,
    development: &DevelopmentState,
    frame: &PerceptionFrame,
    tick: &alife_gpu_backend::GpuClosedLoopTick,
    sequence_id: ExperienceSequenceId,
    correct: bool,
) -> Result<ExperiencePatch, ScaffoldContractError> {
    let selection = NeuralActionSelection {
        candidate_index: tick.selection.candidate_index,
        logit: tick.selection.logit,
        confidence: tick.selection.confidence,
        active_tiles: tick.selection.active_tiles,
        active_synapses: tick.selection.active_synapses,
    };
    let candidate = frame.candidates()[usize::from(selection.candidate_index)];
    let command = candidate.to_command(handle.organism_id(), selection.confidence)?;
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence_id,
        handle.class_id(),
        handle.phenotype_hash(),
        genome.id,
        genome.schema_version,
        development.clone(),
        frame.clone(),
    )?;
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id,
        handle.phenotype_hash(),
        tick.dispatch_generation,
        tick.active_activation_side,
        frame,
        selection,
        command,
    )?;
    let outcome = PostActionOutcome::new(
        handle.organism_id(),
        sequence_id,
        Tick::new(frame.tick().raw().saturating_add(1)),
        correct,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::None,
            target_entity: None,
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.05)?,
        },
        HomeostaticDelta {
            drives: alife_core::DriveDelta::zero(),
            hormones: EndocrineDelta::zero(),
        },
        SignedValence::new(if correct { 0.5 } else { -0.25 })?,
        NormalizedScalar::new(0.0)?,
        NormalizedScalar::new(if correct { 0.0 } else { 0.5 })?,
        SignedValence::new(0.0)?,
        NormalizedScalar::new(0.0)?,
    )?;
    ExperiencePatchBuilder::new(sequence_id)
        .record_pre_action(pre_action)?
        .record_decision(decision)?
        .record_outcome(outcome)?
        .seal()
}

const fn mutation_tag(mutation: HardeningMutationKind) -> u64 {
    mutation as u64 + 0x4841_5244_454E_0000
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    let value = value ^ (value >> 31);
    if value == 0 {
        1
    } else {
        value
    }
}
