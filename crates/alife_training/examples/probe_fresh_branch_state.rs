//! Explicit fresh dendritic-state factorial. No acquired-state or genetic edits.
#[allow(dead_code)]
#[path = "support/motor_balance_assay.rs"]
mod assay;
#[allow(dead_code)]
#[path = "support/motor_probe_common.rs"]
mod common;

#[path = "support/motor_operating_point.rs"]
mod readout;
use alife_core::*;
use alife_gpu_backend::GpuV11CausalState;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask, TrainingSequence};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
fn branch_plan(
    inherited: &DendriticBranchSet,
    name: &str,
    gain: f32,
    threshold: f32,
) -> Result<(DendriticBranchSet, Value)> {
    let branches = inherited
        .branches()
        .iter()
        .map(|b| {
            assert_eq!(b.output_gain, 6.);
            assert_eq!(b.threshold, -1.);
            DendriticBranch::new(b.target, threshold, gain, b.inputs.clone())
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let changed = DendriticBranchSet::new(branches)?;
    changed.validate_for_neuron_count(2048)?;
    assert_eq!(changed.branches().len(), inherited.branches().len());
    for (a, b) in changed.branches().iter().zip(inherited.branches()) {
        assert_eq!(a.target, b.target);
        assert_eq!(a.inputs, b.inputs);
        assert_eq!(a.output_gain, gain);
        assert_eq!(a.threshold, threshold);
    }
    let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&changed)?));
    Ok((
        changed.clone(),
        json!({"schema":"explicit_fresh_branch_plan_v1","profile":name,"output_gain":gain,"threshold":threshold,"branch_count":inherited.branches().len(),"branch_plan_sha256":digest,"branches":changed.branches(),"targets_and_input_weights_preserved":true,"rest_signal":1747,"inspect_signal":1760,"approach_reference":1791,"rest_reference":1759,"approach_signal":1776}),
    ))
}
fn sequence(
    p: &BrainPhenotype,
    base: [f32; 11],
    changed: [f32; 11],
    d: &DendriticBranchSet,
    start: f32,
    cut: bool,
) -> TrainingSequence {
    let mut q = common::combined_sequence(p, base, changed, d, cut);
    q.ticks.extend(
        common::combined_sequence(p, base, base, d, false)
            .ticks
            .into_iter()
            .take(32),
    );
    let region = p.lobe_layout().region(LobeKind::ActionPlanning).unwrap();
    q.initial.activations[region.start as usize..(region.start + region.len) as usize].fill(start);
    // Deliberately held equal across activation starts; these are synthetic
    // fresh-state controls, not physically equilibrated biological states.
    assert!(q.initial.metabolic_load.iter().all(|x| *x == 0.));
    assert!(q.initial.activity_ema.iter().all(|x| *x == 0.));
    q
}
fn conditions(base: [f32; 11]) -> Vec<(String, [f32; 11])> {
    let mut rows = vec![("balanced".into(), base)];
    for (i, name) in assay::LABELS.iter().enumerate() {
        let mut a = base;
        a[i] = if i == 6 { 0.1 } else { 0.9 };
        rows.push((name.to_string(), a));
    }
    for (name, axes) in [
        ("hunger_fatigue", vec![0, 1]),
        ("hunger_pain", vec![0, 3]),
        ("fatigue_boredom", vec![1, 9]),
        ("hunger_low_atp", vec![0, 6]),
        ("fatigue_low_atp", vec![1, 6]),
        ("triple", vec![0, 1, 6]),
    ] {
        let mut a = base;
        for i in axes {
            a[i] = if i == 6 { 0.1 } else { 0.9 };
        }
        rows.push((name.into(), a));
    }
    for fatigue in [0., 0.05, 0.2, 0.4, 0.6, 0.8, 1.] {
        let mut a = base;
        a[1] = fatigue;
        rows.push((format!("fatigue_level_{fatigue}"), a));
    }
    rows
}
fn run() -> Result<Value> {
    let (_, base_p, source) = assay::frozen::source(7, true)?;
    let (original, circuits) = assay::frozen::whole_care(&base_p)?;
    let inspection =
        json!({"schema":"fresh_branch_state_scope_v1","world_steps":0,"learning_steps":0});
    if !std::env::args().any(|a| a == "--run") {
        return Ok(inspection);
    }
    let inherited = GpuV11CausalState::for_phenotype(&base_p)?
        .dendritic_branches()
        .clone();
    let base = [0.1, 0.1, 0., 0., 0.1, 0.45, 0.75, 0., 0.1, 0.1, 0.];
    let cases = conditions(base);
    let mut arms = vec![];
    for (factor, gain, threshold) in [
        ("Inherited", 6., -1.),
        ("GainOnly", 0.25, -1.),
        ("ThresholdOnly", 6., 0.1),
        ("BoundedCombined", 0.25, 0.1),
        ("ZeroOutputAblation", 0., -1.),
    ] {
        let (branch_state, plan) = branch_plan(&inherited, factor, gain, threshold)?;
        for start in [0., -0.5, 0.5] {
            let mut paired_traces = BTreeMap::new();
            for differential in [false, true] {
                let (w, _) =
                    readout::counterbalance(&base_p, &original, &circuits, 0., differential)?;
                let asset = FoundationWeightAsset::from_trained_weights(
                    &base_p,
                    w,
                    source.manifest().training_stage(),
                )?;
                let p = common::bind_candidate(7, &asset)?;
                asset.validate_against(&p)?;
                assert_eq!(p.synapses().len(), base_p.synapses().len());
                for (a, b) in p.synapses().iter().zip(base_p.synapses()) {
                    assert_eq!(
                        (a.source(), a.target(), a.route_index(), a.kind()),
                        (b.source(), b.target(), b.route_index(), b.kind())
                    );
                }
                assert_eq!(p.neuron_dynamics(), base_p.neuron_dynamics());
                assert_eq!(p.sensor_encoder(), base_p.sensor_encoder());
                assert_eq!(p.candidate_decoder(), base_p.candidate_decoder());
                assert_eq!(
                    GpuV11CausalState::for_phenotype(&p)?.dendritic_branches(),
                    &inherited
                );
                let mask = StageTrainableMask::from_synapse_indices(
                    &p,
                    &(0..p.synapses().len() as u32).collect::<Vec<_>>(),
                )?;
                let trainer = CpuFoundationTrainer::new(
                    p.clone(),
                    asset.clone(),
                    mask,
                    AdamWConfig::default(),
                )?;
                let checkpoint = serde_json::to_vec(&trainer.checkpoint())?;
                let q0 = sequence(&p, base, base, &branch_state, start, false);
                let baseline = trainer.evaluate_replay(&q0)?;
                let state_sha = format!("{:x}", Sha256::digest(serde_json::to_vec(&q0.initial)?));
                let mut rows = vec![];
                for (name, a) in &cases {
                    assert_eq!(sequence(&p, base, *a, &branch_state, start, true), q0);
                    let eval = trainer.evaluate_replay(&sequence(
                        &p,
                        base,
                        *a,
                        &branch_state,
                        start,
                        false,
                    ))?;
                    if differential {
                        assert_eq!(
                            &eval.final_activations,
                            paired_traces.get(name).ok_or("missing paired trace")?
                        );
                    } else {
                        paired_traces.insert(name.clone(), eval.final_activations.clone());
                    }
                    for (ci, context) in common::CONTEXTS.iter().enumerate() {
                        let range = ci * 8..ci * 8 + 8;
                        let samples=[32,33,34,36,40,48,64,65,66,68,72,80,96].map(|frame|{
                        let t=frame-1;let active=if(32..64).contains(&t){*a}else{base};
                        let phase=if t<32{"baseline"}else if t<64{"perturbation"}else{"washout"};
                        let logits=&eval.candidate_logits[t][range.clone()];let reference=&baseline.candidate_logits[t][range.clone()];
                        let innate=p.candidate_decoder().families().iter().map(|f|f.innate_contribution(assay::drives(active),assay::features(f.family(),context))).collect::<Vec<_>>();
                        let motors=p.candidate_decoder().families().iter().map(|f|{
                            let ids=p.synapses().iter().filter_map(|s|match s.kind(){CompiledSynapseKind::Decoder(c)if c.head()==DecoderHeadKind::ActionCandidate&&c.family()==f.family()=>Some(s.source()as usize),_=>None}).collect::<BTreeSet<_>>();
                            let values=ids.iter().map(|&m|eval.final_activations[t][m]).collect::<Vec<_>>();
                            json!({"family":f.family(),"min":values.iter().copied().fold(f32::INFINITY,f32::min),"max":values.iter().copied().fold(f32::NEG_INFINITY,f32::max),"mean":values.iter().sum::<f32>()/values.len()as f32,"max_abs_delta":ids.iter().map(|&m|(eval.final_activations[t][m]-baseline.final_activations[t][m]).abs()).fold(0_f32,f32::max)})
                        }).collect::<Vec<_>>();
                        let path_neurons=["rest_signal","inspect_signal","approach_reference","rest_reference","approach_signal"].map(|role|{let id=plan[role].as_u64().unwrap()as usize;json!({"role":role,"id":id,"activation":eval.final_activations[t][id],"baseline_activation":baseline.final_activations[t][id]})});
                        json!({"frame":frame,"phase":phase,"neural_logits":logits,"baseline_logits":reference,"neural_delta":logits.iter().zip(reference).map(|(x,b)|x-b).collect::<Vec<_>>(),"innate":innate,"total":logits.iter().zip(&innate).map(|(n,i)|n+i).collect::<Vec<_>>(),"motors":motors,"path_neurons":path_neurons})
                    });
                        rows.push(json!({"condition":name,"context":context,"perturbed_drives":a,"samples":samples}));
                    }
                }
                assert_eq!(checkpoint, serde_json::to_vec(&trainer.checkpoint())?);
                eprintln!("completed {factor:?} start={start} differential={differential}");
                arms.push(json!({"factor":factor,"motor_start":start,"initial_metabolic_load":0,"initial_activity_ema":0,"differential_readout":differential,"candidate_asset_digest":asset.digest(),"candidate_phenotype_hash":p.phenotype_hash(),"initial_state_sha256":state_sha,"plan":plan,"rows":rows,"optimizer_preserved":true,"inherited_branch_inputs_preserved":true,"declared_branch_plan_only":true,"input_cuts_exact":true,"readout_pair_traces_exact":true}));
            }
        }
    }
    Ok(
        json!({"schema":"fresh_branch_state_replay_v1","inspection":inspection,"seed":7,"backend":"native_cpu_replay","frames":{"warmup":32,"perturbation":32,"washout":32},"world_steps":0,"learning_steps":0,"families":base_p.candidate_decoder().families().iter().map(|f|f.family()).collect::<Vec<_>>(),"arms":arms}),
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
    fn branch_profiles_change_only_declared_fields() {
        let (_, p, _) = assay::frozen::source(7, true).unwrap();
        let inherited = GpuV11CausalState::for_phenotype(&p)
            .unwrap()
            .dendritic_branches()
            .clone();
        for (name, g, t) in [
            ("Inherited", 6., -1.),
            ("GainOnly", 0.25, -1.),
            ("ThresholdOnly", 6., 0.1),
            ("BoundedCombined", 0.25, 0.1),
            ("ZeroOutputAblation", 0., -1.),
        ] {
            let (d, plan) = branch_plan(&inherited, name, g, t).unwrap();
            assert_eq!(plan["branch_count"], 64);
            if name == "Inherited" {
                assert_eq!(d, inherited);
            } else {
                assert_ne!(d, inherited);
            }
            let encoded = serde_json::to_vec(&d).unwrap();
            assert_eq!(
                serde_json::from_slice::<DendriticBranchSet>(&encoded).unwrap(),
                d
            );
        }
    }
    #[test]
    fn starts_and_washout_are_explicit_and_cut_is_exact() {
        let (_, p, _) = assay::frozen::source(7, true).unwrap();
        let d = GpuV11CausalState::for_phenotype(&p)
            .unwrap()
            .dendritic_branches()
            .clone();
        let base = [0.1; 11];
        let mut a = base;
        a[1] = 0.9;
        for start in [-0.5, 0., 0.5] {
            let q = sequence(&p, base, a, &d, start, false);
            assert_eq!(q.ticks.len(), 96);
            assert_eq!(q.initial.dendrites, d);
            assert_eq!(q.ticks[0].encoded_inputs, q.ticks[95].encoded_inputs);
            assert_ne!(q.ticks[32].encoded_inputs, q.ticks[95].encoded_inputs);
            assert_eq!(
                sequence(&p, base, a, &d, start, true),
                sequence(&p, base, base, &d, start, false)
            );
        }
    }
}
