//! Detached fixed-context CPU routing assay. No world ticks, learning or private state.
#[path = "support/multidrive_frozen.rs"]
mod frozen;
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
const FRAMES: usize = 64;
const BURN: usize = 32;
const LABELS: [&str; 11] = [
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
fn drives(a: [f32; 11]) -> DriveSnapshot {
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
fn features(family: CandidateActionFamily, context: &str) -> CandidateFeatureVector {
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
fn encoded(p: &BrainPhenotype, a: [f32; 11]) -> Vec<f32> {
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
fn sequence(
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
fn reachability(
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
fn run() -> Result<Value> {
    let mut arms = Vec::new();
    let (config, _, _) = frozen::source(7, false)?;
    let canonical_asset = FoundationWeightAsset::builtin_n2048_v1(config.sensor_profile)?;
    let game =
        alife_world::create_canonical_new_game_with_n2048_candidate(&config, &canonical_asset)?;
    let biology = &game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .phenotype()
        .chemistry
        .biochemical;
    let drive_receptors = biology
        .receptors()
        .iter()
        .filter(|r| matches!(r.target, BiochemicalTargetLocus::Drive(_)))
        .collect::<Vec<_>>();
    for arm in ["default_n2048", "repaired_genetic", "whole_care6"] {
        let (_, p, asset) = frozen::source(7, arm != "default_n2048")?;
        let (weights, circuits) = if arm == "whole_care6" {
            frozen::whole_care(&p)?
        } else {
            (
                p.synapses().iter().map(|s| s.genetic_weight()).collect(),
                vec![],
            )
        };
        let d = GpuV11CausalState::for_phenotype(&p)?
            .dendritic_branches()
            .clone();
        let mask = StageTrainableMask::from_synapse_indices(
            &p,
            &(0..p.synapses().len() as u32).collect::<Vec<_>>(),
        )?;
        let source_asset_digest = asset.digest();
        let weight_digest = format!(
            "{:x}",
            Sha256::digest(
                weights
                    .iter()
                    .flat_map(|w| w.to_le_bytes())
                    .collect::<Vec<_>>()
            )
        );
        let mut trainer =
            CpuFoundationTrainer::new(p.clone(), asset, mask, AdamWConfig::default())?;
        let mut cp = trainer.checkpoint();
        cp.weights = weights.clone();
        trainer.restore_checkpoint(cp)?;
        let before = serde_json::to_vec(&trainer.checkpoint())?;
        let base = [0.1, 0.1, 0., 0., 0.1, 0.45, 0.75, 0., 0.1, 0.1, 0.];
        let mut perturbations = vec![("balanced".to_string(), base)];
        for (i, name) in LABELS.iter().enumerate() {
            let mut a = base;
            a[i] = if i == 6 { 0.1 } else { 0.9 };
            perturbations.push((name.to_string(), a));
        }
        for (name, indices) in [
            ("hunger_fatigue", vec![0, 1]),
            ("hunger_pain", vec![0, 3]),
            ("fatigue_boredom", vec![1, 9]),
        ] {
            let mut a = base;
            for i in indices {
                a[i] = 0.9;
            }
            perturbations.push((name.into(), a));
        }
        let mut rows = Vec::new();
        for context in ["nutritious_contact", "aversive_contact", "distant"] {
            let baseline =
                trainer.evaluate_replay(&sequence(&p, base, base, context, &d, false))?;
            for (name, a) in &perturbations {
                let cut = sequence(&p, base, *a, context, &d, true);
                assert_eq!(cut, sequence(&p, base, base, context, &d, false));
                let eval = trainer.evaluate_replay(&sequence(&p, base, *a, context, &d, false))?;
                eprintln!("completed {arm} {context} {name}");
                let samples=[1,2,4,8,16,32].into_iter().map(|frame| {
                    let t=BURN+frame-1;
                    let deltas=eval.candidate_logits[t].iter().zip(&baseline.candidate_logits[t]).map(|(v,b)|v-b).collect::<Vec<_>>();
                    let innate=p.candidate_decoder().families().iter().map(|f|f.innate_contribution(drives(*a),features(f.family(),context))).collect::<Vec<_>>();
                    let total=eval.candidate_logits[t].iter().zip(&innate).map(|(n,i)|n+i).collect::<Vec<_>>();
                    json!({"perturbation_frame":frame,"motor_deltas":p.candidate_decoder().families().iter().map(|f| {
                        let motors=p.synapses().iter().filter_map(|s|match s.kind() {CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==f.family()=>Some(s.source() as usize),_=>None}).collect::<BTreeSet<_>>();
                        let values=motors.iter().map(|&m|eval.final_activations[t][m]-baseline.final_activations[t][m]).collect::<Vec<_>>();
                        json!({"family":f.family(),"motor_ids":motors,"baseline_activation":motors.iter().map(|&m|baseline.final_activations[t][m]).collect::<Vec<_>>(),"activation":motors.iter().map(|&m|eval.final_activations[t][m]).collect::<Vec<_>>(),"delta":values})
                    }).collect::<Vec<_>>(),"baseline_neural_logits":baseline.candidate_logits[t],"neural_logits":eval.candidate_logits[t],"neural_delta":deltas,"direct_innate":innate,"neural_plus_innate":total})
                }).collect::<Vec<_>>();
                rows.push(
                    json!({"context":context,"perturbation":name,"drives":a,"samples":samples}),
                );
            }
        }
        assert_eq!(
            before,
            serde_json::to_vec(&trainer.checkpoint())?,
            "assay changed optimizer or weights"
        );
        let reach=(0..11).map(|i|json!({"lane":i,"label":LABELS[i],"structural_recurrent":reachability(&p,&weights,&d,i,false,false),"nonzero_recurrent":reachability(&p,&weights,&d,i,true,false),"nonzero_with_dendrites":reachability(&p,&weights,&d,i,true,true)})).collect::<Vec<_>>();
        arms.push(json!({"arm":arm,"base_compiled_asset_digest":source_asset_digest,"evaluated_weights_f32le_sha256":weight_digest,"phenotype_hash":p.phenotype_hash(),"sensor_profile":p.sensor_profile(),"microsteps":p.microstep_count(),"synapses":p.synapses().len(),"inherited_dendrites":d.branches().len(),"families":p.candidate_decoder().families().iter().map(|f|f.family()).collect::<Vec<_>>(),"circuits":circuits,"reachability":reach,"rows":rows,"input_cut_exact":true,"weights_optimizer_unchanged":true}));
    }
    Ok(
        json!({"schema":"detached_multidrive_routing_v1","backend":"native_cpu_replay","seed":7,"burn_in_frames":BURN,"post_perturbation_frames":FRAMES-BURN,"context":"synthetic fixed sensor/hormone lanes zero; inherited encoder and dendrites; receptor gains 1; no biological advance or biochemical feedback; offsets and structural plasticity zero; innate reported separately","world_steps":0,"training_steps":0,"expressed_drive_receptors":drive_receptors,"expressed_emitters":biology.emitters(),"arms":arms}),
    )
}
fn main() -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&run()?)?);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atp_is_availability_and_idle_rest_remain_candidates() {
        assert_eq!(drives([0.1; 11]).brain_atp, 0.1);
        assert_eq!(
            features(CandidateActionFamily::Idle, "nutritious_contact"),
            CandidateFeatureVector::zero().with_intrinsic_action_basis(ActionKind::Idle)
        );
        assert_eq!(
            features(CandidateActionFamily::Rest, "nutritious_contact").0[23],
            1.
        );
    }
    #[test]
    fn encoder_injects_each_drive_and_cut_is_exact() {
        let (_, p, _) = frozen::source(7, false).unwrap();
        let d = GpuV11CausalState::for_phenotype(&p)
            .unwrap()
            .dendritic_branches()
            .clone();
        let base = [0.1; 11];
        let baseline = encoded(&p, base);
        for lane in 0..11 {
            let mut changed = base;
            changed[lane] = 0.9;
            let targets = p
                .sensor_encoder()
                .assignments()
                .iter()
                .filter(|a| {
                    a.source_group() == SensorEncoderSourceGroup::Homeostasis
                        && a.source_index() as usize == lane
                })
                .map(|a| a.target_neuron() as usize)
                .collect::<BTreeSet<_>>();
            assert!(!targets.is_empty(), "unbound neural input lane {lane}");
            let perturbed = encoded(&p, changed);
            assert!(targets.iter().any(|&i| perturbed[i] != baseline[i]));
            assert!(perturbed
                .iter()
                .zip(&baseline)
                .enumerate()
                .all(|(i, (a, b))| targets.contains(&i) || a == b));
            let cut = sequence(&p, base, changed, "distant", &d, true);
            let control = sequence(&p, base, base, "distant", &d, false);
            assert_eq!(cut, control);
            assert_eq!(cut.initial.dendrites, d);
        }
    }
}
