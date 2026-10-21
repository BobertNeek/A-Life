//! Retained Era 1 causal evidence contracts and validation.

use alife_core::{
    ActionKind, CandidateActionFamily, CanonicalDigestBuilder, Era1Ability, Era1Control,
    Era1EvidencePartition, Era1TrialReceipt, ExperiencePatch, ExperienceSequenceId,
    LanguageGroundingLedger, MetricReading, OrganismId, PerceptionFrameDigest, PhenotypeHash,
    ScaffoldContractError, SpeechMotorPayload, Tick, TrackedObjectId, UtteranceGroundingReceiptV2,
    Validate, WorldEntityId,
};
use alife_gpu_backend::GpuSelectorDiagnosticReceipt;
use alife_world::{
    Era1TrialManifest, Era1TrialPhase, Era1WorldFamily, Era1WorldTransition, WorldObjectKind,
    ERA1_ACQUISITION_END_TICK, ERA1_PROBE_START_TICK, ERA1_TRIAL_END_TICK,
};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use alife_core::{ActionCommand, BiochemistryState, CreatureGenome};
#[cfg(test)]
use alife_world::{
    build_era1_trial_world, HeadlessActionBiologyReceipt, HeadlessWorld, HeadlessWorldCommand,
    WorldOrganismRecord,
};

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
                != score_for_partition(self.receipt.partition, &expected_assessment)
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
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
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
                    matches!(
                        step.phase,
                        Era1TrialPhase::Acquisition | Era1TrialPhase::Probe
                    ) && step.selected_family == CandidateActionFamily::Ingest
                        && step.target_kind == Some(WorldObjectKind::Food)
                }
                Era1Ability::HazardAvoidance => {
                    matches!(
                        step.phase,
                        Era1TrialPhase::Acquisition | Era1TrialPhase::Probe
                    ) && step.selected_family == CandidateActionFamily::Avoid
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

#[cfg(test)]
fn run_peer_demonstration(
    manifest: &Era1TrialManifest,
    world: &mut HeadlessWorld,
) -> Result<Era1PeerDemonstrationReceipt, ScaffoldContractError> {
    let target_entity = world
        .entity_id("era1-object-a")
        .ok_or(ScaffoldContractError::InvalidId)?;
    let tick = world.tick();
    let world_before_digest = world.canonical_signature_digest()?.words;
    let mut command = HeadlessWorldCommand::approach(manifest.familiar_peer, target_entity)?;
    // The scripted demonstrator owns this endpoint, including held-out transforms.
    // This does not expose a hidden destination to the learner's policy.
    command.target_position = Some(
        world
            .entity(target_entity)
            .ok_or(ScaffoldContractError::InvalidId)?
            .position,
    );
    let result = world.apply_command(&command)?;
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

#[cfg(test)]
fn register_trial_organism(
    world: &mut HeadlessWorld,
    organism_id: OrganismId,
    genome: &CreatureGenome,
) -> Result<WorldOrganismRecord, ScaffoldContractError> {
    let world_entity_id = world
        .organism_entity_ids()
        .into_iter()
        .find_map(|(candidate, entity)| (candidate == organism_id).then_some(entity))
        .ok_or(ScaffoldContractError::InvalidId)?;
    let phenotype = genome.express()?;
    let biochemistry = BiochemistryState::new(&phenotype, world.tick())?;
    let record = WorldOrganismRecord::new(
        organism_id,
        world_entity_id,
        genome.clone(),
        phenotype,
        biochemistry,
        world.tick(),
    )
    .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
    world.register_organism_record(record.clone())?;
    Ok(record)
}

#[cfg(test)]
fn apply_trial_neural_command(
    world: &mut HeadlessWorld,
    command: &ActionCommand,
    world_entity_id: WorldEntityId,
    outcome_tick: Tick,
    speech_payload: Option<SpeechMotorPayload>,
    prompted: bool,
) -> Result<HeadlessActionBiologyReceipt, ScaffoldContractError> {
    world.apply_registered_neural_command(
        command,
        world_entity_id,
        outcome_tick,
        speech_payload,
        prompted,
    )
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

#[cfg(test)]
mod registered_trial_world_tests {
    use super::*;
    use alife_core::{BrainCapacityClass, FoundationGeneticIdentity, Vec3f};
    use alife_world::HeadlessScenarioBuilder;

    const SUBJECT: OrganismId = OrganismId(41);

    fn trial_genome() -> CreatureGenome {
        CreatureGenome::early_mammal_founder(
            0xE10_4101,
            FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap()
    }

    fn trial_world() -> HeadlessWorld {
        HeadlessScenarioBuilder::new(41_001)
            .agent("subject", SUBJECT, Vec3f::ZERO)
            .build()
            .unwrap()
    }

    #[test]
    fn scripted_peer_demonstration_moves_and_replays_across_layout_transforms() {
        for held_out_transform in [false, true] {
            for world_variant_id in 1..=3 {
                let manifest = Era1TrialManifest::new(
                    41_001,
                    Era1WorldFamily::PeerDemonstration,
                    SUBJECT,
                    OrganismId(42),
                    OrganismId(43),
                    world_variant_id,
                    held_out_transform,
                    1,
                )
                .unwrap();
                let mut world = build_era1_trial_world(&manifest).unwrap();
                let mut replay = world.clone();
                let peer = world
                    .organism_entity_ids()
                    .into_iter()
                    .find_map(|(organism, entity)| {
                        (organism == manifest.familiar_peer).then_some(entity)
                    })
                    .unwrap();
                let before_position = world.entity(peer).unwrap().position;
                let before_digest = world.canonical_signature_digest().unwrap().words;
                let receipt = run_peer_demonstration(&manifest, &mut world).unwrap();

                assert!(receipt.succeeded);
                assert_eq!(receipt.actor, manifest.familiar_peer);
                assert_eq!(receipt.action, ActionKind::Move);
                assert_eq!(
                    receipt.target_entity,
                    world.entity_id("era1-object-a").unwrap()
                );
                assert_ne!(world.entity(peer).unwrap().position, before_position);
                assert_eq!(receipt.world_before_digest, before_digest);
                assert_ne!(receipt.world_after_digest, before_digest);
                assert_eq!(
                    receipt.world_after_digest,
                    world.canonical_signature_digest().unwrap().words
                );
                assert_eq!(
                    run_peer_demonstration(&manifest, &mut replay).unwrap(),
                    receipt
                );
                assert_eq!(
                    replay.canonical_signature_digest().unwrap(),
                    world.canonical_signature_digest().unwrap()
                );
            }
        }
    }

    #[test]
    fn registered_trial_command_advances_biology_and_replay_matches() {
        // This catches either trial path bypassing the registered transaction and
        // silently leaving canonical organism biology unchanged.
        let genome = trial_genome();
        let command = HeadlessWorldCommand::idle(SUBJECT).unwrap();

        let mut live = trial_world();
        let initial_record = register_trial_organism(&mut live, SUBJECT, &genome).unwrap();
        let live_receipt = apply_trial_neural_command(
            &mut live,
            &command,
            initial_record.world_entity_id(),
            Tick(1),
            None,
            false,
        )
        .unwrap();

        assert_eq!(live_receipt.biology_before.tick, Tick::ZERO);
        assert_eq!(live_receipt.biology_after.tick, Tick(1));
        assert_ne!(live_receipt.biology_before, live_receipt.biology_after);
        assert_eq!(
            live.organism_registry()
                .get(SUBJECT)
                .unwrap()
                .biochemistry(),
            &live_receipt.biology_after
        );

        let mut replay = trial_world();
        replay
            .register_organism_record(initial_record.clone())
            .unwrap();
        let replay_receipt = apply_trial_neural_command(
            &mut replay,
            &command,
            initial_record.world_entity_id(),
            Tick(1),
            None,
            false,
        )
        .unwrap();

        assert_eq!(replay_receipt.action_result, live_receipt.action_result);
        assert_eq!(replay_receipt.biology_before, live_receipt.biology_before);
        assert_eq!(replay_receipt.biology_after, live_receipt.biology_after);
        assert_eq!(
            replay.canonical_signature_digest().unwrap(),
            live.canonical_signature_digest().unwrap()
        );
    }
}

#[cfg(test)]
mod selector_diagnostic_receipt_tests {
    use super::*;
    use alife_gpu_backend::{
        GpuSelectorBindingIdentity, GpuSelectorCandidateDiagnostic, GpuSelectorCandidateValidity,
        GpuSelectorExplorationMode, GpuSelectorPolicyIdentity, GpuSelectorSynapseContribution,
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
            contributions: if detailed {
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
            } else {
                Vec::new()
            },
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
                schema_version: GpuSelectorPolicyIdentity::PRODUCTION_V1.schema_version,
                frame_digest: PerceptionFrameDigest([1, 2, 3, 4]),
                phenotype_hash: PhenotypeHash([5, 6, 7, 8]),
                dispatch_generation: 9,
                policy: GpuSelectorPolicyIdentity::PRODUCTION_V1,
                adaptive_exploration: None,
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

    fn partition_assessment() -> Era1LearningAssessment {
        Era1LearningAssessment {
            early_acquisition: MetricReading::Measured {
                value_q16: 1,
                exposures: 1,
            },
            late_acquisition: MetricReading::Measured {
                value_q16: 2,
                exposures: 1,
            },
            delay: MetricReading::Measured {
                value_q16: 3,
                exposures: 1,
            },
            probe: MetricReading::Measured {
                value_q16: 4,
                exposures: 1,
            },
            acquisition_improvement_q16: 1,
            probe_change_from_early_q16: 3,
            demonstrated: true,
            grounding_receipts: Vec::new(),
            causal_proof: Era1AbilityCausalProof {
                ability: Era1Ability::FlexibleForaging,
                world_family: Era1WorldFamily::ForagingHazardMaze,
                phase_step_counts: [1, 1, 1],
                required_context_proven: true,
                successful_behavior_ticks: vec![Tick::new(1)],
            },
        }
    }

    #[test]
    fn transfer_and_offspring_partitions_score_probe_behavior() {
        let assessment = partition_assessment();
        assert_eq!(
            score_for_partition(Era1EvidencePartition::Acquisition, &assessment),
            assessment.late_acquisition
        );
        assert_eq!(
            score_for_partition(Era1EvidencePartition::HeldOutTransfer, &assessment),
            assessment.probe
        );
        assert_eq!(
            score_for_partition(Era1EvidencePartition::ReproducedOffspring, &assessment),
            assessment.probe
        );
    }
}
