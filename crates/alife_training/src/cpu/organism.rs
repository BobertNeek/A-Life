//! Per-organism sparse recurrent and waking plasticity state. The collector
//! supplies canonical encoded perception/receptor evidence; there is no body or
//! chemistry simulator here and no synthetic-biology training entry point.
use super::{finite, invalid, CpuFoundationTrainer};
use crate::{
    PpoJointAction, TrainingError, TrainingInitialState, TrainingReplayTick, TrainingSequence,
};
use alife_core::{
    ActionId, BrainPhenotype, CandidateFeatureDigest, CompiledSynapseKind, DecoderHeadKind,
    DevelopmentState, ExperiencePatch, ExperienceSequenceId, JointActionSelectionV1,
    NeuralReceptorEffects, NeuralReceptorFrame, OrganismId, OutcomeCreditPacket,
    PerceptionFrameDigest, PhenotypeHash, PlasticityGenomeParameters, Validate,
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CpuOrganismCheckpoint {
    pub schema_version: u32,
    pub organism_id: OrganismId,
    pub phenotype_hash: PhenotypeHash,
    pub neural: TrainingInitialState,
    /// Opposite ping-pong bank, used by native eligibility even at zero steps.
    pub inactive_activations: Vec<f32>,
    pub lifetime: Vec<f32>,
    pub fast: Vec<f32>,
    pub eligibility: Vec<f32>,
    pub last_decision_tick: Option<u64>,
    pub last_outcome_tick: Option<u64>,
    pub weight_generation: u64,
    pub dispatch_generation: u64,
    pub active_activation_side: u8,
}
/// Identity of the actual selected decision, bound before world execution.
#[derive(Debug, Clone)]
pub struct CpuDecisionBinding {
    pub sequence_id: ExperienceSequenceId,
    pub frame_digest: PerceptionFrameDigest,
    pub action_id: ActionId,
    pub candidate_feature_digest: CandidateFeatureDigest,
    pub dispatch_generation: u64,
    pub active_activation_side: u8,
    pub joint_selection: JointActionSelectionV1,
}
#[derive(Clone)]
struct Pending {
    tick: u64,
    context: TrainingReplayTick,
    pre: Vec<f32>,
    staged: Option<Vec<f32>>,
    representative: Option<u16>,
    binding: Option<CpuDecisionBinding>,
}
/// Private somatic weights and recurrent state. Each collector row owns one
/// instance; the founder optimizer is passed read-only during decisions.
#[derive(Clone)]
pub struct CpuOrganismState {
    checkpoint: CpuOrganismCheckpoint,
    pending: Option<Pending>,
}
pub struct CpuDecisionOutput {
    pub candidate_logits: Vec<f32>,
    pub initial: TrainingInitialState,
    pub replay_tick: TrainingReplayTick,
    pub dispatch_generation: u64,
    pub active_activation_side: u8,
}
impl CpuOrganismState {
    pub fn new(
        p: &BrainPhenotype,
        id: OrganismId,
        initial: TrainingInitialState,
    ) -> Result<Self, TrainingError> {
        let count = p.synapses().len();
        Self::from_checkpoint(
            p,
            CpuOrganismCheckpoint {
                schema_version: 2,
                organism_id: id,
                phenotype_hash: p.phenotype_hash(),
                inactive_activations: vec![0.; p.neuron_count() as usize],
                neural: initial,
                lifetime: vec![0.; count],
                fast: vec![0.; count],
                eligibility: vec![0.; count],
                last_decision_tick: None,
                last_outcome_tick: None,
                weight_generation: 0,
                dispatch_generation: 0,
                active_activation_side: 0,
            },
        )
    }
    pub fn from_checkpoint(
        p: &BrainPhenotype,
        checkpoint: CpuOrganismCheckpoint,
    ) -> Result<Self, TrainingError> {
        checkpoint.organism_id.validate()?;
        let n = p.neuron_count() as usize;
        let count = p.synapses().len();
        if checkpoint.schema_version != 2
            || checkpoint.phenotype_hash != p.phenotype_hash()
            || checkpoint.active_activation_side > 1
            || checkpoint.neural.activations.len() != n
            || checkpoint.inactive_activations.len() != n
            || checkpoint.neural.activity_ema.len() != n
            || checkpoint.neural.metabolic_load.len() != n
            || checkpoint.lifetime.len() != count
            || checkpoint.fast.len() != count
            || checkpoint.eligibility.len() != count
            || checkpoint
                .neural
                .activity_ema
                .iter()
                .chain(&checkpoint.neural.metabolic_load)
                .any(|v| !(0.0..=1.0).contains(v))
            || checkpoint.last_outcome_tick.is_some() != checkpoint.last_decision_tick.is_some()
            || (checkpoint.last_decision_tick.is_none()
                && checkpoint.eligibility.iter().any(|e| *e != 0.))
            || checkpoint
                .last_outcome_tick
                .zip(checkpoint.last_decision_tick)
                .is_some_and(|(o, d)| o < d)
        {
            return Err(invalid());
        }
        for xs in [
            &checkpoint.neural.activations,
            &checkpoint.inactive_activations,
            &checkpoint.neural.activity_ema,
            &checkpoint.neural.metabolic_load,
            &checkpoint.lifetime,
            &checkpoint.fast,
            &checkpoint.eligibility,
        ] {
            finite(xs)?;
        }
        checkpoint
            .neural
            .dendrites
            .validate_for_neuron_count(p.neuron_count())?;
        Ok(Self {
            checkpoint,
            pending: None,
        })
    }
    /// Checkpoint only a completed causal boundary; unresolved action credit is
    /// never silently dropped. Native founder/critic checkpoints remain separate.
    pub fn checkpoint(&self) -> Result<CpuOrganismCheckpoint, TrainingError> {
        if self.pending.is_some() {
            return Err(invalid());
        }
        Ok(self.checkpoint.clone())
    }
    pub fn neural_state(&self) -> &TrainingInitialState {
        &self.checkpoint.neural
    }
    pub fn effective_offsets(&self, p: &BrainPhenotype) -> Result<Vec<f32>, TrainingError> {
        if self.checkpoint.phenotype_hash != p.phenotype_hash() {
            return Err(invalid());
        }
        let offsets = p
            .synapses()
            .iter()
            .enumerate()
            .map(|(i, s)| self.checkpoint.lifetime[i] + s.alpha() * self.checkpoint.fast[i])
            .collect::<Vec<_>>();
        finite(&offsets)?;
        Ok(offsets)
    }
    /// The context must be assembled from the organism's actual perception and
    /// canonical biology. Receptor effects override detached replay fields;
    /// somatic offsets always come from this organism, never another batch row.
    pub fn advance(
        &mut self,
        founder: &CpuFoundationTrainer,
        mut context: TrainingReplayTick,
        effects: &NeuralReceptorEffects,
    ) -> Result<CpuDecisionOutput, TrainingError> {
        effects.validate_contract()?;
        let tick = effects.source_tick.raw();
        let p = founder.phenotype();
        if self.pending.is_some()
            || founder
                .weights()
                .iter()
                .zip(p.synapses())
                .any(|(w, s)| w.to_bits() != s.genetic_weight().to_bits())
            || self.checkpoint.last_outcome_tick.is_some_and(|t| tick < t)
            || self
                .checkpoint
                .last_decision_tick
                .is_some_and(|t| tick <= t)
        {
            return Err(invalid());
        }
        context.projection_gain = effects.projection_gain;
        context.local_threshold_shift = effects.local_threshold_shift;
        context.effective_weight_offsets = self.effective_offsets(p)?;
        let initial = self.checkpoint.neural.clone();
        let q = TrainingSequence {
            phenotype_hash: p.phenotype_hash(),
            initial: initial.clone(),
            ticks: vec![context.clone()],
            burn_in_ticks: 0,
            memory_candidate_gain: p
                .candidate_decoder()
                .memory_channel()
                .map_or(0., |m| m.max_candidate_gain()),
        };
        q.validate_for(p)?;
        let tape = founder.forward(&q)?;
        let pre = if context.microstep_count == 0 {
            self.checkpoint.inactive_activations.clone()
        } else {
            tape.a[context.microstep_count as usize - 1].clone()
        };
        let logits = tape.evaluation.candidate_logits[0].clone();
        let generation = self
            .checkpoint
            .dispatch_generation
            .checked_add(1)
            .ok_or_else(invalid)?;
        self.checkpoint.inactive_activations = pre.clone();
        self.checkpoint.neural = TrainingInitialState {
            activations: tape.evaluation.final_activations[0].clone(),
            activity_ema: tape.evaluation.final_activity_ema[0].clone(),
            metabolic_load: tape.evaluation.final_metabolic_load[0].clone(),
            dendrites: initial.dendrites.clone(),
        };
        self.checkpoint.dispatch_generation = generation;
        self.checkpoint.active_activation_side ^= (context.microstep_count & 1) as u8;
        self.pending = Some(Pending {
            tick,
            context: context.clone(),
            pre,
            staged: None,
            representative: None,
            binding: None,
        });
        Ok(CpuDecisionOutput {
            candidate_logits: logits,
            initial,
            replay_tick: context,
            dispatch_generation: generation,
            active_activation_side: self.checkpoint.active_activation_side,
        })
    }
    /// Eligibility is staged after actual joint selection, before world outcome.
    /// Repeated joint candidates contribute once. Speech credit requires the
    /// matching payload-selection receipt supplied by the collector.
    pub fn stage_selection(
        &mut self,
        p: &BrainPhenotype,
        action: &PpoJointAction,
        binding: CpuDecisionBinding,
        speech_receipt: bool,
    ) -> Result<(), TrainingError> {
        action.validate(action.temperature)?;
        binding.joint_selection.validate()?;
        let pending = self.pending.as_ref().ok_or_else(invalid)?;
        if pending.staged.is_some()
            || action.candidate_count as usize != pending.context.candidates.len()
            || self.checkpoint.phenotype_hash != p.phenotype_hash()
            || binding.dispatch_generation != self.checkpoint.dispatch_generation
            || binding.active_activation_side != self.checkpoint.active_activation_side
            || binding.joint_selection.candidate_slots()
                != action.motor_candidates.map(|i| i.map_or(0, |i| i + 1))
        {
            return Err(invalid());
        }
        let elapsed = self
            .checkpoint
            .last_decision_tick
            .map_or(0, |old| pending.tick - old);
        let mut selected = vec![action.representative as usize];
        for index in action.motor_candidates.into_iter().flatten() {
            if !selected.contains(&(index as usize)) {
                selected.push(index as usize);
            }
        }
        // Slot 3 is the vocal channel in the native six-slot joint ABI.
        let vocal_selected = action.motor_candidates[3].is_some();
        let mut staged = self.checkpoint.eligibility.clone();
        for (i, s) in p.synapses().iter().enumerate() {
            let receptor = &p.plasticity_receptors()[s.receptor_index() as usize];
            let local = match s.kind() {
                CompiledSynapseKind::Recurrent
                    if pending.context.enabled_routes[s.route_index() as usize] =>
                {
                    pending.pre[s.source() as usize]
                        * self.checkpoint.neural.activations[s.target() as usize]
                }
                CompiledSynapseKind::Recurrent => 0.,
                CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::SpeechPayload => {
                    if speech_receipt && vocal_selected {
                        self.checkpoint.neural.activations[s.source() as usize]
                            * self.checkpoint.neural.activations[s.target() as usize]
                    } else {
                        0.
                    }
                }
                CompiledSynapseKind::Decoder(c) => selected
                    .iter()
                    .filter_map(|&j| {
                        let candidate = &pending.context.candidates[j];
                        (candidate.family == c.family()).then(|| {
                            candidate.decoder_inputs[c.input_lane() as usize]
                                * if c.head() == DecoderHeadKind::ActionCandidate {
                                    self.checkpoint.neural.activations[s.source() as usize]
                                } else {
                                    1.
                                }
                        })
                    })
                    .sum(),
            };
            let next = decay(receptor.eligibility_decay(), elapsed) * staged[i] + local;
            finite(&[next])?;
            // Disabled recurrent routes decay without adding or clamping, as WGSL.
            staged[i] = if matches!(s.kind(), CompiledSynapseKind::Recurrent)
                && !pending.context.enabled_routes[s.route_index() as usize]
            {
                next
            } else {
                next.clamp(-1., 1.)
            };
        }
        let pending = self.pending.as_mut().unwrap();
        pending.staged = Some(staged);
        pending.representative = Some(action.representative);
        pending.binding = Some(binding);
        Ok(())
    }
    /// Apply only canonical sealed credit with the receptor frame from its real
    /// chemistry tick. No raw reward scalar API is exposed.
    pub fn apply_credit(
        &mut self,
        founder: &CpuFoundationTrainer,
        patch: &ExperiencePatch,
        receptors: &NeuralReceptorFrame,
        development: &DevelopmentState,
        parameters: &PlasticityGenomeParameters,
    ) -> Result<(), TrainingError> {
        let p = founder.phenotype();
        if p.phenotype_hash() != self.checkpoint.phenotype_hash
            || founder
                .weights()
                .iter()
                .zip(p.synapses())
                .any(|(w, s)| w.to_bits() != s.genetic_weight().to_bits())
        {
            return Err(invalid());
        }
        let credit = OutcomeCreditPacket::from_sealed_patch(patch)?;
        if credit.policy_backend() != alife_core::PolicyBackend::NeuralClosedLoopCpu {
            return Err(invalid());
        }
        let pending = self.pending.as_ref().ok_or_else(invalid)?;
        let staged = pending.staged.as_ref().ok_or_else(invalid)?;
        let binding = pending.binding.as_ref().ok_or_else(invalid)?;
        binding
            .joint_selection
            .validate_decision(patch.decision())?;
        if credit.organism_id() != self.checkpoint.organism_id
            || credit.phenotype_hash() != self.checkpoint.phenotype_hash
            || credit.originating_tick().raw() != pending.tick
            || credit.outcome_tick().raw() < pending.tick
            || Some(credit.selected_candidate()) != pending.representative
            || credit.sequence_id() != binding.sequence_id
            || credit.frame_digest() != binding.frame_digest
            || credit.selected_action() != binding.action_id
            || credit.candidate_feature_digest() != binding.candidate_feature_digest
            || credit.dispatch_generation() != binding.dispatch_generation
            || credit.active_activation_side() != binding.active_activation_side
            || pending.context.candidates[credit.selected_candidate() as usize].family
                != credit.selected_family()
        {
            return Err(invalid());
        }
        let credit = credit.with_biochemical_receptors(receptors)?;
        let mut developmental = alife_gpu_backend::GpuDevelopmentalPlasticity::default();
        developmental.update(p, development, parameters)?;
        let words = developmental.words();
        let frame = credit.modulator().frame();
        let mut next = self.checkpoint.clone();
        next.weight_generation = next.weight_generation.checked_add(1).ok_or_else(invalid)?;
        for (i, s) in p.synapses().iter().enumerate() {
            let r = &p.plasticity_receptors()[s.receptor_index() as usize];
            let factor = r.receptor_profile().project(&frame)?;
            let recurrent = matches!(s.kind(), CompiledSynapseKind::Recurrent);
            let rate = |base: f32, lane: usize| {
                if recurrent && words[0] != 0 {
                    (base * f32::from_bits(words[4 + s.route_index() as usize]))
                        .min(f32::from_bits(words[lane]))
                } else {
                    base
                }
            };
            let eligibility = staged[i]
                * decay(
                    r.eligibility_decay(),
                    credit.outcome_tick().raw() - pending.tick,
                );
            let post = next.neural.activations[s.target() as usize];
            let (min, max) = r.fast_weight_bounds();
            next.fast[i] = waking_fast_update(
                founder.weights()[i],
                next.lifetime[i],
                next.fast[i],
                s.alpha(),
                post,
                eligibility,
                rate(r.learning_rate(), 1),
                rate(r.normalization_rate(), 3),
                factor,
                min,
                max,
            )?;
        }
        next.eligibility = staged.clone();
        next.last_decision_tick = Some(pending.tick);
        next.last_outcome_tick = Some(credit.outcome_tick().raw());
        self.checkpoint = next;
        self.pending = None;
        Ok(())
    }
    /// Synchronous version of the native bounded replay/stage/reset transaction.
    /// The enclosing host commits its world and all sidecars atomically.
    pub fn consolidate_sleep(
        &mut self,
        founder: &CpuFoundationTrainer,
        evidence: &alife_core::sleep::SleepReplayEvidence,
        development: &DevelopmentState,
        parameters: &PlasticityGenomeParameters,
    ) -> Result<(), TrainingError> {
        if self.pending.is_some()
            || self.checkpoint.phenotype_hash != founder.phenotype().phenotype_hash()
        {
            return Err(invalid());
        }
        evidence.validate_contract()?;
        let p = founder.phenotype();
        evidence.batch.validate_contract(
            p.replay_capture_plan().event_capacity(),
            p.replay_capture_plan().sample_capacity(),
            p.synapses().len() as u32,
        )?;
        if evidence
            .prediction_targets
            .iter()
            .any(|t| t.organism_id != self.checkpoint.organism_id)
            || evidence
                .batch
                .synapse_spans
                .iter()
                .map(|s| s.local_synapse_id)
                .ne(p.replay_capture_plan().global_synapse_ids().iter().copied())
        {
            return Err(invalid());
        }
        let mut developmental = alife_gpu_backend::GpuDevelopmentalPlasticity::default();
        developmental.update(p, development, parameters)?;
        let words = developmental.words();
        let mut next = self.checkpoint.clone();
        for span in &evidence.batch.synapse_spans {
            let i = span.local_synapse_id as usize;
            let s = &p.synapses()[i];
            let r = &p.plasticity_receptors()[s.receptor_index() as usize];
            let mut credit = 0.;
            for sample in &evidence.batch.eligibility_samples
                [span.sample_start as usize..(span.sample_start + span.sample_count) as usize]
            {
                credit += f32::from(sample.eligibility_q15) / 32767.
                    * r.receptor_profile().project(
                        &evidence.batch.events[sample.event_index as usize]
                            .modulator
                            .frame(),
                    )?;
            }
            let rate = if matches!(s.kind(), CompiledSynapseKind::Recurrent) && words[0] != 0 {
                (r.sleep_replay_rate() * f32::from_bits(words[4 + s.route_index() as usize]))
                    .min(f32::from_bits(words[2]))
            } else {
                r.sleep_replay_rate()
            };
            let (min, max) = r.fast_weight_bounds();
            next.fast[i] = (next.fast[i] + rate * s.alpha() * credit).clamp(min, max);
        }
        let plan = p.sleep_consolidation_plan();
        for (i, s) in p.synapses().iter().enumerate() {
            if p.plasticity_receptors()[s.receptor_index() as usize].is_delta_enabled() {
                next.lifetime[i] = (next.lifetime[i] + plan.staging_rate() * next.fast[i])
                    .clamp(-plan.weight_limit(), plan.weight_limit());
                next.fast[i] *= 1. - plan.fast_decay_rate();
            }
        }
        next.eligibility.fill(0.);
        next.weight_generation = next.weight_generation.checked_add(1).ok_or_else(invalid)?;
        finite(&next.fast)?;
        finite(&next.lifetime)?;
        self.checkpoint = next;
        Ok(())
    }
}
#[allow(clippy::too_many_arguments)]
fn waking_fast_update(
    genetic: f32,
    lifetime: f32,
    fast: f32,
    alpha: f32,
    post: f32,
    eligibility: f32,
    learning_rate: f32,
    normalization_rate: f32,
    factor: f32,
    min: f32,
    max: f32,
) -> Result<f32, TrainingError> {
    let effective = genetic + lifetime + alpha * fast;
    // Match WGSL fast_plasticity_delta: both terms follow this synapse's
    // outcome-decayed credit, including its magnitude on normalization.
    let signed_credit = factor * eligibility;
    let delta = learning_rate * alpha * signed_credit
        - normalization_rate * signed_credit.abs() * post * post * effective;
    let next = fast + delta;
    finite(&[effective, delta, next])?;
    Ok(next.clamp(min, max))
}
fn decay(reference: f32, elapsed: u64) -> f32 {
    if elapsed == 0 || reference == 1. {
        1.
    } else if reference == 0. {
        0.
    } else if elapsed == 1 {
        reference
    } else {
        reference.powf((elapsed >> 32) as f32 * 4294967296. + (elapsed as u32) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::*;
    fn fixture() -> (CpuFoundationTrainer, BrainGenome, DevelopmentState) {
        let capacity = BrainCapacityClass::n2048();
        let genome = BrainGenome::scaffold(0x7311, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
        let source =
            FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
        let p = PhenotypeCompiler::compile_from_foundation_asset(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
            &source,
        )
        .unwrap();
        let mask = crate::StageTrainableMask::from_synapse_indices(&p, &[17]).unwrap();
        (
            CpuFoundationTrainer::new(p, source, mask, crate::AdamWConfig::default()).unwrap(),
            genome,
            development,
        )
    }
    fn effects() -> NeuralReceptorEffects {
        NeuralReceptorEffects {
            source_tick: Tick::new(8),
            source_chemistry_version: 1,
            interoceptive_gain: 1.,
            regional_excitability: 1.,
            projection_gain: 0.9,
            local_threshold_shift: 0.02,
            attention_gain: 1.,
            plasticity_appetitive: 0.2,
            plasticity_aversive: 0.1,
            structural_growth_gate: 0.,
            sleep_gate: 0.,
            consolidation_gate: 0.,
        }
    }
    fn staged_fixture(
        founder: &CpuFoundationTrainer,
        vocal: bool,
        receipt: bool,
        microsteps: u32,
    ) -> CpuOrganismState {
        let p = founder.phenotype();
        let n = p.neuron_count() as usize;
        let initial = TrainingInitialState {
            activations: vec![0.1; n],
            activity_ema: vec![0.2; n],
            metabolic_load: vec![0.03; n],
            dendrites: Default::default(),
        };
        let mut state = CpuOrganismState::new(p, OrganismId(7), initial).unwrap();
        state.checkpoint.last_decision_tick = Some(5);
        state.checkpoint.last_outcome_tick = Some(5);
        state.checkpoint.eligibility.fill(0.4);
        state.checkpoint.inactive_activations.fill(-0.3);
        let mut routes = vec![true; p.projections().len()];
        routes[p.synapses()[0].route_index() as usize] = false;
        let context = TrainingReplayTick {
            encoded_inputs: vec![0.2; n],
            projection_gain: 1.,
            local_threshold_shift: 0.,
            microstep_count: microsteps,
            enabled_routes: routes,
            effective_weight_offsets: vec![0.; p.synapses().len()],
            structural_synapses: vec![],
            candidates: vec![
                crate::TrainingReplayCandidate {
                    family: CandidateActionFamily::Inspect,
                    decoder_inputs: [0.2; 54],
                    innate_bias: 0.,
                },
                crate::TrainingReplayCandidate {
                    family: CandidateActionFamily::Other,
                    decoder_inputs: [0.3; 54],
                    innate_bias: 0.,
                },
            ],
        };
        let output = state.advance(founder, context, &effects()).unwrap();
        assert_eq!(output.dispatch_generation, 1);
        assert_eq!(output.replay_tick.projection_gain, 0.9);
        assert!(state.checkpoint().is_err());
        let action = PpoJointAction {
            candidate_count: 2,
            representative_mask: 3,
            motor_masks: [0, 0, 0, if vocal { 2 } else { 0 }, 1, 0],
            representative: 0,
            forced_slots: vec![Some(4), Some(3)],
            motor_candidates: [
                None,
                None,
                None,
                if vocal { Some(1) } else { None },
                Some(0),
                None,
            ],
            old_joint_log_probability: -0.5,
            temperature: 1.,
        };
        let bundle =
            MotorCommandBundle::new(OrganismId(7), ExperienceSequenceId(9), Tick::new(8), vec![])
                .unwrap();
        let binding = CpuDecisionBinding {
            sequence_id: ExperienceSequenceId(9),
            frame_digest: PerceptionFrameDigest([1; 4]),
            action_id: ActionId(4),
            candidate_feature_digest: CandidateFeatureDigest([1; 2]),
            dispatch_generation: 1,
            active_activation_side: output.active_activation_side,
            joint_selection: JointActionSelectionV1::new(
                action.motor_candidates.map(|i| i.map_or(0, |i| i + 1)),
                &bundle,
            )
            .unwrap(),
        };
        let mut mismatched = binding.clone();
        mismatched.dispatch_generation += 1;
        assert!(state
            .stage_selection(p, &action, mismatched, receipt)
            .is_err());
        assert!(state.pending.as_ref().unwrap().staged.is_none());
        state.stage_selection(p, &action, binding, receipt).unwrap();
        state
    }
    #[test]
    fn actual_staging_matches_elapsed_traces_route_masks_and_selected_decoders() {
        let (founder, _, _) = fixture();
        let p = founder.phenotype();
        let mut state = staged_fixture(&founder, false, true, 1);
        let pending = state.pending.as_ref().unwrap();
        let staged = pending.staged.as_ref().unwrap();
        for (i, s) in p.synapses().iter().enumerate().take(256) {
            let r = &p.plasticity_receptors()[s.receptor_index() as usize];
            let previous = 0.4 * r.eligibility_decay().powi(3);
            let expected = if pending.context.enabled_routes[s.route_index() as usize] {
                (previous + 0.1 * state.neural_state().activations[s.target() as usize])
                    .clamp(-1., 1.)
            } else {
                previous
            };
            assert!((staged[i] - expected).abs() < 2e-6);
        }
        let (i,s)=p.synapses().iter().enumerate().find(|(_,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate&&c.family()==CandidateActionFamily::Inspect)).unwrap();
        let r = &p.plasticity_receptors()[s.receptor_index() as usize];
        let expected = (0.4 * r.eligibility_decay().powi(3)
            + 0.2 * state.neural_state().activations[s.source() as usize])
            .clamp(-1., 1.);
        assert!((staged[i] - expected).abs() < 2e-6);
        let context = pending.context.clone();
        assert!(state.advance(&founder, context, &effects()).is_err());
    }
    #[test]
    fn zero_step_collection_preserves_both_banks_and_native_eligibility_input() {
        let (founder, _, _) = fixture();
        let p = founder.phenotype();
        let state = staged_fixture(&founder, false, true, 0);
        assert_eq!(state.checkpoint.active_activation_side, 0);
        assert!(state.neural_state().activations.iter().all(|v| *v == 0.1));
        assert!(state
            .checkpoint
            .inactive_activations
            .iter()
            .all(|v| *v == -0.3));
        assert!(state.neural_state().activity_ema.iter().all(|v| *v == 0.2));
        assert!(state
            .neural_state()
            .metabolic_load
            .iter()
            .all(|v| *v == 0.03));
        let pending = state.pending.as_ref().unwrap();
        let staged = pending.staged.as_ref().unwrap();
        for (i, s) in p
            .synapses()
            .iter()
            .enumerate()
            .filter(|(_, s)| matches!(s.kind(), CompiledSynapseKind::Recurrent))
        {
            let receptor = &p.plasticity_receptors()[s.receptor_index() as usize];
            let decayed = 0.4 * receptor.eligibility_decay().powi(3);
            let expected = if pending.context.enabled_routes[s.route_index() as usize] {
                (decayed - 0.3 * 0.1).clamp(-1., 1.)
            } else {
                decayed
            };
            assert!((staged[i] - expected).abs() < 2e-6);
        }
        let mut completed = state.checkpoint.clone();
        completed.last_decision_tick = Some(8);
        completed.last_outcome_tick = Some(9);
        let bytes = serde_json::to_vec(&completed).unwrap();
        let restored =
            CpuOrganismState::from_checkpoint(p, serde_json::from_slice(&bytes).unwrap()).unwrap();
        assert_eq!(restored.checkpoint().unwrap(), completed);
    }
    #[test]
    fn speech_eligibility_requires_both_vocal_selection_and_payload_receipt() {
        let (founder, _, _) = fixture();
        let p = founder.phenotype();
        for (vocal, receipt) in [(false, true), (true, false), (true, true)] {
            let state = staged_fixture(&founder, vocal, receipt, 1);
            let staged = state.pending.as_ref().unwrap().staged.as_ref().unwrap();
            for (i,s) in p.synapses().iter().enumerate().filter(|(_,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::SpeechPayload)) {
                let r=&p.plasticity_receptors()[s.receptor_index() as usize];let local=if vocal&&receipt {state.neural_state().activations[s.source() as usize]*state.neural_state().activations[s.target() as usize]} else {0.};
                let expected=(0.4*r.eligibility_decay().powi(3)+local).clamp(-1.,1.);assert!((staged[i]-expected).abs()<2e-6);
            }
        }
    }
    // Intentionally lacks measured physiology: this valid sealed test patch
    // must never become permission for a numerical kernel to fabricate credit.
    fn ungrounded_patch(
        p: &BrainPhenotype,
        genome: &BrainGenome,
        development: &DevelopmentState,
    ) -> ExperiencePatch {
        let id = OrganismId(7);
        let tick = Tick::new(8);
        let sequence = ExperienceSequenceId(9);
        let candidate = ActionCandidate::new(
            0,
            ActionKind::Inspect.canonical_id(),
            ActionKind::Inspect,
            CandidateActionFamily::Inspect,
            CandidateObservationRef::None,
            ActionTarget::NONE,
            CandidateFeatureVector::zero(),
            Confidence::new(0.8).unwrap(),
            NormalizedScalar::new(0.1).unwrap(),
            DurationTicks::new(1),
            DurationTicks::new(1),
        )
        .unwrap();
        let command = candidate
            .to_command(id, Confidence::new(0.8).unwrap())
            .unwrap();
        let frame = PerceptionFrame::new(
            id,
            tick,
            SensorProfile::PrivilegedAffordanceV1,
            SensorySnapshot::new(
                id,
                tick,
                Vec3f::ZERO,
                SensoryChannels::ZERO,
                Default::default(),
            )
            .unwrap(),
            BodySnapshot {
                pose: Pose::IDENTITY,
                velocity: Velocity::ZERO,
            },
            HomeostaticSnapshot::baseline(tick),
            vec![candidate],
            SensorProfileProvenance::new(
                SensorProfile::PrivilegedAffordanceV1,
                SensoryAbiVersion::CURRENT,
                tick,
            )
            .unwrap(),
            vec![],
        )
        .unwrap();
        let pre = PreActionSnapshot::from_neural_frame(
            sequence,
            p.brain_class_id(),
            p.phenotype_hash(),
            genome.id,
            genome.schema_version,
            development.clone(),
            frame.clone(),
        )
        .unwrap();
        let decision = DecisionSnapshot::from_neural_selection(
            sequence,
            p.phenotype_hash(),
            1,
            1,
            &frame,
            NeuralActionSelection {
                candidate_index: 0,
                logit: 0.,
                confidence: Confidence::new(0.8).unwrap(),
                active_tiles: 1,
                active_synapses: 1,
            },
            command,
        )
        .unwrap();
        let outcome = PostActionOutcome::new(
            id,
            sequence,
            Tick::new(9),
            true,
            PhysicalActionOutcome {
                contact: PhysicalContactKind::None,
                target_entity: None,
                displacement: Vec3f::ZERO,
                collision_normal: None,
                energy_cost: NormalizedScalar::new(0.).unwrap(),
            },
            HomeostaticDelta::zero(),
            SignedValence::ZERO,
            NormalizedScalar::new(0.).unwrap(),
            NormalizedScalar::new(0.).unwrap(),
            SignedValence::ZERO,
            NormalizedScalar::new(0.).unwrap(),
        )
        .unwrap();
        ExperiencePatchBuilder::new(sequence)
            .record_pre_action(pre)
            .unwrap()
            .record_decision(decision)
            .unwrap()
            .record_outcome(outcome)
            .unwrap()
            .seal()
            .unwrap()
    }
    #[test]
    fn rejected_ungrounded_credit_keeps_pending_and_all_private_state() {
        let (founder, genome, development) = fixture();
        let mut state = staged_fixture(&founder, false, false, 1);
        let before = state.checkpoint.clone();
        let pending = state.pending.as_ref().unwrap().staged.clone();
        let actor = founder.checkpoint();
        let patch = ungrounded_patch(founder.phenotype(), &genome, &development);
        let frame = NeuralReceptorFrame {
            schema_version: 1,
            source_chemistry_version: 1,
            source_tick: Tick::new(8),
            activations: vec![],
        };
        assert!(state
            .apply_credit(
                &founder,
                &patch,
                &frame,
                &development,
                genome.plasticity_parameters()
            )
            .is_err());
        assert_eq!(state.checkpoint, before);
        assert_eq!(state.pending.as_ref().unwrap().staged, pending);
        assert_eq!(founder.checkpoint(), actor);
    }
    #[test]
    fn waking_hebbian_oja_uses_all_three_weight_components_and_alpha() {
        let next =
            waking_fast_update(0.2, 0.3, 0.4, 0.5, 0.6, 0.8, 0.1, 0.2, -0.5, -1., 1.).unwrap();
        // Effective=.7, signed credit=-.4, Hebbian=-.02, Oja=-.02016.
        assert!((next - 0.35984).abs() < 1e-6);
        assert!(
            waking_fast_update(f32::MAX, f32::MAX, 0., 0., 0., 0., 0., 0., 0., -1., 1.).is_err()
        );
    }
    #[test]
    fn waking_normalization_follows_signed_local_credit_magnitude() {
        // Same ordinary update as the production WGSL law: inherited .2,
        // lifetime .3 and expressed fast .5*.4 give effective strength .7.
        // Opposite signs reverse the Hebbian drive, never the contraction.
        for (factor, eligibility, expected) in [
            (0.0, 0.8, 0.4),
            (0.5, 0.0, 0.4),
            (-0.5, 0.0, 0.4),
            (0.5, 0.8, 0.39984),
            (-0.5, 0.8, 0.35984),
            (0.5, -0.8, 0.35984),
            (-0.5, -0.8, 0.39984),
        ] {
            let next = waking_fast_update(
                0.2,
                0.3,
                0.4,
                0.5,
                0.6,
                eligibility,
                0.1,
                0.2,
                factor,
                -1.,
                1.,
            )
            .unwrap();
            assert!(
                (next - expected).abs() < 1e-7,
                "factor={factor}, eligibility={eligibility}: {next} != {expected}"
            );
            if factor == 0.0 || eligibility == 0.0 {
                assert_eq!(next.to_bits(), 0.4_f32.to_bits());
            }
        }
        // A receptor event cannot erase retained fast state on an uncredited
        // route/readout, even across repeated outcomes with an active post.
        let mut fast = 0.4;
        for factor in [0.0, 0.5, -0.5].into_iter().cycle().take(1000) {
            fast = waking_fast_update(0.2, 0.3, fast, 0.5, 0.6, 0.0, 0.1, 0.2, factor, -1., 1.)
                .unwrap();
        }
        assert_eq!(fast.to_bits(), 0.4_f32.to_bits());
    }
    #[test]
    fn waking_fast_update_matches_documented_production_coordination_vectors() {
        // Independent numeric vectors recorded with the final WGSL law in
        // docs/reports/oja-plasticity-credit-2026-10-20.md. No device is used.
        for (factor, expected_delta) in [
            (0.0, 0.0),
            (0.01, 0.0000092),
            (-0.01, -0.0000108),
            (1.0, 0.00092),
            (-1.0, -0.00108),
        ] {
            let next =
                waking_fast_update(0.5, 0.25, 0.2, 0.25, 0.5, 0.4, 0.01, 0.001, factor, -2., 2.)
                    .unwrap();
            assert!(
                (next - (0.2 + expected_delta)).abs() < 2e-8,
                "factor={factor}: {next}"
            );
        }
    }
    #[test]
    fn waking_credit_gating_preserves_clipping_and_alpha_semantics() {
        for (factor, expected) in [(1.0, 0.5), (-1.0, -0.5)] {
            assert_eq!(
                waking_fast_update(0.2, 0.3, 0.4, 1., 0.6, 1., 1., 0.2, factor, -0.5, 0.5).unwrap(),
                expected
            );
        }
        // Zero credit still enforces existing bounds on already invalid fast
        // values. Alpha zero suppresses drive, not eligible normalization.
        assert_eq!(
            waking_fast_update(0.2, 0.3, 2., 0.5, 0.6, 0., 0.1, 0.2, 0., -1., 1.).unwrap(),
            1.
        );
        let latent =
            waking_fast_update(0.2, 0.3, 0.4, 0., 0.6, 0.8, 0.1, 0.2, -0.5, -1., 1.).unwrap();
        assert!((latent - 0.3856).abs() < 1e-7);
    }
    #[test]
    fn eligibility_decay_uses_elapsed_simulation_ticks_including_high_word() {
        assert_eq!(decay(0., 0), 1.);
        assert_eq!(decay(0., 1), 0.);
        assert_eq!(decay(1., u64::MAX), 1.);
        assert!((decay(0.5, 3) - 0.125).abs() < 1e-7);
        assert_eq!(decay(0.9, 1u64 << 32), 0.);
    }
}
