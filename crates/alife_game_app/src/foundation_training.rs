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

fn invalid() -> ScaffoldContractError {
    ScaffoldContractError::InvalidDecisionEvidence
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
        &first.before.phenotype,
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
        SensorProfile::GroundedObjectSlotsV1,
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
    pub final_food_distance: Option<f32>,
    pub lesson: Option<FoundationTeacherLesson>,
    pub lesson_completed: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationTeacherLesson {
    Feeding,
    HazardAvoidance,
    ObstacleNavigation,
    Recovery,
}

/// Prerequisite fidelity check, not behavioral acceptance or a training campaign.
pub fn run_foundation_training_pilot(
    output: &Path,
    seed: u64,
    tick_count: usize,
) -> Result<FoundationPilotReceipt> {
    run_foundation_training_pilot_inner(output, seed, seed, tick_count, false, None, None)
}

/// Diagnostic of the existing legal teacher path against actual ingestion.
pub fn run_foundation_teacher_pilot(
    output: &Path,
    seed: u64,
    tick_count: usize,
) -> Result<FoundationPilotReceipt> {
    run_foundation_training_pilot_inner(output, seed, seed, tick_count, true, None, None)
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
    )
}

fn grounded_care_teacher(
    frame: &alife_core::PerceptionFrame,
    enabled_channels: u32,
) -> std::result::Result<alife_gpu_backend::GpuTrainingDemonstratorAction, ScaffoldContractError> {
    let contact = |candidate: &alife_core::ActionCandidate| match candidate.observation {
        alife_core::CandidateObservationRef::ObjectSlot(index) => frame
            .grounded_object_slots()
            .iter()
            .find(|slot| slot.slot_index == index)
            .is_some_and(|slot| slot.contact >= 0.5),
        alife_core::CandidateObservationRef::None => false,
    };
    let nearest_slot = frame
        .grounded_object_slots()
        .iter()
        .min_by(|left, right| left.distance.total_cmp(&right.distance))
        .map(|slot| slot.slot_index);
    let ingest_target = frame
        .candidates()
        .iter()
        .find(|candidate| {
            candidate.family == alife_core::CandidateActionFamily::Ingest
                && candidate.observation
                    == nearest_slot.map_or(
                        alife_core::CandidateObservationRef::None,
                        alife_core::CandidateObservationRef::ObjectSlot,
                    )
        })
        .and_then(|candidate| candidate.target.entity);
    let chosen = frame
        .candidates()
        .iter()
        .find(|candidate| {
            candidate.family == alife_core::CandidateActionFamily::Ingest
                && candidate.target.entity == ingest_target
                && contact(candidate)
        })
        .or_else(|| {
            frame.candidates().iter().find(|candidate| {
                candidate.family == alife_core::CandidateActionFamily::Approach
                    && ingest_target.is_some()
                    && candidate.target.entity == ingest_target
            })
        })
        .or_else(|| {
            [
                alife_core::CandidateActionFamily::Contact,
                alife_core::CandidateActionFamily::Inspect,
                alife_core::CandidateActionFamily::Rest,
                alife_core::CandidateActionFamily::Idle,
            ]
            .into_iter()
            .find_map(|family| {
                frame
                    .candidates()
                    .iter()
                    .find(|candidate| candidate.family == family)
            })
        })
        .ok_or(ScaffoldContractError::InvalidActionDecision)?;
    assemble_grounded_teacher(frame, enabled_channels, chosen)
}

fn grounded_lesson_teacher(
    frame: &alife_core::PerceptionFrame,
    enabled_channels: u32,
    lesson: FoundationTeacherLesson,
    lesson_tick: usize,
    food: alife_core::WorldEntityId,
    hazard: alife_core::WorldEntityId,
    waypoint: alife_core::WorldEntityId,
) -> std::result::Result<alife_gpu_backend::GpuTrainingDemonstratorAction, ScaffoldContractError> {
    use alife_core::CandidateActionFamily as Family;
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
    let chosen = match lesson {
        FoundationTeacherLesson::Feeding => target(
            food,
            if food_contact {
                Family::Ingest
            } else {
                Family::Approach
            },
        ),
        FoundationTeacherLesson::HazardAvoidance if lesson_tick < 12 => {
            target(hazard, Family::Avoid)
        }
        FoundationTeacherLesson::ObstacleNavigation if lesson_tick < 18 => {
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
        FoundationTeacherLesson::Recovery if lesson_tick < 12 => target(food, Family::Approach),
        FoundationTeacherLesson::HazardAvoidance | FoundationTeacherLesson::Recovery => frame
            .candidates()
            .iter()
            .find(|candidate| candidate.family == Family::Rest),
    }
    .or_else(|| {
        frame
            .candidates()
            .iter()
            .find(|candidate| candidate.family == Family::Rest)
    })
    .ok_or(ScaffoldContractError::InvalidActionDecision)?;
    assemble_grounded_teacher(frame, enabled_channels, chosen)
}

fn assemble_grounded_teacher(
    frame: &alife_core::PerceptionFrame,
    enabled_channels: u32,
    chosen: &alife_core::ActionCandidate,
) -> std::result::Result<alife_gpu_backend::GpuTrainingDemonstratorAction, ScaffoldContractError> {
    let forced = match chosen.kind {
        alife_core::ActionKind::Move => 0,
        alife_core::ActionKind::Interact | alife_core::ActionKind::Write => 2,
        alife_core::ActionKind::Vocalize => 3,
        _ => 4,
    };
    let mut motor_indices = [u16::MAX; 6];
    for candidate in frame.candidates() {
        let slot = match candidate.kind {
            alife_core::ActionKind::Move => Some(0),
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

fn run_foundation_training_pilot_inner(
    output: &Path,
    seed: u64,
    founder_seed_base: u64,
    tick_count: usize,
    teacher_mode: bool,
    food_position: Option<[f32; 2]>,
    lesson: Option<FoundationTeacherLesson>,
) -> Result<FoundationPilotReceipt> {
    if !(1..=512).contains(&tick_count) || seed == 0 || founder_seed_base == 0 {
        return Err(invalid().into());
    }
    std::fs::create_dir(output)?; // A fresh run never overwrites an earlier receipt.
    let asset = initial_n2048_care_asset(founder_seed_base)?;
    std::fs::write(
        output.join("initial.alife-foundation"),
        asset.encode_canonical()?,
    )?;
    let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = founder_seed_base;
    let mut game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)?;
    game.world.set_age_death_disabled_for_new_game(true)?;
    let teacher_lesson = teacher_mode.then_some(lesson.unwrap_or(FoundationTeacherLesson::Feeding));
    if food_position.is_some() && teacher_lesson != Some(FoundationTeacherLesson::Feeding) {
        return Err("custom food placement is only supported for feeding lessons".into());
    }
    let food = game
        .world
        .entity_id("food-01")
        .ok_or("teacher world has no food")?;
    let hazard = game
        .world
        .entity_id("hazard-01")
        .ok_or("teacher world has no hazard")?;
    let waypoint = game
        .world
        .entity_id("obstacle-02")
        .ok_or("teacher world has no second obstacle")?;
    if let Some([x, z]) = food_position {
        move_scenario_object(&mut game.world, food, alife_core::Vec3f::new(x, 0.0, z))?;
    }
    match teacher_lesson {
        Some(FoundationTeacherLesson::HazardAvoidance) => {
            move_scenario_object(
                &mut game.world,
                food,
                alife_core::Vec3f::new(50.0, 0.0, 50.0),
            )?;
            move_scenario_object(
                &mut game.world,
                hazard,
                alife_core::Vec3f::new(5.0, 0.0, 0.0),
            )?;
        }
        Some(FoundationTeacherLesson::ObstacleNavigation) => {
            let blocker = game
                .world
                .entity_id("obstacle-01")
                .ok_or("teacher world has no first obstacle")?;
            move_scenario_object(&mut game.world, food, alife_core::Vec3f::new(7.0, 0.0, 0.0))?;
            move_scenario_object(
                &mut game.world,
                blocker,
                alife_core::Vec3f::new(4.5, 0.0, 0.0),
            )?;
            move_scenario_object(
                &mut game.world,
                waypoint,
                alife_core::Vec3f::new(3.0, 0.0, 7.0),
            )?;
        }
        Some(FoundationTeacherLesson::Recovery) => {
            move_scenario_object(&mut game.world, food, alife_core::Vec3f::new(9.0, 0.0, 0.0))?;
        }
        Some(FoundationTeacherLesson::Feeding) | None => {}
    }
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    let mut runtime = GpuLiveBrainRuntime::new_profiled_foundation_training(
        backend,
        game.world,
        seed,
        BrainScaleTier::Standard2048,
        SensorProfile::GroundedObjectSlotsV1,
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
    if teacher_mode {
        if let Some(lesson) = lesson {
            let mut lesson_tick = 0;
            runtime.set_foundation_demonstrator(move |frame, channels| {
                let result = grounded_lesson_teacher(
                    frame,
                    channels,
                    lesson,
                    lesson_tick,
                    food,
                    hazard,
                    waypoint,
                );
                lesson_tick += 1;
                result
            })?;
        } else {
            runtime.set_foundation_demonstrator(grounded_care_teacher)?;
        }
    }
    let initial_food_distance = teacher_mode
        .then(|| teacher_food_distance(&runtime))
        .transpose()?;
    let started = Instant::now();
    let mut steps = Vec::with_capacity(tick_count);
    for tick in 0..tick_count {
        runtime
            .tick()
            .map_err(|error| format!("pilot production tick {tick}: {error}"))?;
        let mut collected = runtime.take_foundation_training_steps();
        if collected.len() != 1 {
            return Err(format!(
                "pilot tick {tick}: expected one waking capture, got {}",
                collected.len()
            )
            .into());
        }
        let completed_teacher_lesson = teacher_mode
            && teacher_lesson.is_some_and(|lesson| {
                matches!(
                    lesson,
                    FoundationTeacherLesson::Feeding | FoundationTeacherLesson::ObstacleNavigation
                ) && teacher_step_consumed(&collected[0])
            });
        steps.append(&mut collected);
        if completed_teacher_lesson {
            break;
        }
    }
    let collection_seconds = started.elapsed().as_secs_f64();
    let final_food_distance = teacher_mode
        .then(|| teacher_food_distance(&runtime))
        .transpose()?;
    let consumed_events = steps
        .iter()
        .filter(|step| teacher_step_consumed(step))
        .count() as u64;
    let lesson_completed = teacher_lesson.map(|lesson| match lesson {
        FoundationTeacherLesson::Feeding | FoundationTeacherLesson::ObstacleNavigation => {
            consumed_events > 0
        }
        FoundationTeacherLesson::HazardAvoidance => {
            let initial = 2.0_f32;
            let world = runtime.world();
            let subject = world.organism_entity_ids()[0].1;
            let from = world.entity(subject).map(|object| object.position);
            let to = world.entity(hazard).map(|object| object.position);
            from.zip(to).is_some_and(|(from, to)| {
                ((from.x - to.x).powi(2) + (from.z - to.z).powi(2)).sqrt() > initial + 0.5
            })
        }
        FoundationTeacherLesson::Recovery => {
            let exerted = &steps[..steps.len().min(12)];
            let rested = steps.get(12..).unwrap_or_default();
            let mean_cost = |period: &[crate::FoundationTrainingStep]| {
                let costs = period
                    .iter()
                    .map(|step| {
                        step.patch.outcome().measured_physiology.map(|transition| {
                            transition.before.body.energy - transition.after.body.energy
                        })
                    })
                    .collect::<Option<Vec<_>>>()?;
                (!costs.is_empty()).then(|| costs.iter().sum::<f32>() / costs.len() as f32)
            };
            exerted.iter().any(|step| {
                step.patch.outcome().physical.contact == alife_core::PhysicalContactKind::Moved
            }) && rested.len() >= 12
                && rested.iter().all(|step| {
                    step.frame
                        .candidates()
                        .get(step.behavior.representative_index as usize)
                        .is_some_and(|candidate| {
                            candidate.family == alife_core::CandidateActionFamily::Rest
                        })
                })
                && mean_cost(exerted)
                    .zip(mean_cost(rested))
                    .is_some_and(|(active_cost, rest_cost)| rest_cost + 1e-6 < active_cost)
                && exerted
                    .last()
                    .and_then(|step| step.patch.outcome().measured_physiology)
                    .zip(
                        rested
                            .last()
                            .and_then(|step| step.patch.outcome().measured_physiology),
                    )
                    .is_some_and(|(active, rest)| {
                        rest.after.homeostasis.drives.fatigue
                            <= active.after.homeostasis.drives.fatigue
                    })
        }
    });
    if lesson_completed == Some(false) {
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
                    "position": step.frame.body().pose.translation,
                    "chosen_action": chosen.map(|candidate| candidate.action_id.raw()),
                    "chosen_target": chosen.and_then(|candidate| candidate.target.entity),
                    "chosen_family": chosen.map(|candidate| candidate.family),
                    "motor_indices": step.behavior.motor_indices,
                    "physical": outcome.physical,
                    "channel_outcomes": outcome.joint.as_ref().map(|joint| &joint.channel_outcomes),
                    "physiology": outcome.measured_physiology.map(|transition| serde_json::json!({
                        "before_energy": transition.before.body.energy,
                        "after_energy": transition.after.body.energy,
                        "before_brain_atp": transition.before.homeostasis.drives.brain_atp,
                        "after_brain_atp": transition.after.homeostasis.drives.brain_atp,
                        "before_fatigue": transition.before.homeostasis.drives.fatigue,
                        "after_fatigue": transition.after.homeostasis.drives.fatigue,
                    })),
                })
            })
            .collect::<Vec<_>>();
        std::fs::write(
            output.join("lesson-diagnostic.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "lesson": teacher_lesson,
                "consumed_events": consumed_events,
                "initial_food_distance": initial_food_distance,
                "final_food_distance": final_food_distance,
                "steps": trace,
            }))?,
        )?;
        return Err("teacher lesson did not complete its measured world outcome".into());
    }
    let sequence = foundation_replay_sequence(&steps, 0)
        .map_err(|error| format!("pilot replay conversion: {error}"))?;
    let phenotype = steps[0].before.phenotype.clone();
    let ids: Vec<u32> = phenotype.synapses().iter().enumerate().filter_map(|(i, s)| {
        (!matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::SpeechPayload))
            .then_some(i as u32)
    }).collect();
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
            source,
            steps[0].before.phenotype.clone(),
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
    }
    let started = Instant::now();
    let replay = trainer
        .evaluate_replay(&sequence)
        .map_err(|error| format!("pilot trainer replay: {error}"))?;
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
        lesson: teacher_lesson,
        lesson_completed,
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

/// One exact parity gate at a new waking segment, including structural growth.
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
const FOUNDATION_REPLAY_SCHEMA: u32 = 1;

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
    let revision = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
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
        if step.before.phenotype != self.phenotype
            || step.after_inference.phenotype != self.phenotype
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
