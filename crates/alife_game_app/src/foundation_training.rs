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
    AdamWConfig, FoundationTrainer, StageTrainableMask, TrainingInitialState,
    TrainingReplayCandidate, TrainingReplayTick, TrainingSequence,
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
    match snapshot.active_weight_bank {
        0 => Ok((
            ranges.lifetime_weight_words.clone(),
            ranges.fast_weight_words.clone(),
        )),
        1 => Ok((
            ranges.lifetime_weight_bank_1_words.clone(),
            ranges.fast_weight_bank_1_words.clone(),
        )),
        _ => Err(invalid().into()),
    }
}

fn validate_snapshot_topology(snapshot: &GpuTrainingStateSnapshot) -> Result<()> {
    let phenotype = &snapshot.phenotype;
    // Sparse spans are structural additions on top of the compiled graph. An
    // unchanged overlay is still a different graph and cannot be replayed here.
    if !snapshot.v11.sparse_spans.is_empty() || snapshot.v11.pending_lifetime_synapse.is_some() {
        return Err(
            "replay must split/reject structural synapses absent from the compiled phenotype"
                .into(),
        );
    }
    let record = snapshot.brain_slot.record();
    let counts = snapshot.brain_slot.typed_counts();
    let neurons = phenotype.neuron_count();
    let recurrent = phenotype
        .synapses()
        .iter()
        .filter(|s| s.kind() == CompiledSynapseKind::Recurrent)
        .count();
    if snapshot.handle.phenotype_hash() != phenotype.phenotype_hash()
        || snapshot.handle.class_id() != phenotype.brain_class_id()
        || record.class_id != u32::from(phenotype.brain_class_id().raw())
        || record.slot != snapshot.handle.slot()
        || record.slot_generation != snapshot.handle.generation()
        || record.neuron_count != neurons
        || record.microstep_count != u32::from(phenotype.microstep_count())
        || record.synapse_count as usize != phenotype.synapses().len()
        || record.recurrent_synapse_count as usize != recurrent
        || counts.source_indices != recurrent
        || counts.route_indices != recurrent
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
    let phenotype = &first.before.phenotype;
    let upload = alife_gpu_backend::closed_loop_buffers::GpuPhenotypeUpload::try_from(phenotype)?;
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
        if lifetime.len() != phenotype.synapses().len() || fast.len() != lifetime.len() {
            return Err(invalid().into());
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
                    lifetime[local] + s.alpha() * fast[local]
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

#[derive(Debug, serde::Serialize)]
pub struct FoundationPilotReceipt {
    pub seed: u64,
    pub ticks: usize,
    pub collection_seconds: f64,
    pub replay_seconds: f64,
    pub maximum_logit_error: f32,
    pub maximum_activation_error: f32,
    pub maximum_activity_ema_error: f32,
    pub maximum_metabolic_error: f32,
    pub source_asset_digest: String,
}

/// Prerequisite fidelity check, not behavioral acceptance or a training campaign.
pub fn run_foundation_training_pilot(
    output: &Path,
    seed: u64,
    tick_count: usize,
) -> Result<FoundationPilotReceipt> {
    if !(1..=512).contains(&tick_count) || seed == 0 {
        return Err(invalid().into());
    }
    std::fs::create_dir(output)?; // A fresh run never overwrites an earlier receipt.
    let asset = initial_n2048_care_asset(seed)?;
    std::fs::write(
        output.join("initial.alife-foundation"),
        asset.encode_canonical()?,
    )?;
    let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    let mut game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)?;
    game.world.set_age_death_disabled_for_new_game(true)?;
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
        steps.append(&mut collected);
    }
    let collection_seconds = started.elapsed().as_secs_f64();
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
        ticks: tick_count,
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
    };
    std::fs::write(
        output.join("pilot.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    Ok(receipt)
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
