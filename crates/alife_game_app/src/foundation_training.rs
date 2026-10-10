//! Offline orchestration over the ordinary game runtime and existing trainer.
//! No alternate world step, neural policy, or biological update lives here.

use std::{ops::Range, path::Path, time::Instant};

use alife_core::{
    BrainCapacityClass, BrainGenome, BrainScaleTier, CompiledSynapseKind, DecoderHeadKind,
    DevelopmentState, FoundationWeightAsset, NormalizedScalar, PhenotypeCompiler,
    ScaffoldContractError, SensorProfile, Tick, Validate,
};
use alife_gpu_backend::{
    training_rollout::GpuTrainingStateSnapshot, GpuClosedLoopBackend, GpuRuntimeProfile,
    GpuTrainingSamplingConfig,
};
use alife_training::{
    AdamWConfig, FoundationTrainer, StageTrainableMask, TrainingFrozenSynapse,
    TrainingInitialState, TrainingReplayCandidate, TrainingReplayTick, TrainingSequence,
};

use crate::{FoundationTrainingStep, GpuLiveBrainRuntime};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub(crate) mod diagnostics;
mod maze;
pub use diagnostics::{FoundationLessonBudget, FoundationSpeechBehaviorDiagnostics};

fn invalid() -> ScaffoldContractError {
    ScaffoldContractError::InvalidDecisionEvidence
}

pub(crate) fn prime_foundation_lesson_prior(
    runtime: &mut GpuLiveBrainRuntime,
    _output: &Path,
    lesson: Option<FoundationTeacherLesson>,
) -> Result<()> {
    if matches!(
        lesson,
        Some(FoundationTeacherLesson::EatHeldFood | FoundationTeacherLesson::GrabFood)
    ) {
        // Grounded senses and ordinary organism contracts remain mandatory.
        runtime.require_foundation_rich_information()?;
    }
    runtime.prime_foundation_semantic_prior()?;
    Ok(())
}

pub(crate) fn validate_foundation_lesson_prior(
    runtime: &GpuLiveBrainRuntime,
    output: &Path,
    lesson: Option<FoundationTeacherLesson>,
) -> Result<()> {
    if !matches!(
        lesson,
        Some(FoundationTeacherLesson::EatHeldFood | FoundationTeacherLesson::GrabFood)
    ) {
        return Ok(());
    }
    let complete = runtime.semantic_prior_metrics().is_some_and(|metrics| {
        metrics.decision_frames > 0
            && metrics.decision_frames_without_prior == 0
            && metrics.decision_frames_with_prior == metrics.decision_frames
    });
    std::fs::write(
        output.join("semantic-prior-readiness.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "lesson": lesson,
            "rich_information_complete": complete,
            "failure_policy": "log_and_continue_without_prior",
            "semantic_prior": runtime.semantic_prior_metrics(),
        }))?,
    )?;
    if !complete {
        eprintln!(
            "warning: lesson continued with unaided decisions; see semantic-prior-readiness.json"
        );
    }
    Ok(())
}

fn snapshot_words(snapshot: &GpuTrainingStateSnapshot, range: Range<u32>) -> Result<&[u32]> {
    let start = range
        .start
        .checked_sub(snapshot.mutable_word_base)
        .ok_or_else(invalid)? as usize;
    let end = range
        .end
        .checked_sub(snapshot.mutable_word_base)
        .ok_or_else(invalid)? as usize;
    snapshot
        .mutable_words
        .get(start..end)
        .ok_or_else(|| invalid().into())
}

fn snapshot_values(snapshot: &GpuTrainingStateSnapshot, range: Range<u32>) -> Result<Vec<f32>> {
    let values: Vec<f32> = snapshot_words(snapshot, range)?
        .iter()
        .map(|w| f32::from_bits(*w))
        .collect();
    if values.iter().any(|v| !v.is_finite()) {
        return Err(invalid().into());
    }
    Ok(values)
}

fn activation_range(snapshot: &GpuTrainingStateSnapshot) -> Result<Range<u32>> {
    let ranges = snapshot.brain_slot.word_ranges();
    match snapshot.active_activation_side {
        0 => Ok(ranges.activation_a_words.clone()),
        1 => Ok(ranges.activation_b_words.clone()),
        _ => Err(invalid().into()),
    }
}

fn snapshot_activation(snapshot: &GpuTrainingStateSnapshot) -> Result<Vec<f32>> {
    snapshot_values(snapshot, activation_range(snapshot)?)
}

fn active_weight_ranges(snapshot: &GpuTrainingStateSnapshot) -> Result<(Range<u32>, Range<u32>)> {
    let ranges = snapshot.brain_slot.word_ranges();
    let (lifetime, fast) = match snapshot.active_weight_bank {
        0 => (
            ranges.lifetime_weight_words.clone(),
            ranges.fast_weight_words.clone(),
        ),
        1 => (
            ranges.lifetime_weight_bank_1_words.clone(),
            ranges.fast_weight_bank_1_words.clone(),
        ),
        _ => return Err(invalid().into()),
    };
    let active = snapshot.brain_slot.record().synapse_count;
    if active > lifetime.end - lifetime.start || active > fast.end - fast.start {
        return Err(invalid().into());
    }
    Ok((
        lifetime.start..lifetime.start + active,
        fast.start..fast.start + active,
    ))
}

fn validate_snapshot_topology(snapshot: &GpuTrainingStateSnapshot) -> Result<()> {
    let phenotype = &snapshot.phenotype;
    if snapshot.v11.pending_lifetime_synapse.is_some() {
        return Err("replay cannot capture an uncommitted structural synapse".into());
    }
    let record = snapshot.brain_slot.record();
    let counts = snapshot.brain_slot.typed_counts();
    let neurons = phenotype.neuron_count();
    let recurrent = phenotype
        .synapses()
        .iter()
        .filter(|s| s.kind() == CompiledSynapseKind::Recurrent)
        .count();
    let added = snapshot.structural_synapses.len();
    if added > alife_core::MAX_STRUCTURAL_EDGES
        || record.synapse_count as usize != phenotype.synapses().len() + added
        || record.recurrent_synapse_count as usize != recurrent + added
        || snapshot
            .structural_synapses
            .iter()
            .enumerate()
            .any(|(i, edge)| {
                edge.slot_index >= record.recurrent_synapse_count
                    || edge.source >= neurons
                    || edge.target >= neurons
                    || edge.route as usize >= phenotype.projections().len()
                    || !edge.genetic_weight.is_finite()
                    || edge.alpha.to_bits() != (-0.0_f32).to_bits()
                    || (i > 0 && snapshot.structural_synapses[i - 1].slot_index >= edge.slot_index)
            })
    {
        return Err("captured structural synapses do not match the live GPU slot".into());
    }
    if snapshot.handle.phenotype_hash() != phenotype.phenotype_hash()
        || snapshot.handle.class_id() != phenotype.brain_class_id()
        || record.class_id != u32::from(phenotype.brain_class_id().raw())
        || record.slot != snapshot.handle.slot()
        || record.slot_generation != snapshot.handle.generation()
        || record.neuron_count != neurons
        || record.microstep_count != u32::from(phenotype.microstep_count())
        || counts.source_indices != recurrent + added
        || counts.route_indices != recurrent + added
        || counts.target_offsets != neurons as usize + 1
        || counts.neuron_dynamics != neurons as usize
        || counts.projections != phenotype.projections().len()
        || counts.route_metadata != phenotype.projections().len()
        || snapshot.v11.neuron_count != neurons
        || snapshot.brain_slot.decoder_input_stride()
            != u32::from(phenotype.candidate_decoder().flattened_input_lane_count())
    {
        return Err("captured GPU topology does not match the compiled replay phenotype".into());
    }
    snapshot
        .v11
        .dendritic_branches
        .validate_for_neuron_count(neurons)?;
    let ranges = snapshot.brain_slot.word_ranges();
    if snapshot_activation(snapshot)?.len() != neurons as usize {
        return Err(invalid().into());
    }
    let homeostasis = snapshot_values(snapshot, ranges.homeostasis_words.clone())?;
    if homeostasis.len() != neurons as usize * 2
        || homeostasis.iter().any(|v| !(0.0..=1.0).contains(v))
    {
        return Err(invalid().into());
    }
    Ok(())
}

fn validate_recurrent_continuity(
    previous: &GpuTrainingStateSnapshot,
    next: &GpuTrainingStateSnapshot,
) -> Result<()> {
    if previous.handle != next.handle
        || previous.brain_slot != next.brain_slot
        || previous.logical_dispatch_generation != next.logical_dispatch_generation
        || previous.active_activation_side != next.active_activation_side
        || previous.v11.dendritic_branches != next.v11.dendritic_branches
        || next.active_weight_generation < previous.active_weight_generation
        || snapshot_words(previous, activation_range(previous)?)?
            != snapshot_words(next, activation_range(next)?)?
        || snapshot_words(
            previous,
            previous.brain_slot.word_ranges().homeostasis_words.clone(),
        )? != snapshot_words(
            next,
            next.brain_slot.word_ranges().homeostasis_words.clone(),
        )?
    {
        return Err("replay must split at sleep, reset, topology, or recurrent/homeostasis state discontinuity".into());
    }
    // Ordinary post-action learning can advance the active weight generation.
    // The next row supplies that detached context. A bank/value change without
    // a generation change is inconsistent capture evidence, not learning.
    if previous.active_weight_generation == next.active_weight_generation {
        let (pl, pf) = active_weight_ranges(previous)?;
        let (nl, nf) = active_weight_ranges(next)?;
        if previous.active_weight_bank != next.active_weight_bank
            || snapshot_words(previous, pl)? != snapshot_words(next, nl)?
            || snapshot_words(previous, pf)? != snapshot_words(next, nf)?
        {
            return Err("replay weight state changed without a recorded generation".into());
        }
    }
    Ok(())
}

/// Converts validated ordinary-runtime captures into detached recurrent replay.
/// Requires one contiguous, unchanged-topology waking segment from one organism.
pub fn foundation_replay_sequence(
    steps: &[FoundationTrainingStep],
    burn_in_ticks: usize,
) -> Result<TrainingSequence> {
    let first = steps.first().ok_or_else(invalid)?;
    let upload = alife_gpu_backend::closed_loop_buffers::GpuPhenotypeUpload::try_from(
        first.before.phenotype.as_ref(),
    )?;
    foundation_replay_sequence_with_upload(steps, burn_in_ticks, &upload)
}

fn foundation_replay_sequence_with_upload(
    steps: &[FoundationTrainingStep],
    burn_in_ticks: usize,
    upload: &alife_gpu_backend::closed_loop_buffers::GpuPhenotypeUpload,
) -> Result<TrainingSequence> {
    let first = steps.first().ok_or_else(invalid)?;
    let phenotype = &first.before.phenotype;
    let homeostasis = snapshot_values(
        &first.before,
        first
            .before
            .brain_slot
            .word_ranges()
            .homeostasis_words
            .clone(),
    )?;
    let initial = TrainingInitialState {
        activations: snapshot_activation(&first.before)?,
        activity_ema: homeostasis.chunks_exact(2).map(|r| r[0]).collect(),
        metabolic_load: homeostasis.chunks_exact(2).map(|r| r[1]).collect(),
        dendrites: first.before.v11.dendritic_branches.clone(),
    };
    let mut ticks = Vec::with_capacity(steps.len());
    for (index, step) in steps.iter().enumerate() {
        step.patch.validate_contract()?;
        validate_snapshot_topology(&step.before)?;
        validate_snapshot_topology(&step.after_inference)?;
        if index > 0 {
            validate_recurrent_continuity(&steps[index - 1].after_inference, &step.before)?;
        }
        if step.before.phenotype != *phenotype
            || step.after_inference.phenotype != *phenotype
            || step.frame.organism_id() != first.frame.organism_id()
            || Some(step.frame.tick().raw()) != first.frame.tick().raw().checked_add(index as u64)
            || step.before.handle != first.before.handle
            || step.after_inference.handle != step.before.handle
            || step.before.brain_slot != step.after_inference.brain_slot
            || step.before.structural_synapses != step.after_inference.structural_synapses
            || step.before.handle.organism_id() != step.frame.organism_id()
            || step.before.tick != step.frame.tick().raw()
            || step.after_inference.tick != step.frame.tick().raw()
            || step.behavior.tick != step.frame.tick().raw()
            || step.behavior.organism_id != step.frame.organism_id().raw()
            || step.after_inference.logical_dispatch_generation != step.behavior.dispatch_generation
            || step.before.logical_dispatch_generation
                > step.after_inference.logical_dispatch_generation
            || step.before.active_weight_generation != step.behavior.active_weight_generation
            || step.after_inference.active_weight_generation != step.before.active_weight_generation
            || step.after_inference.active_weight_bank != step.before.active_weight_bank
            || step.after_inference.active_activation_side
                != (step.before.active_activation_side ^ (step.throttle.microsteps & 1))
            || step.behavior.frame_digest != step.frame.frame_digest()
            || step.patch.pre_action().perception().frame_digest() != step.frame.frame_digest()
            || step.behavior.dispatch_generation != step.work.dispatch_generation
            || step.before.v11.dendritic_branches != initial.dendrites
            || step.after_inference.v11.dendritic_branches != initial.dendrites
            || step.behavior.decoder_input_stride
                != usize::from(phenotype.candidate_decoder().flattened_input_lane_count())
            || step.behavior.decoder_input_stride > 54
            || step.behavior.decoder_input_stride < 24
            || step.behavior.decoder_inputs.len()
                != step.frame.candidates().len() * step.behavior.decoder_input_stride
            || step.behavior.logits.len() != step.frame.candidates().len()
        {
            return Err(format!("capture identity mismatch at row {index}: tick/frame/before/after/behavior={}/{}/{}/{}, dispatch(before/after/receipt)={}/{}/{}, weights(before/after/receipt)={}/{}/{}, activation(before/after)/microsteps={}/{}/{}, stride/expected={},{}; logits/candidates={}/{}",
                step.frame.tick().raw(), step.before.tick, step.after_inference.tick, step.behavior.tick,
                step.before.logical_dispatch_generation, step.after_inference.logical_dispatch_generation, step.behavior.dispatch_generation,
                step.before.active_weight_generation, step.after_inference.active_weight_generation, step.behavior.active_weight_generation,
                step.before.active_activation_side, step.after_inference.active_activation_side, step.throttle.microsteps,
                step.behavior.decoder_input_stride, phenotype.candidate_decoder().flattened_input_lane_count(),
                step.behavior.logits.len(), step.frame.candidates().len()).into());
        }
        let (lifetime, fast) = active_weight_ranges(&step.before)?;
        let (after_lifetime, after_fast) = active_weight_ranges(&step.after_inference)?;
        if snapshot_words(&step.before, lifetime.clone())?
            != snapshot_words(&step.after_inference, after_lifetime)?
            || snapshot_words(&step.before, fast.clone())?
                != snapshot_words(&step.after_inference, after_fast)?
        {
            return Err("weights changed inside the captured forward dispatch".into());
        }
        let lifetime = snapshot_values(&step.before, lifetime)?;
        let fast = snapshot_values(&step.before, fast)?;
        if lifetime.len() != phenotype.synapses().len() + step.before.structural_synapses.len()
            || fast.len() != lifetime.len()
        {
            return Err(invalid().into());
        }
        let mut genetic_slot_indices = Vec::with_capacity(phenotype.synapses().len());
        let mut added = step.before.structural_synapses.iter().peekable();
        for slot in 0..lifetime.len() {
            if added
                .peek()
                .is_some_and(|edge| edge.slot_index as usize == slot)
            {
                added.next();
            } else {
                genetic_slot_indices.push(slot);
            }
        }
        if genetic_slot_indices.len() != phenotype.synapses().len() || added.next().is_some() {
            return Err("live structural slot mapping is incomplete".into());
        }
        let effects = step
            .memory_upload
            .neural_receptor_effects
            .as_ref()
            .ok_or_else(invalid)?;
        effects.validate_contract()?;
        if effects.source_tick != step.frame.tick() {
            return Err(invalid().into());
        }
        let mut enabled_routes = vec![false; phenotype.projections().len()];
        for route in &step.throttle.enabled_route_ids {
            let enabled = enabled_routes
                .get_mut(usize::from(*route))
                .ok_or_else(invalid)?;
            if *enabled {
                return Err(invalid().into());
            }
            *enabled = true;
        }
        if phenotype.synapses().iter().any(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(c)
            if c.head() != DecoderHeadKind::SpeechPayload && usize::from(c.input_lane()) >= step.behavior.decoder_input_stride)) {
            return Err("captured decoder lanes omit a compiled head".into());
        }
        let mut candidates = Vec::with_capacity(step.frame.candidates().len());
        for (candidate_index, candidate) in step.frame.candidates().iter().enumerate() {
            let start = candidate_index * step.behavior.decoder_input_stride;
            let inputs = step
                .behavior
                .decoder_inputs
                .get(start..start + step.behavior.decoder_input_stride)
                .ok_or_else(invalid)?;
            let mut decoder_inputs = [0.0; 54];
            decoder_inputs[..inputs.len()].copy_from_slice(inputs);
            candidates.push(TrainingReplayCandidate {
                family: candidate.family,
                decoder_inputs,
                innate_bias: phenotype
                    .candidate_decoder()
                    .families()
                    .iter()
                    .find(|family| family.family() == candidate.family)
                    .ok_or_else(invalid)?
                    .innate_contribution(step.frame.homeostasis().drives, candidate.features),
            });
        }
        ticks.push(TrainingReplayTick {
            encoded_inputs: snapshot_values(
                &step.after_inference,
                step.after_inference
                    .brain_slot
                    .word_ranges()
                    .encoded_input_words
                    .clone(),
            )?,
            projection_gain: effects.projection_gain,
            local_threshold_shift: effects.local_threshold_shift,
            microstep_count: u32::from(step.throttle.microsteps),
            enabled_routes,
            effective_weight_offsets: phenotype
                .synapses()
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let local = upload.canonical_to_local_synapse[i] as usize;
                    let slot = genetic_slot_indices[local];
                    lifetime[slot] + s.alpha() * fast[slot]
                })
                .collect(),
            structural_synapses: step
                .before
                .structural_synapses
                .iter()
                .map(|edge| {
                    let projection = &phenotype.projections()[edge.route as usize];
                    let slot = edge.slot_index as usize;
                    TrainingFrozenSynapse {
                        source: edge.source,
                        target: edge.target,
                        route: edge.route,
                        cadence: u32::from(projection.update_cadence().raw()),
                        effective_weight: edge.genetic_weight
                            + lifetime[slot]
                            + edge.alpha * fast[slot],
                    }
                })
                .collect(),
            candidates,
        });
    }
    let sequence = TrainingSequence {
        phenotype_hash: phenotype.phenotype_hash(),
        initial,
        ticks,
        burn_in_ticks,
        memory_candidate_gain: phenotype
            .candidate_decoder()
            .memory_channel()
            .map_or(0.0, |p| p.max_candidate_gain()),
    };
    sequence.validate_for(phenotype)?;
    Ok(sequence)
}

/// Native canonical N2048 coordinates, never relabelled legacy migrated weights.
pub fn initial_n2048_care_asset(seed: u64) -> Result<FoundationWeightAsset> {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(seed, capacity.id());
    let development = DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0)?);
    let native = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedTerrainVisionV1,
    )?;
    Ok(FoundationWeightAsset::from_phenotype_for_genetic_birth(
        &native,
    )?)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FoundationPilotReceipt {
    pub seed: u64,
    pub founder_seed_base: u64,
    pub ticks: usize,
    pub collection_seconds: f64,
    pub replay_seconds: f64,
    pub maximum_logit_error: f32,
    pub maximum_activation_error: f32,
    pub maximum_activity_ema_error: f32,
    pub maximum_metabolic_error: f32,
    pub source_asset_digest: String,
    pub teacher_mode: bool,
    pub consumed_events: u64,
    pub demonstration_replay_records: usize,
    pub food_position: Option<[f32; 2]>,
    pub initial_food_distance: Option<f32>,
    /// EatHeldFood uses the admitted learner only; null after death or removal.
    pub final_food_distance: Option<f32>,
    pub lesson: Option<FoundationTeacherLesson>,
    pub lesson_completed: Option<bool>,
    #[serde(default)]
    pub vocabulary_token: Option<u16>,
    #[serde(default)]
    pub initial_world_signature: String,
    #[serde(default)]
    pub silent_request: bool,
    #[serde(default)]
    pub request_target: Option<alife_core::WorldEntityId>,
    #[serde(default)]
    pub request_responses: Vec<FoundationRequestResponse>,
    #[serde(default)]
    pub semantic_prior: Option<crate::gpu_live_runtime::SemanticPriorMetrics>,
    #[serde(default)]
    pub heard_word_frames: usize,
    /// Exposure to any configured Teacher token, independent of speech supervision.
    /// Does not prove full command delivery or comprehension.
    #[serde(default)]
    pub teacher_cue_frames: usize,
    #[serde(default)]
    pub teacher_cue_tokens: Vec<u16>,
    #[serde(default)]
    pub held_food_setup: Option<FoundationHeldFoodSetup>,
    #[serde(default)]
    pub grab_food_setup: Option<crate::FoundationGrabFoodSetup>,
    #[serde(default)]
    pub grab_food_acquisitions: Vec<crate::FoundationGrabAcquisition>,
    #[serde(default)]
    pub held_food_consumption_events: usize,
    /// Assessment-only possession snapshots taken before each learner tick.
    #[serde(default)]
    pub held_food_possession_frames: Vec<u64>,
    /// Terminal lifecycle/archive evidence for the admitted held-food learner.
    #[serde(default)]
    pub held_food_terminal_death_tick: Option<u64>,
    /// One-shot terminal biology retained from the successful retirement tick.
    #[serde(default)]
    pub held_food_terminal_biology: Option<alife_core::BiochemistryState>,
    #[serde(default)]
    pub vocabulary_target_actions: usize,
    #[serde(default)]
    pub literal_word_matches: usize,
    #[serde(default)]
    pub correct_utterances: usize,
    #[serde(default)]
    pub unprompted_correct_utterances: usize,
    #[serde(default)]
    pub speech_opportunities: usize,
    #[serde(default)]
    pub speech_behavior: Option<FoundationSpeechBehaviorDiagnostics>,
    #[serde(default)]
    pub maze_nodes_reached: usize,
    #[serde(default)]
    pub maze_route_nodes: usize,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum FoundationTeacherLesson {
    Feeding,
    HazardAvoidance,
    ObstacleNavigation,
    Recovery,
    VisionSearch,
    MazeNavigation,
    VocabularyReception,
    VocabularyProduction,
    EatHeldFood,
    GrabFood,
}

/// Legal possession established before learner capture, not a learner action.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FoundationHeldFoodSetup {
    pub target: alife_core::WorldEntityId,
    pub organism: alife_core::OrganismId,
    pub setup_tick: u64,
    pub grab_command: alife_core::ActionCommand,
    pub grab_physical: alife_core::PhysicalActionOutcome,
    pub initial_energy: f32,
    pub initial_hunger: f32,
    pub initial_health: f32,
    pub initial_sleeping: bool,
    #[serde(default)]
    pub sampling_seed: u64,
    #[serde(default)]
    pub sampling_attempts: u32,
    #[serde(default)]
    pub realized_body_position: Option<alife_core::Vec3f>,
    #[serde(default)]
    pub realized_body_yaw: Option<f32>,
    #[serde(default)]
    pub realized_grip_position: Option<alife_core::Vec3f>,
}

/// Prerequisite fidelity check, not behavioral acceptance or a training campaign.
pub fn run_foundation_training_pilot(
    output: &Path,
    seed: u64,
    tick_count: usize,
) -> Result<FoundationPilotReceipt> {
    run_foundation_training_pilot_inner(output, seed, seed, tick_count, false, None, None, None)
}

/// Diagnostic of the existing legal teacher path against actual ingestion.
pub fn run_foundation_teacher_pilot(
    output: &Path,
    seed: u64,
    tick_count: usize,
) -> Result<FoundationPilotReceipt> {
    run_foundation_training_pilot_inner(output, seed, seed, tick_count, true, None, None, None)
}

pub fn run_foundation_teacher_pilot_with_scenario(
    output: &Path,
    world_seed: u64,
    founder_seed_base: u64,
    tick_count: usize,
    food_position: Option<[f32; 2]>,
) -> Result<FoundationPilotReceipt> {
    run_foundation_training_pilot_inner(
        output,
        world_seed,
        founder_seed_base,
        tick_count,
        true,
        food_position,
        None,
        None,
    )
}

pub fn run_foundation_teacher_pilot_with_lesson(
    output: &Path,
    world_seed: u64,
    founder_seed_base: u64,
    tick_count: usize,
    lesson: FoundationTeacherLesson,
    food_position: Option<[f32; 2]>,
) -> Result<FoundationPilotReceipt> {
    run_foundation_training_pilot_inner(
        output,
        world_seed,
        founder_seed_base,
        tick_count,
        true,
        food_position,
        Some(lesson),
        None,
    )
}

pub fn run_adapted_foundation_teacher_pilot(
    source: &Path,
    output: &Path,
    seed: u64,
    ticks: usize,
    lesson: FoundationTeacherLesson,
) -> Result<FoundationPilotReceipt> {
    let receipt: crate::FoundationAdaptationReceipt =
        serde_json::from_slice(&std::fs::read(source.join("adaptation.json"))?)?;
    let asset = FoundationWeightAsset::decode_canonical(&std::fs::read(
        source.join("trained.alife-foundation"),
    )?)?;
    let digest: String = asset
        .digest()
        .bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if digest != receipt.adapted_asset_digest
        || asset.manifest().sensor_profile() != SensorProfile::GroundedTerrainVisionV1
    {
        return Err("teacher source is not the sealed terrain adaptation".into());
    }
    run_foundation_training_pilot_inner(
        output,
        seed,
        receipt.founder_seed_base,
        ticks,
        true,
        None,
        Some(lesson),
        Some(asset),
    )
}

/// Offline evaluation controls change only the audible request, after the scene
/// has been constructed. They are not learner inputs or executable teacher policy.
#[derive(Debug, Clone, Copy)]
pub struct FoundationEvaluationRequest {
    pub token: u16,
    pub silent: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FoundationRequestResponse {
    pub tick: u64,
    pub action: u32,
    pub target: Option<alife_core::WorldEntityId>,
}

pub fn run_foundation_request_evaluation(
    output: &Path,
    world_seed: u64,
    founder_seed_base: u64,
    tick_count: usize,
    asset: FoundationWeightAsset,
    request: FoundationEvaluationRequest,
) -> Result<FoundationPilotReceipt> {
    if !matches!(request.token, 1 | 2 | 9 | 13 | 15 | 16) {
        return Err("controlled request supports food/toy, get/play, and root/fruit pairs".into());
    }
    run_foundation_training_pilot_with_request(
        output,
        world_seed,
        founder_seed_base,
        tick_count,
        false,
        None,
        Some(FoundationTeacherLesson::VocabularyReception),
        Some(asset),
        Some(request),
    )
}

/// A frozen founder acts without a demonstrator in the same production world.
pub fn run_foundation_evaluation_pilot(
    output: &Path,
    world_seed: u64,
    founder_seed_base: u64,
    tick_count: usize,
    lesson: FoundationTeacherLesson,
    food_position: Option<[f32; 2]>,
    asset: FoundationWeightAsset,
) -> Result<FoundationPilotReceipt> {
    run_foundation_training_pilot_inner(
        output,
        world_seed,
        founder_seed_base,
        tick_count,
        false,
        food_position,
        Some(lesson),
        Some(asset),
    )
}

#[derive(Default)]
struct GroundedNavigationTeacher {
    maze: maze::MazeTeacher,
    vocabulary_target: Option<alife_core::WorldEntityId>,
    vocabulary_position: Option<alife_core::Vec3f>,
    vocabulary_word: Option<u32>,
    vocabulary_noun: Option<u16>,
    last_speech_tick: Option<u64>,
    previous_receipt: std::sync::Arc<std::sync::Mutex<Option<alife_core::ExperiencePatch>>>,
    held_targets: std::sync::Arc<std::sync::Mutex<Vec<alife_core::WorldEntityId>>>,
    remembered_food: Option<alife_core::Vec3f>,
    search_phase: u8,
    detour_side: f32,
    clear_steps: u8,
}

impl GroundedNavigationTeacher {
    fn choose<'a>(
        &mut self,
        frame: &'a alife_core::PerceptionFrame,
        food: alife_core::WorldEntityId,
    ) -> Option<&'a alife_core::ActionCandidate> {
        use alife_core::CandidateActionFamily as Family;
        use alife_world::HeadlessActionIds as Id;
        let action = |id| frame.candidates().iter().find(|c| c.action_id == id);
        let food_candidate = frame
            .candidates()
            .iter()
            .find(|c| c.target.entity == Some(food) && c.family == Family::Approach);
        let slot = food_candidate.and_then(|c| match c.observation {
            alife_core::CandidateObservationRef::ObjectSlot(index) => frame
                .grounded_object_slots()
                .iter()
                .find(|s| s.slot_index == index),
            _ => None,
        });
        let gaze = 2.0
            * frame
                .body()
                .pose
                .rotation
                .y
                .atan2(frame.body().pose.rotation.w);
        if let Some(offset) = &mut self.remembered_food {
            // Proprioception reports the last interval's actual displacement,
            // including zero movement when blocked. No absolute GPS is used.
            offset.x -= frame.body().velocity.linear.x;
            offset.z -= frame.body().velocity.linear.z;
        }
        if let Some(slot) = slot {
            let heading = gaze + slot.bearing[0].atan2(slot.bearing[1]);
            let range = slot.distance * alife_world::HEADLESS_VISION_RADIUS;
            self.remembered_food = Some(alife_core::Vec3f::new(
                range * heading.cos(),
                0.0,
                range * heading.sin(),
            ));
            if slot.contact >= 0.5 {
                return frame
                    .candidates()
                    .iter()
                    .find(|c| c.target.entity == Some(food) && c.family == Family::Ingest);
            }
        }
        if self.remembered_food.is_none() {
            let id = match self.search_phase % 7 {
                0 | 1 => Id::LOOK_LEFT,
                2 | 3 | 4 => Id::LOOK_RIGHT,
                5 => Id::LOOK_CENTER,
                _ => Id::TURN_LEFT,
            };
            self.search_phase = self.search_phase.wrapping_add(1);
            return action(id);
        }
        // Recentering is an actual motor choice. Heading is sensed orientation,
        // never a lookup of a hidden food's world position.
        if self.search_phase != 0 {
            self.search_phase = 0;
            return action(Id::LOOK_CENTER);
        }
        let fan = &frame.sensory().channels.visual_affordance;
        let forward_clearance = fan[7].min(fan[8]);
        if forward_clearance < 0.14 {
            if self.detour_side == 0.0 {
                self.detour_side =
                    if fan[10..13].iter().sum::<f32>() >= fan[3..6].iter().sum::<f32>() {
                        1.0
                    } else {
                        -1.0
                    };
            }
            self.clear_steps = 0;
            return action(if self.detour_side > 0.0 {
                Id::TURN_LEFT
            } else {
                Id::TURN_RIGHT
            });
        }
        if self.detour_side != 0.0 && self.clear_steps < 3 {
            self.clear_steps += 1;
            return action(Id::STEP_FORWARD);
        }
        let remembered = self.remembered_food?;
        let heading = remembered.z.atan2(remembered.x);
        let error = (heading - gaze + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        if slot.is_some() {
            self.detour_side = 0.0;
        }
        if error.abs() > 0.22 {
            return action(if error > 0.0 {
                Id::TURN_LEFT
            } else {
                Id::TURN_RIGHT
            });
        }
        if slot.is_some() {
            food_candidate
        } else {
            action(Id::STEP_FORWARD)
        }
    }
}

fn grounded_lesson_teacher(
    frame: &alife_core::PerceptionFrame,
    enabled_channels: u32,
    lesson: FoundationTeacherLesson,
    waypoint_passed: &mut bool,
    food: alife_core::WorldEntityId,
    hazard: alife_core::WorldEntityId,
    waypoint: alife_core::WorldEntityId,
    navigation: &mut GroundedNavigationTeacher,
) -> std::result::Result<alife_gpu_backend::GpuTrainingDemonstratorAction, ScaffoldContractError> {
    use alife_core::CandidateActionFamily as Family;
    let held_targets = navigation.held_targets.lock().unwrap().clone();
    if lesson == FoundationTeacherLesson::GrabFood {
        let chosen = frame
            .candidates()
            .iter()
            .find(|candidate| {
                candidate.action_id == alife_world::HeadlessActionIds::GRAB
                    && candidate.target.entity == Some(food)
            })
            .ok_or(ScaffoldContractError::InvalidActionDecision)?;
        return assemble_grounded_teacher(frame, enabled_channels, chosen);
    }
    if lesson == FoundationTeacherLesson::EatHeldFood {
        // The acquisition transaction belongs to setup. The demonstration has
        // only the already-held ingest primitive; no route or Grab policy.
        let chosen = frame
            .candidates()
            .iter()
            .find(|candidate| {
                candidate.action_id == alife_world::HeadlessActionIds::EAT
                    && candidate.target.entity == Some(food)
            })
            .ok_or(ScaffoldContractError::InvalidActionDecision)?;
        return assemble_grounded_teacher(frame, enabled_channels, chosen);
    }
    if lesson == FoundationTeacherLesson::MazeNavigation {
        let chosen = navigation
            .maze
            .choose(frame, food)
            .ok_or(ScaffoldContractError::InvalidActionDecision)?;
        return assemble_grounded_teacher(
            frame,
            enabled_channels,
            teacher_possession_action(frame, chosen, &held_targets)?,
        );
    }
    if matches!(
        lesson,
        FoundationTeacherLesson::VocabularyReception
            | FoundationTeacherLesson::VocabularyProduction
    ) {
        let heard = &frame.sensory().language_context.heard_tokens;
        let teacher_words = heard
            .iter()
            .flatten()
            .filter(|h| h.source_kind == alife_core::UtteranceSourceKind::Teacher)
            .collect::<Vec<_>>();
        let word = teacher_words
            .iter()
            .find(|h| (5..=10).contains(&h.token_id) || h.token_id == 13)
            .or_else(|| teacher_words.last())
            .map(|h| h.token_id);
        if let Some(word) = word {
            navigation.vocabulary_word = Some(word);
        }
        let action = if lesson == FoundationTeacherLesson::VocabularyProduction
            && !teacher_words.is_empty()
        {
            frame.candidates().iter().find(|c| c.family == Family::Idle)
        } else {
            vocabulary_reception_choice(
                frame,
                navigation.vocabulary_word,
                navigation.vocabulary_target,
                navigation.vocabulary_position,
            )
        }
        .ok_or(ScaffoldContractError::InvalidActionDecision)?;
        let action = teacher_possession_action(frame, action, &held_targets)?;
        // Ordinary past action/physical receipts, never future outcomes or route inputs.
        let speak = grounded_speech_label(
            frame,
            navigation.vocabulary_word.map(|w| w as u16),
            navigation.vocabulary_noun,
            navigation.vocabulary_target,
            navigation.previous_receipt.lock().unwrap().as_ref(),
            lesson == FoundationTeacherLesson::VocabularyProduction,
        )
        .is_some_and(|label| label.token.is_some());
        if speak
            && navigation.last_speech_tick.is_none_or(|at| {
                frame.tick().raw().saturating_sub(at)
                    >= alife_world::SPONTANEOUS_SPEECH_COOLDOWN_TICKS
            })
        {
            if let Some(vocal) = frame
                .candidates()
                .iter()
                .find(|c| c.kind == alife_core::ActionKind::Vocalize)
            {
                navigation.last_speech_tick = Some(frame.tick().raw());
                let mut demonstration = assemble_grounded_teacher(frame, enabled_channels, vocal)?;
                let slot = match action.kind {
                    alife_core::ActionKind::Move => 0,
                    alife_core::ActionKind::Look => 1,
                    alife_core::ActionKind::Interact => 2,
                    _ => 4,
                };
                if enabled_channels & (1 << slot) != 0
                    && action.kind != alife_core::ActionKind::Idle
                {
                    demonstration.motor_indices[slot] = action.candidate_index;
                }
                return Ok(demonstration);
            }
        }
        return assemble_grounded_teacher(frame, enabled_channels, action);
    }
    if frame.sensor_profile() == SensorProfile::GroundedTerrainVisionV1
        && matches!(
            lesson,
            FoundationTeacherLesson::Feeding
                | FoundationTeacherLesson::ObstacleNavigation
                | FoundationTeacherLesson::VisionSearch
                | FoundationTeacherLesson::Recovery
        )
    {
        let chosen = if lesson == FoundationTeacherLesson::Recovery
            && frame.homeostasis().drives.fatigue >= 0.12
        {
            frame.candidates().iter().find(|c| c.family == Family::Rest)
        } else {
            navigation.choose(frame, food)
        }
        .ok_or(ScaffoldContractError::InvalidActionDecision)?;
        return assemble_grounded_teacher(
            frame,
            enabled_channels,
            teacher_possession_action(frame, chosen, &held_targets)?,
        );
    }
    let target = |entity, family| {
        frame
            .candidates()
            .iter()
            .find(|candidate| candidate.target.entity == Some(entity) && candidate.family == family)
    };
    let food_contact = frame.candidates().iter().any(|candidate| {
        candidate.target.entity == Some(food)
            && matches!(candidate.observation, alife_core::CandidateObservationRef::ObjectSlot(index)
                if frame.grounded_object_slots().iter().any(|slot| slot.slot_index == index && slot.contact >= 0.5))
    });
    let waypoint_near = target(waypoint, Family::Approach)
        .and_then(|candidate| match candidate.observation {
            alife_core::CandidateObservationRef::ObjectSlot(index) => frame
                .grounded_object_slots()
                .iter()
                .find(|slot| slot.slot_index == index),
            alife_core::CandidateObservationRef::None => None,
        })
        .is_some_and(|slot| slot.contact >= 0.5 || slot.distance <= 0.25);
    if waypoint_near {
        *waypoint_passed = true;
    }
    let hazard_near = target(hazard, Family::Avoid)
        .and_then(|candidate| match candidate.observation {
            alife_core::CandidateObservationRef::ObjectSlot(index) => frame
                .grounded_object_slots()
                .iter()
                .find(|slot| slot.slot_index == index),
            alife_core::CandidateObservationRef::None => None,
        })
        .is_some_and(|slot| slot.distance < 0.75);
    if frame.sensor_profile() == SensorProfile::GroundedTerrainVisionV1
        && lesson == FoundationTeacherLesson::HazardAvoidance
    {
        let chosen = if hazard_near {
            target(hazard, Family::Avoid)
        } else {
            navigation.choose(frame, food)
        }
        .ok_or(ScaffoldContractError::InvalidActionDecision)?;
        return assemble_grounded_teacher(
            frame,
            enabled_channels,
            teacher_possession_action(frame, chosen, &held_targets)?,
        );
    }
    let chosen = match lesson {
        FoundationTeacherLesson::Feeding => target(
            food,
            if food_contact {
                Family::Ingest
            } else {
                Family::Approach
            },
        ),
        FoundationTeacherLesson::HazardAvoidance if hazard_near => target(hazard, Family::Avoid),
        FoundationTeacherLesson::ObstacleNavigation if !*waypoint_passed => {
            target(waypoint, Family::Approach)
        }
        FoundationTeacherLesson::ObstacleNavigation => target(
            food,
            if food_contact {
                Family::Ingest
            } else {
                Family::Approach
            },
        ),
        FoundationTeacherLesson::HazardAvoidance => target(
            food,
            if food_contact {
                Family::Ingest
            } else {
                Family::Approach
            },
        ),
        FoundationTeacherLesson::Recovery if frame.homeostasis().drives.fatigue >= 0.12 => frame
            .candidates()
            .iter()
            .find(|candidate| candidate.family == Family::Rest),
        FoundationTeacherLesson::Recovery => target(
            food,
            if food_contact {
                Family::Ingest
            } else {
                Family::Approach
            },
        ),
        _ => navigation.choose(frame, food),
    }
    .ok_or(ScaffoldContractError::InvalidActionDecision)?;
    assemble_grounded_teacher(
        frame,
        enabled_channels,
        teacher_possession_action(frame, chosen, &held_targets)?,
    )
}

/// Offline demonstrator sequencing only. This possession snapshot never enters
/// candidate enumeration or the learner's inference inputs.
fn teacher_possession_action<'a>(
    frame: &'a alife_core::PerceptionFrame,
    chosen: &'a alife_core::ActionCandidate,
    held_targets: &[alife_core::WorldEntityId],
) -> std::result::Result<&'a alife_core::ActionCandidate, ScaffoldContractError> {
    if chosen.action_id != alife_world::HeadlessActionIds::EAT {
        return Ok(chosen);
    }
    let target = chosen
        .target
        .entity
        .ok_or(ScaffoldContractError::InvalidActionDecision)?;
    if held_targets.contains(&target) {
        return Ok(chosen);
    }
    frame
        .candidates()
        .iter()
        .find(|candidate| {
            candidate.action_id == alife_world::HeadlessActionIds::GRAB
                && candidate.target.entity == Some(target)
        })
        .ok_or(ScaffoldContractError::InvalidActionDecision)
}

fn assemble_grounded_teacher(
    frame: &alife_core::PerceptionFrame,
    enabled_channels: u32,
    chosen: &alife_core::ActionCandidate,
) -> std::result::Result<alife_gpu_backend::GpuTrainingDemonstratorAction, ScaffoldContractError> {
    let forced = match chosen.kind {
        alife_core::ActionKind::Move => 0,
        alife_core::ActionKind::Look => 1,
        alife_core::ActionKind::Interact | alife_core::ActionKind::Write => 2,
        alife_core::ActionKind::Vocalize => 3,
        _ => 4,
    };
    let mut motor_indices = [u16::MAX; 6];
    for candidate in frame.candidates() {
        let slot = match candidate.kind {
            alife_core::ActionKind::Move => Some(0),
            alife_core::ActionKind::Look => Some(1),
            alife_core::ActionKind::Interact | alife_core::ActionKind::Write => Some(2),
            alife_core::ActionKind::Vocalize => Some(3),
            alife_core::ActionKind::Hold
            | alife_core::ActionKind::Rest
            | alife_core::ActionKind::Inspect => Some(4),
            alife_core::ActionKind::Idle | alife_core::ActionKind::Gesture => None,
        };
        if let Some(slot) = slot {
            if enabled_channels & (1 << slot) != 0
                && (motor_indices[slot] == u16::MAX
                    || (slot == 0
                        && candidate.action_id == alife_world::HeadlessActionIds::NO_LOCOMOTION)
                    || (slot == 1
                        && candidate.action_id == alife_world::HeadlessActionIds::HOLD_GAZE)
                    || (slot == 2
                        && candidate.action_id == alife_world::HeadlessActionIds::NO_MANIPULATION)
                    || (slot == 4
                        && candidate.action_id == alife_world::HeadlessActionIds::NO_POSTURE))
            {
                motor_indices[slot] = candidate.candidate_index;
            }
        }
    }
    motor_indices[forced] = chosen.candidate_index;
    if std::env::var_os("ALIFE_FOUNDATION_TEACHER_TRACE").is_some() {
        eprintln!(
            "teacher tick={} enabled={enabled_channels:#08b} representative={} kind={:?} family={:?} motors={motor_indices:?} slots={:?} candidates={:?}",
            frame.tick().raw(),
            chosen.candidate_index,
            chosen.kind,
            chosen.family,
            frame
                .grounded_object_slots()
                .iter()
                .map(|slot| (slot.slot_index, slot.distance, slot.contact))
                .collect::<Vec<_>>(),
            frame
                .candidates()
                .iter()
                .map(|candidate| (candidate.candidate_index, candidate.kind, candidate.family))
                .collect::<Vec<_>>()
        );
    }
    Ok(alife_gpu_backend::GpuTrainingDemonstratorAction {
        representative_index: chosen.candidate_index,
        motor_indices,
    })
}

fn vocabulary_reception_choice(
    frame: &alife_core::PerceptionFrame,
    word: Option<u32>,
    target: Option<alife_core::WorldEntityId>,
    known_position: Option<alife_core::Vec3f>,
) -> Option<&alife_core::ActionCandidate> {
    use alife_core::{ActionKind as Kind, CandidateActionFamily as Family};
    let family = match word {
        Some(1..=5 | 14..=18) => Family::Inspect,
        Some(7) => Family::Avoid,
        Some(8) => Family::Ingest,
        Some(9 | 13) => Family::Contact,
        Some(10 | 12) => Family::Rest,
        Some(11) => Family::Ingest,
        Some(6) => Family::Approach,
        _ => Family::Idle,
    };
    let range = frame
        .candidates()
        .iter()
        .find(|c| c.target.entity == target)
        .and_then(|c| match c.observation {
            alife_core::CandidateObservationRef::ObjectSlot(i) => frame
                .grounded_object_slots()
                .iter()
                .find(|s| s.slot_index == i)
                .map(|s| s.distance * alife_world::HEADLESS_VISION_RADIUS),
            _ => None,
        });
    if range.is_none() && !matches!(family, Family::Rest | Family::Idle | Family::Avoid) {
        if let Some(position) = known_position {
            let body = frame.body().pose;
            let heading = (position.z - body.translation.z).atan2(position.x - body.translation.x);
            let gaze = 2.0 * body.rotation.y.atan2(body.rotation.w);
            let error = (heading - gaze + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            let primitive = if error.abs() > 0.18 {
                if error > 0.0 {
                    alife_world::HeadlessActionIds::TURN_LEFT
                } else {
                    alife_world::HeadlessActionIds::TURN_RIGHT
                }
            } else {
                alife_world::HeadlessActionIds::STEP_FORWARD
            };
            return frame.candidates().iter().find(|c| c.action_id == primitive);
        }
    }
    let at_contact = frame
        .candidates()
        .iter()
        .find(|c| c.target.entity == target)
        .and_then(|c| match c.observation {
            alife_core::CandidateObservationRef::ObjectSlot(i) => frame
                .grounded_object_slots()
                .iter()
                .find(|s| s.slot_index == i)
                .map(|s| s.contact >= 0.5),
            _ => None,
        })
        .unwrap_or(false);
    // Picking up is complete once something is held. Repeating `get` must not
    // immediately release the object through Grab's reversible physical toggle.
    if word == Some(9) && frame.sensory().channels.tactile_contact[2] > 0.5 {
        return frame.candidates().iter().find(|c| c.family == Family::Idle);
    }
    if matches!(family, Family::Ingest | Family::Contact) && !at_contact {
        return frame
            .candidates()
            .iter()
            .find(|c| c.target.entity == target && c.family == Family::Approach);
    }
    let retreat_range = range.or_else(|| {
        known_position.map(|p| {
            (p.x - frame.body().pose.translation.x).hypot(p.z - frame.body().pose.translation.z)
        })
    });
    if (family == Family::Avoid && retreat_range.is_some_and(|r| r > 4.0))
        || (family == Family::Approach && range.is_some_and(|r| r <= 1.5))
    {
        return frame.candidates().iter().find(|c| c.family == Family::Idle);
    }
    if family == Family::Avoid && range.is_none() && retreat_range.is_some_and(|r| r <= 4.0) {
        // Retreat toward the away heading even if crowding or occlusion hid
        // the first Flee candidate. A blind forward step can approach instead.
        // The planner's remembered position remains teacher-private.
        if let Some(position) = known_position {
            let body = frame.body().pose;
            let heading = (body.translation.z - position.z).atan2(body.translation.x - position.x);
            let facing = 2.0 * body.rotation.y.atan2(body.rotation.w);
            let error = (heading - facing + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            if error.abs() > 0.18 {
                let primitive = if error > 0.0 {
                    alife_world::HeadlessActionIds::TURN_LEFT
                } else {
                    alife_world::HeadlessActionIds::TURN_RIGHT
                };
                return frame.candidates().iter().find(|c| c.action_id == primitive);
            }
        }
        return frame
            .candidates()
            .iter()
            .find(|c| c.action_id == alife_world::HeadlessActionIds::STEP_FORWARD);
    }
    frame
        .candidates()
        .iter()
        .filter(|c| {
            c.family == family
                && c.kind != Kind::Vocalize
                && (word != Some(9) || c.action_id == alife_world::HeadlessActionIds::GRAB)
                && (word != Some(13) || c.action_id == alife_world::HeadlessActionIds::PLAY)
                && (matches!(family, Family::Idle | Family::Rest) || c.target.entity == target)
        })
        .min_by(|a, b| a.features.0[2].total_cmp(&b.features.0[2]))
        .or_else(|| {
            frame
                .candidates()
                .iter()
                .find(|c| c.target.entity == target && c.family == Family::Approach)
        })
        .or_else(|| frame.candidates().iter().find(|c| c.family == Family::Idle))
}

pub(crate) fn grounded_speech_label(
    frame: &alife_core::PerceptionFrame,
    word: Option<u16>,
    noun: Option<u16>,
    target: Option<alife_core::WorldEntityId>,
    previous: Option<&alife_core::ExperiencePatch>,
    production: bool,
) -> Option<alife_training::ReplaySpeechTarget> {
    let word = word?;
    let ready = production
        && frame
            .sensory()
            .language_context
            .heard_tokens
            .iter()
            .all(Option::is_none);
    let mut label = alife_training::ReplaySpeechTarget {
        token: if ready { Some(word) } else { None },
        continuation: [0; 5],
        act: alife_core::SpeechActKind::Declare,
        weight: if ready { 1.0 } else { 0.25 },
    };
    if !ready {
        return Some(label);
    }
    let needs = frame.homeostasis().drives;
    let need = match word {
        11 if needs.hunger >= 0.12 => Some((11, 1)),
        12 if needs.fatigue >= 0.12 => Some((12, 10)),
        _ => None,
    };
    let acted = previous.is_some_and(|patch| {
        patch.outcome().success
            && !matches!(
                patch.outcome().physical.contact,
                alife_core::PhysicalContactKind::Blocked
                    | alife_core::PhysicalContactKind::Collision
            )
            && patch
                .pre_action()
                .perception()
                .candidates()
                .iter()
                .any(|candidate| {
                    let expected = match word {
                        1..=5 | 14..=18 => {
                            candidate.family == alife_core::CandidateActionFamily::Inspect
                        }
                        6 => candidate.family == alife_core::CandidateActionFamily::Approach,
                        7 => candidate.family == alife_core::CandidateActionFamily::Avoid,
                        8 => candidate.action_id == alife_world::HeadlessActionIds::EAT,
                        9 => {
                            candidate.action_id == alife_world::HeadlessActionIds::GRAB
                                && frame.sensory().channels.tactile_contact[2] > 0.5
                        }
                        13 => candidate.action_id == alife_world::HeadlessActionIds::PLAY,
                        10 => candidate.family == alife_core::CandidateActionFamily::Rest,
                        _ => false,
                    };
                    expected
                        && (word == 10 || candidate.target.entity == target)
                        && patch
                            .decision()
                            .selected_bundle
                            .as_ref()
                            .is_some_and(|bundle| {
                                bundle.channels.iter().any(|channel| {
                                    channel.primitive == candidate.action_id
                                        && (word == 10
                                            || channel.target.and_then(|t| t.entity) == target)
                                })
                            })
                })
    });
    if let Some((first, second)) = need {
        label.token = Some(first);
        label.continuation[0] = second;
    } else if acted {
        if matches!(word, 5..=9 | 13) {
            label.continuation[0] = noun.unwrap_or(1);
        }
    } else {
        label.token = None;
        label.weight = 0.25;
    }
    Some(label)
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct DemonstrationSpeechLabels {
    pub source: FoundationReplaySource,
    pub record_digests: Vec<[u8; 32]>,
    pub targets: Vec<Option<alife_training::ReplaySpeechTarget>>,
}

fn run_foundation_training_pilot_inner(
    output: &Path,
    seed: u64,
    founder_seed_base: u64,
    tick_count: usize,
    teacher_mode: bool,
    food_position: Option<[f32; 2]>,
    lesson: Option<FoundationTeacherLesson>,
    asset_override: Option<FoundationWeightAsset>,
) -> Result<FoundationPilotReceipt> {
    run_foundation_training_pilot_with_request(
        output,
        seed,
        founder_seed_base,
        tick_count,
        teacher_mode,
        food_position,
        lesson,
        asset_override,
        None,
    )
}

fn run_foundation_training_pilot_with_request(
    output: &Path,
    seed: u64,
    founder_seed_base: u64,
    tick_count: usize,
    teacher_mode: bool,
    food_position: Option<[f32; 2]>,
    lesson: Option<FoundationTeacherLesson>,
    asset_override: Option<FoundationWeightAsset>,
    request: Option<FoundationEvaluationRequest>,
) -> Result<FoundationPilotReceipt> {
    if !(1..=2048).contains(&tick_count) || seed == 0 || founder_seed_base == 0 {
        return Err(invalid().into());
    }
    std::fs::create_dir(output)?; // A fresh run never overwrites an earlier receipt.
    let asset = if let Some(asset) = asset_override {
        asset
    } else {
        initial_n2048_care_asset(founder_seed_base)?
    };
    std::fs::write(
        output.join("initial.alife-foundation"),
        asset.encode_canonical()?,
    )?;
    let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = founder_seed_base;
    config.sensor_profile = asset.manifest().sensor_profile();
    let mut game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)?;
    game.world.set_age_death_disabled_for_new_game(true)?;
    let scenario_lesson =
        lesson.or_else(|| teacher_mode.then_some(FoundationTeacherLesson::Feeding));
    let mut scenario =
        configure_foundation_scenario(&mut game.world, seed, scenario_lesson, food_position, true)?;
    scenario.repeat_vocabulary =
        teacher_mode && scenario_lesson != Some(FoundationTeacherLesson::VocabularyProduction);
    scenario.training_language_feedback = teacher_mode
        && !matches!(
            scenario_lesson,
            Some(FoundationTeacherLesson::EatHeldFood | FoundationTeacherLesson::GrabFood)
        );
    if let Some(request) = request {
        // Construct once from the same seed; never relocate objects, change needs,
        // or change a toy's mobility to match the counterfactual request.
        let target = match request.token {
            1 => Some(scenario.food),
            2 | 9 | 13 => game.world.entity_id("vocabulary-toy"),
            15 | 16 => game
                .world
                .object_snapshots()
                .iter()
                .find(|object| {
                    object.kind == alife_world::WorldObjectKind::Food
                        && (object.nutrition
                            - alife_world::FoodVariety::from_seed(u64::from(request.token - 15))
                                .nutrition())
                        .abs()
                            < 0.0001
                })
                .map(|object| object.id),
            _ => None,
        }
        .ok_or("controlled request has no physical referent in this scene")?;
        if matches!(request.token, 9 | 13)
            && game
                .world
                .entity(target)
                .is_none_or(|object| object.kind != alife_world::WorldObjectKind::Ball)
        {
            return Err("get/play comparison needs the same movable ball scene".into());
        }
        scenario.vocabulary_token = Some(request.token);
        scenario.vocabulary_target = Some(target);
        scenario.vocabulary_noun = Some(if matches!(request.token, 2 | 9 | 13) {
            14
        } else {
            request.token
        });
        scenario.silent_request = request.silent;
        let origin = game
            .world
            .entity(game.world.organism_entity_ids()[0].1)
            .ok_or("comparison subject missing")?
            .position;
        let position = alife_core::Vec3f::new(origin.x - 2.0, origin.y, origin.z + 0.5);
        let raw = game
            .world
            .organism_entity_ids()
            .iter()
            .map(|(id, _)| id.raw())
            .max()
            .unwrap_or(0)
            + 2;
        if let Some(teacher) = game.world.entity_id("nursery-teacher") {
            game.world.editor_move_object(teacher, position)?;
        } else {
            game.world.spawn_social_agent(
                "nursery-teacher",
                alife_core::OrganismId(raw),
                position,
                0.75,
            )?;
        }
        scenario.fixed_speaker_position = Some(position);
    }
    let initial_world_signature = format!("{:?}", game.world.canonical_signature_digest()?);
    let food = scenario.food;
    let food_position = scenario_lesson.map(|_| {
        let position = game
            .world
            .entity(food)
            .expect("scenario food exists")
            .position;
        [position.x, position.z]
    });
    let hazard = scenario.hazard;
    let waypoint = scenario.waypoint;
    let initial_hazard_distance = scenario.initial_hazard_distance;
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    let mut runtime = GpuLiveBrainRuntime::new_profiled_foundation_training(
        backend,
        game.world,
        seed,
        BrainScaleTier::Standard2048,
        config.sensor_profile,
        alife_archive::LineageLibraryConfig::profile_default(output.join("lineage")),
        format!("n2048-training-pilot-{seed}"),
        alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
        GpuTrainingSamplingConfig {
            seed: seed as u32,
            counter: 0,
            temperature: 1.0,
            demonstrator: None,
        },
    )
    .map_err(|error| format!("pilot runtime admission: {error}"))?;
    let teacher_previous = std::sync::Arc::new(std::sync::Mutex::new(None));
    let teacher_held_targets = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    if teacher_mode {
        let mut navigation = GroundedNavigationTeacher::default();
        navigation.maze.route = scenario.demonstration_route.clone();
        navigation.vocabulary_target = scenario.vocabulary_target;
        navigation.previous_receipt = teacher_previous.clone();
        navigation.held_targets = teacher_held_targets.clone();
        navigation.vocabulary_noun = scenario.vocabulary_noun;
        navigation.vocabulary_position = scenario
            .vocabulary_target
            .and_then(|id| runtime.world().entity(id))
            .map(|o| o.position);
        let lesson = scenario_lesson.ok_or("teacher lesson is missing")?;
        let mut waypoint_passed = false;
        runtime.set_foundation_demonstrator(move |frame, channels| {
            grounded_lesson_teacher(
                frame,
                channels,
                lesson,
                &mut waypoint_passed,
                food,
                hazard,
                waypoint,
                &mut navigation,
            )
        })?;
    }
    let initial_food_distance = if let Some(setup) = scenario.grab_food_setup.as_ref() {
        held_food_learner_distance(runtime.world(), food, setup.organism)
    } else if scenario_lesson == Some(FoundationTeacherLesson::EatHeldFood) {
        scenario
            .held_food_setup
            .as_ref()
            .and_then(|setup| held_food_learner_distance(runtime.world(), food, setup.organism))
    } else {
        scenario_lesson
            .is_some()
            .then(|| teacher_food_distance(&runtime))
            .transpose()?
    };
    close_foundation_navigation_gate(&mut runtime, &scenario)?;
    prime_foundation_lesson_prior(&mut runtime, output, scenario_lesson)?;
    let started = Instant::now();
    let mut steps = Vec::with_capacity(tick_count);
    let mut held_food_possession_frames = Vec::new();
    let mut grab_food_acquisitions = Vec::new();
    let mut held_food_terminal_biology = None;
    let mut correct_utterances = 0;
    let mut unprompted_correct_utterances = 0;
    let mut speech_opportunities = 0;
    let mut speech_behavior = FoundationSpeechBehaviorDiagnostics::default();
    for tick in 0..tick_count {
        if tick != 0 {
            close_foundation_navigation_gate(&mut runtime, &scenario)?;
            if matches!(
                scenario_lesson,
                Some(FoundationTeacherLesson::EatHeldFood | FoundationTeacherLesson::GrabFood)
            ) {
                prime_foundation_lesson_prior(&mut runtime, output, scenario_lesson)?;
            }
        }
        if teacher_mode {
            let learner = runtime.world().organism_entity_ids()[0].0;
            *teacher_held_targets.lock().unwrap() = runtime
                .world()
                .object_snapshots()
                .into_iter()
                .filter(|object| object.carried_by == Some(learner) && !object.consumed)
                .map(|object| object.id)
                .collect();
        }
        let held_food_before = runtime.world().entity(food).is_some_and(|object| {
            scenario
                .held_food_setup
                .as_ref()
                .is_some_and(|setup| object.carried_by == Some(setup.organism) && !object.consumed)
        });
        if held_food_before {
            held_food_possession_frames.push(runtime.world().tick().raw());
        }
        let grab_before = scenario.grab_food_setup.as_ref().and_then(|setup| {
            crate::foundation_grab_food::food_ownership(runtime.world(), setup.target)
        });
        runtime
            .tick()
            .map_err(|error| format!("pilot production tick {tick}: {error}"))?;
        if let Some(terminal) = scenario
            .held_food_setup
            .as_ref()
            .and_then(|setup| runtime.take_foundation_terminal_biology(setup.organism))
        {
            held_food_terminal_biology = Some(terminal);
        }
        let mut collected = runtime.take_foundation_training_steps();
        if collected.is_empty() && !teacher_mode {
            // A sleeping or terminal learner has no waking decision to replay.
            // Seal the contiguous segment already observed; failing the whole
            // evaluation would hide the behavioral outcome.
            break;
        }
        if collected.len() != 1 {
            return Err(format!(
                "pilot tick {tick}: expected one waking capture, got {}",
                collected.len()
            )
            .into());
        }
        if let Some(setup) = scenario.grab_food_setup.as_ref() {
            if let Some(acquisition) = crate::foundation_grab_food::observe_grab_acquisition(
                setup,
                grab_before,
                runtime.world(),
                &collected[0],
                steps.len(),
            )? {
                grab_food_acquisitions.push(acquisition);
            }
        }
        let speech_label = grounded_speech_label(
            &collected[0].frame,
            scenario.vocabulary_token,
            scenario.vocabulary_noun,
            scenario.vocabulary_target,
            steps.last().map(|s: &FoundationTrainingStep| &s.patch),
            scenario_lesson == Some(FoundationTeacherLesson::VocabularyProduction),
        );
        let behavior = &collected[0].behavior;
        let (available, selected) = diagnostics::vocalize_action_evidence(
            &behavior.forced_motor_slots,
            behavior.representative_index,
            &behavior.motor_indices,
            behavior.representative_mask | behavior.motor_masks[3],
        );
        let (valid, matched) = diagnostics::speech_output_evidence(
            &runtime.world().audible_utterances(),
            collected[0].frame.organism_id(),
            collected[0].frame.tick(),
            speech_label,
        );
        speech_behavior.observe(
            available,
            selected,
            speech_label.is_some_and(|l| l.token.is_some()),
            valid,
            matched,
        );
        if let Some(label) = speech_label {
            if label.token.is_some() {
                speech_opportunities += 1;
                if matched {
                    correct_utterances += 1;
                    if collected[0]
                        .frame
                        .tick()
                        .raw()
                        .saturating_sub(scenario.start_tick)
                        >= 16
                    {
                        unprompted_correct_utterances += 1;
                    }
                }
            }
        }
        let consumed_lesson = scenario_lesson.is_some_and(|lesson| {
            (lesson == FoundationTeacherLesson::EatHeldFood
                && held_food_step_consumed(&collected[0], food, held_food_before))
                || matches!(
                    lesson,
                    FoundationTeacherLesson::Feeding
                        | FoundationTeacherLesson::ObstacleNavigation
                        | FoundationTeacherLesson::VisionSearch
                        | FoundationTeacherLesson::MazeNavigation
                        | FoundationTeacherLesson::Recovery
                ) && teacher_step_consumed(&collected[0])
                || (teacher_mode
                    && matches!(scenario.vocabulary_token, Some(8 | 11))
                    && teacher_step_consumed(&collected[0])
                    && teacher_step_targets(&collected[0], food))
        });
        *teacher_previous.lock().unwrap() = Some(collected[0].patch.clone());
        let production_consumed_previous = teacher_mode
            && scenario_lesson == Some(FoundationTeacherLesson::VocabularyProduction)
            && steps.last().is_some_and(teacher_step_consumed);
        steps.append(&mut collected);
        let hazard_settled = teacher_mode
            && scenario_lesson == Some(FoundationTeacherLesson::HazardAvoidance)
            && steps.last().is_some_and(|step| {
                teacher_step_consumed(step) && teacher_step_targets(step, food)
            });
        if (consumed_lesson
            && scenario_lesson != Some(FoundationTeacherLesson::VocabularyProduction))
            || production_consumed_previous
            || hazard_settled
            || !grab_food_acquisitions.is_empty()
        {
            break;
        }
    }
    let collection_seconds = started.elapsed().as_secs_f64();
    let final_food_distance = if let Some(setup) = scenario.grab_food_setup.as_ref() {
        held_food_learner_distance(runtime.world(), food, setup.organism)
    } else if scenario_lesson == Some(FoundationTeacherLesson::EatHeldFood) {
        scenario
            .held_food_setup
            .as_ref()
            .and_then(|setup| held_food_learner_distance(runtime.world(), food, setup.organism))
    } else {
        scenario_lesson
            .is_some()
            .then(|| teacher_food_distance(&runtime))
            .transpose()?
    };
    let held_food_terminal_death_tick = scenario.held_food_setup.as_ref().and_then(|setup| {
        runtime
            .world()
            .organism_registry()
            .get(setup.organism)
            .and_then(|record| record.lifecycle().death_tick())
            .or_else(|| {
                runtime
                    .archive_retirement_receipt(setup.organism)
                    .map(|receipt| receipt.death_tick)
            })
            .map(Tick::raw)
    });
    let consumed_events = steps
        .iter()
        .filter(|step| teacher_step_consumed(step))
        .count() as u64;
    let food_consumed = steps
        .iter()
        .any(|step| teacher_step_consumed(step) && teacher_step_targets(step, food));
    let heard_word_frames = steps
        .iter()
        .filter(|s| {
            s.frame
                .sensory()
                .language_context
                .heard_tokens
                .iter()
                .flatten()
                .any(|h| Some(h.token_id) == scenario.vocabulary_token.map(u32::from))
        })
        .count();
    let teacher_cue_frames = steps
        .iter()
        .filter(|step| heard_foundation_teacher_cue(&step.frame, &scenario.teacher_cue_tokens))
        .count();
    let held_food_consumption_events = steps
        .iter()
        .filter(|step| {
            held_food_step_consumed(
                step,
                food,
                held_food_possession_frames.contains(&step.frame.tick().raw()),
            )
        })
        .count();
    let vocabulary_target_actions = steps
        .iter()
        .filter(|s| {
            let expected = match scenario.vocabulary_token {
                Some(1..=5 | 14..=18) => alife_core::CandidateActionFamily::Inspect,
                Some(6) => alife_core::CandidateActionFamily::Approach,
                Some(7) => alife_core::CandidateActionFamily::Avoid,
                Some(8 | 11) => alife_core::CandidateActionFamily::Ingest,
                Some(9 | 13) => alife_core::CandidateActionFamily::Contact,
                Some(10 | 12) => alife_core::CandidateActionFamily::Rest,
                _ => alife_core::CandidateActionFamily::Idle,
            };
            let matches = |c: &alife_core::ActionCandidate| {
                c.family == expected
                    && (scenario.vocabulary_token != Some(9)
                        || c.action_id == alife_world::HeadlessActionIds::GRAB)
                    && (scenario.vocabulary_token != Some(13)
                        || c.action_id == alife_world::HeadlessActionIds::PLAY)
                    && (matches!(scenario.vocabulary_token, Some(10 | 12))
                        || (c.target.entity == scenario.vocabulary_target
                            && c.target.entity.is_some()))
            };
            !teacher_step_blocked(s)
                && (s
                    .frame
                    .candidates()
                    .get(s.behavior.representative_index as usize)
                    .is_some_and(matches)
                    || s.behavior
                        .motor_indices
                        .iter()
                        .filter_map(|i| s.frame.candidates().get(*i as usize))
                        .any(matches))
        })
        .count();
    let request_responses = if request.is_some() {
        steps
            .iter()
            .filter(|step| !teacher_step_blocked(step))
            .flat_map(|step| {
                step.behavior
                    .motor_indices
                    .iter()
                    .chain(std::iter::once(&step.behavior.representative_index))
                    .filter_map(|index| step.frame.candidates().get(*index as usize))
                    .filter(|candidate| match scenario.vocabulary_token {
                        Some(9 | 13) => matches!(
                            candidate.action_id,
                            alife_world::HeadlessActionIds::GRAB
                                | alife_world::HeadlessActionIds::PLAY
                        ),
                        _ => candidate.family == alife_core::CandidateActionFamily::Inspect,
                    })
                    .map(|candidate| FoundationRequestResponse {
                        tick: step.frame.tick().raw(),
                        action: candidate.action_id.raw(),
                        target: candidate.target.entity,
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    } else {
        Vec::new()
    };
    let literal_word_matches = steps
        .iter()
        .filter(|s| {
            s.frame
                .sensory()
                .language_context
                .vocalized_token
                .is_some_and(|v| Some(v.token_id) == scenario.vocabulary_token.map(u32::from))
        })
        .count();
    let mut maze_nodes_reached = 0;
    for step in &steps {
        while let Some(target) = scenario.demonstration_route.get(maze_nodes_reached) {
            let position = step.frame.body().pose.translation;
            if (position.x - target.x).hypot(position.z - target.z) > 0.24 {
                break;
            }
            maze_nodes_reached += 1;
        }
    }
    let lesson_completed = scenario_lesson.map(|lesson| match lesson {
        FoundationTeacherLesson::GrabFood => !grab_food_acquisitions.is_empty(),
        FoundationTeacherLesson::EatHeldFood => {
            scenario.held_food_setup.is_some()
                && held_food_consumption_events > 0
                && runtime
                    .world()
                    .entity(food)
                    .is_some_and(|object| object.consumed)
        }
        FoundationTeacherLesson::Feeding => {
            food_consumed && steps.iter().all(|step| !teacher_step_blocked(step))
        }
        FoundationTeacherLesson::VisionSearch => food_consumed,
        FoundationTeacherLesson::MazeNavigation => {
            food_consumed
                && (!teacher_mode || maze_nodes_reached == scenario.demonstration_route.len())
                && steps.iter().any(|s| {
                    !s.frame
                        .candidates()
                        .iter()
                        .any(|c| c.target.entity == Some(food))
                })
                && steps.iter().all(|s| !teacher_step_blocked(s))
        }
        FoundationTeacherLesson::VocabularyReception
        | FoundationTeacherLesson::VocabularyProduction => {
            heard_word_frames > 0
                && if teacher_mode {
                    (vocabulary_target_actions > 0)
                        && (scenario.vocabulary_token != Some(9)
                            || scenario
                                .vocabulary_target
                                .and_then(|id| runtime.world().entity(id))
                                .is_some_and(|object| {
                                    object.carried_by == Some(steps[0].frame.organism_id())
                                }))
                        && steps.iter().all(|s| !teacher_step_blocked(s))
                } else if lesson == FoundationTeacherLesson::VocabularyProduction {
                    correct_utterances > 0 && unprompted_correct_utterances > 0
                } else {
                    vocabulary_target_actions > 0
                        && (scenario.vocabulary_token != Some(9)
                            || scenario
                                .vocabulary_target
                                .and_then(|id| runtime.world().entity(id))
                                .is_some_and(|object| {
                                    object.carried_by == Some(steps[0].frame.organism_id())
                                }))
                }
        }
        FoundationTeacherLesson::ObstacleNavigation => {
            if config.sensor_profile == SensorProfile::GroundedTerrainVisionV1 {
                let visible = |step: &FoundationTrainingStep| {
                    step.frame
                        .candidates()
                        .iter()
                        .any(|c| c.target.entity == Some(food))
                };
                food_consumed
                    && steps.iter().all(|step| !teacher_step_blocked(step))
                    && steps.first().is_some_and(visible)
                    && steps.iter().any(|step| !visible(step))
                    && steps.iter().any(|step| {
                        matches!(
                            step.patch.outcome().physical.contact,
                            alife_core::PhysicalContactKind::Moved
                        )
                    })
            } else {
                let food_start = steps
                    .iter()
                    .position(|step| teacher_step_targets(step, food));
                food_consumed
                    && steps.iter().all(|step| !teacher_step_blocked(step))
                    && food_start.is_some_and(|index| {
                        index > 0
                            && steps[..index]
                                .iter()
                                .all(|step| teacher_step_targets(step, waypoint))
                            && steps[index..]
                                .iter()
                                .all(|step| teacher_step_targets(step, food))
                    })
            }
        }
        FoundationTeacherLesson::HazardAvoidance => {
            let initial = initial_hazard_distance.unwrap_or(f32::INFINITY);
            let world = runtime.world();
            let subject = world.organism_entity_ids()[0].1;
            let from = world.entity(subject).map(|object| object.position);
            let to = world.entity(hazard).map(|object| object.position);
            from.zip(to).is_some_and(|(from, to)| {
                ((from.x - to.x).powi(2) + (from.z - to.z).powi(2)).sqrt() > initial + 0.5
            }) && food_consumed
                && steps.iter().any(|step| teacher_step_targets(step, hazard))
                && steps.iter().all(|step| !teacher_step_blocked(step))
                && steps.iter().all(|step| {
                    step.patch.outcome().physical.contact
                        != alife_core::PhysicalContactKind::Collision
                })
        }
        FoundationTeacherLesson::Recovery => {
            let initial = steps[0].frame.homeostasis().drives.fatigue;
            let final_fatigue = steps
                .last()
                .and_then(|step| step.patch.outcome().measured_physiology)
                .map(|transition| transition.after.homeostasis.drives.fatigue);
            let rest_end = steps.iter().position(|step| !teacher_step_rest(step));
            initial >= 0.12
                && final_fatigue.is_some_and(|final_fatigue| final_fatigue <= initial - 0.02)
                && rest_end.is_some_and(|index| {
                    index > 0
                        && steps[..index].iter().all(teacher_step_rest)
                        && steps[index..].iter().all(|step| {
                            if config.sensor_profile == SensorProfile::GroundedTerrainVisionV1 {
                                !teacher_step_rest(step)
                            } else {
                                teacher_step_targets(step, food)
                            }
                        })
                })
                && food_consumed
                && steps.iter().all(|step| !teacher_step_blocked(step))
        }
    });
    if scenario_lesson.is_some() {
        let trace = steps
            .iter()
            .map(|step| {
                let chosen = step
                    .frame
                    .candidates()
                    .get(step.behavior.representative_index as usize);
                let outcome = step.patch.outcome();
                serde_json::json!({
                    "tick": step.frame.tick().raw(),
                    "lesson_target_held_before_decision": scenario.held_food_setup.as_ref().map(|_| held_food_possession_frames.contains(&step.frame.tick().raw())),
                    "position": step.frame.body().pose.translation,
                    "gaze_rotation": step.frame.body().pose.rotation,
                    "terrain_ranges": step.frame.sensory().channels.visual_affordance,
                    "heard_tokens": step.frame.sensory().language_context.heard_tokens,
                    "vocalized_token": step.frame.sensory().language_context.vocalized_token,
                    "semantic_context": step.frame.sensory().semantic_context,
                    "chosen_action": chosen.map(|candidate| candidate.action_id.raw()),
                    "chosen_target": chosen.and_then(|candidate| candidate.target.entity),
                    "chosen_family": chosen.map(|candidate| candidate.family),
                    "object_slots": step.frame.grounded_object_slots(),
                    "representative_mask": step.behavior.representative_mask,
                    "motor_masks": step.behavior.motor_masks,
                    "motor_indices": step.behavior.motor_indices,
                    "candidates": step.frame.candidates().iter().enumerate().map(|(index, candidate)| serde_json::json!({
                        "index": candidate.candidate_index,
                        "action": candidate.action_id.raw(),
                        "family": candidate.family,
                        "target": candidate.target.entity,
                        "logit": step.behavior.logits[index],
                    })).collect::<Vec<_>>(),
                    "physical": outcome.physical,
                    "channel_outcomes": outcome.joint.as_ref().map(|joint| &joint.channel_outcomes),
                    "experienced_valence": outcome.experienced_valence(),
                    "physiology": outcome.measured_physiology.map(|transition| serde_json::json!({
                        "before_energy": transition.before.body.energy,
                        "after_energy": transition.after.body.energy,
                        "before_hunger": transition.before.homeostasis.drives.hunger,
                        "after_hunger": transition.after.homeostasis.drives.hunger,
                        "before_health": transition.before.body.health,
                        "after_health": transition.after.body.health,
                        "before_injury": transition.before.body.injury,
                        "after_injury": transition.after.body.injury,
                        "before_pain": transition.before.homeostasis.drives.pain,
                        "after_pain": transition.after.homeostasis.drives.pain,
                        "before_brain_atp": transition.before.homeostasis.drives.brain_atp,
                        "after_brain_atp": transition.after.homeostasis.drives.brain_atp,
                        "before_fatigue": transition.before.homeostasis.drives.fatigue,
                        "after_fatigue": transition.after.homeostasis.drives.fatigue,
                    })),
                })
            })
            .collect::<Vec<_>>();
        std::fs::write(
            output.join(if lesson_completed == Some(false) {
                "lesson-diagnostic.json"
            } else {
                "lesson-trace.json"
            }),
            serde_json::to_vec_pretty(&serde_json::json!({
                "lesson": scenario_lesson,
                "consumed_events": consumed_events,
                "initial_food_distance": initial_food_distance,
                "final_food_distance": final_food_distance,
                "steps": trace,
                "maze_demonstration_route": scenario.demonstration_route,
                "maze_nodes_reached": maze_nodes_reached,
                "vocabulary_token": scenario.vocabulary_token,
                "teacher_cue_tokens": scenario.teacher_cue_tokens,
                "teacher_cue_frames": teacher_cue_frames,
                "teacher_cue_kind": "contextual_request_not_completed_action_narration",
                "grab_food_setup": scenario.grab_food_setup,
                "grab_food_acquisitions": grab_food_acquisitions,
                "held_food_setup": scenario.held_food_setup,
                "held_food_consumption_events": held_food_consumption_events,
                "held_food_possession_frames": held_food_possession_frames,
                "held_food_terminal_death_tick": held_food_terminal_death_tick,
                "held_food_terminal_biology": held_food_terminal_biology,
                "semantic_prior": runtime.semantic_prior_metrics(),
                "vocabulary_target": scenario.vocabulary_target,
                "language_praises": scenario.language_praise_count.get(),
                "heard_language_encoder_lanes": steps[0].before.phenotype.sensor_encoder().assignments().iter().filter(|a|a.source_group()==alife_core::SensorEncoderSourceGroup::HeardLanguage).count(),
                "semantic_prior_encoder_lanes": steps[0].before.phenotype.sensor_encoder().assignments().iter().filter(|a|a.source_group()==alife_core::SensorEncoderSourceGroup::SemanticPrior).count(),
            }))?,
        )?;
    }
    validate_foundation_lesson_prior(&runtime, output, scenario_lesson)?;
    if lesson_completed == Some(false) && teacher_mode {
        return Err("teacher lesson did not complete its measured world outcome".into());
    }
    let phenotype = steps[0].before.phenotype.as_ref().clone();
    let ids: Vec<u32> = (0..phenotype.synapses().len() as u32).collect();
    let mask = StageTrainableMask::from_synapse_indices(&phenotype, &ids)?;
    // Reuse the same physical adapter/device context, with a Training consumer.
    let trainer_backend = runtime.new_staging_like_live()?;
    let mut trainer = FoundationTrainer::from_session(
        alife_runtime::GpuAuthoritativeSession::new(
            trainer_backend,
            alife_runtime::GpuSessionConsumerKind::Training,
        ),
        phenotype,
        asset.clone(),
        mask,
        AdamWConfig::default(),
    )
    .map_err(|error| format!("pilot trainer admission: {error}"))?;
    if teacher_mode {
        let checkpoint = serde_json::to_vec(&trainer.checkpoint()?)?;
        let source = foundation_replay_source(&steps[0].before.phenotype, &asset, 0, &checkpoint)?;
        std::fs::write(
            output.join("replay-source.json"),
            serde_json::to_vec_pretty(&source)?,
        )?;
        let mut writer = FoundationReplayWriter::new(
            &output.join("replay"),
            source.clone(),
            steps[0].before.phenotype.as_ref().clone(),
            &asset,
            FoundationReplayBudget::default(),
        )?;
        let references = steps
            .iter()
            .map(|step| writer.append(step))
            .collect::<Result<Vec<_>>>()?;
        std::fs::write(
            output.join("replay-manifest.json"),
            serde_json::to_vec(&references)?,
        )?;
        std::fs::write(output.join("actor-checkpoint.json"), checkpoint)?;
        let labels = DemonstrationSpeechLabels {
            source,
            record_digests: references.iter().map(|r| r.digest).collect(),
            targets: steps
                .iter()
                .enumerate()
                .map(|(index, s)| {
                    grounded_speech_label(
                        &s.frame,
                        scenario.vocabulary_token,
                        scenario.vocabulary_noun,
                        scenario.vocabulary_target,
                        index.checked_sub(1).map(|i| &steps[i].patch),
                        scenario_lesson == Some(FoundationTeacherLesson::VocabularyProduction),
                    )
                })
                .collect(),
        };
        std::fs::write(
            output.join("speech-labels.json"),
            serde_json::to_vec(&labels)?,
        )?;
    }
    let started = Instant::now();
    let mut replay = alife_training::TrainingReplayEvaluation {
        candidate_logits: Vec::new(),
        final_activations: Vec::new(),
        final_activity_ema: Vec::new(),
        final_metabolic_load: Vec::new(),
    };
    for chunk in steps.chunks(256) {
        let sequence = foundation_replay_sequence(chunk, 0)
            .map_err(|e| format!("pilot replay conversion: {e}"))?;
        let mut window = trainer
            .evaluate_replay(&sequence)
            .map_err(|e| format!("pilot trainer replay: {e}"))?;
        replay.candidate_logits.append(&mut window.candidate_logits);
        replay
            .final_activations
            .append(&mut window.final_activations);
        replay
            .final_activity_ema
            .append(&mut window.final_activity_ema);
        replay
            .final_metabolic_load
            .append(&mut window.final_metabolic_load);
    }
    let replay_seconds = started.elapsed().as_secs_f64();
    let mut maximum_logit_error = 0.0_f32;
    let mut maximum_activation_error = 0.0_f32;
    let mut maximum_activity_ema_error = 0.0_f32;
    let mut maximum_metabolic_error = 0.0_f32;
    if [
        replay.candidate_logits.len(),
        replay.final_activations.len(),
        replay.final_activity_ema.len(),
        replay.final_metabolic_load.len(),
    ]
    .into_iter()
    .any(|len| len != steps.len())
    {
        return Err(invalid().into());
    }
    for (i, step) in steps.iter().enumerate() {
        maximum_activation_error = maximum_activation_error.max(compare_replay_values(
            i,
            "activation",
            &replay.final_activations[i],
            &snapshot_activation(&step.after_inference)?,
        )?);
        let observed = &step.behavior.logits;
        if replay.candidate_logits[i].len() != observed.len() {
            return Err(invalid().into());
        }
        let support = step
            .behavior
            .motor_masks
            .iter()
            .fold(step.behavior.representative_mask, |mask, channel| {
                mask | channel
            });
        for (candidate, (actual, expected)) in
            replay.candidate_logits[i].iter().zip(observed).enumerate()
        {
            if !expected.is_finite() {
                if support & (1u32 << candidate) != 0 {
                    return Err("nonfinite production logit is in sampled policy support".into());
                }
                continue;
            } // Masked candidates are not policy support.
            let error = (actual - expected).abs();
            maximum_logit_error = maximum_logit_error.max(error);
            if !actual.is_finite() || error > 1e-4 + 1e-4 * expected.abs() {
                let activation_error = replay.final_activations[i]
                    .iter()
                    .zip(snapshot_activation(&step.after_inference)?)
                    .map(|(a, b)| (a - b).abs())
                    .fold(0.0_f32, f32::max);
                return Err(format!("tick {i} candidate {candidate} {:?}: logit mismatch {actual} vs {expected}; max activation error={activation_error}, bank={}, generation={}",
                    step.frame.candidates()[candidate].family, step.before.active_weight_bank, step.before.active_weight_generation).into());
            }
        }
        let homeostasis = snapshot_values(
            &step.after_inference,
            step.after_inference
                .brain_slot
                .word_ranges()
                .homeostasis_words
                .clone(),
        )?;
        let ema: Vec<f32> = homeostasis.chunks_exact(2).map(|row| row[0]).collect();
        let metabolic: Vec<f32> = homeostasis.chunks_exact(2).map(|row| row[1]).collect();
        maximum_activity_ema_error = maximum_activity_ema_error.max(compare_replay_values(
            i,
            "activity EMA",
            &replay.final_activity_ema[i],
            &ema,
        )?);
        maximum_metabolic_error = maximum_metabolic_error.max(compare_replay_values(
            i,
            "metabolic load",
            &replay.final_metabolic_load[i],
            &metabolic,
        )?);
    }
    let receipt = FoundationPilotReceipt {
        seed,
        founder_seed_base,
        ticks: steps.len(),
        collection_seconds,
        replay_seconds,
        maximum_logit_error,
        maximum_activation_error,
        maximum_activity_ema_error,
        maximum_metabolic_error,
        source_asset_digest: asset
            .digest()
            .bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        teacher_mode,
        consumed_events,
        demonstration_replay_records: if teacher_mode { steps.len() } else { 0 },
        food_position,
        initial_food_distance,
        final_food_distance,
        lesson: scenario_lesson,
        lesson_completed,
        vocabulary_token: scenario.vocabulary_token,
        initial_world_signature,
        silent_request: scenario.silent_request,
        request_target: scenario.vocabulary_target,
        request_responses,
        semantic_prior: runtime.semantic_prior_metrics().cloned(),
        heard_word_frames,
        teacher_cue_frames,
        teacher_cue_tokens: scenario.teacher_cue_tokens.clone(),
        held_food_setup: scenario.held_food_setup.clone(),
        grab_food_setup: scenario.grab_food_setup.clone(),
        grab_food_acquisitions,
        held_food_consumption_events,
        held_food_possession_frames,
        held_food_terminal_death_tick,
        held_food_terminal_biology,
        vocabulary_target_actions,
        literal_word_matches,
        correct_utterances,
        unprompted_correct_utterances,
        speech_opportunities,
        speech_behavior: Some(speech_behavior),
        maze_nodes_reached,
        maze_route_nodes: scenario.demonstration_route.len(),
    };
    std::fs::write(
        output.join("pilot.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    Ok(receipt)
}

fn teacher_step_consumed(step: &crate::FoundationTrainingStep) -> bool {
    let outcome = step.patch.outcome();
    outcome.physical.contact == alife_core::PhysicalContactKind::Consumed
        || outcome.joint.as_ref().is_some_and(|joint| {
            joint.channel_outcomes.iter().any(|channel| {
                channel.physical.contact == alife_core::PhysicalContactKind::Consumed
            })
        })
}

pub(crate) fn held_food_step_consumed(
    step: &crate::FoundationTrainingStep,
    target: alife_core::WorldEntityId,
    held_before: bool,
) -> bool {
    held_food_action_consumed(
        &step.patch.decision().selected_action,
        step.patch.decision().selected_bundle.as_ref(),
        step.patch.outcome(),
        target,
        held_before,
    )
}

fn held_food_action_consumed(
    action: &alife_core::ActionCommand,
    bundle: Option<&alife_core::MotorCommandBundle>,
    outcome: &alife_core::PostActionOutcome,
    target: alife_core::WorldEntityId,
    held_before: bool,
) -> bool {
    if !held_before {
        return false;
    }
    let consumed = |physical: &alife_core::PhysicalActionOutcome| {
        physical.contact == alife_core::PhysicalContactKind::Consumed
            && physical.target_entity == Some(target)
    };
    let selected_and_consumed = if let Some(bundle) = bundle {
        bundle.channels.iter().any(|command| {
            command.primitive == alife_world::HeadlessActionIds::EAT
                && command.target.and_then(|t| t.entity) == Some(target)
                && outcome.joint.as_ref().is_some_and(|joint| {
                    joint.channel_outcomes.iter().any(|channel| {
                        channel.channel == command.channel && consumed(&channel.physical)
                    })
                })
        })
    } else {
        action.action_id == alife_world::HeadlessActionIds::EAT
            && action.target_entity == Some(target)
            && consumed(&outcome.physical)
    };
    selected_and_consumed
        && outcome
            .measured_physiology
            .is_some_and(held_food_body_benefit)
}

fn held_food_body_benefit(transition: alife_core::MeasuredPhysiologyTransition) -> bool {
    // A physical consumption receipt alone does not establish nourishment.
    // Use ordinary organism-owned energy/hunger change and reject fresh harm.
    (transition.after.body.energy > transition.before.body.energy
        || transition.after.homeostasis.drives.hunger < transition.before.homeostasis.drives.hunger)
        && transition.after.body.health >= transition.before.body.health
        && transition.after.body.injury <= transition.before.body.injury
        && transition.after.homeostasis.drives.pain <= transition.before.homeostasis.drives.pain
}

/// Any-token exposure used by `teacher_cue_frames`, including noun-only speech.
/// Full command delivery requires ordered tokens from the same Teacher utterance.
pub(crate) fn heard_foundation_teacher_cue(
    frame: &alife_core::PerceptionFrame,
    tokens: &[u16],
) -> bool {
    frame
        .sensory()
        .language_context
        .heard_tokens
        .iter()
        .flatten()
        .any(|heard| {
            heard.source_kind == alife_core::UtteranceSourceKind::Teacher
                && tokens
                    .iter()
                    .any(|token| u32::from(*token) == heard.token_id)
        })
}

fn teacher_step_rest(step: &crate::FoundationTrainingStep) -> bool {
    step.frame
        .candidates()
        .get(step.behavior.representative_index as usize)
        .is_some_and(|candidate| candidate.family == alife_core::CandidateActionFamily::Rest)
}

fn teacher_step_targets(
    step: &crate::FoundationTrainingStep,
    target: alife_core::WorldEntityId,
) -> bool {
    step.frame
        .candidates()
        .get(step.behavior.representative_index as usize)
        .is_some_and(|candidate| candidate.target.entity == Some(target))
}

fn teacher_step_blocked(step: &crate::FoundationTrainingStep) -> bool {
    step.patch.outcome().physical.contact == alife_core::PhysicalContactKind::Blocked
        || step.patch.outcome().joint.as_ref().is_some_and(|joint| {
            joint
                .channel_outcomes
                .iter()
                .any(|channel| channel.physical.contact == alife_core::PhysicalContactKind::Blocked)
        })
}

fn move_scenario_object(
    world: &mut alife_world::HeadlessWorld,
    entity: alife_core::WorldEntityId,
    position: alife_core::Vec3f,
) -> Result<()> {
    let mut physical = world
        .entity(entity)
        .ok_or("scenario object is missing")?
        .grounded_physical;
    world.editor_move_object(entity, position)?;
    // A scenario placement is static scenery, not an observed moving object.
    physical.velocity = alife_core::Vec3f::ZERO;
    world.set_grounded_physical_properties(entity, physical)?;
    Ok(())
}

#[derive(Debug, Clone)]
pub(crate) struct FoundationScenarioSetup {
    pub(crate) food: alife_core::WorldEntityId,
    pub(crate) hazard: alife_core::WorldEntityId,
    pub(crate) waypoint: alife_core::WorldEntityId,
    pub(crate) initial_hazard_distance: Option<f32>,
    pub(crate) closing_gate: Option<(alife_core::WorldEntityId, alife_core::Vec3f)>,
    pub(crate) maze_walls: Vec<(alife_core::WorldEntityId, alife_core::Vec3f)>,
    pub(crate) demonstration_route: Vec<alife_core::Vec3f>,
    pub(crate) vocabulary_token: Option<u16>,
    pub(crate) teacher_cue_tokens: Vec<u16>,
    pub(crate) held_food_setup: Option<FoundationHeldFoodSetup>,
    pub(crate) grab_food_setup: Option<crate::FoundationGrabFoodSetup>,
    pub(crate) vocabulary_target: Option<alife_core::WorldEntityId>,
    pub(crate) vocabulary_noun: Option<u16>,
    pub(crate) repeat_vocabulary: bool,
    pub(crate) silent_request: bool,
    pub(crate) fixed_speaker_position: Option<alife_core::Vec3f>,
    pub(crate) training_language_feedback: bool,
    pub(crate) start_tick: u64,
    last_language_praise_tick: std::cell::Cell<Option<u64>>,
    language_praise_count: std::cell::Cell<u8>,
}

pub(crate) fn close_foundation_navigation_gate(
    runtime: &mut GpuLiveBrainRuntime,
    scenario: &FoundationScenarioSetup,
) -> Result<()> {
    expose_foundation_teacher_cue(runtime.world_mut(), scenario)?;
    if runtime.world().tick().raw() >= 2 {
        for (wall, position) in &scenario.maze_walls {
            if runtime
                .world()
                .entity(*wall)
                .is_some_and(|object| object.position != *position)
            {
                move_scenario_object(runtime.world_mut(), *wall, *position)?;
            }
        }
    }
    if let Some(token) = scenario.vocabulary_token {
        if scenario.training_language_feedback
            && scenario.language_praise_count.get() < 2
            && scenario
                .last_language_praise_tick
                .get()
                .is_none_or(|at| runtime.world().tick().raw().saturating_sub(at) >= 24)
        {
            let (subject, _) = runtime.world().organism_entity_ids()[0];
            let patches = runtime.sealed_patches();
            let expected = patches.last().and_then(|patch| {
                grounded_speech_label(
                    patch.pre_action().perception(),
                    Some(token),
                    scenario.vocabulary_noun,
                    scenario.vocabulary_target,
                    patches.iter().rev().nth(1),
                    true,
                )
            });
            let spoken = expected
                .and_then(|label| {
                    label.token.map(|first| {
                        let mut words = vec![first];
                        words.extend(label.continuation.into_iter().take_while(|v| *v != 0));
                        words
                    })
                })
                .is_some_and(|words| {
                    runtime
                        .world()
                        .audible_utterances()
                        .iter()
                        .any(|utterance| {
                            utterance.speaker_id == Some(subject)
                                && utterance.source_kind
                                    == alife_core::UtteranceSourceKind::Creature
                                && utterance.emitted_tick.raw() + 1 >= runtime.world().tick().raw()
                                && utterance
                                    .tokens
                                    .iter()
                                    .map(|word| word.raw())
                                    .eq(words.iter().copied())
                        })
                });
            let acted = runtime.sealed_patches().last().is_some_and(|patch| {
                let expected = match token {
                    1..=5 | 14..=18 => alife_core::CandidateActionFamily::Inspect,
                    6 => alife_core::CandidateActionFamily::Approach,
                    7 => alife_core::CandidateActionFamily::Avoid,
                    8 | 11 => alife_core::CandidateActionFamily::Ingest,
                    9 | 13 => alife_core::CandidateActionFamily::Contact,
                    _ => alife_core::CandidateActionFamily::Rest,
                };
                patch.decision().neural_evidence().ok().is_some_and(|e| {
                    e.action_family == expected
                        && patch
                            .pre_action()
                            .perception()
                            .candidates()
                            .get(e.candidate_index as usize)
                            .is_some_and(|c| {
                                (token != 9 || c.action_id == alife_world::HeadlessActionIds::GRAB)
                                    && (token != 13
                                        || c.action_id == alife_world::HeadlessActionIds::PLAY)
                                    && (matches!(token, 10 | 12)
                                        || c.target.entity == scenario.vocabulary_target)
                            })
                }) && (token != 9
                    || scenario
                        .vocabulary_target
                        .and_then(|id| runtime.world().entity(id))
                        .is_some_and(|object| object.carried_by == Some(subject)))
                    && !matches!(
                        patch.outcome().physical.contact,
                        alife_core::PhysicalContactKind::Blocked
                            | alife_core::PhysicalContactKind::Collision
                    )
            });
            if spoken || acted {
                if let Some(source) = runtime
                    .world()
                    .entity_id("nursery-teacher")
                    .and_then(|id| runtime.world().entity(id))
                    .map(|o| o.position)
                {
                    // An ordinary spatial Hand praise stimulus: world reach and
                    // occlusion can reject it. No synthetic reward is written.
                    if runtime.provide_player_care(subject, source, true).is_ok() {
                        scenario
                            .last_language_praise_tick
                            .set(Some(runtime.world().tick().raw()));
                        scenario
                            .language_praise_count
                            .set(scenario.language_praise_count.get() + 1);
                    }
                }
            }
        }
        if !scenario.silent_request
            && (runtime.world().tick().raw() == scenario.start_tick
                || (scenario.repeat_vocabulary
                    && runtime
                        .world()
                        .tick()
                        .raw()
                        .saturating_sub(scenario.start_tick)
                        % 24
                        == 0
                    && (!matches!(token, 8 | 11)
                        || scenario
                            .vocabulary_target
                            .and_then(|id| runtime.world().entity(id))
                            .is_some_and(|object| !object.consumed))))
        {
            let (subject, entity) = runtime.world().organism_entity_ids()[0];
            let position = runtime
                .world()
                .entity(entity)
                .ok_or("vocabulary subject missing")?
                .position;
            let target_position = scenario
                .vocabulary_target
                .and_then(|id| runtime.world().entity(id))
                .map_or(position, |o| o.position);
            let mut tokens = vec![alife_core::LanguageTokenId::new(token)?];
            if (5..=9).contains(&token) || token == 13 {
                if let Some(noun) = scenario.vocabulary_noun {
                    tokens.push(alife_core::LanguageTokenId::new(noun)?);
                }
            }
            let dx = target_position.x - position.x;
            let dz = target_position.z - position.z;
            let norm = dx.hypot(dz).max(0.1);
            // Stand beside the object, not on the learner's sightline to it.
            let teacher_position =
                scenario
                    .fixed_speaker_position
                    .unwrap_or(alife_core::Vec3f::new(
                        target_position.x - 2.0 * dz / norm,
                        target_position.y,
                        target_position.z + 2.0 * dx / norm,
                    ));
            alife_school::LanguageNursery::speak_in_world(
                runtime.world_mut(),
                subject,
                teacher_position,
                tokens,
            )?;
        }
    }
    if runtime.world().tick().raw() >= 2 {
        if let Some((gate, position)) = scenario.closing_gate {
            if runtime
                .world()
                .entity(gate)
                .is_some_and(|object| object.position != position)
            {
                move_scenario_object(runtime.world_mut(), gate, position)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn expose_foundation_teacher_cue(
    world: &mut alife_world::HeadlessWorld,
    scenario: &FoundationScenarioSetup,
) -> Result<()> {
    if scenario.teacher_cue_tokens.is_empty()
        || !world
            .tick()
            .raw()
            .saturating_sub(scenario.start_tick)
            .is_multiple_of(24)
        || world.entity(scenario.food).is_none_or(|food| food.consumed)
    {
        return Ok(());
    }
    let (subject, entity) = world.organism_entity_ids()[0];
    world.entity(entity).ok_or("teacher cue subject missing")?;
    let food = world
        .entity(scenario.food)
        .ok_or("teacher cue food missing")?;
    let position = food.position;
    let tokens: &[u16] = if scenario.held_food_setup.is_some() && food.carried_by != Some(subject) {
        &[1] // Name the released object; do not request the held-food action.
    } else if scenario.grab_food_setup.is_some() && food.carried_by == Some(subject) {
        &[1] // Name the acquired object; do not repeat the satisfied Get request.
    } else {
        &scenario.teacher_cue_tokens
    };
    // These are contextual naming/request words for the real food object,
    // not a declaration that the learner has already completed an action.
    alife_school::LanguageNursery::speak_in_world(
        world,
        subject,
        alife_core::Vec3f::new(position.x, position.y, position.z + 2.0),
        tokens
            .iter()
            .copied()
            .map(alife_core::LanguageTokenId::new)
            .collect::<std::result::Result<Vec<_>, _>>()?,
    )?;
    Ok(())
}

fn scenario_random(seed: u64, stream: u64) -> f32 {
    // SplitMix64 keeps adjacent cohort seeds from producing adjacent layouts.
    let mut value = seed.wrapping_add(stream.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (((value ^ (value >> 31)) >> 40) as u32 as f32) / 16_777_216.0
}

const HELD_FOOD_POSE_ATTEMPTS: u32 = 32;
const HELD_FOOD_POSITION_RADIUS: f32 = 4.0;

fn sample_foundation_held_food_pose(
    world: &alife_world::HeadlessWorld,
    subject: alife_core::WorldEntityId,
    food: alife_core::WorldEntityId,
    seed: u64,
) -> Result<(alife_core::Vec3f, f32, u32)> {
    let body = world.entity(subject).ok_or("held-food body missing")?;
    let food_radius = world.entity(food).ok_or("held-food target missing")?.radius;
    let terrain = world.terrain().ok_or("held-food terrain missing")?;
    let objects = world.object_snapshots();
    for attempt in 1..=HELD_FOOD_POSE_ATTEMPTS {
        let stream = 1000 + u64::from(attempt) * 3;
        let x = body.position.x
            + HELD_FOOD_POSITION_RADIUS * (2.0 * scenario_random(seed, stream) - 1.0);
        let z = body.position.z
            + HELD_FOOD_POSITION_RADIUS * (2.0 * scenario_random(seed, stream + 1) - 1.0);
        let yaw = std::f32::consts::TAU * scenario_random(seed, stream + 2) - std::f32::consts::PI;
        let Some(y) = terrain.surface().height(x, z) else {
            continue;
        };
        let position = alife_core::Vec3f::new(x, y, z);
        let grip = alife_core::Vec3f::new(x + 0.5 * yaw.cos(), y, z + 0.5 * yaw.sin());
        if !terrain.walkable(x, z) || terrain.resolve_move(position, grip).is_none() {
            continue;
        }
        let clear = objects
            .iter()
            .filter(|object| object.id != subject && object.id != food && !object.consumed)
            .all(|object| {
                (object.position.x - x).hypot(object.position.z - z)
                    > body.radius + object.radius + 0.1
                    && (object.position.x - grip.x).hypot(object.position.z - grip.z)
                        > food_radius + object.radius + 0.1
            });
        if clear {
            return Ok((position, yaw, attempt));
        }
    }
    Err(format!("held-food pose sampling exhausted {HELD_FOOD_POSE_ATTEMPTS} blocked or invalid ground attempts for seed {seed}").into())
}

fn establish_foundation_held_food(
    world: &mut alife_world::HeadlessWorld,
    target: alife_core::WorldEntityId,
    seed: u64,
) -> Result<FoundationHeldFoodSetup> {
    establish_foundation_food(world, target, seed, true)?
        .0
        .ok_or_else(|| "held-food setup did not produce its receipt".into())
}

fn establish_foundation_grab_food(
    world: &mut alife_world::HeadlessWorld,
    target: alife_core::WorldEntityId,
    seed: u64,
) -> Result<crate::FoundationGrabFoodSetup> {
    establish_foundation_food(world, target, seed, false)?
        .1
        .ok_or_else(|| "Grab-only setup did not produce its receipt".into())
}

fn establish_foundation_food(
    world: &mut alife_world::HeadlessWorld,
    target: alife_core::WorldEntityId,
    seed: u64,
    setup_grab: bool,
) -> Result<(
    Option<FoundationHeldFoodSetup>,
    Option<crate::FoundationGrabFoodSetup>,
)> {
    let (organism, entity) = world.organism_entity_ids()[0];
    // Starting biology supplies the need. If expression needs time, age the
    // ordinary organism instead of assigning reserves or injecting hunger.
    for _ in 0..2400 {
        let state = world
            .organism_registry()
            .get(organism)
            .ok_or("held-food organism missing")?
            .biochemistry();
        if state.homeostasis.drives.hunger >= 0.12 && state.body.energy <= 0.9 {
            break;
        }
        world.try_advance_tick()?;
    }
    let state = world
        .organism_registry()
        .get(organism)
        .ok_or("held-food organism missing")?
        .biochemistry();
    if state.body.sleeping
        || state.body.health < 0.95
        || state.body.injury > 0.01
        || state.homeostasis.drives.pain > 0.01
        || state.body.energy <= 0.2
        || state.body.energy > 0.9
        || state.homeostasis.drives.hunger < 0.12
    {
        return Err("held-food lesson requires a healthy awake organism with ordinary measurable hunger and reserve headroom".into());
    }
    let food = world.entity(target).ok_or("held-food target missing")?;
    if food.kind != alife_world::WorldObjectKind::Food
        || food.carried_by.is_some()
        || food.consumed
        || food.nutrition <= 0.0
        || food.hazard_pain != 0.0
    {
        return Err("held-food lesson requires safe unconsumed nutritious food".into());
    }
    let (position, requested_yaw, sampling_attempts) =
        sample_foundation_held_food_pose(world, entity, target, seed)?;
    // Arrangement remains outside learner capture. Facing changes execute
    // ordinary registered body turns and their ordinary physiological cost.
    let mut arranged = world.clone();
    move_scenario_object(&mut arranged, entity, position)?;
    for _ in 0..10 {
        let yaw = arranged
            .entity(entity)
            .ok_or("held-food body missing")?
            .body_yaw;
        let delta = (requested_yaw - yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        if delta.abs() <= 0.0001 {
            break;
        }
        let turn = alife_core::ActionCommand::structured(
            organism,
            if delta >= 0.0 {
                alife_world::HeadlessActionIds::TURN_LEFT
            } else {
                alife_world::HeadlessActionIds::TURN_RIGHT
            },
            alife_core::ActionKind::Look,
            alife_core::ActionTarget::NONE,
            alife_core::Intensity::new((delta.abs() / 20.0_f32.to_radians()).min(1.0))?,
            alife_core::DurationTicks::new(1),
            alife_core::Confidence::new(0.9)?,
            0,
            None,
            None,
            None,
        )?;
        let next = Tick::new(arranged.tick().raw().checked_add(1).ok_or_else(invalid)?);
        let receipt = arranged.apply_registered_command(&turn, entity, next)?;
        arranged.try_advance_tick()?;
        if !receipt.action_result.execution.succeeded {
            return Err("held-food sampled body turn was physically blocked".into());
        }
    }
    let realized_yaw = arranged
        .entity(entity)
        .ok_or("held-food body missing")?
        .body_yaw;
    if (realized_yaw - requested_yaw).abs() > 0.0002 {
        return Err("held-food bounded ordinary turns did not realize sampled facing".into());
    }
    move_scenario_object(
        &mut arranged,
        target,
        alife_core::Vec3f::new(
            position.x + 0.5 * realized_yaw.cos(),
            position.y,
            position.z + 0.5 * realized_yaw.sin(),
        ),
    )?;
    let command = alife_core::ActionCommand::structured(
        organism,
        alife_world::HeadlessActionIds::GRAB,
        alife_core::ActionKind::Hold,
        alife_core::ActionTarget::new(Some(target), None),
        alife_core::Intensity::new(1.0)?,
        alife_core::DurationTicks::new(1),
        alife_core::Confidence::new(0.9)?,
        0,
        None,
        None,
        None,
    )?;
    if !setup_grab {
        let record = arranged
            .organism_registry()
            .get(organism)
            .ok_or("Grab-only learner missing after arrangement")?;
        let state = record.biochemistry();
        if state.body.sleeping
            || state.body.health < 0.95
            || state.body.injury > 0.01
            || state.homeostasis.drives.pain > 0.01
            || state.body.energy <= 0.2
            || state.body.energy > 0.9
            || state.homeostasis.drives.hunger < 0.12
        {
            return Err(
                "Grab-only ordinary arrangement did not preserve healthy measurable need".into(),
            );
        }
        let body = arranged.entity(entity).ok_or("Grab-only body missing")?;
        let food = arranged.entity(target).ok_or("Grab-only food missing")?;
        if food.carried_by.is_some() || food.consumed {
            return Err("Grab-only setup requires unowned unconsumed food".into());
        }
        let setup = crate::FoundationGrabFoodSetup {
            target,
            organism,
            setup_tick: arranged.tick().raw(),
            initial_energy: state.body.energy,
            initial_hunger: state.homeostasis.drives.hunger,
            initial_health: state.body.health,
            initial_sleeping: state.body.sleeping,
            initial_owner: food.carried_by,
            initial_consumed: food.consumed,
            sampling_seed: seed,
            sampling_attempts,
            realized_body_position: body.position,
            realized_body_yaw: body.body_yaw,
            food_position: food.position,
        };
        *world = arranged;
        return Ok((None, Some(setup)));
    }
    let next = Tick::new(arranged.tick().raw().checked_add(1).ok_or_else(invalid)?);
    let receipt = arranged.apply_registered_command(&command, entity, next)?;
    arranged.try_advance_tick()?;
    if !receipt.action_result.execution.succeeded
        || receipt.action_result.execution.physical.contact
            != alife_core::PhysicalContactKind::Touch
        || arranged
            .entity(target)
            .is_none_or(|food| food.carried_by != Some(organism))
    {
        return Err("held-food legal setup Grab did not establish possession".into());
    }
    let state = arranged
        .organism_registry()
        .get(organism)
        .ok_or("held-food organism missing after setup")?
        .biochemistry();
    if state.body.sleeping
        || state.body.health < 0.95
        || state.body.injury > 0.01
        || state.homeostasis.drives.pain > 0.01
        || state.body.energy <= 0.2
        || state.body.energy > 0.9
        || state.homeostasis.drives.hunger < 0.12
    {
        return Err("held-food ordinary pose setup did not preserve a healthy awake learner with measurable need".into());
    }
    let body = arranged
        .entity(entity)
        .ok_or("held-food body missing after setup")?;
    let grip = arranged
        .entity(target)
        .ok_or("held-food target missing after setup")?
        .position;
    let setup = FoundationHeldFoodSetup {
        target,
        organism,
        setup_tick: arranged.tick().raw(),
        grab_command: command,
        grab_physical: receipt.action_result.execution.physical,
        initial_energy: state.body.energy,
        initial_hunger: state.homeostasis.drives.hunger,
        initial_health: state.body.health,
        initial_sleeping: state.body.sleeping,
        sampling_seed: seed,
        sampling_attempts,
        realized_body_position: Some(body.position),
        realized_body_yaw: Some(body.body_yaw),
        realized_grip_position: Some(grip),
    };
    *world = arranged;
    Ok((Some(setup), None))
}

/// Canonical fixtures start in legacy X/Y coordinates. Production terrain
/// admission already converts them to X/Z and grounds movement. Offline lives
/// must use that same admission path instead of learning to walk into the air.
pub(crate) fn ground_foundation_training_world(
    world: &mut alife_world::HeadlessWorld,
) -> Result<()> {
    if world.terrain().is_none() {
        let terrain = alife_world::WorldTerrain::new(
            alife_world::TerrainData {
                width: 2,
                depth: 2,
                origin_x: -256.0,
                origin_z: -256.0,
                spacing: 512.0,
                heights: vec![0.0; 4],
                obstacles: Vec::new(),
                water_level: None,
            },
            alife_world::LocomotionLimits::default(),
        )?;
        world.enable_terrain_for_new_game(terrain, alife_core::Vec3f::ZERO)?;
    }
    Ok(())
}

pub(crate) fn configure_foundation_scenario(
    world: &mut alife_world::HeadlessWorld,
    seed: u64,
    lesson: Option<FoundationTeacherLesson>,
    food_position: Option<[f32; 2]>,
    precondition_recovery: bool,
) -> Result<FoundationScenarioSetup> {
    ground_foundation_training_world(world)?;
    if food_position.is_some() && lesson != Some(FoundationTeacherLesson::Feeding) {
        return Err("custom food placement is only supported for feeding lessons".into());
    }
    let food = world
        .entity_id("food-01")
        .ok_or("teacher world has no food")?;
    // Separate physical variety from the vocabulary residue; otherwise all
    // demonstrations of the word food would always show the same recipe.
    world.set_food_variety(food, alife_world::FoodVariety::from_seed(seed / 12))?;
    let hazard = world
        .entity_id("hazard-01")
        .ok_or("teacher world has no hazard")?;
    let waypoint = world
        .entity_id("obstacle-02")
        .ok_or("teacher world has no second obstacle")?;
    let organism = world.organism_entity_ids()[0].1;
    let origin = world
        .entity(organism)
        .ok_or("scenario founder is missing")?
        .position;
    for label in ["meadow-ball", "meadow-activity-toy"] {
        if let Some(id) = world.entity_id(label) {
            move_scenario_object(
                world,
                id,
                alife_core::Vec3f::new(origin.x - 80.0, origin.y, origin.z - 80.0),
            )?;
        }
    }
    let mut closing_gate = None;
    let mut maze_walls = Vec::new();
    let mut demonstration_route = Vec::new();
    let mut vocabulary_target = None;
    let mut vocabulary_noun = None;
    let mut vocabulary_token = None;
    match lesson {
        Some(FoundationTeacherLesson::EatHeldFood | FoundationTeacherLesson::GrabFood) => {
            // Keep either first manipulation primitive free of navigation.
            // Eat setup acquires food; Grab setup leaves it unowned within reach.
            for label in ["obstacle-01", "obstacle-02", "hazard-01", "food-02"] {
                if let Some(entity) = world.entity_id(label) {
                    move_scenario_object(
                        world,
                        entity,
                        alife_core::Vec3f::new(origin.x - 30.0, origin.y, origin.z + 20.0),
                    )?;
                }
            }
        }
        Some(FoundationTeacherLesson::Feeding | FoundationTeacherLesson::VisionSearch) => {
            let angle = std::f32::consts::TAU * scenario_random(seed, 0);
            let distance = 2.2 + 1.4 * scenario_random(seed, 1);
            let [x, z] = food_position.unwrap_or([
                origin.x + distance * angle.cos(),
                origin.z + distance * angle.sin(),
            ]);
            move_scenario_object(world, food, alife_core::Vec3f::new(x, 0.0, z))?;
        }
        Some(FoundationTeacherLesson::HazardAvoidance) => {
            // Once the hazard is out of the sensed danger range, the same
            // teacher must resume care instead of learning to idle forever.
            move_scenario_object(
                world,
                food,
                alife_core::Vec3f::new(
                    origin.x - 5.0 - scenario_random(seed, 2),
                    0.0,
                    origin.z + 1.4 * (scenario_random(seed, 3) - 0.5),
                ),
            )?;
            move_scenario_object(
                world,
                hazard,
                alife_core::Vec3f::new(5.0, 0.0, (seed % 3) as f32 - 1.0),
            )?;
        }
        Some(FoundationTeacherLesson::ObstacleNavigation) => {
            let angle = (scenario_random(seed, 4) - 0.5) * 2.6;
            let forward = [angle.cos(), angle.sin()];
            let lateral = [-forward[1], forward[0]];
            let side = if seed & 2 == 0 { -1.0 } else { 1.0 };
            let food_distance = 6.2 + 0.7 * scenario_random(seed, 5);
            let food_lateral = scenario_random(seed, 6) - 0.5;
            let blocker_distance = 4.0 + 0.2 * scenario_random(seed, 7);
            let position = |ahead: f32, across: f32| {
                alife_core::Vec3f::new(
                    origin.x + ahead * forward[0] + across * lateral[0],
                    0.0,
                    origin.z + ahead * forward[1] + across * lateral[1],
                )
            };
            let blocker = world
                .entity_id("obstacle-01")
                .ok_or("teacher world has no first obstacle")?;
            move_scenario_object(world, food, position(food_distance, food_lateral))?;
            move_scenario_object(world, blocker, position(blocker_distance, side * 7.0))?;
            closing_gate = Some((
                blocker,
                position(
                    blocker_distance,
                    food_lateral * blocker_distance / food_distance,
                ),
            ));
            move_scenario_object(world, waypoint, position(0.0, side * 6.4))?;
            // This lesson isolates routing; the hazard belongs to its own lesson.
            move_scenario_object(world, hazard, position(-8.0, 7.0))?;
        }
        Some(FoundationTeacherLesson::Recovery) => {
            move_scenario_object(
                world,
                food,
                alife_core::Vec3f::new(
                    origin.x + 1.5 + scenario_random(seed, 8),
                    0.0,
                    origin.z + 2.0 * (scenario_random(seed, 9) - 0.5),
                ),
            )?;
        }
        Some(FoundationTeacherLesson::MazeNavigation) => {
            for label in ["obstacle-01", "obstacle-02", "hazard-01"] {
                if let Some(entity) = world.entity_id(label) {
                    move_scenario_object(
                        world,
                        entity,
                        alife_core::Vec3f::new(origin.x - 30.0, origin.y, origin.z + 20.0),
                    )?;
                }
            }
            let layout = maze::configure(world, seed, origin, food)?;
            maze_walls = layout.walls;
            demonstration_route = layout.demonstration_route;
        }
        Some(
            FoundationTeacherLesson::VocabularyReception
            | FoundationTeacherLesson::VocabularyProduction,
        ) => {
            let words = alife_core::FOUNDATION_LESSON_VOCABULARY;
            vocabulary_token = Some(words[(seed % words.len() as u64) as usize]);
            if let Some(word @ 15..=17) = vocabulary_token {
                world.set_food_variety(
                    food,
                    alife_world::FoodVariety::from_seed(u64::from(word - 15)),
                )?;
            }
            move_scenario_object(
                world,
                food,
                alife_core::Vec3f::new(origin.x + 1.5, origin.y, origin.z),
            )?;
            // `get` must select the movable example. Noun lessons alternate
            // physical toy uses across seeds rather than equating toy with a ball.
            let movable = matches!(vocabulary_token, Some(9 | 14))
                || (vocabulary_token != Some(18) && (seed / 18) % 2 == 0);
            let toy = world.spawn_toy(
                "vocabulary-toy",
                alife_core::Vec3f::new(origin.x + 1.5, origin.y, origin.z + 2.0),
                movable,
            )?;
            let creature = world.spawn_social_agent(
                "vocabulary-creature",
                alife_core::OrganismId(
                    world
                        .organism_entity_ids()
                        .iter()
                        .map(|(id, _)| id.raw())
                        .max()
                        .unwrap_or(0)
                        + 1,
                ),
                alife_core::Vec3f::new(origin.x + 3.0, origin.y, origin.z - 2.0),
                0.75,
            )?;
            move_scenario_object(
                world,
                waypoint,
                alife_core::Vec3f::new(origin.x + 3.0, origin.y, origin.z + 3.0),
            )?;
            if let Some(entity) = world.entity_id("obstacle-01") {
                move_scenario_object(
                    world,
                    entity,
                    alife_core::Vec3f::new(origin.x - 30.0, origin.y, origin.z + 20.0),
                )?;
            }
            move_scenario_object(
                world,
                hazard,
                alife_core::Vec3f::new(origin.x - 30.0, origin.y, origin.z - 20.0),
            )?;
            let targets = [food, toy, creature, waypoint];
            for (i, entity) in targets.iter().enumerate() {
                let angle = (-75.0
                    + 50.0 * ((i + ((seed / 10) % 4) as usize) % 4) as f32
                    + (scenario_random(seed, 19) - 0.5) * 40.0)
                    .to_radians();
                move_scenario_object(
                    world,
                    *entity,
                    alife_core::Vec3f::new(
                        origin.x + 3.0 * angle.cos(),
                        origin.y,
                        origin.z + 3.0 * angle.sin(),
                    ),
                )?;
            }
            // Category and subtype words must survive a physical same-category choice.
            let food_position = world
                .entity(food)
                .ok_or("vocabulary food missing")?
                .position;
            let food_bearing = (food_position.z - origin.z).atan2(food_position.x - origin.x)
                + 25.0_f32.to_radians();
            if let Some(other_food) = world.entity_id("food-02") {
                let variety = match vocabulary_token {
                    Some(word @ 15..=17) => u64::from(word - 15),
                    _ => seed / 12,
                };
                world.set_food_variety(
                    other_food,
                    alife_world::FoodVariety::from_seed(variety + 1),
                )?;
                move_scenario_object(
                    world,
                    other_food,
                    alife_core::Vec3f::new(
                        origin.x + 3.8 * food_bearing.cos(),
                        origin.y,
                        origin.z + 3.8 * food_bearing.sin(),
                    ),
                )?;
            }
            let toy_position = world.entity(toy).ok_or("vocabulary toy missing")?.position;
            let toy_bearing = (toy_position.z - origin.z).atan2(toy_position.x - origin.x)
                - 25.0_f32.to_radians();
            world.spawn_toy(
                "vocabulary-other-toy",
                alife_core::Vec3f::new(
                    origin.x + 3.8 * toy_bearing.cos(),
                    origin.y,
                    origin.z + 3.8 * toy_bearing.sin(),
                ),
                !movable,
            )?;
            let noun = match vocabulary_token {
                Some(n @ 1..=4) => n,
                Some(9 | 13 | 14 | 18) => 2,
                Some(5..=7) => 1 + ((seed / 10) % 4) as u16,
                _ => 1,
            };
            vocabulary_noun = Some(match noun {
                1 => 15 + ((seed / 12) % 3) as u16,
                2 => {
                    if matches!(vocabulary_token, Some(9 | 14))
                        || (vocabulary_token != Some(18) && (seed / 18) % 2 == 0)
                    {
                        14
                    } else {
                        18
                    }
                }
                _ => noun,
            });
            vocabulary_target = Some(targets[noun as usize - 1]);
            if vocabulary_token == Some(7) {
                // Retreat must start from an actionable visible referent. The
                // terrain profile exposes only three objects to the motor
                // candidate window; four equally distant examples can hide
                // the named object there even while it is in forward sight.
                // Keep distractors, but make this initial referent nearer.
                let target = targets[noun as usize - 1];
                let position = world
                    .entity(target)
                    .ok_or("retreat target missing")?
                    .position;
                let scale = (2.5 + scenario_random(seed, 23) * 0.2) / 3.0;
                move_scenario_object(
                    world,
                    target,
                    alife_core::Vec3f::new(
                        origin.x + (position.x - origin.x) * scale,
                        position.y,
                        origin.z + (position.z - origin.z) * scale,
                    ),
                )?;
            }
        }
        None => {}
    }
    if precondition_recovery
        && (lesson == Some(FoundationTeacherLesson::Recovery) || vocabulary_token == Some(12))
    {
        let organism = world.organism_entity_ids()[0].0;
        let required_fatigue = 0.12 + (seed % 4) as f32 * 0.025;
        // The inherited waking emitter releases at most 0.00007 per tick,
        // before developmental expression and decay. Even its maximum rate
        // cannot reach the upper 0.195 lesson target within 2,400 ticks.
        // Allow ordinary biology to reach the target; never inject tiredness.
        for _ in 0..7_200 {
            let fatigue = world
                .organism_registry()
                .get(organism)
                .ok_or("recovery organism is missing")?
                .biochemistry()
                .homeostasis
                .drives
                .fatigue;
            if fatigue >= required_fatigue {
                break;
            }
            world.try_advance_tick()?;
        }
        let fatigue = world
            .organism_registry()
            .get(organism)
            .ok_or("recovery organism is missing")?
            .biochemistry()
            .homeostasis
            .drives
            .fatigue;
        if fatigue < required_fatigue {
            return Err(format!(
                "ordinary world aging produced fatigue {fatigue:.6}, below lesson target {required_fatigue:.6} after 7200 ticks"
            )
            .into());
        }
    }
    if precondition_recovery && vocabulary_token == Some(11) {
        let organism = world.organism_entity_ids()[0].0;
        for _ in 0..2400 {
            let need = world
                .organism_registry()
                .get(organism)
                .ok_or("hunger organism missing")?
                .biochemistry()
                .homeostasis
                .drives
                .hunger;
            if need >= 0.12 {
                break;
            }
            world.try_advance_tick()?;
        }
        if world
            .organism_registry()
            .get(organism)
            .ok_or("hunger organism missing")?
            .biochemistry()
            .homeostasis
            .drives
            .hunger
            < 0.12
        {
            return Err("ordinary biology did not produce a measurable hunger need".into());
        }
    }
    let initial_hazard_distance = if lesson == Some(FoundationTeacherLesson::HazardAvoidance) {
        let organism = world.organism_entity_ids()[0].1;
        let subject = world
            .entity(organism)
            .ok_or("evaluation organism is missing")?
            .position;
        let hazard_position = world
            .entity(hazard)
            .ok_or("evaluation hazard is missing")?
            .position;
        Some(
            ((subject.x - hazard_position.x).powi(2) + (subject.z - hazard_position.z).powi(2))
                .sqrt(),
        )
    } else {
        None
    };
    let held_food_setup = if lesson == Some(FoundationTeacherLesson::EatHeldFood) {
        Some(establish_foundation_held_food(world, food, seed)?)
    } else {
        None
    };
    let grab_food_setup = if lesson == Some(FoundationTeacherLesson::GrabFood) {
        Some(establish_foundation_grab_food(world, food, seed)?)
    } else {
        None
    };
    let teacher_cue_tokens = if grab_food_setup.is_some() {
        vec![9, 1] // Contextual request: "get food", through spatial hearing.
    } else if held_food_setup.is_some() {
        vec![8, 1] // Contextual request: "eat food", through spatial hearing.
    } else {
        Vec::new()
    };
    Ok(FoundationScenarioSetup {
        food,
        hazard,
        waypoint,
        initial_hazard_distance,
        closing_gate,
        maze_walls,
        demonstration_route,
        vocabulary_token,
        teacher_cue_tokens,
        held_food_setup,
        grab_food_setup,
        vocabulary_target,
        vocabulary_noun,
        repeat_vocabulary: true,
        silent_request: false,
        fixed_speaker_position: None,
        training_language_feedback: !matches!(
            lesson,
            Some(FoundationTeacherLesson::EatHeldFood | FoundationTeacherLesson::GrabFood)
        ),
        start_tick: world.tick().raw(),
        last_language_praise_tick: std::cell::Cell::new(None),
        language_praise_count: std::cell::Cell::new(0),
    })
}

fn held_food_learner_distance(
    world: &alife_world::HeadlessWorld,
    food: alife_core::WorldEntityId,
    learner: alife_core::OrganismId,
) -> Option<f32> {
    // A remaining teacher or peer is never a substitute for the learner.
    // Retired body positions are not guessed from a former carried object.
    let record = world.organism_registry().get(learner)?;
    if !record.lifecycle().is_alive() {
        return None;
    }
    let body = world.entity(record.world_entity_id())?;
    if body.organism_id != Some(learner) {
        return None;
    }
    let food = world.entity(food)?.position;
    let position = body.position;
    Some(
        ((food.x - position.x).powi(2)
            + (food.y - position.y).powi(2)
            + (food.z - position.z).powi(2))
        .sqrt(),
    )
}

fn teacher_food_distance(runtime: &GpuLiveBrainRuntime) -> Result<f32> {
    let world = runtime.world();
    let food = world
        .entity_id("food-01")
        .ok_or("teacher world has no food")?;
    let organism = world
        .organism_entity_ids()
        .into_iter()
        .next()
        .ok_or("teacher world has no organism")?
        .1;
    let food = world
        .entity(food)
        .ok_or("teacher food is missing")?
        .position;
    let organism = world
        .entity(organism)
        .ok_or("teacher organism is missing")?
        .position;
    Ok(((food.x - organism.x).powi(2)
        + (food.y - organism.y).powi(2)
        + (food.z - organism.z).powi(2))
    .sqrt())
}

fn compare_replay_values(
    tick: usize,
    label: &str,
    actual: &[f32],
    expected: &[f32],
) -> Result<f32> {
    if actual.len() != expected.len() {
        return Err(invalid().into());
    }
    let mut maximum = 0.0f32;
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        let error = (actual - expected).abs();
        if !actual.is_finite() || !expected.is_finite() || error > 1.0e-4 + 1.0e-4 * expected.abs()
        {
            return Err(
                format!("tick {tick}: {label}[{index}] mismatch {actual} vs {expected}").into(),
            );
        }
        maximum = maximum.max(error);
    }
    Ok(maximum)
}

/// Verify exact GPU rollout/replay agreement at a new waking segment, including structural growth.
/// This is deliberately run once per sleep boundary, not on every collection tick.
pub fn verify_foundation_replay_step(
    trainer: &mut FoundationTrainer,
    step: &FoundationTrainingStep,
) -> Result<()> {
    let sequence = foundation_replay_sequence(std::slice::from_ref(step), 0)?;
    let replay = trainer.evaluate_replay(&sequence)?;
    compare_replay_values(
        0,
        "activation",
        &replay.final_activations[0],
        &snapshot_activation(&step.after_inference)?,
    )?;
    let observed = &step.behavior.logits;
    let support = step
        .behavior
        .motor_masks
        .iter()
        .fold(step.behavior.representative_mask, |mask, channel| {
            mask | channel
        });
    if replay.candidate_logits[0].len() != observed.len() {
        return Err(invalid().into());
    }
    for (candidate, (actual, expected)) in
        replay.candidate_logits[0].iter().zip(observed).enumerate()
    {
        if !expected.is_finite() {
            if support & (1u32 << candidate) != 0 {
                return Err("nonfinite production logit is in sampled policy support".into());
            }
            continue;
        }
        compare_replay_values(0, "candidate logit", &[*actual], &[*expected])?;
    }
    let homeostasis = snapshot_values(
        &step.after_inference,
        step.after_inference
            .brain_slot
            .word_ranges()
            .homeostasis_words
            .clone(),
    )?;
    let ema: Vec<f32> = homeostasis.chunks_exact(2).map(|row| row[0]).collect();
    let metabolic: Vec<f32> = homeostasis.chunks_exact(2).map(|row| row[1]).collect();
    compare_replay_values(0, "activity EMA", &replay.final_activity_ema[0], &ema)?;
    compare_replay_values(
        0,
        "metabolic load",
        &replay.final_metabolic_load[0],
        &metabolic,
    )?;
    Ok(())
}

pub const FOUNDATION_REPLAY_HOST_LIMIT_BYTES: u64 = 8 * 1024 * 1024 * 1024;
pub const FOUNDATION_REPLAY_DISK_LIMIT_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const FOUNDATION_REPLAY_RECORD_LIMIT_BYTES: u64 = 256 * 1024 * 1024;
// Schema 2 always encodes candidate innate_bias, including zero. Schema 1's
// omitted positional fields cannot be recovered without guessing recorded data.
const FOUNDATION_REPLAY_SCHEMA: u32 = 2;

fn deserialize_replay_schema<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<u32, D::Error> {
    let schema = <u32 as serde::Deserialize>::deserialize(deserializer)?;
    if schema != FOUNDATION_REPLAY_SCHEMA {
        return Err(serde::de::Error::custom(format!(
            "unsupported compact replay schema {schema}; expected {FOUNDATION_REPLAY_SCHEMA}; regenerate the recorded corpus"
        )));
    }
    Ok(schema)
}

/// Recorded provenance, not authentication. The caller obtains the checkpoint
/// digest from the actual frozen actor, and keeps that actor pinned for the life.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FoundationReplaySource {
    pub source_revision: String,
    pub policy_version: u64,
    pub actor_checkpoint_digest: alife_core::Blake3Digest,
    pub foundation_asset_digest: alife_core::Blake3Digest,
    pub phenotype_hash: alife_core::PhenotypeHash,
    pub compiler_inputs_digest: [u64; 4],
}

pub(crate) fn foundation_replay_source(
    phenotype: &alife_core::BrainPhenotype,
    asset: &FoundationWeightAsset,
    policy_version: u64,
    actor_checkpoint: &[u8],
) -> Result<FoundationReplaySource> {
    // Launch from the qualified checkout; a downloaded binary's build-host path
    // may not exist on this computer.
    let revision = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !revision.status.success() {
        return Err("could not identify training source revision".into());
    }
    Ok(FoundationReplaySource {
        source_revision: String::from_utf8(revision.stdout)?.trim().to_owned(),
        policy_version,
        actor_checkpoint_digest: alife_core::Blake3Digest::from_bytes(
            *blake3::hash(actor_checkpoint).as_bytes(),
        ),
        foundation_asset_digest: asset.digest(),
        phenotype_hash: phenotype.phenotype_hash(),
        compiler_inputs_digest: phenotype.compiler_inputs_digest(),
    })
}

/// Resident-process ceiling, not merely a cap on the serialized record. The
/// runner can use check_additional before requesting its next GPU capture too.
#[derive(Debug, Clone, Copy)]
pub struct FoundationReplayBudget {
    pub host_limit_bytes: u64,
}
impl Default for FoundationReplayBudget {
    fn default() -> Self {
        Self {
            host_limit_bytes: FOUNDATION_REPLAY_HOST_LIMIT_BYTES,
        }
    }
}
impl FoundationReplayBudget {
    pub fn check_additional(self, bytes: u64) -> Result<()> {
        if self.host_limit_bytes == 0 || self.host_limit_bytes > FOUNDATION_REPLAY_HOST_LIMIT_BYTES
        {
            return Err("replay host limit must be positive and at most 8 GiB".into());
        }
        let allocated = replay_process_bytes()?;
        if allocated
            .checked_add(bytes)
            .and_then(|n| n.checked_add(64 * 1024 * 1024))
            .is_none_or(|n| n > self.host_limit_bytes)
        {
            return Err(
                "replay allocation would exceed the host memory ceiling; shorten the replay window"
                    .into(),
            );
        }
        Ok(())
    }
}

#[cfg(windows)]
fn replay_process_bytes() -> Result<u64> {
    use windows_sys::Win32::System::{
        ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::GetCurrentProcess,
    };
    // SAFETY: zero initializes a plain Windows ABI output record; its cb is set
    // before the API receives a valid writable pointer to that exact record.
    let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // SAFETY: process pseudo-handle and record pointer remain valid for this call.
    if unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(counters.WorkingSetSize.max(counters.PagefileUsage) as u64)
}
#[cfg(not(windows))]
fn replay_process_bytes() -> Result<u64> {
    Err("bounded production replay currently requires the Windows process-memory query".into())
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct ReplayContinuity {
    organism_id: u64,
    slot: u32,
    slot_generation: u32,
    dispatch_generation: u64,
    activation_side: u8,
    weight_generation: u64,
    weight_bank: u8,
    recurrent_digest: [u8; 32],
    weight_digest: [u8; 32],
    dendrites_digest: [u8; 32],
}
impl ReplayContinuity {
    fn capture(snapshot: &GpuTrainingStateSnapshot) -> Result<Self> {
        let mut recurrent = blake3::Hasher::new();
        for range in [
            activation_range(snapshot)?,
            snapshot.brain_slot.word_ranges().homeostasis_words.clone(),
        ] {
            for word in snapshot_words(snapshot, range)? {
                recurrent.update(&word.to_le_bytes());
            }
        }
        let mut weights = blake3::Hasher::new();
        let (lifetime, fast) = active_weight_ranges(snapshot)?;
        for range in [lifetime, fast] {
            for word in snapshot_words(snapshot, range)? {
                weights.update(&word.to_le_bytes());
            }
        }
        Ok(Self {
            organism_id: snapshot.handle.organism_id().raw(),
            slot: snapshot.handle.slot(),
            slot_generation: snapshot.handle.generation(),
            dispatch_generation: snapshot.logical_dispatch_generation,
            activation_side: snapshot.active_activation_side,
            weight_generation: snapshot.active_weight_generation,
            weight_bank: snapshot.active_weight_bank,
            recurrent_digest: *recurrent.finalize().as_bytes(),
            weight_digest: *weights.finalize().as_bytes(),
            dendrites_digest: *blake3::hash(&serde_json::to_vec(&snapshot.v11.dendritic_branches)?)
                .as_bytes(),
        })
    }
    fn validate_next(&self, next: &Self) -> Result<()> {
        if self.organism_id != next.organism_id
            || self.slot != next.slot
            || self.slot_generation != next.slot_generation
            || self.dispatch_generation != next.dispatch_generation
            || self.activation_side != next.activation_side
            || self.recurrent_digest != next.recurrent_digest
            || self.dendrites_digest != next.dendrites_digest
            || next.weight_generation < self.weight_generation
            || (next.weight_generation == self.weight_generation
                && (next.weight_bank != self.weight_bank
                    || next.weight_digest != self.weight_digest))
        {
            return Err(
                "replay discontinuity: start an explicit segment after sleep/reset/state change"
                    .into(),
            );
        }
        Ok(())
    }
}

/// A compact disk record contains replay inputs and the original sealed world
/// outcome. It contains no full mutable GPU allocation or second world state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FoundationReplayRecord {
    #[serde(deserialize_with = "deserialize_replay_schema")]
    schema: u32,
    pub source: FoundationReplaySource,
    pub record_index: u64,
    pub segment: u64,
    pub tick: u64,
    pub organism_id: u64,
    pub sequence: TrainingSequence,
    pub behavior: alife_gpu_backend::GpuTrainingRolloutReceipt,
    #[serde(with = "replay_patch_json")]
    pub patch: alife_core::ExperiencePatch,
    before: ReplayContinuity,
    after: ReplayContinuity,
}

// ExperiencePatch contains an untagged legacy variant. Preserve its validated
// JSON wire inside the compact binary envelope instead of changing that type.
mod replay_patch_json {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        patch: &alife_core::ExperiencePatch,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let bytes = serde_json::to_vec(patch).map_err(serde::ser::Error::custom)?;
        bytes.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<alife_core::ExperiencePatch, D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        serde_json::from_slice(&bytes).map_err(serde::de::Error::custom)
    }
}

/// Keep these small references in the life manifest, not full GPU captures.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FoundationReplayRecordRef {
    pub record_index: u64,
    pub segment: u64,
    pub tick: u64,
    pub encoded_bytes: u64,
    pub digest: [u8; 32],
}
impl FoundationReplayRecordRef {
    pub fn file_name(&self) -> String {
        format!("replay-{:012}.bin.zst", self.record_index)
    }
}

/// Streaming writer retains only the compiled topology and one compact boundary
/// fingerprint. Drop each FoundationTrainingStep after append returns.
pub struct FoundationReplayWriter {
    directory: std::path::PathBuf,
    source: FoundationReplaySource,
    phenotype: alife_core::BrainPhenotype,
    upload: alife_gpu_backend::closed_loop_buffers::GpuPhenotypeUpload,
    budget: FoundationReplayBudget,
    next_index: u64,
    written_bytes: u64,
    segment: u64,
    previous: Option<(u64, ReplayContinuity, alife_gpu_backend::GpuBrainSlot)>,
}
impl FoundationReplayWriter {
    pub fn new(
        directory: &Path,
        source: FoundationReplaySource,
        phenotype: alife_core::BrainPhenotype,
        asset: &FoundationWeightAsset,
        budget: FoundationReplayBudget,
    ) -> Result<Self> {
        asset.validate_against(&phenotype)?;
        if source.source_revision.is_empty()
            || source.source_revision.len() > 128
            || source.foundation_asset_digest != asset.digest()
            || source.phenotype_hash != phenotype.phenotype_hash()
            || source.compiler_inputs_digest != phenotype.compiler_inputs_digest()
            || phenotype
                .synapses()
                .iter()
                .zip(asset.weights())
                .any(|(s, w)| s.genetic_weight().to_bits() != w.to_bits())
        {
            return Err("replay source is not the exact frozen actor phenotype".into());
        }
        budget.check_additional((phenotype.synapses().len() as u64).saturating_mul(256))?;
        std::fs::create_dir_all(directory)?;
        if std::fs::read_dir(directory)?.next().is_some() {
            return Err("replay writer requires a new empty life directory; existing records are never overwritten".into());
        }
        let upload =
            alife_gpu_backend::closed_loop_buffers::GpuPhenotypeUpload::try_from(&phenotype)?;
        Ok(Self {
            directory: directory.to_path_buf(),
            source,
            phenotype,
            upload,
            budget,
            next_index: 0,
            written_bytes: 0,
            segment: 0,
            previous: None,
        })
    }

    /// A real sleep/reset/gap ends recurrent continuity. This does not terminate
    /// the biological life or GAE; the runner records its actual outcome separately.
    pub fn start_segment(&mut self) -> Result<()> {
        self.segment = self.segment.checked_add(1).ok_or_else(invalid)?;
        self.previous = None;
        Ok(())
    }

    pub fn append(&mut self, step: &FoundationTrainingStep) -> Result<FoundationReplayRecordRef> {
        if step.before.phenotype.as_ref() != &self.phenotype
            || step.after_inference.phenotype.as_ref() != &self.phenotype
        {
            return Err("frozen genetic actor changed within a replay life".into());
        }
        // Reserve conversion's temporary float arrays, candidate/patch copies,
        // and serialized evidence before creating another record-sized object.
        let scratch = (step.before.mutable_words.len() as u64)
            .checked_add(step.after_inference.mutable_words.len() as u64)
            .and_then(|n| n.checked_mul(16))
            .ok_or_else(invalid)?;
        self.budget.check_additional(scratch)?;
        let sequence =
            foundation_replay_sequence_with_upload(std::slice::from_ref(step), 0, &self.upload)?;
        if alife_core::OutcomeCreditPacket::from_sealed_patch(&step.patch)? != step.outcome_credit {
            return Err("capture outcome credit differs from its sealed world patch".into());
        }
        let before = ReplayContinuity::capture(&step.before)?;
        let after = ReplayContinuity::capture(&step.after_inference)?;
        if let Some((tick, previous, slot)) = &self.previous {
            if tick.checked_add(1) != Some(step.frame.tick().raw())
                || *slot != step.before.brain_slot
            {
                return Err("replay tick/allocation gap requires an explicit new segment".into());
            }
            previous.validate_next(&before)?;
        }
        let record = FoundationReplayRecord {
            schema: FOUNDATION_REPLAY_SCHEMA,
            source: self.source.clone(),
            record_index: self.next_index,
            segment: self.segment,
            tick: step.frame.tick().raw(),
            organism_id: step.frame.organism_id().raw(),
            sequence,
            behavior: step.behavior.clone(),
            patch: step.patch.clone(),
            before,
            after: after.clone(),
        };
        record.validate(&self.source, &self.phenotype)?;
        if self.written_bytes >= FOUNDATION_REPLAY_DISK_LIMIT_BYTES {
            return Err("compact replay reached its 4 GiB disk ceiling".into());
        }
        let reference = write_replay_record(&self.directory, &record)?;
        let next_bytes = self
            .written_bytes
            .checked_add(reference.encoded_bytes)
            .ok_or_else(invalid)?;
        if next_bytes > FOUNDATION_REPLAY_DISK_LIMIT_BYTES {
            std::fs::remove_file(self.directory.join(reference.file_name()))?;
            return Err("compact replay would exceed its 4 GiB disk ceiling".into());
        }
        self.previous = Some((record.tick, after, step.after_inference.brain_slot.clone()));
        self.next_index = self.next_index.checked_add(1).ok_or_else(invalid)?;
        self.written_bytes = next_bytes;
        Ok(reference)
    }
}

impl FoundationReplayRecord {
    fn validate(
        &self,
        expected: &FoundationReplaySource,
        phenotype: &alife_core::BrainPhenotype,
    ) -> Result<()> {
        if self.schema != FOUNDATION_REPLAY_SCHEMA
            || &self.source != expected
            || self.source.phenotype_hash != phenotype.phenotype_hash()
            || self.source.compiler_inputs_digest != phenotype.compiler_inputs_digest()
            || self.sequence.ticks.len() != 1
            || self.sequence.burn_in_ticks != 0
            || self.tick != self.behavior.tick
            || self.organism_id != self.behavior.organism_id
            || self.organism_id != self.before.organism_id
            || self.organism_id != self.after.organism_id
            || self.patch.pre_action().perception().frame_digest() != self.behavior.frame_digest
            || self.patch.pre_action().perception().tick().raw() != self.tick
            || self.patch.pre_action().perception().organism_id().raw() != self.organism_id
            || self.before.weight_generation != self.behavior.active_weight_generation
            || self.after.weight_generation != self.before.weight_generation
            || self.after.weight_bank != self.before.weight_bank
            || self.after.weight_digest != self.before.weight_digest
            || self.after.dispatch_generation != self.behavior.dispatch_generation
            || self.after.activation_side
                != (self.before.activation_side
                    ^ (self.sequence.ticks[0].microstep_count as u8 & 1))
        {
            return Err("replay record identity/continuity contract mismatch".into());
        }
        self.sequence.validate_for(phenotype)?;
        // Check float bits after decoding too. A codec/parser change must fail
        // rather than silently alter the recurrent boundary used by replay.
        let mut recurrent = blake3::Hasher::new();
        for value in &self.sequence.initial.activations {
            recurrent.update(&value.to_bits().to_le_bytes());
        }
        for (activity, metabolic) in self
            .sequence
            .initial
            .activity_ema
            .iter()
            .zip(&self.sequence.initial.metabolic_load)
        {
            recurrent.update(&activity.to_bits().to_le_bytes());
            recurrent.update(&metabolic.to_bits().to_le_bytes());
        }
        if recurrent.finalize().as_bytes() != &self.before.recurrent_digest
            || blake3::hash(&serde_json::to_vec(&self.sequence.initial.dendrites)?).as_bytes()
                != &self.before.dendrites_digest
        {
            return Err("decoded replay initial state differs from the captured boundary".into());
        }
        self.patch.validate_contract()?;
        self.behavior.sampling.validate()?;
        // The sealed patch retains actual body/homeostasis and measured outcome,
        // rather than constructing reward or physiology from the proposed action.
        alife_core::OutcomeCreditPacket::from_sealed_patch(&self.patch)?;
        let tick = &self.sequence.ticks[0];
        let count = tick.candidates.len();
        if count == 0
            || count > 32
            || self.behavior.logits.len() != count
            || self.behavior.forced_motor_slots.len() != count
            || self.behavior.decoder_input_stride > 54
            || self.behavior.decoder_input_stride < 24
            || self.behavior.decoder_inputs.len() != count * self.behavior.decoder_input_stride
            || self.patch.pre_action().perception().candidates().len() != count
            || !self
                .behavior
                .logits
                .iter()
                .chain(self.behavior.factor_log_probabilities.iter())
                .all(|x| x.is_finite())
            || !self.behavior.policy_joint_log_probability.is_finite()
        {
            return Err(invalid().into());
        }
        for (index, candidate) in tick.candidates.iter().enumerate() {
            let begin = index * self.behavior.decoder_input_stride;
            if candidate.family != self.patch.pre_action().perception().candidates()[index].family
                || candidate.decoder_inputs[..self.behavior.decoder_input_stride]
                    != self.behavior.decoder_inputs
                        [begin..begin + self.behavior.decoder_input_stride]
            {
                return Err("replay candidate inputs differ from GPU behavior evidence".into());
            }
        }
        if self.behavior.behavior == alife_gpu_backend::GpuTrainingBehaviorKind::OnPolicy {
            self.behavior.on_policy_log_probability()?;
        } else if self.behavior.joint_log_probability.is_some()
            || self.behavior.sampling.demonstrator.is_none()
        {
            return Err("demonstration record cannot claim on-policy behavior likelihood".into());
        }
        Ok(())
    }
}

struct ReplayDigestWriter<W> {
    inner: W,
    hasher: blake3::Hasher,
    bytes: u64,
}
impl<W: std::io::Write> std::io::Write for ReplayDigestWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.bytes.saturating_add(bytes.len() as u64) > FOUNDATION_REPLAY_RECORD_LIMIT_BYTES {
            return Err(std::io::Error::other(
                "compact replay record exceeds file limit",
            ));
        }
        let n = self.inner.write(bytes)?;
        self.hasher.update(&bytes[..n]);
        self.bytes += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
fn write_replay_record(
    directory: &Path,
    record: &FoundationReplayRecord,
) -> Result<FoundationReplayRecordRef> {
    use std::io::Write;
    let mut reference = FoundationReplayRecordRef {
        record_index: record.record_index,
        segment: record.segment,
        tick: record.tick,
        encoded_bytes: 0,
        digest: [0; 32],
    };
    let destination = directory.join(reference.file_name());
    let temporary = directory.join(format!("replay-{:012}.pending", record.record_index));
    if destination.exists() {
        return Err("replay record already exists".into());
    }
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| -> Result<_> {
        let mut writer = ReplayDigestWriter {
            inner: std::io::BufWriter::with_capacity(64 * 1024, file),
            hasher: blake3::Hasher::new(),
            bytes: 0,
        };
        let mut encoder = zstd::stream::write::Encoder::new(&mut writer, 3)?;
        bincode::serde::encode_into_std_write(record, &mut encoder, bincode::config::standard())?;
        encoder.finish()?;
        writer.flush()?;
        writer.inner.get_ref().sync_all()?;
        reference.encoded_bytes = writer.bytes;
        reference.digest = *writer.hasher.finalize().as_bytes();
        drop(writer);
        std::fs::rename(&temporary, &destination)?;
        Ok(reference)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// Load one verified record without reading the entire life. BLAKE3 binds the
/// exact encoded bytes and provenance; typed validation still runs after decode.
pub fn load_foundation_replay_record(
    directory: &Path,
    reference: &FoundationReplayRecordRef,
    expected: &FoundationReplaySource,
    phenotype: &alife_core::BrainPhenotype,
    budget: FoundationReplayBudget,
) -> Result<FoundationReplayRecord> {
    use std::io::{Read, Seek};
    if reference.encoded_bytes == 0
        || reference.encoded_bytes > FOUNDATION_REPLAY_RECORD_LIMIT_BYTES
    {
        return Err(invalid().into());
    }
    // Conservative allowance for JSON decoding, vector capacities and validation
    // temporaries. Existing loaded windows are included in measured process use.
    budget.check_additional(
        reference
            .encoded_bytes
            .checked_mul(8)
            .and_then(|n| n.checked_add(FOUNDATION_REPLAY_RECORD_LIMIT_BYTES))
            .ok_or_else(invalid)?,
    )?;
    let mut file = std::fs::File::open(directory.join(reference.file_name()))?;
    if file.metadata()?.len() != reference.encoded_bytes {
        return Err("replay file length mismatch".into());
    }
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    if hasher.finalize().as_bytes() != &reference.digest {
        return Err("replay file digest mismatch".into());
    }
    file.rewind()?;
    let decoder = zstd::stream::read::Decoder::new(std::io::BufReader::new(file))?;
    let record: FoundationReplayRecord = bincode::serde::decode_from_std_read(
        &mut decoder.take(FOUNDATION_REPLAY_RECORD_LIMIT_BYTES + 1),
        bincode::config::standard().with_limit::<268435456>(),
    )?;
    record.validate(expected, phenotype)?;
    if record.record_index != reference.record_index
        || record.segment != reference.segment
        || record.tick != reference.tick
    {
        return Err(invalid().into());
    }
    Ok(record)
}

pub struct FoundationReplayWindow {
    pub sequence: TrainingSequence,
    /// Aligned with every replay row, including burn-in. The loss builder must
    /// exclude burn-in and calculate global life GAE before slicing its targets.
    pub behavior: Vec<alife_gpu_backend::GpuTrainingRolloutReceipt>,
    pub patches: Vec<alife_core::ExperiencePatch>,
    pub first_tick: u64,
    pub segment: u64,
}

/// Reload a bounded contiguous window from immutable record references. A caller
/// may overlap earlier rows as burn-in; this neither duplicates loss rows nor
/// invents a reset state. Segment boundaries cannot be crossed silently.
pub fn load_foundation_replay_window(
    directory: &Path,
    references: &[FoundationReplayRecordRef],
    burn_in_ticks: usize,
    expected: &FoundationReplaySource,
    phenotype: &alife_core::BrainPhenotype,
    budget: FoundationReplayBudget,
) -> Result<FoundationReplayWindow> {
    if references.is_empty()
        || references.len() > alife_training::MAX_TRAINING_SEQUENCE_TICKS
        || burn_in_ticks >= references.len()
    {
        return Err(invalid().into());
    }
    let total = references.iter().try_fold(0u64, |sum, r| {
        sum.checked_add(r.encoded_bytes).ok_or_else(invalid)
    })?;
    budget.check_additional(total.checked_mul(8).ok_or_else(invalid)?)?;
    let mut records = references.iter();
    let first = load_foundation_replay_record(
        directory,
        records.next().ok_or_else(invalid)?,
        expected,
        phenotype,
        budget,
    )?;
    let mut window = FoundationReplayWindow {
        sequence: first.sequence,
        behavior: vec![first.behavior],
        patches: vec![first.patch],
        first_tick: first.tick,
        segment: first.segment,
    };
    let mut previous = first.after;
    let mut tick = first.tick;
    let mut index = first.record_index;
    for reference in records {
        if reference.segment != window.segment
            || tick.checked_add(1) != Some(reference.tick)
            || index.checked_add(1) != Some(reference.record_index)
        {
            return Err("replay window crosses a gap or segment boundary".into());
        }
        let record =
            load_foundation_replay_record(directory, reference, expected, phenotype, budget)?;
        previous.validate_next(&record.before)?;
        window.sequence.ticks.extend(record.sequence.ticks);
        window.behavior.push(record.behavior);
        window.patches.push(record.patch);
        previous = record.after;
        tick = record.tick;
        index = record.record_index;
    }
    window.sequence.burn_in_ticks = burn_in_ticks;
    window.sequence.validate_for(phenotype)?;
    Ok(window)
}

#[cfg(test)]
mod held_food_lesson_tests {
    use super::*;
    use alife_core::experience::ChannelPhysicalOutcome;
    use alife_core::{
        ChannelCommand, Confidence, DurationTicks, ExperienceSequenceId, HomeostaticDelta,
        Intensity, JointPhysicalOutcome, MotorChannel, MotorCommandBundle, PhysicalContactKind,
        PostActionOutcome, SignedValence, Vec3f,
    };

    fn unconfigured(seed: u64) -> alife_world::HeadlessWorld {
        let asset = initial_n2048_care_asset(seed).unwrap();
        let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1).unwrap();
        config.brain_class = BrainScaleTier::Standard2048;
        config.sensor_profile = asset.manifest().sensor_profile();
        alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)
            .unwrap()
            .world
    }

    fn prepared(seed: u64) -> (alife_world::HeadlessWorld, FoundationScenarioSetup) {
        let mut world = unconfigured(seed);
        let scenario = configure_foundation_scenario(
            &mut world,
            seed,
            Some(FoundationTeacherLesson::EatHeldFood),
            None,
            true,
        )
        .unwrap();
        (world, scenario)
    }

    #[test]
    fn grab_food_setup_leaves_healthy_natural_need_and_legally_reachable_unheld_food() {
        for seed in [42, 91, 132] {
            let mut world = unconfigured(seed);
            let scenario = configure_foundation_scenario(
                &mut world,
                seed,
                Some(FoundationTeacherLesson::GrabFood),
                None,
                false,
            )
            .unwrap();
            let setup = scenario.grab_food_setup.as_ref().unwrap();
            assert!(scenario.held_food_setup.is_none());
            assert_eq!(setup.initial_owner, None);
            assert!(!setup.initial_consumed && !setup.initial_sleeping);
            assert!(setup.initial_hunger >= 0.12 && setup.initial_energy <= 0.9);
            assert!(setup.initial_health >= 0.95);
            assert_eq!(scenario.teacher_cue_tokens, vec![9, 1]);
            assert!(!scenario.training_language_feedback && scenario.vocabulary_token.is_none());
            let (organism, entity) = world.organism_entity_ids()[0];
            let grab = alife_core::ActionCommand::structured(
                organism,
                alife_world::HeadlessActionIds::GRAB,
                alife_core::ActionKind::Hold,
                alife_core::ActionTarget::new(Some(setup.target), None),
                Intensity::new(1.0).unwrap(),
                DurationTicks::new(1),
                Confidence::new(0.9).unwrap(),
                0,
                None,
                None,
                None,
            )
            .unwrap();
            let receipt = world
                .apply_registered_command(&grab, entity, Tick(world.tick().raw() + 1))
                .unwrap();
            world.try_advance_tick().unwrap();
            assert!(receipt.action_result.execution.succeeded);
            assert_eq!(
                receipt.action_result.execution.physical.contact,
                PhysicalContactKind::Touch
            );
            assert_eq!(
                world.entity(setup.target).unwrap().carried_by,
                Some(organism)
            );
            assert!(!world.entity(setup.target).unwrap().consumed);
        }
    }

    #[test]
    fn ordinary_teacher_meal_choice_sequences_real_grab_then_eat_exact_target() {
        let mut world = unconfigured(42);
        let scenario = configure_foundation_scenario(
            &mut world,
            42,
            Some(FoundationTeacherLesson::GrabFood),
            None,
            false,
        )
        .unwrap();
        let perceived = frame(&mut world);
        let eat = perceived
            .candidates()
            .iter()
            .find(|candidate| {
                candidate.action_id == alife_world::HeadlessActionIds::EAT
                    && candidate.target.entity == Some(scenario.food)
            })
            .unwrap();
        let empty = Vec::new();
        let grab = teacher_possession_action(&perceived, eat, &empty).unwrap();
        assert_eq!(grab.action_id, alife_world::HeadlessActionIds::GRAB);
        assert_eq!(grab.target.entity, Some(scenario.food));
        let held = vec![scenario.food];
        assert_eq!(
            teacher_possession_action(&perceived, eat, &held)
                .unwrap()
                .action_id,
            alife_world::HeadlessActionIds::EAT
        );
        let different = vec![scenario.hazard];
        assert_eq!(
            teacher_possession_action(&perceived, eat, &different)
                .unwrap()
                .action_id,
            alife_world::HeadlessActionIds::GRAB
        );
        let (organism, entity) = world.organism_entity_ids()[0];
        let grab_command = alife_core::ActionCommand::structured(
            organism,
            grab.action_id,
            grab.kind,
            alife_core::ActionTarget::new(Some(scenario.food), None),
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            Confidence::new(0.9).unwrap(),
            0,
            None,
            None,
            None,
        )
        .unwrap();
        let grabbed = world
            .apply_registered_command(&grab_command, entity, Tick(world.tick().raw() + 1))
            .unwrap();
        world.try_advance_tick().unwrap();
        assert!(grabbed.action_result.execution.succeeded);
        assert_eq!(
            grabbed.action_result.execution.physical.contact,
            PhysicalContactKind::Touch
        );
        let actual_held: Vec<_> = world
            .object_snapshots()
            .into_iter()
            .filter(|object| object.carried_by == Some(organism) && !object.consumed)
            .map(|object| object.id)
            .collect();
        let after_grab = frame(&mut world);
        let eat_candidate = after_grab
            .candidates()
            .iter()
            .find(|candidate| {
                candidate.action_id == alife_world::HeadlessActionIds::EAT
                    && candidate.target.entity == Some(scenario.food)
            })
            .unwrap();
        assert_eq!(
            teacher_possession_action(&after_grab, eat_candidate, &actual_held)
                .unwrap()
                .action_id,
            alife_world::HeadlessActionIds::EAT
        );
        let (_, eaten) = eat_outcome(&mut world, scenario.food);
        assert!(eaten.success);
        assert_eq!(eaten.physical.contact, PhysicalContactKind::Consumed);
        assert!(world.entity(scenario.food).unwrap().consumed);
        assert_eq!(world.entity(scenario.food).unwrap().carried_by, None);
    }

    #[test]
    fn held_food_seeded_pose_is_repeatable_varied_grounded_and_legally_held() {
        let mut poses = std::collections::BTreeSet::new();
        let mut facings = std::collections::BTreeSet::new();
        for seed in [42, 91, 132] {
            let mut original = unconfigured(seed);
            ground_foundation_training_world(&mut original).unwrap();
            let origin = original
                .entity(original.organism_entity_ids()[0].1)
                .unwrap()
                .position;
            let (world, scenario) = prepared(seed);
            let (_, repeated) = prepared(seed);
            let setup = scenario.held_food_setup.as_ref().unwrap();
            let again = repeated.held_food_setup.as_ref().unwrap();
            let position = setup.realized_body_position.unwrap();
            let yaw = setup.realized_body_yaw.unwrap();
            let grip = setup.realized_grip_position.unwrap();
            assert_eq!(setup.sampling_seed, seed);
            assert!((1..=HELD_FOOD_POSE_ATTEMPTS).contains(&setup.sampling_attempts));
            assert_eq!(setup.realized_body_position, again.realized_body_position);
            assert_eq!(setup.realized_body_yaw, again.realized_body_yaw);
            assert_eq!(setup.realized_grip_position, again.realized_grip_position);
            assert_eq!(setup.sampling_attempts, again.sampling_attempts);
            assert!((position.x - origin.x).abs() <= HELD_FOOD_POSITION_RADIUS);
            assert!((position.z - origin.z).abs() <= HELD_FOOD_POSITION_RADIUS);
            assert!(world.terrain().unwrap().walkable(position.x, position.z));
            assert_eq!(
                Some(position.y),
                world
                    .terrain()
                    .unwrap()
                    .surface()
                    .height(position.x, position.z)
            );
            assert!((grip.x - position.x - 0.5 * yaw.cos()).abs() < 0.0001);
            assert!((grip.z - position.z - 0.5 * yaw.sin()).abs() < 0.0001);
            assert_eq!(
                world.entity(scenario.food).unwrap().carried_by,
                Some(setup.organism)
            );
            poses.insert((position.x.to_bits(), position.z.to_bits(), yaw.to_bits()));
            facings.insert(yaw.to_bits());
        }
        assert_eq!(poses.len(), 3);
        assert_eq!(facings.len(), 3);
    }

    #[test]
    fn held_food_pose_sampling_rejects_blocked_attempts_and_is_bounded() {
        let seed = 42;
        let first_x = HELD_FOOD_POSITION_RADIUS * (2.0 * scenario_random(seed, 1003) - 1.0);
        let first_z = HELD_FOOD_POSITION_RADIUS * (2.0 * scenario_random(seed, 1004) - 1.0);
        let mut world = alife_world::HeadlessScenarioBuilder::new(seed)
            .agent("subject", alife_core::OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(10.0, 0.0, 0.0), 0.6)
            .obstacle("first-blocker", Vec3f::new(first_x, first_z, 0.0), 0.25)
            .build()
            .unwrap();
        ground_foundation_training_world(&mut world).unwrap();
        let body = world.entity_id("subject").unwrap();
        let food = world.entity_id("food").unwrap();
        let (_, _, attempts) = sample_foundation_held_food_pose(&world, body, food, seed).unwrap();
        assert!(attempts > 1 && attempts <= HELD_FOOD_POSE_ATTEMPTS);
        let mut blocked = alife_world::HeadlessScenarioBuilder::new(seed)
            .agent("subject", alife_core::OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(10.0, 0.0, 0.0), 0.6)
            .obstacle("block-all", Vec3f::ZERO, 100.0)
            .build()
            .unwrap();
        ground_foundation_training_world(&mut blocked).unwrap();
        let before = blocked.canonical_signature_digest().unwrap();
        let error = sample_foundation_held_food_pose(
            &blocked,
            blocked.entity_id("subject").unwrap(),
            blocked.entity_id("food").unwrap(),
            seed,
        )
        .unwrap_err();
        assert!(error.to_string().contains("exhausted 32"));
        assert_eq!(before, blocked.canonical_signature_digest().unwrap());
    }

    #[test]
    fn older_lesson_modes_preserve_fixed_founder_position_and_facing() {
        let mut initial = unconfigured(42);
        ground_foundation_training_world(&mut initial).unwrap();
        let body_id = initial.organism_entity_ids()[0].1;
        let expected = initial.entity(body_id).unwrap().clone();
        for lesson in [
            None,
            Some(FoundationTeacherLesson::Feeding),
            Some(FoundationTeacherLesson::VisionSearch),
            Some(FoundationTeacherLesson::HazardAvoidance),
            Some(FoundationTeacherLesson::ObstacleNavigation),
        ] {
            let mut world = initial.clone();
            let setup = configure_foundation_scenario(&mut world, 42, lesson, None, false).unwrap();
            let body = world.entity(body_id).unwrap();
            assert_eq!(body.position, expected.position);
            assert_eq!(body.body_yaw, expected.body_yaw);
            assert!(setup.held_food_setup.is_none());
        }
    }

    fn frame(world: &mut alife_world::HeadlessWorld) -> alife_core::PerceptionFrame {
        let subject = world.organism_entity_ids()[0].0;
        let homeostasis = world
            .organism_registry()
            .get(subject)
            .unwrap()
            .biochemistry()
            .homeostasis;
        world
            .perception_frame(
                subject,
                world.tick(),
                SensorProfile::GroundedTerrainVisionV1,
                homeostasis,
            )
            .unwrap()
    }

    fn eat_outcome(
        world: &mut alife_world::HeadlessWorld,
        target: alife_core::WorldEntityId,
    ) -> (alife_core::ActionCommand, PostActionOutcome) {
        let (subject, entity) = world.organism_entity_ids()[0];
        let command = alife_world::HeadlessWorldCommand::eat(subject, target).unwrap();
        let receipt = world
            .apply_registered_command(&command, entity, Tick(world.tick().raw() + 1))
            .unwrap();
        world.try_advance_tick().unwrap();
        let transition = alife_core::MeasuredPhysiologyTransition::new(
            receipt.biology_before,
            receipt.biology_after,
        )
        .unwrap();
        let outcome = PostActionOutcome::new(
            subject,
            ExperienceSequenceId(1),
            world.tick(),
            receipt.action_result.execution.succeeded,
            receipt.action_result.execution.physical,
            HomeostaticDelta::zero(),
            SignedValence::ZERO,
            NormalizedScalar::ZERO,
            NormalizedScalar::ZERO,
            SignedValence::ZERO,
            NormalizedScalar::ZERO,
        )
        .unwrap()
        .with_measured_physiology(transition)
        .unwrap();
        (command, outcome)
    }

    #[test]
    fn held_food_setup_is_legal_separate_and_ready_for_only_eat() {
        for seed in [42, 91, 132] {
            let (mut world, scenario) = prepared(seed);
            let setup = scenario.held_food_setup.as_ref().unwrap();
            assert_eq!(
                setup.grab_command.action_id,
                alife_world::HeadlessActionIds::GRAB
            );
            assert_eq!(setup.grab_physical.contact, PhysicalContactKind::Touch);
            assert_eq!(setup.setup_tick, scenario.start_tick);
            assert_eq!(
                world.entity(scenario.food).unwrap().carried_by,
                Some(setup.organism)
            );
            assert!(!world.entity(scenario.food).unwrap().consumed);
            assert!(setup.initial_hunger >= 0.12 && setup.initial_energy <= 0.9);
            assert!(setup.initial_health >= 0.95 && !setup.initial_sleeping);
            let frame = frame(&mut world);
            let mut navigation = GroundedNavigationTeacher::default();
            let teacher = grounded_lesson_teacher(
                &frame,
                31,
                FoundationTeacherLesson::EatHeldFood,
                &mut false,
                scenario.food,
                scenario.hazard,
                scenario.waypoint,
                &mut navigation,
            )
            .unwrap();
            let selected = &frame.candidates()[teacher.representative_index as usize];
            assert_eq!(selected.action_id, alife_world::HeadlessActionIds::EAT);
            assert_eq!(selected.target.entity, Some(scenario.food));
            let (eat, outcome) = eat_outcome(&mut world, scenario.food);
            assert!(world.entity(scenario.food).unwrap().consumed);
            assert!(held_food_action_consumed(
                &eat,
                None,
                &outcome,
                scenario.food,
                true
            ));
        }
    }

    fn assert_teacher_utterance(frame: &alife_core::PerceptionFrame, expected: &[u32]) {
        let heard: Vec<_> = frame
            .sensory()
            .language_context
            .heard_tokens
            .iter()
            .flatten()
            .filter(|word| word.source_kind == alife_core::UtteranceSourceKind::Teacher)
            .collect();
        assert_eq!(heard.len(), expected.len());
        if heard.is_empty() {
            return;
        }
        let utterance = heard[0].utterance_id;
        for (position, (word, token)) in heard.iter().zip(expected).enumerate() {
            assert_eq!(word.utterance_id, utterance);
            assert_eq!(usize::from(word.sequence_position), position);
            assert_eq!(word.token_id, *token);
        }
    }

    #[test]
    fn unheld_grab_food_cue_delivers_ordered_get_food_before_acquisition() {
        let mut world = unconfigured(42);
        let scenario = configure_foundation_scenario(
            &mut world,
            42,
            Some(FoundationTeacherLesson::GrabFood),
            None,
            false,
        )
        .unwrap();
        assert_eq!(scenario.teacher_cue_tokens, vec![9, 1]);
        assert!(scenario.held_food_setup.is_none());
        assert_eq!(world.entity(scenario.food).unwrap().carried_by, None);
        expose_foundation_teacher_cue(&mut world, &scenario).unwrap();
        assert_teacher_utterance(&frame(&mut world), &[9, 1]);
        assert_eq!(world.entity(scenario.food).unwrap().carried_by, None);
        assert!(!world.entity(scenario.food).unwrap().consumed);
    }

    fn assert_emitted_food_cue(
        world: &mut alife_world::HeadlessWorld,
        scenario: &FoundationScenarioSetup,
        expected: &[u32],
    ) {
        expose_foundation_teacher_cue(world, scenario).unwrap();
        let emitted: Vec<_> = world
            .audible_utterances()
            .into_iter()
            .filter(|utterance| utterance.source_kind == alife_core::UtteranceSourceKind::Teacher)
            .collect();
        assert_eq!(emitted.len(), usize::from(!expected.is_empty()));
        let perceived = frame(world);
        assert_teacher_utterance(&perceived, expected);
        if let Some(utterance) = emitted.first() {
            assert_eq!(
                utterance
                    .tokens
                    .iter()
                    .map(|token| u32::from(token.raw()))
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(utterance.addressee, Some(world.organism_entity_ids()[0].0));
            assert_eq!(utterance.emitted_tick, world.tick());
            assert_eq!(utterance.expires_after_tick.raw(), world.tick().raw() + 1);
            let food = world.entity(scenario.food).unwrap();
            assert_eq!(
                utterance.source_position,
                Vec3f::new(food.position.x, food.position.y, food.position.z + 2.0)
            );
            assert_eq!(
                perceived
                    .sensory()
                    .language_context
                    .heard_tokens
                    .iter()
                    .flatten()
                    .next()
                    .unwrap()
                    .utterance_id,
                utterance.utterance_id
            );
        }
    }

    #[test]
    fn grab_food_cue_tracks_legal_unheld_held_released_and_consumed_transitions() {
        let mut world = unconfigured(42);
        let scenario = configure_foundation_scenario(
            &mut world,
            42,
            Some(FoundationTeacherLesson::GrabFood),
            None,
            false,
        )
        .unwrap();
        let (subject, entity) = world.organism_entity_ids()[0];
        let grab = alife_core::ActionCommand::structured(
            subject,
            alife_world::HeadlessActionIds::GRAB,
            alife_core::ActionKind::Hold,
            alife_core::ActionTarget::new(Some(scenario.food), None),
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            Confidence::new(0.9).unwrap(),
            0,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(world.entity(scenario.food).unwrap().carried_by, None);
        assert_emitted_food_cue(&mut world, &scenario, &[9, 1]);
        let original = world.audible_utterances();
        let acquired = world
            .apply_registered_command(&grab, entity, Tick(world.tick().raw() + 1))
            .unwrap();
        world.try_advance_tick().unwrap();
        assert!(acquired.action_result.execution.succeeded);
        assert_eq!(
            acquired.action_result.execution.physical.contact,
            PhysicalContactKind::Touch
        );
        assert_eq!(
            world.entity(scenario.food).unwrap().carried_by,
            Some(subject)
        );
        // The previous phrase retains its ordinary lifetime; possession does
        // not retract already emitted speech or add an off-cadence cue.
        expose_foundation_teacher_cue(&mut world, &scenario).unwrap();
        assert_eq!(world.audible_utterances(), original);
        assert_teacher_utterance(&frame(&mut world), &[9, 1]);
        world.try_advance_tick().unwrap();
        assert_emitted_food_cue(&mut world, &scenario, &[]);
        while world.tick().raw() < scenario.start_tick + 24 {
            world.try_advance_tick().unwrap();
        }
        assert_emitted_food_cue(&mut world, &scenario, &[1]);
        let released = world
            .apply_registered_command(&grab, entity, Tick(world.tick().raw() + 1))
            .unwrap();
        world.try_advance_tick().unwrap();
        assert!(released.action_result.execution.succeeded);
        assert_eq!(world.entity(scenario.food).unwrap().carried_by, None);
        assert!(!world.entity(scenario.food).unwrap().consumed);
        while world.tick().raw() < scenario.start_tick + 48 {
            world.try_advance_tick().unwrap();
        }
        assert_emitted_food_cue(&mut world, &scenario, &[9, 1]);
        let reacquired = world
            .apply_registered_command(&grab, entity, Tick(world.tick().raw() + 1))
            .unwrap();
        world.try_advance_tick().unwrap();
        assert!(reacquired.action_result.execution.succeeded);
        assert_eq!(
            world.entity(scenario.food).unwrap().carried_by,
            Some(subject)
        );
        let (_, eaten) = eat_outcome(&mut world, scenario.food);
        assert!(eaten.success);
        assert_eq!(eaten.physical.contact, PhysicalContactKind::Consumed);
        assert!(world.entity(scenario.food).unwrap().consumed);
        while world.tick().raw() < scenario.start_tick + 72 {
            world.try_advance_tick().unwrap();
        }
        assert_emitted_food_cue(&mut world, &scenario, &[]);
    }

    #[test]
    fn grab_food_cue_retains_get_food_when_another_organism_holds_the_target() {
        let mut world = unconfigured(42);
        let scenario = configure_foundation_scenario(
            &mut world,
            42,
            Some(FoundationTeacherLesson::GrabFood),
            None,
            false,
        )
        .unwrap();
        let other = alife_core::OrganismId(2);
        let food_position = world.entity(scenario.food).unwrap().position;
        world
            .spawn_social_agent("cue-peer", other, food_position, 0.75)
            .unwrap();
        let grab = alife_core::ActionCommand::structured(
            other,
            alife_world::HeadlessActionIds::GRAB,
            alife_core::ActionKind::Hold,
            alife_core::ActionTarget::new(Some(scenario.food), None),
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            Confidence::new(0.9).unwrap(),
            0,
            None,
            None,
            None,
        )
        .unwrap();
        // This social actor is a physical fixture, without an organism record.
        // Use the ordinary world action path; no peer biology is being tested.
        let acquired = world.apply_command(&grab).unwrap();
        world.try_advance_tick().unwrap();
        assert!(acquired.execution.succeeded);
        assert_eq!(world.entity(scenario.food).unwrap().carried_by, Some(other));
        while world.tick().raw() < scenario.start_tick + 24 {
            world.try_advance_tick().unwrap();
        }
        // Existing semantics suppress Get only after this subject acquires
        // the target. Another holder does not satisfy its acquisition lesson.
        assert_emitted_food_cue(&mut world, &scenario, &[9, 1]);
        assert_eq!(world.entity(scenario.food).unwrap().carried_by, Some(other));
    }

    #[test]
    fn held_food_cues_use_hearing_without_positive_or_silence_speech_targets() {
        let (mut world, scenario) = prepared(42);
        assert_eq!(scenario.vocabulary_token, None);
        assert!(!scenario.training_language_feedback);
        assert_eq!(scenario.teacher_cue_tokens, vec![8, 1]);
        expose_foundation_teacher_cue(&mut world, &scenario).unwrap();
        let perceived = frame(&mut world);
        assert_teacher_utterance(&perceived, &[8, 1]);
        assert!(heard_foundation_teacher_cue(
            &perceived,
            &scenario.teacher_cue_tokens
        ));
        for production in [false, true] {
            assert!(grounded_speech_label(
                &perceived,
                scenario.vocabulary_token,
                None,
                None,
                None,
                production
            )
            .is_none());
        }
        assert_eq!(scenario.language_praise_count.get(), 0);
        assert!(perceived
            .sensory()
            .language_context
            .heard_tokens
            .iter()
            .flatten()
            .any(|word| word.token_id == 8
                && word.source_kind == alife_core::UtteranceSourceKind::Teacher));
    }

    #[test]
    fn terminal_held_food_distance_is_null_and_never_uses_remaining_teacher() {
        let (mut world, scenario) = prepared(42);
        let learner = scenario.held_food_setup.as_ref().unwrap().organism;
        expose_foundation_teacher_cue(&mut world, &scenario).unwrap();
        assert!(held_food_learner_distance(&world, scenario.food, learner).is_some());
        // A legal terminal fixture tests assessment without GPU retirement or
        // claiming that this fixture itself demonstrates a natural death.
        let mut terminal = world.organism_registry().get(learner).unwrap().clone();
        terminal.mark_dead(world.tick()).unwrap();
        terminal
            .link_birth_manifest(alife_core::Blake3Digest::from_bytes([41; 32]))
            .unwrap();
        terminal
            .link_life_manifest(alife_core::Blake3Digest::from_bytes([42; 32]))
            .unwrap();
        world.replace_organism_record_exact(terminal).unwrap();
        assert_eq!(
            held_food_learner_distance(&world, scenario.food, learner),
            None
        );
        world.retire_dead_organism(learner).unwrap();
        assert!(world.entity_id("nursery-teacher").is_some());
        assert!(!world.organism_entity_ids().is_empty());
        let distance = held_food_learner_distance(&world, scenario.food, learner);
        assert_eq!(distance, None);
        assert!(
            serde_json::json!({"final_food_distance": distance})["final_food_distance"].is_null()
        );
    }

    #[test]
    fn released_food_gets_naming_only_and_cannot_pass_held_eating_score() {
        let (mut world, scenario) = prepared(42);
        let (subject, entity) = world.organism_entity_ids()[0];
        let drop = alife_core::ActionCommand::structured(
            subject,
            alife_world::HeadlessActionIds::GRAB,
            alife_core::ActionKind::Hold,
            alife_core::ActionTarget::new(Some(scenario.food), None),
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            Confidence::new(0.9).unwrap(),
            0,
            None,
            None,
            None,
        )
        .unwrap();
        world
            .apply_registered_command(&drop, entity, Tick(world.tick().raw() + 1))
            .unwrap();
        world.try_advance_tick().unwrap();
        assert_eq!(world.entity(scenario.food).unwrap().carried_by, None);
        assert!(!world.entity(scenario.food).unwrap().consumed);
        let mut released = scenario.clone();
        released.start_tick = world.tick().raw();
        expose_foundation_teacher_cue(&mut world, &released).unwrap();
        let perceived = frame(&mut world);
        assert_teacher_utterance(&perceived, &[1]);
        // The existing frame counter includes noun-only exposure; a positive
        // count does not establish that the full [8, 1] command was heard.
        assert!(heard_foundation_teacher_cue(
            &perceived,
            &released.teacher_cue_tokens
        ));
        let (eat, outcome) = eat_outcome(&mut world, scenario.food);
        assert!(!outcome.success);
        assert_eq!(outcome.physical.contact, PhysicalContactKind::Touch);
        assert_eq!(outcome.physical.target_entity, Some(scenario.food));
        assert_eq!(world.entity(scenario.food).unwrap().carried_by, None);
        assert!(!world.entity(scenario.food).unwrap().consumed);
        assert!(!held_food_action_consumed(
            &eat,
            None,
            &outcome,
            scenario.food,
            false
        ));
        assert!(!held_food_action_consumed(
            &eat,
            None,
            &outcome,
            scenario.food,
            true
        ));
    }

    #[test]
    fn eating_score_requires_selected_exact_target_actual_consumption_and_biology() {
        let (mut world, scenario) = prepared(42);
        let (eat, outcome) = eat_outcome(&mut world, scenario.food);
        let other = world.entity_id("hazard-01").unwrap();
        assert!(!held_food_action_consumed(
            &eat, None, &outcome, other, true
        ));
        let mut wrong = eat;
        wrong.target_entity = Some(other);
        assert!(!held_food_action_consumed(
            &wrong,
            None,
            &outcome,
            scenario.food,
            true
        ));
        let mut no_biology = outcome.clone();
        no_biology.measured_physiology = None;
        assert!(!held_food_action_consumed(
            &eat,
            None,
            &no_biology,
            scenario.food,
            true
        ));
        let mut attempt = outcome.clone();
        attempt.physical.contact = PhysicalContactKind::Touch;
        assert!(!held_food_action_consumed(
            &eat,
            None,
            &attempt,
            scenario.food,
            true
        ));
        let mut no_benefit = outcome.clone();
        let transition = no_benefit.measured_physiology.as_mut().unwrap();
        transition.after.body.energy = transition.before.body.energy;
        transition.after.homeostasis.drives.hunger = transition.before.homeostasis.drives.hunger;
        assert!(!held_food_action_consumed(
            &eat,
            None,
            &no_benefit,
            scenario.food,
            true
        ));
    }

    #[test]
    fn actual_unreachable_and_nonfood_eat_attempts_do_not_complete_lesson() {
        let (mut world, scenario) = prepared(42);
        let (remote_eat, remote_outcome) = eat_outcome(&mut world, scenario.hazard);
        assert!(!remote_outcome.success);
        assert_ne!(
            remote_outcome.physical.contact,
            PhysicalContactKind::Consumed
        );
        assert!(!held_food_action_consumed(
            &remote_eat,
            None,
            &remote_outcome,
            scenario.food,
            true
        ));
        let (_, subject) = world.organism_entity_ids()[0];
        let position = world.entity(subject).unwrap().position;
        move_scenario_object(
            &mut world,
            scenario.hazard,
            Vec3f::new(position.x + 0.5, position.y, position.z),
        )
        .unwrap();
        let (nonfood_eat, nonfood_outcome) = eat_outcome(&mut world, scenario.hazard);
        assert!(!nonfood_outcome.success);
        assert_ne!(
            nonfood_outcome.physical.contact,
            PhysicalContactKind::Consumed
        );
        assert!(!held_food_action_consumed(
            &nonfood_eat,
            None,
            &nonfood_outcome,
            scenario.food,
            true
        ));
        assert!(!world.entity(scenario.food).unwrap().consumed);
    }

    #[test]
    fn eating_score_retains_exact_consumption_when_other_bundle_channel_is_blocked() {
        let (mut world, scenario) = prepared(42);
        let (eat, mut outcome) = eat_outcome(&mut world, scenario.food);
        let command = ChannelCommand::new(
            MotorChannel::Manipulation,
            alife_world::HeadlessActionIds::EAT,
            Some(alife_core::ActionTarget::new(Some(scenario.food), None)),
            Vec3f::ZERO,
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            0.0,
            Confidence::new(1.0).unwrap(),
            0,
        )
        .unwrap();
        let bundle = MotorCommandBundle::new(
            eat.organism_id,
            ExperienceSequenceId(1),
            Tick(world.tick().raw() - 1),
            vec![
                command,
                ChannelCommand::new(
                    MotorChannel::Locomotion,
                    alife_world::HeadlessActionIds::STEP_FORWARD,
                    None,
                    Vec3f::new(1.0, 0.0, 0.0),
                    Intensity::new(1.0).unwrap(),
                    DurationTicks::new(1),
                    0.0,
                    Confidence::new(1.0).unwrap(),
                    0,
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let consumption = outcome.physical;
        outcome.physical.contact = PhysicalContactKind::Blocked;
        outcome.physical.target_entity = None;
        outcome.success = false;
        outcome.joint = Some(
            JointPhysicalOutcome::new(outcome.physical, Vec::new())
                .unwrap()
                .with_channel_outcomes(vec![
                    ChannelPhysicalOutcome::new(MotorChannel::Manipulation, consumption).unwrap(),
                    ChannelPhysicalOutcome::new(MotorChannel::Locomotion, outcome.physical)
                        .unwrap(),
                ])
                .unwrap(),
        );
        assert!(held_food_action_consumed(
            &eat,
            Some(&bundle),
            &outcome,
            scenario.food,
            true
        ));
        outcome.joint.as_mut().unwrap().channel_outcomes[0]
            .physical
            .target_entity = world.entity_id("hazard-01");
        assert!(!held_food_action_consumed(
            &eat,
            Some(&bundle),
            &outcome,
            scenario.food,
            true
        ));
    }
}

#[cfg(test)]
mod compact_replay_codec_tests {
    use super::*;

    fn candidates(biases: &[f32]) -> Vec<alife_training::TrainingReplayCandidate> {
        biases
            .iter()
            .enumerate()
            .map(
                |(index, innate_bias)| alife_training::TrainingReplayCandidate {
                    family: alife_core::CandidateActionFamily::Idle,
                    decoder_inputs: std::array::from_fn(|lane| (index * 54 + lane) as f32 / 64.0),
                    innate_bias: *innate_bias,
                },
            )
            .collect()
    }

    #[test]
    fn compact_replay_codec_reproduces_legacy_zero_bias_field_omission() {
        #[derive(serde::Serialize)]
        struct LegacyCandidate {
            family: alife_core::CandidateActionFamily,
            decoder_inputs: Vec<f32>,
            #[serde(skip_serializing_if = "is_zero")]
            innate_bias: f32,
        }
        fn is_zero(value: &f32) -> bool {
            *value == 0.0
        }
        let candidate = candidates(&[0.0]).remove(0);
        let legacy = LegacyCandidate {
            family: candidate.family,
            decoder_inputs: candidate.decoder_inputs.to_vec(),
            innate_bias: candidate.innate_bias,
        };
        let json = serde_json::to_vec(&legacy).unwrap();
        let json_decoded: alife_training::TrainingReplayCandidate =
            serde_json::from_slice(&json).unwrap();
        assert_eq!(json_decoded, candidate);
        let bytes = bincode::serde::encode_to_vec(legacy, bincode::config::standard()).unwrap();
        assert!(
            bincode::serde::decode_from_slice::<alife_training::TrainingReplayCandidate, _>(
                &bytes,
                bincode::config::standard(),
            )
            .is_err()
        );
    }

    #[test]
    fn compact_replay_codec_preserves_mixed_candidate_bias_bits_and_following_fields() {
        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        struct Probe {
            #[serde(deserialize_with = "deserialize_replay_schema")]
            schema: u32,
            sequence: TrainingSequence,
            following: u64,
        }
        for biases in [&[0.0][..], &[0.375][..], &[0.0, 0.375, -0.0, -0.625][..]] {
            let probe = Probe {
                schema: FOUNDATION_REPLAY_SCHEMA,
                sequence: TrainingSequence {
                    phenotype_hash: alife_core::PhenotypeHash([17; 4]),
                    initial: alife_training::TrainingInitialState {
                        activations: vec![0.25, -0.0],
                        activity_ema: vec![0.5, 0.0],
                        metabolic_load: vec![0.75, 1.0],
                        dendrites: Default::default(),
                    },
                    ticks: vec![alife_training::TrainingReplayTick {
                        encoded_inputs: vec![1.0, -0.0],
                        projection_gain: 0.5,
                        local_threshold_shift: -0.25,
                        microstep_count: 1,
                        enabled_routes: vec![true, false],
                        effective_weight_offsets: vec![0.125, -0.0],
                        structural_synapses: Vec::new(),
                        candidates: candidates(biases),
                    }],
                    burn_in_ticks: 0,
                    memory_candidate_gain: 0.5,
                },
                following: 0xDEAD_BEEF_1234_5678,
            };
            let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), 3).unwrap();
            bincode::serde::encode_into_std_write(
                &probe,
                &mut encoder,
                bincode::config::standard(),
            )
            .unwrap();
            let bytes = encoder.finish().unwrap();
            let mut decoder = zstd::stream::read::Decoder::new(bytes.as_slice()).unwrap();
            let decoded: Probe =
                bincode::serde::decode_from_std_read(&mut decoder, bincode::config::standard())
                    .unwrap();
            assert_eq!(decoded, probe);
            assert_eq!(
                bincode::serde::encode_to_vec(&decoded, bincode::config::standard()).unwrap(),
                bincode::serde::encode_to_vec(&probe, bincode::config::standard()).unwrap(),
                "every numeric bit and following field must survive compact replay"
            );
            for (expected, actual) in probe.sequence.ticks[0]
                .candidates
                .iter()
                .zip(&decoded.sequence.ticks[0].candidates)
            {
                assert_eq!(actual.innate_bias.to_bits(), expected.innate_bias.to_bits());
                assert_eq!(
                    actual.decoder_inputs.map(f32::to_bits),
                    expected.decoder_inputs.map(f32::to_bits)
                );
            }
            assert_eq!(
                decoded
                    .sequence
                    .initial
                    .activations
                    .iter()
                    .map(|v| v.to_bits())
                    .collect::<Vec<_>>(),
                probe
                    .sequence
                    .initial
                    .activations
                    .iter()
                    .map(|v| v.to_bits())
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn compact_replay_codec_rejects_old_schema_before_decoding_the_body() {
        // A header alone proves rejection precedes the missing/corrupt body.
        let bytes = bincode::serde::encode_to_vec(1u32, bincode::config::standard()).unwrap();
        let error = bincode::serde::decode_from_slice::<FoundationReplayRecord, _>(
            &bytes,
            bincode::config::standard(),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("unsupported compact replay schema 1"));
    }
}
