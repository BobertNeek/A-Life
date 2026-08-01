//! GPU-authoritative causal trial loop for the Era 1 Norn-plus battery.

use alife_core::{
    ActionKind, BrainCapacityClass, BrainGenome, CandidateActionFamily, CanonicalDigestBuilder,
    Confidence, ConsolidationIntent, CreatureGenome, DecisionSnapshot, DevelopmentState,
    Era1Ability, Era1Control, Era1EvidencePartition, Era1TrialIdentity, Era1TrialReceipt,
    ExperiencePatchBuilder, ExperienceSequenceId, FoundationWeightAsset, HomeostaticParameters,
    HomeostaticSnapshot, MemoryBankConfig, MemorySidecarState, MetricReading,
    NeuralActionSelection, OrganismId, PerceptionFrameDigest, PhenotypeCompiler, PhenotypeHash,
    PolicyBackend, PostActionOutcome, PreActionSnapshot, ScaffoldContractError, SensorProfile,
    SensorProfileIdentity, SensoryAbiVersion, Tick, UtteranceGroundingReceiptV2,
    UtteranceSourceKind, Validate,
};
use alife_gpu_backend::{
    GpuBrainHandle, GpuClosedLoopBackend, GpuClosedLoopMemoryBatchInput,
    GpuClosedLoopMemoryTickInput, GpuRuntimeProfile,
};
use alife_runtime::{GpuAuthoritativeSession, GpuSessionConsumerKind};
use alife_world::{
    apply_era1_world_transition, build_era1_trial_world, Era1TrialManifest, Era1TrialPhase,
    Era1WorldFamily, HeadlessWorld, WorldObjectKind, ERA1_ACQUISITION_END_TICK,
    ERA1_PROBE_START_TICK, ERA1_TRIAL_END_TICK,
};

use crate::TrainingError;

const ERA1_MEMORY_CAPACITY: usize = 64;
const ERA1_MEMORY_MAX_FEATURE_LEN: usize = 64;
const ERA1_MEMORY_MAX_MATCH_COUNT: usize = 4;
const ERA1_MEMORY_MIN_MATCH_SCORE: f32 = 0.72;
const PERCEPTION_DIGEST_DOMAIN: &[u8] = b"alife.era1.perception-evidence.v1";
const SEALED_DIGEST_DOMAIN: &[u8] = b"alife.era1.sealed-evidence.v1";

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Era1LearningDisposition {
    Applied = 1,
    Discarded = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Era1CausalStepReceipt {
    pub organism_id: OrganismId,
    pub phenotype_hash: PhenotypeHash,
    pub sequence_id: ExperienceSequenceId,
    pub tick: Tick,
    pub world_before_digest: [u64; 4],
    pub world_after_action_digest: [u64; 4],
    pub frame_digest: PerceptionFrameDigest,
    pub memory_organism_id: OrganismId,
    pub memory_bank_digest: [u64; 4],
    pub memory_context_final_digest: PerceptionFrameDigest,
    pub pending_frame_digest: PerceptionFrameDigest,
    pub pending_receipt_digest: [u64; 4],
    pub learning: Era1LearningDisposition,
    pub memory_observed: bool,
    pub peer_visible: bool,
    pub outcome_success: bool,
    pub phase: Era1TrialPhase,
    pub selected_action: ActionKind,
    pub selected_family: CandidateActionFamily,
    pub target_kind: Option<WorldObjectKind>,
    pub behavior_success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Era1LearningAssessment {
    pub early_acquisition: MetricReading,
    pub late_acquisition: MetricReading,
    pub delay: MetricReading,
    pub probe: MetricReading,
    pub acquisition_improvement_q16: i32,
    pub probe_change_from_early_q16: i32,
    pub demonstrated: bool,
    pub grounding_receipts: Vec<UtteranceGroundingReceiptV2>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Era1TrialRunEvidence {
    pub receipt: Era1TrialReceipt,
    pub initial_world_digest: [u64; 4],
    pub steps: Vec<Era1CausalStepReceipt>,
    pub gpu_dispatches: u64,
    pub sealed_outcomes: u64,
    pub memory_context_dispatches: u64,
    pub learning_commits: u64,
    pub eligibility_discards: u64,
    pub memory_updates: u64,
    pub sleep_commits: u32,
    pub social_context_present: bool,
    pub adapter_name: String,
    pub backend_api: String,
    pub learning_assessment: Era1LearningAssessment,
}

impl Era1TrialRunEvidence {
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.receipt.validate_contract()?;
        let step_count = u64::try_from(self.steps.len())
            .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
        if self.initial_world_digest == [0; 4]
            || self.receipt.world_digest != self.initial_world_digest
            || self.gpu_dispatches != step_count
            || self.gpu_dispatches != self.sealed_outcomes
            || self.gpu_dispatches != self.memory_context_dispatches
            || self.gpu_dispatches != ERA1_TRIAL_END_TICK
            || self.adapter_name != self.receipt.adapter_name
            || self.backend_api != self.receipt.backend_api
            || aggregate_perception(&self.steps) != self.receipt.perception_digest
            || aggregate_sealed(&self.steps) != self.receipt.sealed_evidence_digest
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }

        let expected_assessment = assess_learning(
            self.receipt.ability,
            &self.steps,
            self.learning_assessment.grounding_receipts.clone(),
        )?;
        if self.learning_assessment != expected_assessment
            || self.receipt.score
                != score_for_partition(self.receipt.partition, &expected_assessment)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }

        for (index, step) in self.steps.iter().enumerate() {
            let sequence = u64::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
            if step.organism_id != self.receipt.identity.organism_id
                || step.memory_organism_id != self.receipt.identity.organism_id
                || step.phenotype_hash != self.receipt.phenotype_hash
                || step.sequence_id.raw() != sequence
                || step.tick.raw() != sequence - 1
                || step.world_before_digest == [0; 4]
                || step.world_after_action_digest == [0; 4]
                || step.frame_digest.0 == [0; 4]
                || step.frame_digest != step.pending_frame_digest
                || step.frame_digest != step.memory_context_final_digest
                || step.memory_bank_digest == [0; 4]
                || step.pending_receipt_digest == [0; 4]
                || step.phase != phase_for_tick(step.tick)
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
        }

        match self.receipt.control {
            Era1Control::PlasticityDisabled => {
                if self.learning_commits != 0
                    || self.eligibility_discards != step_count
                    || self
                        .steps
                        .iter()
                        .any(|step| step.learning != Era1LearningDisposition::Discarded)
                {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
            }
            _ => {
                if self.learning_commits != step_count
                    || self.eligibility_discards != 0
                    || self
                        .steps
                        .iter()
                        .any(|step| step.learning != Era1LearningDisposition::Applied)
                {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
            }
        }

        if self.receipt.control == Era1Control::MemoryDisabled {
            if self.memory_updates != 0
                || self.steps.iter().any(|step| step.memory_observed)
                || self
                    .steps
                    .windows(2)
                    .any(|pair| pair[0].memory_bank_digest != pair[1].memory_bank_digest)
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
        } else if self.memory_updates != step_count
            || self.steps.iter().any(|step| !step.memory_observed)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }

        if self.receipt.control == Era1Control::SocialDisabled
            && (self.social_context_present || self.steps.iter().any(|step| step.peer_visible))
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }

        let expected_sleep = u32::from(
            self.receipt.ability == Era1Ability::PostSleepRetention
                && self.receipt.control != Era1Control::SleepDisabled,
        );
        if self.sleep_commits != expected_sleep {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

pub struct Era1TrialRunRequest<'a> {
    organism_id: OrganismId,
    generation: u32,
    genome: &'a CreatureGenome,
    manifest: &'a Era1TrialManifest,
    ability: Era1Ability,
    control: Era1Control,
    partition: Era1EvidencePartition,
    source_commit: &'a str,
    source_tree: &'a str,
}

impl<'a> Era1TrialRunRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        organism_id: OrganismId,
        generation: u32,
        genome: &'a CreatureGenome,
        manifest: &'a Era1TrialManifest,
        ability: Era1Ability,
        control: Era1Control,
        partition: Era1EvidencePartition,
        source_commit: &'a str,
        source_tree: &'a str,
    ) -> Result<Self, ScaffoldContractError> {
        let request = Self {
            organism_id,
            generation,
            genome,
            manifest,
            ability,
            control,
            partition,
            source_commit,
            source_tree,
        };
        request.validate_contract()?;
        Ok(request)
    }

    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.genome.validate_contract()?;
        self.manifest.validate_contract()?;
        let founder_shape = self.generation == 0 && self.genome.parent_genome_ids.is_empty();
        let offspring_shape = self.generation > 0 && self.genome.parent_genome_ids.len() == 2;
        if self.organism_id != self.manifest.subject
            || !(founder_shape || offspring_shape)
            || expected_family(self.ability) != self.manifest.family
            || !valid_git_object_id(self.source_commit)
            || !valid_git_object_id(self.source_tree)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

pub struct Era1TrialRunner {
    session: GpuAuthoritativeSession,
    foundation: FoundationWeightAsset,
    capacity: BrainCapacityClass,
    adapter_name: String,
    backend_api: String,
}

impl Era1TrialRunner {
    pub fn new_required() -> Result<Self, TrainingError> {
        let foundation =
            FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)?;
        let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
        let hardware = backend.hardware_receipt();
        Ok(Self {
            adapter_name: hardware.adapter_name.clone(),
            backend_api: hardware.backend_api.clone(),
            session: GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Challenge),
            foundation,
            capacity: BrainCapacityClass::n2048(),
        })
    }

    pub fn run(
        &mut self,
        request: Era1TrialRunRequest<'_>,
    ) -> Result<Era1TrialRunEvidence, TrainingError> {
        request.validate_contract()?;
        let (brain_genome, development) = self.express_compatible_creature(request.genome)?;
        let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
            &brain_genome,
            &self.capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
            &self.foundation,
        )?;
        let handle = self.session.insert_brain(request.organism_id, phenotype)?;
        let attempt = self.run_inserted(request, brain_genome, development, handle);
        if let Ok(Some(pending)) = self.session.pending_eligibility(handle) {
            let _ = self
                .session
                .discard_pending_eligibility(handle, pending.identity());
        }
        let removal = self.session.remove_brain(handle);
        match (attempt, removal) {
            (Ok(evidence), Ok(())) => Ok(evidence),
            (Err(error), _) | (_, Err(error)) => Err(error.into()),
        }
    }

    fn run_inserted(
        &mut self,
        request: Era1TrialRunRequest<'_>,
        brain_genome: BrainGenome,
        mut development: DevelopmentState,
        handle: GpuBrainHandle,
    ) -> Result<Era1TrialRunEvidence, ScaffoldContractError> {
        let mut world = build_era1_trial_world(request.manifest)?;
        if request.control == Era1Control::SocialDisabled {
            remove_peer_agents(&mut world, request.organism_id)?;
        }
        let initial_world_digest = world.canonical_signature_digest()?.words;
        let mut memory = MemorySidecarState::new_profiled(
            request.organism_id,
            SensorProfileIdentity {
                profile_id: SensorProfile::GroundedObjectSlotsV1.into(),
                profile_schema_version: 1,
                sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
            },
            MemoryBankConfig::new(
                ERA1_MEMORY_CAPACITY,
                ERA1_MEMORY_MAX_FEATURE_LEN,
                ERA1_MEMORY_MAX_MATCH_COUNT,
                ERA1_MEMORY_MIN_MATCH_SCORE,
                Confidence::new(0.0)?,
            )?,
        )?;
        let mature_age = development.age_ticks.raw();
        let mut homeostasis = HomeostaticSnapshot::baseline(world.tick());
        let mut steps = Vec::with_capacity(ERA1_TRIAL_END_TICK as usize);
        let mut learning_commits = 0_u64;
        let mut eligibility_discards = 0_u64;
        let mut memory_updates = 0_u64;
        let mut sleep_commits = 0_u32;
        let mut grounding_receipts = Vec::new();

        while world.tick().raw() < ERA1_TRIAL_END_TICK {
            if let Some(transition) = request
                .manifest
                .transitions()
                .into_iter()
                .find(|transition| transition.at_tick == world.tick())
            {
                apply_era1_world_transition(request.manifest, transition, &mut world)?;
            }
            if request.control == Era1Control::SocialDisabled {
                remove_peer_agents(&mut world, request.organism_id)?;
            }
            if request.ability == Era1Ability::PostSleepRetention
                && request.control != Era1Control::SleepDisabled
                && world.tick().raw() == ERA1_PROBE_START_TICK
            {
                let replay = self.session.build_sleep_replay_batch(handle)?;
                let consolidation = self.session.prepare_sleep_consolidation(
                    handle,
                    ConsolidationIntent { cycle_id: 1 },
                    &replay,
                )?;
                let job =
                    self.session
                        .submit_sleep_consolidation(handle, &consolidation, &replay)?;
                let staged = self
                    .session
                    .poll_sleep_consolidation(handle, job)?
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
                self.session
                    .commit_sleep_consolidation(handle, &consolidation, &staged.staged)?;
                sleep_commits = sleep_commits.saturating_add(1);
            }

            let world_before = world.canonical_signature_digest()?.words;
            let peer_visible = peer_is_visible(&world, request.organism_id)?;
            development.age_ticks = Tick::new(mature_age.saturating_add(world.tick().raw()));
            let draft = world.perception_frame_draft(
                request.organism_id,
                world.tick(),
                SensorProfile::GroundedObjectSlotsV1,
                homeostasis,
            )?;
            let prepared_recall = memory.recall_frame(&draft)?;
            let memory_bank_digest = prepared_recall.receipt().bank_digest;
            let (frame, finalized_recall) = prepared_recall.finalize(draft)?;
            let memory_upload =
                self.session
                    .prepare_memory_context_upload(handle, &frame, &finalized_recall)?;
            let member = GpuClosedLoopMemoryTickInput::try_new(handle, &frame, &memory_upload)?;
            let batch = GpuClosedLoopMemoryBatchInput::try_new(vec![member])?;
            let mut gpu_ticks = self.session.tick_memory_batch(&batch)?;
            let gpu_tick = gpu_ticks
                .pop()
                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
            let binding = gpu_tick
                .memory_context_binding
                .ok_or(ScaffoldContractError::InvalidMemoryQuery)?;
            let pending = gpu_tick.pending_eligibility;
            let pending_identity = pending.identity();
            let selected = *frame
                .candidates()
                .get(usize::from(gpu_tick.selection.candidate_index))
                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
            if binding.final_frame_digest != frame.frame_digest()
                || pending_identity.phenotype_hash() != handle.phenotype_hash()
                || pending_identity.originating_tick() != frame.tick()
                || pending_identity.frame_digest() != frame.frame_digest()
                || pending_identity.candidate_index() != gpu_tick.selection.candidate_index
                || pending_identity.action_id() != selected.action_id
                || pending_identity.action_family() != selected.family
                || pending_identity.candidate_feature_digest() != selected.feature_digest()?
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
            let target_kind = selected
                .target
                .entity
                .and_then(|entity| world.entity(entity))
                .map(|entity| entity.kind);
            let phase = request.manifest.phase_at(frame.tick())?;

            let sequence_id = ExperienceSequenceId(
                u64::try_from(steps.len())
                    .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?,
            );
            let command =
                selected.to_command(request.organism_id, gpu_tick.selection.confidence)?;
            let pre_action = PreActionSnapshot::from_neural_frame(
                sequence_id,
                handle.class_id(),
                handle.phenotype_hash(),
                brain_genome.id,
                brain_genome.schema_version,
                development.clone(),
                frame.clone(),
            )?;
            let decision = DecisionSnapshot::from_neural_selection(
                sequence_id,
                handle.phenotype_hash(),
                gpu_tick.dispatch_generation,
                gpu_tick.active_activation_side,
                &frame,
                NeuralActionSelection {
                    candidate_index: gpu_tick.selection.candidate_index,
                    logit: gpu_tick.selection.logit,
                    confidence: gpu_tick.selection.confidence,
                    active_tiles: gpu_tick.selection.active_tiles,
                    active_synapses: gpu_tick.selection.active_synapses,
                },
                command,
            )?
            .with_finalized_memory_recall(
                &frame,
                &finalized_recall,
                gpu_tick.selection.candidate_index,
            )?;
            let speech_prompted = frame
                .sensory()
                .language_context
                .heard_tokens
                .iter()
                .flatten()
                .any(|token| token.source_kind == UtteranceSourceKind::Player);
            let action_result = world.apply_neural_command(
                &decision.selected_action,
                gpu_tick.speech_payload,
                speech_prompted,
            )?;
            let outcome_tick = Tick::new(world.tick().raw().saturating_add(1));
            let mut outcome = PostActionOutcome::new(
                request.organism_id,
                sequence_id,
                outcome_tick,
                action_result.observation.success && action_result.execution.succeeded,
                action_result.execution.physical,
                action_result.observation.homeostatic_delta,
                action_result.observation.reward_valence,
                action_result.observation.frustration_delta,
                action_result.observation.pain_delta,
                action_result.observation.energy_delta,
                action_result.observation.prediction_error,
            )?;
            outcome.contradiction_observed = action_result.observation.contradiction_observed
                || !action_result.execution.succeeded;
            let patch = ExperiencePatchBuilder::new(sequence_id)
                .record_pre_action(pre_action)?
                .record_decision(decision)?
                .record_outcome(outcome)?
                .seal()?;
            let world_after_action = world.canonical_signature_digest()?.words;

            let grounding_before = grounding_receipts.len();
            if request.ability == Era1Ability::GroundedLanguage && patch.outcome().success {
                if let Some(target) = patch.decision().selected_action.target_entity {
                    for heard in patch
                        .pre_action()
                        .perception()
                        .sensory()
                        .language_context
                        .heard_tokens
                        .iter()
                        .flatten()
                    {
                        if let Ok(receipt) = UtteranceGroundingReceiptV2::try_from_sealed(
                            &patch,
                            heard.utterance_id,
                            heard.sequence_position,
                            target,
                        ) {
                            grounding_receipts.push(receipt);
                            break;
                        }
                    }
                }
            }
            let behavior_success = behavior_succeeds(
                request.ability,
                selected.family,
                target_kind,
                patch.outcome().success,
                grounding_receipts.len() > grounding_before,
            );

            let learning = if request.control == Era1Control::PlasticityDisabled {
                self.session
                    .discard_pending_eligibility(handle, pending_identity)?;
                eligibility_discards = eligibility_discards.saturating_add(1);
                Era1LearningDisposition::Discarded
            } else {
                self.session.apply_sealed_outcome(handle, &patch)?;
                learning_commits = learning_commits.saturating_add(1);
                Era1LearningDisposition::Applied
            };
            let memory_observed = request.control != Era1Control::MemoryDisabled;
            if memory_observed {
                memory.observe_sealed_patch(&patch)?;
                memory_updates = memory_updates.saturating_add(1);
            }
            steps.push(Era1CausalStepReceipt {
                organism_id: request.organism_id,
                phenotype_hash: handle.phenotype_hash(),
                sequence_id,
                tick: frame.tick(),
                world_before_digest: world_before,
                world_after_action_digest: world_after_action,
                frame_digest: frame.frame_digest(),
                memory_organism_id: memory.organism_id(),
                memory_bank_digest,
                memory_context_final_digest: binding.final_frame_digest,
                pending_frame_digest: pending_identity.frame_digest(),
                pending_receipt_digest: pending.receipt_digest(),
                learning,
                memory_observed,
                peer_visible,
                outcome_success: patch.outcome().success,
                phase,
                selected_action: selected.kind,
                selected_family: selected.family,
                target_kind,
                behavior_success,
            });
            homeostasis = homeostasis.advance(
                outcome_tick,
                patch.outcome().homeostatic_delta,
                HomeostaticParameters::reference(),
            )?;
            world.advance_tick();
        }

        let learning_assessment = assess_learning(request.ability, &steps, grounding_receipts)?;
        let foundation = request.genome.foundation;
        let receipt = Era1TrialReceipt {
            schema_version: alife_core::ERA1_EVALUATION_SCHEMA_VERSION,
            identity: Era1TrialIdentity {
                seed: request.manifest.seed,
                organism_id: request.organism_id,
                genome_id: request.genome.id,
                parent_genome_ids: request.genome.parent_genome_ids.clone(),
                lineage_id: request.genome.lineage_id,
                generation: request.generation,
                brain_class_id: foundation.brain_class_id,
                world_family_id: request.manifest.family as u64,
                world_variant_id: request.manifest.world_variant_id
                    | (if request.manifest.held_out_transform {
                        1_u64 << 63
                    } else {
                        0
                    }),
            },
            ability: request.ability,
            control: request.control,
            partition: request.partition,
            score: score_for_partition(request.partition, &learning_assessment),
            phenotype_hash: handle.phenotype_hash(),
            foundation_id: foundation.foundation_id,
            foundation_version: u32::from(foundation.version),
            sensor_profile: SensorProfile::GroundedObjectSlotsV1,
            policy_backend: PolicyBackend::NeuralClosedLoopGpu,
            world_digest: initial_world_digest,
            perception_digest: aggregate_perception(&steps),
            sealed_evidence_digest: aggregate_sealed(&steps),
            assistance: Vec::new(),
            adapter_name: self.adapter_name.clone(),
            backend_api: self.backend_api.clone(),
            source_commit: request.source_commit.to_string(),
            source_tree: request.source_tree.to_string(),
        };
        receipt.validate_contract()?;
        let evidence = Era1TrialRunEvidence {
            receipt,
            initial_world_digest,
            gpu_dispatches: steps.len() as u64,
            sealed_outcomes: steps.len() as u64,
            memory_context_dispatches: steps.len() as u64,
            learning_commits,
            eligibility_discards,
            memory_updates,
            sleep_commits,
            social_context_present: steps.iter().any(|step| step.peer_visible),
            adapter_name: self.adapter_name.clone(),
            backend_api: self.backend_api.clone(),
            learning_assessment,
            steps,
        };
        evidence.validate_contract()?;
        Ok(evidence)
    }

    fn express_compatible_creature(
        &self,
        genome: &CreatureGenome,
    ) -> Result<(BrainGenome, DevelopmentState), ScaffoldContractError> {
        genome.validate_contract()?;
        let manifest = self.foundation.manifest();
        if genome.foundation.brain_class_id != self.capacity.id()
            || genome.foundation.foundation_id != manifest.foundation_id().raw()
            || u32::from(genome.foundation.version) != manifest.foundation_version().raw()
            || genome.foundation.compatibility_family_id != manifest.compatibility_family_id().raw()
        {
            return Err(ScaffoldContractError::IncompatibleGeneticClass);
        }
        let expressed = genome.express()?;
        let mature_tick = Tick::new(u64::from(expressed.development.maturation_duration_ticks));
        let development = expressed.development_state_at(mature_tick)?;
        Ok((expressed.brain_genome, development))
    }
}

fn phase_for_tick(tick: Tick) -> Era1TrialPhase {
    if tick.raw() < ERA1_ACQUISITION_END_TICK {
        Era1TrialPhase::Acquisition
    } else if tick.raw() < ERA1_PROBE_START_TICK {
        Era1TrialPhase::Delay
    } else {
        Era1TrialPhase::Probe
    }
}

fn behavior_succeeds(
    ability: Era1Ability,
    family: CandidateActionFamily,
    target_kind: Option<WorldObjectKind>,
    outcome_success: bool,
    grounded_utterance: bool,
) -> bool {
    if !outcome_success {
        return false;
    }
    match ability {
        Era1Ability::FlexibleForaging
        | Era1Ability::SpatialMemory
        | Era1Ability::DelayedChoice
        | Era1Ability::RewardReversal
        | Era1Ability::ObjectTransfer
        | Era1Ability::PostSleepRetention => {
            family == CandidateActionFamily::Ingest && target_kind == Some(WorldObjectKind::Food)
        }
        Era1Ability::HazardAvoidance => {
            family == CandidateActionFamily::Avoid && target_kind == Some(WorldObjectKind::Hazard)
        }
        Era1Ability::MultiStepProblem => {
            matches!(
                family,
                CandidateActionFamily::Approach
                    | CandidateActionFamily::Contact
                    | CandidateActionFamily::Ingest
            ) && matches!(
                target_kind,
                Some(WorldObjectKind::Obstacle | WorldObjectKind::Food | WorldObjectKind::Token)
            )
        }
        Era1Ability::IndividualRecognition => {
            matches!(
                family,
                CandidateActionFamily::Inspect
                    | CandidateActionFamily::Approach
                    | CandidateActionFamily::Contact
            ) && target_kind == Some(WorldObjectKind::Agent)
        }
        Era1Ability::Imitation => {
            matches!(
                family,
                CandidateActionFamily::Approach | CandidateActionFamily::Ingest
            ) && target_kind == Some(WorldObjectKind::Food)
        }
        Era1Ability::GroundedLanguage => grounded_utterance,
    }
}

fn assess_learning(
    ability: Era1Ability,
    steps: &[Era1CausalStepReceipt],
    grounding_receipts: Vec<UtteranceGroundingReceiptV2>,
) -> Result<Era1LearningAssessment, ScaffoldContractError> {
    let midpoint = ERA1_ACQUISITION_END_TICK / 2;
    let early = reading_for(steps.iter().filter(|step| step.tick.raw() < midpoint))?;
    let late = reading_for(steps.iter().filter(|step| {
        step.tick.raw() >= midpoint && step.tick.raw() < ERA1_ACQUISITION_END_TICK
    }))?;
    let delay = reading_for(
        steps
            .iter()
            .filter(|step| step.phase == Era1TrialPhase::Delay),
    )?;
    let probe = reading_for(
        steps
            .iter()
            .filter(|step| step.phase == Era1TrialPhase::Probe),
    )?;
    let early_q16 = measured_q16(early)?;
    let late_q16 = measured_q16(late)?;
    let probe_q16 = measured_q16(probe)?;
    let acquisition_improvement_q16 = late_q16 as i32 - early_q16 as i32;
    let probe_change_from_early_q16 = probe_q16 as i32 - early_q16 as i32;
    let demonstrated = match ability {
        Era1Ability::FlexibleForaging | Era1Ability::HazardAvoidance => {
            acquisition_improvement_q16 > 0
        }
        Era1Ability::GroundedLanguage => {
            probe_change_from_early_q16 > 0 && !grounding_receipts.is_empty()
        }
        _ => probe_change_from_early_q16 > 0,
    };
    Ok(Era1LearningAssessment {
        early_acquisition: early,
        late_acquisition: late,
        delay,
        probe,
        acquisition_improvement_q16,
        probe_change_from_early_q16,
        demonstrated,
        grounding_receipts,
    })
}

fn reading_for<'a>(
    steps: impl Iterator<Item = &'a Era1CausalStepReceipt>,
) -> Result<MetricReading, ScaffoldContractError> {
    let mut exposures = 0_u64;
    let mut successes = 0_u32;
    for step in steps {
        exposures = exposures
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        successes = successes.saturating_add(u32::from(step.behavior_success));
    }
    let denominator =
        u32::try_from(exposures).map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
    Ok(MetricReading::Measured {
        value_q16: ratio_q16(successes, denominator),
        exposures,
    })
}

fn measured_q16(reading: MetricReading) -> Result<u32, ScaffoldContractError> {
    match reading {
        MetricReading::Measured { value_q16, .. } => Ok(value_q16),
        MetricReading::Unknown => Err(ScaffoldContractError::InvalidDecisionEvidence),
    }
}

fn score_for_partition(
    partition: Era1EvidencePartition,
    assessment: &Era1LearningAssessment,
) -> MetricReading {
    match partition {
        Era1EvidencePartition::Acquisition => assessment.late_acquisition,
        Era1EvidencePartition::DelayedProbe
        | Era1EvidencePartition::ReversalProbe
        | Era1EvidencePartition::HeldOutTransfer
        | Era1EvidencePartition::PostSleepProbe
        | Era1EvidencePartition::SocialTransfer
        | Era1EvidencePartition::ReproducedOffspring => assessment.probe,
    }
}

fn remove_peer_agents(
    world: &mut HeadlessWorld,
    subject: OrganismId,
) -> Result<(), ScaffoldContractError> {
    let peers = world
        .organism_entity_ids()
        .into_iter()
        .filter_map(|(organism, _)| (organism != subject).then_some(organism))
        .collect::<Vec<_>>();
    for peer in peers {
        world.remove_organism(peer)?;
    }
    Ok(())
}

fn peer_is_visible(
    world: &HeadlessWorld,
    subject: OrganismId,
) -> Result<bool, ScaffoldContractError> {
    let snapshot = world.physical_observation_snapshot(subject, world.tick())?;
    Ok(snapshot.visible.iter().any(|visible| {
        world
            .entity(visible.transport_entity)
            .and_then(|entity| entity.organism_id)
            .is_some_and(|organism| organism != subject)
    }))
}

fn expected_family(ability: Era1Ability) -> Era1WorldFamily {
    match ability {
        Era1Ability::FlexibleForaging | Era1Ability::HazardAvoidance => {
            Era1WorldFamily::ForagingHazardMaze
        }
        Era1Ability::SpatialMemory
        | Era1Ability::DelayedChoice
        | Era1Ability::PostSleepRetention => Era1WorldFamily::DelayedLocation,
        Era1Ability::RewardReversal => Era1WorldFamily::RewardReversal,
        Era1Ability::ObjectTransfer => Era1WorldFamily::TransformedObjectsLayout,
        Era1Ability::MultiStepProblem => Era1WorldFamily::TwoStepAccessProblem,
        Era1Ability::IndividualRecognition => Era1WorldFamily::FamiliarNovelIndividual,
        Era1Ability::Imitation => Era1WorldFamily::PeerDemonstration,
        Era1Ability::GroundedLanguage => Era1WorldFamily::GroundedVocabulary,
    }
}

fn aggregate_perception(steps: &[Era1CausalStepReceipt]) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(PERCEPTION_DIGEST_DOMAIN);
    digest.write_sequence_len(steps.len());
    for step in steps {
        write_digest(&mut digest, step.frame_digest.0);
        write_digest(&mut digest, step.memory_bank_digest);
        write_digest(&mut digest, step.memory_context_final_digest.0);
    }
    digest.finish256()
}

fn aggregate_sealed(steps: &[Era1CausalStepReceipt]) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(SEALED_DIGEST_DOMAIN);
    digest.write_sequence_len(steps.len());
    for step in steps {
        digest.write_u64(step.sequence_id.raw());
        digest.write_u64(step.tick.raw());
        write_digest(&mut digest, step.world_before_digest);
        write_digest(&mut digest, step.world_after_action_digest);
        write_digest(&mut digest, step.pending_receipt_digest);
        digest.write_u8(step.learning as u8);
        digest.write_bool(step.memory_observed);
        digest.write_bool(step.peer_visible);
        digest.write_bool(step.outcome_success);
    }
    digest.finish256()
}

fn write_digest(builder: &mut CanonicalDigestBuilder, words: [u64; 4]) {
    for word in words {
        builder.write_u64(word);
    }
}

fn ratio_q16(numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return 0;
    }
    ((u64::from(numerator) * 65_535) / u64::from(denominator)) as u32
}

fn valid_git_object_id(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
