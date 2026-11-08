//! Detached fresh-asset assay; default only resolves and inspects compiled tuples.
//! Explicit --run executes native replay. --branch-plan reads a verified research
//! initial-state plan; it does not create a durable founder branch ABI.
#[allow(dead_code)]
#[path = "support/motor_balance_assay.rs"]
mod assay;
#[path = "support/dedicated_fatigue.rs"]
mod candidate;
#[allow(dead_code)]
#[path = "support/motor_probe_common.rs"]
mod common;
#[path = "support/motor_operating_point.rs"]
mod readout;
use alife_core::*;
use alife_gpu_backend::GpuV11CausalState;
use alife_training::{
    cpu::*, AdamWConfig, StageTrainableMask, TrainingReplayCandidate, TrainingSequence,
};
use candidate::Factor;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
const CONTEXTS: [&str; 4] = [
    "nutritious_contact",
    "aversive_contact",
    "distant",
    "zero_B",
];
#[cfg(test)]
const FEATURE_CONTROLS: [&str; 8] = [
    "nutritious_contact",
    "aversive_contact",
    "distant",
    "bearing_reversed",
    "zero_B",
    "max_B",
    "no_locomotion",
    "neutral_contact",
];
fn features(f: CandidateActionFamily, context: &str) -> CandidateFeatureVector {
    let mut x = assay::features(
        f,
        if context == "bearing_reversed" {
            "aversive_contact"
        } else {
            context
        },
    );
    if matches!(f, CandidateActionFamily::Idle | CandidateActionFamily::Rest) {
        return x;
    }
    match context {
        "aversive_contact" => {
            x.0[0] = -1.;
            x.0[1] = 0.;
        }
        "bearing_reversed" => {
            x.0[0] = -1.;
            x.0[1] = -1.;
        }
        "zero_B" => {
            for lane in candidate::B_LANES {
                x.0[lane as usize] = 0.;
            }
            x.0[15] = -1.;
        }
        "max_B" => {
            for lane in candidate::B_LANES {
                x.0[lane as usize] = 1.;
            }
            x.0[15] = -1.;
        }
        "no_locomotion" => {
            x = CandidateFeatureVector::zero().with_intrinsic_action_basis(ActionKind::Move);
        }
        _ => {}
    }
    x
}
fn sequence(
    p: &BrainPhenotype,
    base: [f32; 11],
    changed: [f32; 11],
    d: &DendriticBranchSet,
    start: f32,
    cut: bool,
) -> TrainingSequence {
    let mut q = assay::sequence(p, base, changed, "nutritious_contact", d, cut);
    q.ticks.extend(
        assay::sequence(p, base, base, "nutritious_contact", d, false)
            .ticks
            .into_iter()
            .take(32),
    );
    let mut rows = vec![];
    for context in CONTEXTS {
        for f in p.candidate_decoder().families() {
            let mut decoder_inputs = [0.; 54];
            decoder_inputs[..24].copy_from_slice(&features(f.family(), context).0);
            rows.push(TrainingReplayCandidate {
                family: f.family(),
                decoder_inputs,
                innate_bias: 0.,
            });
        }
    }
    for tick in &mut q.ticks {
        tick.candidates = rows.clone();
    }
    let motor = p.lobe_layout().region(LobeKind::ActionPlanning).unwrap();
    q.initial.activations[motor.start as usize..(motor.start + motor.len) as usize].fill(start);
    q
}
fn branch_state(
    inherited: &DendriticBranchSet,
    plan: Option<&Value>,
) -> Result<(DendriticBranchSet, Value)> {
    let Some(plan) = plan else {
        return Ok((
            inherited.clone(),
            json!({"profile":"Inherited","schema":"explicit_fresh_branch_plan_v1","branches":inherited.branches(),"branch_plan_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(inherited)?))}),
        ));
    };
    if plan["schema"] != "explicit_fresh_branch_plan_v1" {
        return Err("unknown external branch plan schema".into());
    }
    let state: DendriticBranchSet = serde_json::from_value(json!({"branches":plan["branches"]}))?;
    state.validate_for_neuron_count(2048)?;
    if state.branches().len() != inherited.branches().len() {
        return Err("branch count changed".into());
    }
    for (a, b) in state.branches().iter().zip(inherited.branches()) {
        if a.target != b.target || a.inputs != b.inputs {
            return Err("external plan changed branch coordinates/input weights".into());
        }
    }
    let sha = format!("{:x}", Sha256::digest(serde_json::to_vec(&state)?));
    if plan["branch_plan_sha256"] != sha {
        return Err("external branch digest mismatch".into());
    }
    Ok((state, plan.clone()))
}
fn conditions(base: [f32; 11]) -> Vec<(String, [f32; 11])> {
    let mut cases = vec![("balanced".into(), base)];
    for (lane, name) in assay::LABELS.iter().enumerate() {
        let mut a = base;
        a[lane] = if lane == 6 { 0.1 } else { 0.9 };
        cases.push((name.to_string(), a));
    }
    for (name, axes) in [
        ("hunger_fatigue", vec![0, 1]),
        ("fatigue_boredom", vec![1, 9]),
        ("fatigue_low_ATP", vec![1, 6]),
    ] {
        let mut a = base;
        for i in axes {
            a[i] = if i == 6 { 0.1 } else { 0.9 };
        }
        cases.push((name.into(), a));
    }
    let mut satiated = base;
    satiated[0] = 0.;
    cases.push(("satiated".into(), satiated));
    satiated[1] = 0.9;
    cases.push(("satiated_fatigued".into(), satiated));
    let mut comfortable = base;
    for lane in [0, 1, 2, 3, 4, 7, 9, 10] {
        comfortable[lane] = 0.;
    }
    cases.push(("comfortable".into(), comfortable));
    for fatigue in [0., 0.2, 0.4, 0.6, 1.] {
        let mut a = base;
        a[1] = fatigue;
        cases.push((format!("fatigue_level_{fatigue}"), a));
    }
    cases
}
fn run() -> Result<Value> {
    let (_, base_p, source) = assay::frozen::source(7, true)?;
    let (original, circuits) = assay::frozen::whole_care(&base_p)?;
    let (baseline, _) = readout::counterbalance(&base_p, &original, &circuits, 0., true)?;
    let args = std::env::args().collect::<Vec<_>>();
    let (baseline, reference_plan) = candidate::full_reference_calibration(
        &base_p,
        &baseline,
        &circuits,
        args.iter().any(|a| a == "--full-reference"),
    )?;
    let (_, inspection) = candidate::build(&base_p, &baseline, Factor::Dedicated)?;
    if !args.iter().any(|a| a == "--run") {
        return Ok(inspection);
    }
    let external = args
        .iter()
        .position(|a| a == "--branch-plan")
        .map(|i| args.get(i + 1).ok_or("--branch-plan needs a file"))
        .transpose()?;
    let external = external
        .map(|path| -> Result<Value> { Ok(serde_json::from_slice(&std::fs::read(path)?)?) })
        .transpose()?;
    let inherited = GpuV11CausalState::for_phenotype(&base_p)?
        .dendritic_branches()
        .clone();
    let (branches, branch_plan) = branch_state(&inherited, external.as_ref())?;
    let base = [0.1, 0.1, 0., 0., 0.1, 0.45, 0.75, 0., 0.1, 0.1, 0.];
    let cases = conditions(base);
    let starts = if args.iter().any(|a| a == "--all-starts") {
        vec![0., -0.5, 0.5]
    } else {
        vec![0.]
    };
    let mut arms = vec![];
    for start in starts {
        let mut dedicated_traces = BTreeMap::new();
        let factors = if args.iter().any(|a| a == "--main-only") {
            vec![Factor::Baseline, Factor::Dedicated]
        } else {
            Factor::ALL.to_vec()
        };
        for factor in factors {
            let (w, plan) = candidate::build(&base_p, &baseline, factor)?;
            let asset = FoundationWeightAsset::from_trained_weights(
                &base_p,
                w,
                source.manifest().training_stage(),
            )?;
            let p = common::bind_candidate(7, &asset)?;
            asset.validate_against(&p)?;
            if factor == Factor::Dedicated {
                assert!(asset.validate_against(&base_p).is_err());
            }
            assert_eq!(p.sensor_encoder(), base_p.sensor_encoder());
            assert_eq!(p.candidate_decoder(), base_p.candidate_decoder());
            assert_eq!(p.neuron_dynamics(), base_p.neuron_dynamics());
            assert_eq!(
                GpuV11CausalState::for_phenotype(&p)?.dendritic_branches(),
                &inherited
            );
            for (a, b) in p.synapses().iter().zip(base_p.synapses()) {
                assert_eq!(
                    (a.source(), a.target(), a.route_index(), a.kind()),
                    (b.source(), b.target(), b.route_index(), b.kind())
                );
            }
            let mask = StageTrainableMask::from_synapse_indices(
                &p,
                &(0..p.synapses().len() as u32).collect::<Vec<_>>(),
            )?;
            let trainer =
                CpuFoundationTrainer::new(p.clone(), asset.clone(), mask, AdamWConfig::default())?;
            let before = serde_json::to_vec(&trainer.checkpoint())?;
            let q0 = sequence(&p, base, base, &branches, start, false);
            let settled = trainer.evaluate_replay(&q0)?;
            let mut rows = vec![];
            for (name, a) in &cases {
                assert_eq!(sequence(&p, base, *a, &branches, start, true), q0);
                let eval =
                    trainer.evaluate_replay(&sequence(&p, base, *a, &branches, start, false))?;
                if factor == Factor::Dedicated {
                    dedicated_traces.insert(name.clone(), eval.final_activations.clone());
                }
                if factor == Factor::ReadoutCut {
                    assert_eq!(
                        &eval.final_activations,
                        dedicated_traces
                            .get(name)
                            .ok_or("missing dedicated trace")?
                    );
                }
                for (ci, context) in CONTEXTS.iter().enumerate() {
                    let range = ci * 8..ci * 8 + 8;
                    let samples=[32,33,34,36,40,48,64,65,72,80,96].map(|frame| {
                        let t=frame-1;
                        let n=&eval.candidate_logits[t][range.clone()];let b=&settled.candidate_logits[t][range.clone()];
                        let motors=[259,1955,899,1747,1759,candidate::SIGNAL,candidate::REFERENCE,1776].map(|id|json!({"id":id,"activation":eval.final_activations[t][id as usize],"baseline":settled.final_activations[t][id as usize]}));
                        let basis=candidate::physical_basis(features(CandidateActionFamily::Approach,context)).unwrap();
                        let contrast=eval.final_activations[t][candidate::SIGNAL as usize]-eval.final_activations[t][candidate::REFERENCE as usize];
                        json!({"frame":frame,"neural_logits":n,"baseline_logits":b,"delta":n.iter().zip(b).map(|(x,y)|x-y).collect::<Vec<_>>(),"path_neurons":motors,"basis_B":basis,"dedicated_contrast":contrast,"predicted_added_readout":if factor==Factor::ReadoutCut||matches!(factor,Factor::Baseline|Factor::SourceCutBaseline|Factor::RelayCutBaseline){0.}else{-4.*basis*contrast}})
                    });
                    rows.push(
                        json!({"condition":name,"context":context,"drives":a,"samples":samples}),
                    );
                }
            }
            assert_eq!(before, serde_json::to_vec(&trainer.checkpoint())?);
            eprintln!("completed {factor:?} start={start}");
            arms.push(json!({"factor":factor,"motor_start":start,"asset_digest":asset.digest(),"phenotype_hash":p.phenotype_hash(),"plan":plan,"rows":rows,"optimizer_unchanged":true,"input_cuts_exact":true,"readout_cut_traces_exact":factor==Factor::ReadoutCut,"initial_state_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(&q0.initial)?))}));
        }
    }
    Ok(
        json!({"schema":"dedicated_fatigue_nonnegative_basis_replay_v1","backend":"native_cpu_replay","world_steps":0,"learning_steps":0,"runtime_activation":false,"branch_state_is_not_durable_founder_abi":true,"source_baseline":"frozen_whole_care6_with_differential_approach_inspect_readouts","branch_plan":branch_plan,"reference_plan":reference_plan,"inspection":inspection,"frames":{"warmup":32,"perturbation":32,"washout":32},"families":["Idle","Rest","Inspect","Approach","Avoid","Contact","Ingest","Other"],"arms":arms}),
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
    fn actual_all_drive_coordinates_and_zero_prior_are_required() {
        let (_, p, _) = assay::frozen::source(7, true).unwrap();
        let (w, c) = assay::frozen::whole_care(&p).unwrap();
        let (w, _) = readout::counterbalance(&p, &w, &c, 0., true).unwrap();
        assert_eq!(candidate::drive_paths(&p).unwrap().len(), 11);
        for factor in Factor::ALL {
            let (next, _) = candidate::build(&p, &w, factor).unwrap();
            if factor == Factor::Baseline {
                assert_eq!(next, w);
            }
        }
        let (changed, _) = candidate::build(&p, &w, Factor::Dedicated).unwrap();
        let shared_contact = p.synapses().iter().position(|s| s.source()==candidate::REFERENCE && matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Approach && c.input_lane()==18)).unwrap();
        assert_eq!(w[shared_contact], 2.);
        assert_eq!(changed[shared_contact], 2.4);
        let mut bad = w.clone();
        bad[9697] = 0.1;
        assert!(candidate::build(&p, &bad, Factor::Dedicated).is_err());
        let (_, old, _) = assay::frozen::source(7, false).unwrap();
        assert!(candidate::drive_paths(&old).is_err());
    }
    #[test]
    fn full_reference_factor_only_changes_declared_decoder_rows() {
        let (_, p, _) = assay::frozen::source(7, true).unwrap();
        let (w, circuits) = assay::frozen::whole_care(&p).unwrap();
        let (w, _) = readout::counterbalance(&p, &w, &circuits, 0., true).unwrap();
        assert_eq!(
            candidate::full_reference_calibration(&p, &w, &circuits, false)
                .unwrap()
                .0,
            w
        );
        let (full, plan) = candidate::full_reference_calibration(&p, &w, &circuits, true).unwrap();
        assert!(!plan["edits"].as_array().unwrap().is_empty());
        for (i, s) in p.synapses().iter().enumerate() {
            if full[i] != w[i] {
                assert!(
                    matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && matches!(c.family(),CandidateActionFamily::Avoid|CandidateActionFamily::Contact|CandidateActionFamily::Ingest))
                );
            }
        }
        candidate::build(&p, &full, Factor::Dedicated).unwrap();
    }
    #[test]
    fn nonnegative_basis_is_invariant_to_signed_cues_and_zero_for_noops() {
        for context in FEATURE_CONTROLS {
            let x = features(CandidateActionFamily::Approach, context);
            let b = candidate::physical_basis(x).unwrap();
            assert!((0.0..=1.).contains(&b));
            let mut y = x;
            for lane in [0, 1, 3, 4, 5, 15, 16, 17, 21] {
                y.0[lane] = -y.0[lane];
            }
            assert_eq!(candidate::physical_basis(y).unwrap(), b);
            assert!(-4. * b <= 0.);
        }
        for context in ["zero_B", "no_locomotion"] {
            assert_eq!(
                candidate::physical_basis(features(CandidateActionFamily::Approach, context))
                    .unwrap(),
                0.
            );
        }
    }
    #[test]
    fn fresh_branch_plan_cannot_change_inputs_or_claim_wrong_digest() {
        let (_, p, _) = assay::frozen::source(7, true).unwrap();
        let d = GpuV11CausalState::for_phenotype(&p)
            .unwrap()
            .dendritic_branches()
            .clone();
        let (_, plan) = branch_state(&d, None).unwrap();
        assert_eq!(branch_state(&d, Some(&plan)).unwrap().0, d);
        let mut wrong = plan.clone();
        wrong["branch_plan_sha256"] = json!("wrong");
        assert!(branch_state(&d, Some(&wrong)).is_err());
        let mut wrong = plan;
        wrong["branches"][0]["inputs"][0]["weight"] = json!(2.);
        assert!(branch_state(&d, Some(&wrong)).is_err());
    }
    #[test]
    fn input_cut_and_washout_preserve_all_nonphysical_decoder_lanes() {
        let (_, p, _) = assay::frozen::source(7, true).unwrap();
        let d = GpuV11CausalState::for_phenotype(&p)
            .unwrap()
            .dendritic_branches()
            .clone();
        let base = [0.1; 11];
        let mut changed = base;
        changed[1] = 0.9;
        let q = sequence(&p, base, changed, &d, 0., false);
        assert_eq!(q.ticks.len(), 96);
        assert_eq!(q.ticks[0].encoded_inputs, q.ticks[95].encoded_inputs);
        assert_eq!(
            sequence(&p, base, changed, &d, 0., true),
            sequence(&p, base, base, &d, 0., false)
        );
        assert!(q
            .ticks
            .iter()
            .flat_map(|t| &t.candidates)
            .all(|c| c.decoder_inputs[24..].iter().all(|x| *x == 0.)));
    }
}
