//! GPU-authoritative causal trial loop for the Era 1 Norn-plus battery.

use alife_core::{
    ActionKind, BrainCapacityClass, BrainGenome, CandidateActionFamily, CandidateObservationRef,
    CanonicalDigestBuilder, Confidence, ConsolidationIntent, CreatureGenome, DecisionSnapshot,
    DevelopmentState, Era1Ability, Era1Control, Era1EvidencePartition, Era1TrialIdentity,
    Era1TrialReceipt, ExperiencePatch, ExperiencePatchBuilder, ExperienceSequenceId,
    FoundationWeightAsset, HomeostaticParameters, HomeostaticSnapshot, LanguageGroundingLedger,
    MemoryBankConfig, MemorySidecarState, MetricReading, NeuralActionSelection, OrganismId,
    PerceptionFrameDigest, PhenotypeCompiler, PhenotypeHash, PolicyBackend, PostActionOutcome,
    PreActionSnapshot, ScaffoldContractError, SensorProfile, SensorProfileIdentity,
    SensoryAbiVersion, SpeechMotorPayload, Tick, TrackedObjectId, UtteranceGroundingReceiptV2,
    UtteranceSourceKind, Validate, WorldEntityId,
};
use alife_gpu_backend::{
    GpuBrainHandle, GpuClosedLoopBackend, GpuClosedLoopMemoryBatchInput,
    GpuClosedLoopMemoryTickInput, GpuLearningEvidenceMismatchReceipt,
    GpuRuntimeApplyFastPlasticityFailureReceipt, GpuRuntimeProfile,
    GpuRuntimeSelectorDiagnosticBuildFailureReceipt,
    GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureReceipt,
    GpuRuntimeSelectorDiagnosticEnableFailure, GpuRuntimeSelectorDiagnosticError,
    GpuRuntimeSelectorDiagnosticFailureReceipt, GpuRuntimeSelectorDiagnosticStage,
    GpuSelectorDiagnosticReceipt,
};
use alife_runtime::{GpuAuthoritativeSession, GpuSessionConsumerKind};
use alife_world::{
    apply_era1_world_transition, build_era1_trial_world, Era1TrialManifest, Era1TrialPhase,
    Era1WorldFamily, Era1WorldTransition, HeadlessWorld, HeadlessWorldCommand, WorldObjectKind,
    ERA1_ACQUISITION_END_TICK, ERA1_PROBE_START_TICK, ERA1_TRIAL_END_TICK,
};
use serde::{Deserialize, Serialize};

use crate::TrainingError;

const ERA1_MEMORY_CAPACITY: usize = 64;
const ERA1_MEMORY_MAX_FEATURE_LEN: usize = 64;
const ERA1_MEMORY_MAX_MATCH_COUNT: usize = 4;
const ERA1_MEMORY_MIN_MATCH_SCORE: f32 = 0.72;
const PERCEPTION_DIGEST_DOMAIN: &[u8] = b"alife.era1.perception-evidence.v1";
const SEALED_DIGEST_DOMAIN: &[u8] = b"alife.era1.sealed-evidence.v1";

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Era1LearningDisposition {
    Applied = 1,
    Discarded = 2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub target_entity: Option<WorldEntityId>,
    pub target_kind: Option<WorldObjectKind>,
    pub target_organism: Option<OrganismId>,
    pub tracked_target: Option<TrackedObjectId>,
    pub cue_present: bool,
    pub familiar_tracked_id: Option<TrackedObjectId>,
    pub novel_tracked_id: Option<TrackedObjectId>,
    pub behavior_success: bool,
    pub speech_payload: Option<SpeechMotorPayload>,
    #[serde(default)]
    pub selector_diagnostic: Option<Era1SelectorDiagnosticReceipt>,
    pub sealed_patch: ExperiencePatch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1SelectorDiagnosticReceipt {
    pub source_commit: String,
    pub source_tree: String,
    pub requested_candidate_indices: Vec<u16>,
    pub dispatch: GpuSelectorDiagnosticReceipt,
}

#[derive(Debug, thiserror::Error)]
pub enum Era1TrialRunError {
    #[error("Era 1 contract error: {0}")]
    Contract(#[from] ScaffoldContractError),
    #[error("Era 1 sealed outcome evidence mismatch: {0}")]
    LearningEvidenceMismatch(GpuLearningEvidenceMismatchReceipt),
    #[error("Era 1 fast-plasticity apply failure: {0}")]
    ApplyFastPlasticity(GpuRuntimeApplyFastPlasticityFailureReceipt),
    #[error("Era 1 selector diagnostic enable-stage failure: {0}")]
    SelectorDiagnosticEnable(GpuRuntimeSelectorDiagnosticEnableFailure),
    #[error("Era 1 selector diagnostic later-stage GPU failure: {0}")]
    SelectorDiagnosticLaterStage(GpuRuntimeSelectorDiagnosticFailureReceipt),
    #[error("Era 1 selector diagnostic DecodeMappedRecords GPU failure: {0}")]
    SelectorDiagnosticDecodeMappedRecords(
        GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureReceipt,
    ),
    #[error("Era 1 selector diagnostic build failure: {0}")]
    SelectorDiagnosticBuild(GpuRuntimeSelectorDiagnosticBuildFailureReceipt),
    #[error("Era 1 selector diagnostic later-stage GPU failure at {stage:?}: {error}")]
    SelectorDiagnosticLaterStageContract {
        stage: GpuRuntimeSelectorDiagnosticStage,
        error: ScaffoldContractError,
    },
}

impl Era1TrialRunError {
    fn into_training_error(self) -> TrainingError {
        match self {
            Self::Contract(error) => TrainingError::Contract(error),
            Self::LearningEvidenceMismatch(_) => {
                TrainingError::Contract(ScaffoldContractError::LearningEvidenceMismatch)
            }
            Self::ApplyFastPlasticity(_) => {
                TrainingError::Contract(ScaffoldContractError::LearningEvidenceMismatch)
            }
            Self::SelectorDiagnosticLaterStageContract { error, .. } => {
                TrainingError::Contract(error)
            }
            Self::SelectorDiagnosticLaterStage(error) => {
                TrainingError::Contract(error.mapped_contract_error())
            }
            Self::SelectorDiagnosticDecodeMappedRecords(error) => {
                TrainingError::Contract(error.mapped_contract_error())
            }
            Self::SelectorDiagnosticBuild(error) => {
                TrainingError::Contract(error.mapped_contract_error())
            }
            Self::SelectorDiagnosticEnable(error) => {
                TrainingError::Contract(error.mapped_contract_error())
            }
        }
    }
}

fn map_selector_diagnostic_error(error: GpuRuntimeSelectorDiagnosticError) -> Era1TrialRunError {
    match error {
        GpuRuntimeSelectorDiagnosticError::Preflight(error) => Era1TrialRunError::Contract(error),
        GpuRuntimeSelectorDiagnosticError::Enable(error) => {
            Era1TrialRunError::SelectorDiagnosticEnable(error)
        }
        GpuRuntimeSelectorDiagnosticError::LaterStage(error) => {
            Era1TrialRunError::SelectorDiagnosticLaterStage(error)
        }
        GpuRuntimeSelectorDiagnosticError::DecodeMappedRecords(error) => {
            Era1TrialRunError::SelectorDiagnosticDecodeMappedRecords(error)
        }
        GpuRuntimeSelectorDiagnosticError::BuildSelectorDiagnostic(error) => {
            Era1TrialRunError::SelectorDiagnosticBuild(error)
        }
        GpuRuntimeSelectorDiagnosticError::LaterStageContract { stage, error } => {
            Era1TrialRunError::SelectorDiagnosticLaterStageContract { stage, error }
        }
    }
}

fn map_learning_apply_failure(
    error: ScaffoldContractError,
    receipt: Option<GpuLearningEvidenceMismatchReceipt>,
) -> Era1TrialRunError {
    if matches!(&error, ScaffoldContractError::LearningEvidenceMismatch) {
        if let Some(receipt) = receipt {
            return Era1TrialRunError::LearningEvidenceMismatch(receipt);
        }
    }
    Era1TrialRunError::Contract(error)
}

impl Era1SelectorDiagnosticReceipt {
    fn validate_source_identity(
        &self,
        source_commit: &str,
        source_tree: &str,
    ) -> Result<(), ScaffoldContractError> {
        if self.source_commit != source_commit || self.source_tree != source_tree {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }

    fn validate_for_step(
        &self,
        source_commit: &str,
        source_tree: &str,
        step: &Era1CausalStepReceipt,
    ) -> Result<(), ScaffoldContractError> {
        self.validate_source_identity(source_commit, source_tree)?;
        self.dispatch.validate_contract()?;
        let neural = step.sealed_patch.decision().neural_evidence()?;
        let chosen = self
            .dispatch
            .candidates
            .get(usize::from(self.dispatch.chosen_candidate_index))
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if self.dispatch.frame_digest != step.frame_digest
            || self.dispatch.phenotype_hash != step.phenotype_hash
            || self.dispatch.dispatch_generation != neural.dispatch_generation
            || self.dispatch.chosen_candidate_index != neural.candidate_index
            || chosen.action_id != neural.action_id
            || chosen.family != step.selected_family
            || chosen.target.entity != step.target_entity
            || self.requested_candidate_indices != self.dispatch.requested_candidate_indices
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1PeerDemonstrationReceipt {
    pub tick: Tick,
    pub actor: OrganismId,
    pub target_entity: WorldEntityId,
    pub action: ActionKind,
    pub world_before_digest: [u64; 4],
    pub world_after_digest: [u64; 4],
    pub succeeded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1TransitionReceipt {
    pub transition: Era1WorldTransition,
    pub world_before_digest: [u64; 4],
    pub world_after_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1AbilityCausalProof {
    pub ability: Era1Ability,
    pub world_family: Era1WorldFamily,
    pub phase_step_counts: [u32; 3],
    pub required_context_proven: bool,
    pub successful_behavior_ticks: Vec<Tick>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1LearningAssessment {
    pub early_acquisition: MetricReading,
    pub late_acquisition: MetricReading,
    pub delay: MetricReading,
    pub probe: MetricReading,
    pub acquisition_improvement_q16: i32,
    pub probe_change_from_early_q16: i32,
    pub demonstrated: bool,
    pub grounding_receipts: Vec<UtteranceGroundingReceiptV2>,
    pub causal_proof: Era1AbilityCausalProof,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1TrialRunEvidence {
    pub receipt: Era1TrialReceipt,
    pub manifest: Era1TrialManifest,
    pub initial_world_digest: [u64; 4],
    pub transition_receipts: Vec<Era1TransitionReceipt>,
    pub peer_demonstration: Option<Era1PeerDemonstrationReceipt>,
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
    pub language_grounding: LanguageGroundingLedger,
    pub learning_assessment: Era1LearningAssessment,
    #[serde(default)]
    pub selector_diagnostics_enabled: bool,
}

impl Era1TrialRunEvidence {
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.receipt.validate_contract()?;
        self.manifest.validate_contract()?;
        self.language_grounding.validate_contract()?;
        let step_count = u64::try_from(self.steps.len())
            .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
        if self.initial_world_digest == [0; 4]
            || self.receipt.world_digest != self.initial_world_digest
            || self.receipt.identity.seed != self.manifest.seed
            || self.receipt.identity.organism_id != self.manifest.subject
            || self.receipt.identity.world_family_id != self.manifest.family as u64
            || self.receipt.identity.world_variant_id != self.manifest.world_variant_id
            || expected_family(self.receipt.ability) != self.manifest.family
            || self.gpu_dispatches != step_count
            || self.gpu_dispatches != self.sealed_outcomes
            || self.gpu_dispatches != self.memory_context_dispatches
            || self.gpu_dispatches != ERA1_TRIAL_END_TICK
            || self.adapter_name != self.receipt.adapter_name
            || self.backend_api != self.receipt.backend_api
            || aggregate_perception(&self.steps) != self.receipt.perception_digest
            || aggregate_sealed(&self.steps) != self.receipt.sealed_evidence_digest
            || self
                .steps
                .iter()
                .any(|step| step.selector_diagnostic.is_some() != self.selector_diagnostics_enabled)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }

        validate_transition_receipts(
            &self.manifest,
            self.receipt.control,
            &self.transition_receipts,
        )?;
        validate_peer_demonstration(
            &self.manifest,
            self.receipt.control,
            self.initial_world_digest,
            self.peer_demonstration.as_ref(),
            &self.steps,
        )?;
        validate_world_replay(self)?;
        if self.language_grounding.utterance_receipts_v2()
            != self.learning_assessment.grounding_receipts
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let (expected_behavior, causal_proof) = derive_causal_behavior(
            self.receipt.ability,
            &self.manifest,
            &self.transition_receipts,
            self.peer_demonstration.as_ref(),
            &self.steps,
            &self.language_grounding,
            self.sleep_commits,
        )?;
        if self
            .steps
            .iter()
            .zip(expected_behavior)
            .any(|(step, expected)| step.behavior_success != expected)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let expected_assessment = assess_learning(
            self.receipt.ability,
            &self.steps,
            self.learning_assessment.grounding_receipts.clone(),
            causal_proof,
        )?;
        if self.learning_assessment != expected_assessment
            || self.receipt.score
                != score_for_partition(
                    self.receipt.ability,
                    self.receipt.partition,
                    &expected_assessment,
                )
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }

        for (index, step) in self.steps.iter().enumerate() {
            step.sealed_patch.validate_contract()?;
            let neural_evidence = step.sealed_patch.decision().neural_evidence()?;
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
                || step.sealed_patch.header().organism_id != step.organism_id
                || step.sealed_patch.header().sequence_id != step.sequence_id
                || step.sealed_patch.header().world_tick != step.tick
                || step.sealed_patch.pre_action().perception().frame_digest() != step.frame_digest
                || step.sealed_patch.decision().selected_action.kind != step.selected_action
                || step.sealed_patch.decision().selected_action.target_entity != step.target_entity
                || neural_evidence.action_family != step.selected_family
                || step.sealed_patch.outcome().success != step.outcome_success
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
            }
            if let Some(diagnostic) = &step.selector_diagnostic {
                diagnostic.validate_for_step(
                    &self.receipt.source_commit,
                    &self.receipt.source_tree,
                    step,
                )?;
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
    selector_diagnostic_candidate_indices: Vec<u16>,
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
            selector_diagnostic_candidate_indices: Vec::new(),
        };
        request.validate_contract()?;
        Ok(request)
    }

    pub fn with_selector_diagnostics_for_candidates<const N: usize>(
        mut self,
        candidate_indices: [u16; N],
    ) -> Self {
        self.selector_diagnostic_candidate_indices = candidate_indices.to_vec();
        self.selector_diagnostic_candidate_indices.sort_unstable();
        self.selector_diagnostic_candidate_indices.dedup();
        self
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
            || self.selector_diagnostic_candidate_indices.len() > 8
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
        self.run_internal(request)
            .map_err(Era1TrialRunError::into_training_error)
    }

    pub fn run_with_selector_diagnostics(
        &mut self,
        request: Era1TrialRunRequest<'_>,
    ) -> Result<Era1TrialRunEvidence, Era1TrialRunError> {
        self.run_internal(request)
    }

    fn run_internal(
        &mut self,
        request: Era1TrialRunRequest<'_>,
    ) -> Result<Era1TrialRunEvidence, Era1TrialRunError> {
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
            (Err(error), _) => Err(error),
            (_, Err(error)) => Err(error.into()),
        }
    }

    fn run_inserted(
        &mut self,
        request: Era1TrialRunRequest<'_>,
        brain_genome: BrainGenome,
        mut development: DevelopmentState,
        handle: GpuBrainHandle,
    ) -> Result<Era1TrialRunEvidence, Era1TrialRunError> {
        let mut world = build_era1_trial_world(request.manifest)?;
        if request.control == Era1Control::SocialDisabled {
            remove_peer_agents(&mut world, request.organism_id)?;
        }
        let initial_world_digest = world.canonical_signature_digest()?.words;
        let peer_demonstration = if request.ability == Era1Ability::Imitation
            && request.control != Era1Control::SocialDisabled
        {
            Some(run_peer_demonstration(request.manifest, &mut world)?)
        } else {
            None
        };
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
        let mut language_grounding = LanguageGroundingLedger::default();
        let mut transition_receipts = Vec::new();

        while world.tick().raw() < ERA1_TRIAL_END_TICK {
            if let Some(transition) = request
                .manifest
                .transitions()
                .into_iter()
                .find(|transition| transition.at_tick == world.tick())
            {
                let world_before_digest = world.canonical_signature_digest()?.words;
                apply_era1_world_transition(request.manifest, transition, &mut world)?;
                transition_receipts.push(Era1TransitionReceipt {
                    transition,
                    world_before_digest,
                    world_after_digest: world.canonical_signature_digest()?.words,
                });
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
            let visibility =
                world.physical_observation_snapshot(request.organism_id, world.tick())?;
            let visible_entities = visibility
                .visible
                .iter()
                .map(|visible| visible.transport_entity)
                .collect::<Vec<_>>();
            let peer_visible = visible_entities.iter().any(|entity| {
                world
                    .entity(*entity)
                    .and_then(|object| object.organism_id)
                    .is_some_and(|organism| organism != request.organism_id)
            });
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
            let mut gpu_ticks = if !request.selector_diagnostic_candidate_indices.is_empty() {
                self.session
                    .tick_memory_batch_with_selector_diagnostics(
                        &batch,
                        &request.selector_diagnostic_candidate_indices,
                    )
                    .map_err(map_selector_diagnostic_error)?
            } else {
                self.session.tick_memory_batch(&batch)?
            };
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
                return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
            }
            let target_kind = selected
                .target
                .entity
                .and_then(|entity| world.entity(entity))
                .map(|entity| entity.kind);
            let target_organism = selected
                .target
                .entity
                .and_then(|entity| world.entity(entity))
                .and_then(|entity| entity.organism_id);
            let tracked_target = match selected.observation {
                CandidateObservationRef::ObjectSlot(slot_index) => frame
                    .grounded_object_slots()
                    .iter()
                    .find(|slot| slot.slot_index == slot_index)
                    .map(|slot| slot.tracked_object_id),
                CandidateObservationRef::None => None,
            };
            let familiar_tracked_id = visible_tracked_organism(
                &world,
                request.organism_id,
                request.manifest.familiar_peer,
                &visible_entities,
            );
            let novel_tracked_id = visible_tracked_organism(
                &world,
                request.organism_id,
                request.manifest.novel_peer,
                &visible_entities,
            );
            let cue_present = world.entity_id("era1-cue").is_some();
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
            let speech_payload = gpu_tick.speech_payload.clone();
            let selector_diagnostic =
                gpu_tick
                    .selector_diagnostic
                    .map(|dispatch| Era1SelectorDiagnosticReceipt {
                        source_commit: request.source_commit.to_owned(),
                        source_tree: request.source_tree.to_owned(),
                        requested_candidate_indices: request
                            .selector_diagnostic_candidate_indices
                            .clone(),
                        dispatch,
                    });
            let action_result = world.apply_neural_command(
                &decision.selected_action,
                speech_payload.clone(),
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
            language_grounding.observe_sealed(&patch)?;

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
                            language_grounding.observe_grounding_v2(receipt.clone())?;
                            grounding_receipts.push(receipt);
                            break;
                        }
                    }
                }
            }
            let _grounded_utterance = grounding_receipts.len() > grounding_before;

            let learning = if request.control == Era1Control::PlasticityDisabled {
                self.session
                    .discard_pending_eligibility(handle, pending_identity)?;
                eligibility_discards = eligibility_discards.saturating_add(1);
                Era1LearningDisposition::Discarded
            } else {
                if let Some(receipt) = self
                    .session
                    .sealed_outcome_credit_mismatch_receipt(handle, &patch)?
                {
                    return Err(Era1TrialRunError::LearningEvidenceMismatch(receipt));
                }
                if let Err(error) = self.session.apply_sealed_outcome(handle, &patch) {
                    if let Some(receipt) = self.session.take_apply_fast_plasticity_failure_receipt()
                    {
                        return Err(Era1TrialRunError::ApplyFastPlasticity(receipt));
                    }
                    let receipt =
                        if matches!(&error, ScaffoldContractError::LearningEvidenceMismatch) {
                            self.session
                                .sealed_outcome_credit_mismatch_receipt(handle, &patch)
                                .ok()
                                .flatten()
                        } else {
                            None
                        };
                    return Err(map_learning_apply_failure(error, receipt));
                }
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
                target_entity: selected.target.entity,
                target_kind,
                target_organism,
                tracked_target,
                cue_present,
                familiar_tracked_id,
                novel_tracked_id,
                behavior_success: false,
                speech_payload,
                selector_diagnostic,
                sealed_patch: patch.clone(),
            });
            homeostasis = homeostasis.advance(
                outcome_tick,
                patch.outcome().homeostatic_delta,
                HomeostaticParameters::reference(),
            )?;
            world.advance_tick();
        }

        let (behavior_success, causal_proof) = derive_causal_behavior(
            request.ability,
            request.manifest,
            &transition_receipts,
            peer_demonstration.as_ref(),
            &steps,
            &language_grounding,
            sleep_commits,
        )?;
        for (step, success) in steps.iter_mut().zip(behavior_success) {
            step.behavior_success = success;
        }
        let learning_assessment =
            assess_learning(request.ability, &steps, grounding_receipts, causal_proof)?;
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
                world_variant_id: request.manifest.world_variant_id,
            },
            ability: request.ability,
            control: request.control,
            partition: request.partition,
            score: score_for_partition(request.ability, request.partition, &learning_assessment),
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
            manifest: request.manifest.clone(),
            initial_world_digest,
            transition_receipts,
            peer_demonstration,
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
            language_grounding,
            learning_assessment,
            selector_diagnostics_enabled: !request.selector_diagnostic_candidate_indices.is_empty(),
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

#[allow(clippy::too_many_arguments)]
fn derive_causal_behavior(
    ability: Era1Ability,
    manifest: &Era1TrialManifest,
    transitions: &[Era1TransitionReceipt],
    peer_demonstration: Option<&Era1PeerDemonstrationReceipt>,
    steps: &[Era1CausalStepReceipt],
    language_grounding: &LanguageGroundingLedger,
    sleep_commits: u32,
) -> Result<(Vec<bool>, Era1AbilityCausalProof), ScaffoldContractError> {
    let phase_step_counts = [
        count_phase(steps, Era1TrialPhase::Acquisition)?,
        count_phase(steps, Era1TrialPhase::Delay)?,
        count_phase(steps, Era1TrialPhase::Probe)?,
    ];
    let complete_phases = phase_step_counts
        == [
            ERA1_ACQUISITION_END_TICK as u32,
            (ERA1_PROBE_START_TICK - ERA1_ACQUISITION_END_TICK) as u32,
            (ERA1_TRIAL_END_TICK - ERA1_PROBE_START_TICK) as u32,
        ];
    let cue_acquired = steps
        .iter()
        .any(|step| step.phase == Era1TrialPhase::Acquisition && step.cue_present);
    let cue_withheld = steps
        .iter()
        .filter(|step| step.phase != Era1TrialPhase::Acquisition)
        .all(|step| !step.cue_present);
    let probe_transition_changed = transitions.iter().any(|receipt| {
        receipt.transition.to == Era1TrialPhase::Probe
            && receipt.world_before_digest != receipt.world_after_digest
    });
    let acquisition_familiar = steps
        .iter()
        .filter(|step| step.phase == Era1TrialPhase::Acquisition)
        .filter_map(|step| step.familiar_tracked_id)
        .next();
    let probe_familiar = steps
        .iter()
        .filter(|step| step.phase == Era1TrialPhase::Probe)
        .filter_map(|step| step.familiar_tracked_id)
        .next();
    let probe_novel = steps
        .iter()
        .filter(|step| step.phase == Era1TrialPhase::Probe)
        .filter_map(|step| step.novel_tracked_id)
        .next();
    let stable_identity = acquisition_familiar.is_some()
        && acquisition_familiar == probe_familiar
        && probe_novel.is_some()
        && probe_novel != probe_familiar;
    let peer_demo_valid = peer_demonstration.is_some_and(|receipt| {
        receipt.actor == manifest.familiar_peer
            && receipt.tick.raw() < ERA1_ACQUISITION_END_TICK
            && receipt.action == ActionKind::Move
            && receipt.succeeded
            && steps
                .iter()
                .any(|step| step.tick == receipt.tick && step.peer_visible)
    });
    let language_receipts = language_grounding.utterance_receipts_v2();
    let required_context_proven = complete_phases
        && expected_family(ability) == manifest.family
        && match ability {
            Era1Ability::FlexibleForaging | Era1Ability::HazardAvoidance => true,
            Era1Ability::SpatialMemory | Era1Ability::DelayedChoice => cue_acquired && cue_withheld,
            Era1Ability::RewardReversal => probe_transition_changed,
            Era1Ability::ObjectTransfer => manifest.held_out_transform && probe_transition_changed,
            Era1Ability::MultiStepProblem => cue_acquired && cue_withheld,
            Era1Ability::IndividualRecognition => stable_identity,
            Era1Ability::Imitation => peer_demo_valid,
            Era1Ability::GroundedLanguage => !language_receipts.is_empty(),
            Era1Ability::PostSleepRetention => cue_acquired && cue_withheld && sleep_commits == 1,
        };

    let mut successes = vec![false; steps.len()];
    if required_context_proven {
        for (index, step) in steps.iter().enumerate() {
            if !step.outcome_success {
                continue;
            }
            successes[index] = match ability {
                Era1Ability::FlexibleForaging => {
                    step.phase == Era1TrialPhase::Acquisition
                        && step.selected_family == CandidateActionFamily::Ingest
                        && step.target_kind == Some(WorldObjectKind::Food)
                }
                Era1Ability::HazardAvoidance => {
                    step.phase == Era1TrialPhase::Acquisition
                        && step.selected_family == CandidateActionFamily::Avoid
                        && step.target_kind == Some(WorldObjectKind::Hazard)
                }
                Era1Ability::SpatialMemory | Era1Ability::DelayedChoice => {
                    step.phase == Era1TrialPhase::Probe
                        && !step.cue_present
                        && step.memory_observed
                        && matches!(
                            step.selected_family,
                            CandidateActionFamily::Approach | CandidateActionFamily::Ingest
                        )
                        && step.target_kind == Some(WorldObjectKind::Food)
                }
                Era1Ability::RewardReversal | Era1Ability::ObjectTransfer => {
                    step.phase == Era1TrialPhase::Probe
                        && ((step.selected_family == CandidateActionFamily::Ingest
                            && step.target_kind == Some(WorldObjectKind::Food))
                            || (step.selected_family == CandidateActionFamily::Avoid
                                && step.target_kind == Some(WorldObjectKind::Hazard)))
                }
                Era1Ability::MultiStepProblem => {
                    step.selected_family == CandidateActionFamily::Ingest
                        && step.target_kind == Some(WorldObjectKind::Food)
                        && steps[..index].iter().any(|prior| {
                            prior.outcome_success
                                && matches!(
                                    prior.selected_family,
                                    CandidateActionFamily::Approach
                                        | CandidateActionFamily::Contact
                                )
                                && matches!(
                                    prior.target_kind,
                                    Some(WorldObjectKind::Obstacle | WorldObjectKind::Token)
                                )
                        })
                }
                Era1Ability::IndividualRecognition => {
                    step.phase == Era1TrialPhase::Probe
                        && step.target_organism == Some(manifest.familiar_peer)
                        && step.tracked_target == probe_familiar
                        && matches!(
                            step.selected_family,
                            CandidateActionFamily::Inspect
                                | CandidateActionFamily::Approach
                                | CandidateActionFamily::Contact
                        )
                }
                Era1Ability::Imitation => peer_demonstration.is_some_and(|demo| {
                    step.tick.raw() >= demo.tick.raw()
                        && step.selected_family == CandidateActionFamily::Approach
                        && step.target_entity == Some(demo.target_entity)
                }),
                Era1Ability::GroundedLanguage => language_receipts
                    .iter()
                    .any(|receipt| receipt.sequence_id == step.sequence_id),
                Era1Ability::PostSleepRetention => {
                    step.phase == Era1TrialPhase::Probe
                        && step.memory_observed
                        && step.selected_family == CandidateActionFamily::Ingest
                        && step.target_kind == Some(WorldObjectKind::Food)
                }
            };
        }
    }
    let successful_behavior_ticks = steps
        .iter()
        .zip(&successes)
        .filter_map(|(step, success)| success.then_some(step.tick))
        .collect();
    let proof = Era1AbilityCausalProof {
        ability,
        world_family: manifest.family,
        phase_step_counts,
        required_context_proven,
        successful_behavior_ticks,
    };
    Ok((successes, proof))
}

fn assess_learning(
    ability: Era1Ability,
    steps: &[Era1CausalStepReceipt],
    grounding_receipts: Vec<UtteranceGroundingReceiptV2>,
    causal_proof: Era1AbilityCausalProof,
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
    let demonstrated = causal_proof.required_context_proven
        && !causal_proof.successful_behavior_ticks.is_empty()
        && match ability {
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
        causal_proof,
    })
}

fn count_phase(
    steps: &[Era1CausalStepReceipt],
    phase: Era1TrialPhase,
) -> Result<u32, ScaffoldContractError> {
    u32::try_from(steps.iter().filter(|step| step.phase == phase).count())
        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)
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
    ability: Era1Ability,
    partition: Era1EvidencePartition,
    assessment: &Era1LearningAssessment,
) -> MetricReading {
    if matches!(
        ability,
        Era1Ability::FlexibleForaging | Era1Ability::HazardAvoidance
    ) {
        return assessment.late_acquisition;
    }
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

fn run_peer_demonstration(
    manifest: &Era1TrialManifest,
    world: &mut HeadlessWorld,
) -> Result<Era1PeerDemonstrationReceipt, ScaffoldContractError> {
    let target_entity = world
        .entity_id("era1-object-a")
        .ok_or(ScaffoldContractError::InvalidId)?;
    let tick = world.tick();
    let world_before_digest = world.canonical_signature_digest()?.words;
    let result = world.apply_command(&HeadlessWorldCommand::approach(
        manifest.familiar_peer,
        target_entity,
    )?)?;
    let receipt = Era1PeerDemonstrationReceipt {
        tick,
        actor: manifest.familiar_peer,
        target_entity,
        action: result.command.kind,
        world_before_digest,
        world_after_digest: world.canonical_signature_digest()?.words,
        succeeded: result.execution.succeeded && result.observation.success,
    };
    Ok(receipt)
}

fn visible_tracked_organism(
    world: &HeadlessWorld,
    observer: OrganismId,
    organism: OrganismId,
    visible_entities: &[WorldEntityId],
) -> Option<TrackedObjectId> {
    let entity_id = world
        .organism_entity_ids()
        .into_iter()
        .find_map(|(candidate, entity)| (candidate == organism).then_some(entity))?;
    if !visible_entities.contains(&entity_id) {
        return None;
    }
    let tracking_key = world.entity(entity_id)?.tracking_key;
    world
        .tracked_objects()
        .records_for(observer)?
        .find(|record| record.tracking_key == tracking_key)
        .map(|record| record.tracked_object_id)
}

fn validate_transition_receipts(
    manifest: &Era1TrialManifest,
    control: Era1Control,
    receipts: &[Era1TransitionReceipt],
) -> Result<(), ScaffoldContractError> {
    let expected = manifest.transitions();
    if receipts.len() != expected.len()
        || receipts.iter().zip(expected).any(|(receipt, transition)| {
            receipt.transition != transition
                || receipt.world_before_digest == [0; 4]
                || receipt.world_after_digest == [0; 4]
        })
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    let required_change = match manifest.family {
        Era1WorldFamily::DelayedLocation | Era1WorldFamily::TwoStepAccessProblem => {
            Some(Era1TrialPhase::Delay)
        }
        Era1WorldFamily::RewardReversal
        | Era1WorldFamily::TransformedObjectsLayout
        | Era1WorldFamily::FamiliarNovelIndividual => Some(Era1TrialPhase::Probe),
        Era1WorldFamily::PeerDemonstration | Era1WorldFamily::GroundedVocabulary
            if control != Era1Control::SocialDisabled =>
        {
            Some(Era1TrialPhase::Delay)
        }
        Era1WorldFamily::PeerDemonstration | Era1WorldFamily::GroundedVocabulary => None,
        Era1WorldFamily::ForagingHazardMaze => None,
    };
    if required_change.is_some_and(|phase| {
        !receipts.iter().any(|receipt| {
            receipt.transition.to == phase
                && receipt.world_before_digest != receipt.world_after_digest
        })
    }) {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    Ok(())
}

fn validate_world_replay(evidence: &Era1TrialRunEvidence) -> Result<(), ScaffoldContractError> {
    let mut world = build_era1_trial_world(&evidence.manifest)?;
    if evidence.receipt.control == Era1Control::SocialDisabled {
        remove_peer_agents(&mut world, evidence.receipt.identity.organism_id)?;
    }
    if world.canonical_signature_digest()?.words != evidence.initial_world_digest {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    if let Some(expected) = evidence.peer_demonstration.as_ref() {
        let replayed = run_peer_demonstration(&evidence.manifest, &mut world)?;
        if &replayed != expected {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
    }
    let mut homeostasis = HomeostaticSnapshot::baseline(world.tick());
    for step in &evidence.steps {
        if let Some(transition) = evidence
            .manifest
            .transitions()
            .into_iter()
            .find(|transition| transition.at_tick == world.tick())
        {
            let expected = evidence
                .transition_receipts
                .iter()
                .find(|receipt| receipt.transition == transition)
                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
            let before = world.canonical_signature_digest()?.words;
            apply_era1_world_transition(&evidence.manifest, transition, &mut world)?;
            let after = world.canonical_signature_digest()?.words;
            if expected.world_before_digest != before || expected.world_after_digest != after {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
        }
        if evidence.receipt.control == Era1Control::SocialDisabled {
            remove_peer_agents(&mut world, evidence.receipt.identity.organism_id)?;
        }
        if world.tick() != step.tick
            || world.canonical_signature_digest()?.words != step.world_before_digest
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let _draft = world.perception_frame_draft(
            evidence.receipt.identity.organism_id,
            world.tick(),
            SensorProfile::GroundedObjectSlotsV1,
            homeostasis,
        )?;
        let prompted = step
            .sealed_patch
            .pre_action()
            .perception()
            .sensory()
            .language_context
            .heard_tokens
            .iter()
            .flatten()
            .any(|token| token.source_kind == UtteranceSourceKind::Player);
        let replayed = world.apply_neural_command(
            &step.sealed_patch.decision().selected_action,
            step.speech_payload.clone(),
            prompted,
        )?;
        if (replayed.execution.succeeded && replayed.observation.success)
            != step.sealed_patch.outcome().success
            || replayed.execution.physical != step.sealed_patch.outcome().physical
            || world.canonical_signature_digest()?.words != step.world_after_action_digest
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        homeostasis = homeostasis.advance(
            step.sealed_patch.outcome().outcome_tick,
            step.sealed_patch.outcome().homeostatic_delta,
            HomeostaticParameters::reference(),
        )?;
        world.advance_tick();
    }
    if world.tick().raw() != ERA1_TRIAL_END_TICK {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    Ok(())
}

fn validate_peer_demonstration(
    manifest: &Era1TrialManifest,
    control: Era1Control,
    initial_world_digest: [u64; 4],
    receipt: Option<&Era1PeerDemonstrationReceipt>,
    steps: &[Era1CausalStepReceipt],
) -> Result<(), ScaffoldContractError> {
    let required = manifest.family == Era1WorldFamily::PeerDemonstration
        && control != Era1Control::SocialDisabled;
    if !required {
        return if receipt.is_none() {
            Ok(())
        } else {
            Err(ScaffoldContractError::InvalidDecisionEvidence)
        };
    }
    let receipt = receipt.ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
    if receipt.tick != Tick::new(0)
        || receipt.actor != manifest.familiar_peer
        || receipt.action != ActionKind::Move
        || !receipt.succeeded
        || receipt.world_before_digest != initial_world_digest
        || receipt.world_after_digest == receipt.world_before_digest
        || steps
            .first()
            .is_none_or(|step| step.world_before_digest != receipt.world_after_digest)
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    Ok(())
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
        digest.write_u8(step.selected_action.raw());
        digest.write_u8(step.selected_family.raw());
        write_optional_u64(&mut digest, step.target_entity.map(WorldEntityId::raw));
        write_optional_u64(&mut digest, step.target_organism.map(OrganismId::raw));
        write_optional_u64(&mut digest, step.tracked_target.map(TrackedObjectId::raw));
        write_optional_u64(&mut digest, step.target_kind.map(|kind| kind as u64));
        digest.write_bool(step.cue_present);
        write_optional_u64(
            &mut digest,
            step.familiar_tracked_id.map(TrackedObjectId::raw),
        );
        write_optional_u64(&mut digest, step.novel_tracked_id.map(TrackedObjectId::raw));
        digest.write_bool(step.behavior_success);
        match &step.speech_payload {
            Some(payload) => {
                digest.write_bool(true);
                digest.write_u8(payload.speech_act as u8);
                digest.write_sequence_len(payload.tokens.len());
                for token in &payload.tokens {
                    digest.write_u16(token.raw());
                }
                digest.write_u32(payload.confidence.raw().to_bits());
            }
            None => digest.write_bool(false),
        }
    }
    digest.finish256()
}

fn write_optional_u64(builder: &mut CanonicalDigestBuilder, value: Option<u64>) {
    match value {
        Some(value) => {
            builder.write_bool(true);
            builder.write_u64(value);
        }
        None => builder.write_bool(false),
    }
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

#[cfg(test)]
mod selector_diagnostic_receipt_tests {
    use super::*;
    use alife_gpu_backend::{
        GpuSelectorBindingIdentity, GpuSelectorCandidateDiagnostic, GpuSelectorCandidateValidity,
        GpuSelectorExplorationMode, GpuSelectorPolicyIdentity, GpuSelectorSynapseContribution,
        GPU_SELECTOR_DIAGNOSTIC_SCHEMA_VERSION,
    };

    fn candidate(
        index: u16,
        action_id: u32,
        pre: f32,
        final_logit: f32,
        detailed: bool,
    ) -> GpuSelectorCandidateDiagnostic {
        GpuSelectorCandidateDiagnostic {
            candidate_index: index,
            action_id: alife_core::ActionId(action_id),
            family: CandidateActionFamily::Inspect,
            target: alife_core::ActionTarget::NONE,
            validity: GpuSelectorCandidateValidity::Valid,
            decoder_family_bias: 0.125,
            binding: detailed.then_some(GpuSelectorBindingIdentity {
                decoder_plan_offset: 1,
                decoder_family_offset: 2,
                decoder_family_start: 2,
                decoder_family_count: 1,
                weight_index_start: 3,
                weight_index_count: 1,
                activation_side: 0,
                activation_offset: 4,
                motor_start: 5,
                feature_offset: 6,
                genetic_weight_offset: 7,
                alpha_offset: 8,
                lifetime_weight_offset: 9,
                fast_weight_offset: 10,
            }),
            contributions: detailed
                .then(|| {
                    vec![GpuSelectorSynapseContribution {
                        synapse_index: 0,
                        global_synapse_id: 11,
                        input_lane: 0,
                        motor_index: 0,
                        motor: 1.0,
                        feature: pre - 0.125,
                        genetic: 1.0,
                        lifetime: 0.0,
                        alpha: 0.0,
                        fast: 1.0,
                        effective_weight: 1.0,
                        signed_contribution: pre - 0.125,
                        running_logit: pre,
                    }]
                })
                .unwrap_or_default(),
            pre_context_logit: Some(pre),
            memory_context_delta: Some(final_logit - pre),
            final_logit: Some(final_logit),
        }
    }

    #[test]
    fn sparse_selector_diagnostic_binds_requested_ranges_and_rejects_unrequested_rows() {
        let source_commit = "1111111111111111111111111111111111111111";
        let source_tree = "2222222222222222222222222222222222222222";
        let receipt = Era1SelectorDiagnosticReceipt {
            source_commit: source_commit.to_owned(),
            source_tree: source_tree.to_owned(),
            requested_candidate_indices: vec![0, 2],
            dispatch: GpuSelectorDiagnosticReceipt {
                schema_version: GPU_SELECTOR_DIAGNOSTIC_SCHEMA_VERSION,
                frame_digest: PerceptionFrameDigest([1, 2, 3, 4]),
                phenotype_hash: PhenotypeHash([5, 6, 7, 8]),
                dispatch_generation: 9,
                policy: GpuSelectorPolicyIdentity::PRODUCTION_V1,
                requested_candidate_indices: vec![0, 2],
                candidates: vec![
                    candidate(0, 4, 0.5, 0.75, true),
                    candidate(1, 5, 0.0, 0.25, false),
                    candidate(2, 6, 0.5, 0.75, true),
                ],
                argmax_candidate_index: 0,
                equal_max_candidate_indices: vec![0, 2],
                chosen_candidate_index: 0,
            },
        };

        receipt.dispatch.validate_contract().unwrap();
        assert!(receipt.dispatch.candidates[1].binding.is_none());
        assert!(receipt.dispatch.candidates[1].contributions.is_empty());
        assert_eq!(
            receipt.dispatch.candidates[0].contributions[0].global_synapse_id,
            11
        );
        assert_eq!(
            receipt.dispatch.candidates[0].contributions[0].running_logit,
            0.5
        );
        let mut invalid = receipt.dispatch.clone();
        invalid.candidates[1] = candidate(1, 5, 0.0, 0.25, true);
        assert!(invalid.validate_contract().is_err());
        assert_eq!(receipt.dispatch.candidates[1].final_logit, Some(0.25));
        assert_eq!(
            receipt.dispatch.policy.exploration_mode,
            GpuSelectorExplorationMode::Disabled
        );
        receipt
            .validate_source_identity(source_commit, source_tree)
            .unwrap();
        assert!(receipt
            .validate_source_identity("3333333333333333333333333333333333333333", source_tree,)
            .is_err());
        let encoded = serde_json::to_vec(&receipt).unwrap();
        let decoded: Era1SelectorDiagnosticReceipt = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, receipt);
    }
}

#[cfg(test)]
mod learning_error_receipt_tests {
    use super::*;
    use alife_gpu_backend::GpuLearningEvidenceMismatchField;

    #[test]
    fn learning_apply_failure_preserves_existing_mismatch_receipt() {
        let receipt = GpuLearningEvidenceMismatchReceipt {
            field: GpuLearningEvidenceMismatchField::ActionId,
            expected: [211, 0, 0, 0],
            actual: [101, 0, 0, 0],
        };

        let error = map_learning_apply_failure(
            ScaffoldContractError::LearningEvidenceMismatch,
            Some(receipt),
        );

        match error {
            Era1TrialRunError::LearningEvidenceMismatch(actual) => assert_eq!(actual, receipt),
            other => panic!("receipt collapsed into {other}"),
        }
    }
}
