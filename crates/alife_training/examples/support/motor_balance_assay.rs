//! Snapshot of the frozen V1 assay helpers; candidate experiments do not alter V1.
//! Detached fixed-context CPU routing assay. No world ticks, learning or private state.
#[path = "multidrive_frozen.rs"]
pub mod frozen;
use alife_core::*;
use alife_gpu_backend::GpuV11CausalState;
use alife_training::{
    cpu::*, AdamWConfig, StageTrainableMask, TrainingInitialState, TrainingReplayCandidate,
    TrainingReplayTick, TrainingSequence,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, VecDeque},
    error::Error,
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
pub const FRAMES: usize = 64;
pub const BURN: usize = 32;
pub const LABELS: [&str; 11] = [
    "hunger",
    "fatigue",
    "fear",
    "pain",
    "loneliness",
    "curiosity",
    "ATP_availability",
    "temperature_stress",
    "reproductive",
    "boredom_if_bound",
    "extension1_unbound",
];
pub fn drives(a: [f32; 11]) -> DriveSnapshot {
    DriveSnapshot {
        hunger: a[0],
        fatigue: a[1],
        fear: a[2],
        pain: a[3],
        loneliness: a[4],
        curiosity: a[5],
        brain_atp: a[6],
        temperature_stress: a[7],
        reproductive_drive: a[8],
        extension: [a[9], a[10]],
    }
}
pub fn features(family: CandidateActionFamily, context: &str) -> CandidateFeatureVector {
    if family == CandidateActionFamily::Idle {
        return CandidateFeatureVector::zero().with_intrinsic_action_basis(ActionKind::Idle);
    }
    if family == CandidateActionFamily::Rest {
        return CandidateFeatureVector::zero().with_intrinsic_action_basis(ActionKind::Rest);
    }
    let mut x = [0.; 24];
    if !matches!(
        family,
        CandidateActionFamily::Idle | CandidateActionFamily::Rest
    ) {
        x[0] = 1.;
        x[2] = if context == "distant" { 0.8 } else { 0.1 };
        x[6] = 0.4;
        x[7] = 0.3;
        x[8] = 0.2;
        x[18] = if context == "distant" { 0. } else { 1. };
        x[15] = match context {
            "nutritious_contact" => 1.,
            "aversive_contact" => -1.,
            _ => 0.,
        };
    }
    CandidateFeatureVector(x)
}
pub fn encoded(p: &BrainPhenotype, a: [f32; 11]) -> Vec<f32> {
    let mut encoded = vec![0.; p.neuron_count() as usize];
    for s in p.sensor_encoder().assignments() {
        // Synthetic controlled context: all other sensor/hormone channels zero.
        // Encoder scale, bias and clamp remain inherited. Receptor gains fixed 1.
        let value =
            if s.source_group() == SensorEncoderSourceGroup::Homeostasis && s.source_index() < 11 {
                a[s.source_index() as usize]
            } else {
                0.
            };
        let (lo, hi) = s.clamp_range();
        encoded[s.target_neuron() as usize] += (value * s.scale() + s.bias()).clamp(lo, hi);
    }
    for v in &mut encoded {
        *v = v.clamp(-1., 1.);
    }
    encoded
}
pub fn sequence(
    p: &BrainPhenotype,
    base: [f32; 11],
    changed: [f32; 11],
    context: &str,
    dendrites: &DendriticBranchSet,
    input_cut: bool,
) -> TrainingSequence {
    let n = p.neuron_count() as usize;
    let ticks = (0..FRAMES)
        .map(|t| {
            let a = if t < BURN || input_cut { base } else { changed };
            let candidates = p
                .candidate_decoder()
                .families()
                .iter()
                .map(|f| {
                    let mut decoder_inputs = [0.; 54];
                    decoder_inputs[..24].copy_from_slice(&features(f.family(), context).0);
                    TrainingReplayCandidate {
                        family: f.family(),
                        decoder_inputs,
                        innate_bias: 0.,
                    }
                })
                .collect();
            TrainingReplayTick {
                encoded_inputs: encoded(p, a),
                projection_gain: 1.,
                local_threshold_shift: 0.,
                microstep_count: p.microstep_count() as u32,
                enabled_routes: vec![true; p.projections().len()],
                effective_weight_offsets: vec![0.; p.synapses().len()],
                structural_synapses: vec![],
                candidates,
            }
        })
        .collect();
    TrainingSequence {
        phenotype_hash: p.phenotype_hash(),
        initial: TrainingInitialState {
            activations: vec![0.; n],
            activity_ema: vec![0.; n],
            metabolic_load: vec![0.; n],
            dendrites: dendrites.clone(),
        },
        ticks,
        burn_in_ticks: BURN,
        memory_candidate_gain: p
            .candidate_decoder()
            .memory_channel()
            .map_or(0., |m| m.max_candidate_gain()),
    }
}
pub fn reachability(
    p: &BrainPhenotype,
    w: &[f32],
    d: &DendriticBranchSet,
    lane: usize,
    effective: bool,
    include_dendrites: bool,
) -> Value {
    let mut edges = vec![Vec::new(); p.neuron_count() as usize];
    for (i, s) in p.synapses().iter().enumerate() {
        if s.kind() == CompiledSynapseKind::Recurrent && (!effective || w[i] != 0.) {
            edges[s.source() as usize].push(s.target() as usize);
        }
    }
    if include_dendrites {
        for b in d.branches() {
            if !effective || b.output_gain != 0. {
                for i in &b.inputs {
                    if !effective || i.weight != 0. {
                        edges[i.source as usize].push(b.target as usize);
                    }
                }
            }
        }
    }
    let roots = p
        .sensor_encoder()
        .assignments()
        .iter()
        .filter(|s| {
            s.source_group() == SensorEncoderSourceGroup::Homeostasis
                && s.source_index() as usize == lane
        })
        .map(|s| s.target_neuron() as usize)
        .collect::<Vec<_>>();
    let mut seen = BTreeSet::new();
    let mut q = VecDeque::from(roots.clone());
    while let Some(n) = q.pop_front() {
        if seen.insert(n) {
            q.extend(edges[n].iter().copied());
        }
    }
    let family=p.candidate_decoder().families().iter().map(|f| {
        let motors=p.synapses().iter().enumerate().filter_map(|(i,s)|match s.kind() {CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==f.family() && (!effective || w[i]!=0.)=>Some(s.source() as usize),_=>None}).collect::<BTreeSet<_>>();
        json!({"family":f.family(),"reachable_motor_count":motors.intersection(&seen).count(),"decoder_motor_count":motors.len()})
    }).collect::<Vec<_>>();
    json!({"roots":roots,"reachable_neurons":seen.len(),"families":family})
}
