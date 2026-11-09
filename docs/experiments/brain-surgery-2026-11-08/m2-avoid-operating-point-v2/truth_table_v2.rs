//! Link as a child module of grouped_host_tests in m2-taste-baseline-v1.
//! Detached authored-source assay. No host step, checkpoint read, or learning.
use alife_core::*;
use alife_training::*;
use alife_training::cpu::CpuFoundationTrainer;
#[path = "source/src/cpu/grouped_host_fixture.rs"]
mod recipe;
const SEED:u64=0x0347_5250_484F_5354;
use crate::{TrainingInitialState, TrainingReplayCandidate, TrainingReplayTick, TrainingSequence};
use serde_json::json;

fn main() {
    use std::io::Write;
    let mut journal=std::fs::OpenOptions::new().write(true).create_new(true).open("v2-rows.jsonl").unwrap();
    let mut emit=|v:serde_json::Value| {writeln!(journal,"{}",v).unwrap();journal.flush().unwrap();journal.sync_data().unwrap();};
    let mut attempted=0usize;
    let mut completed=0usize;
    let baseline = recipe::build(SEED, true).unwrap();
    let repaired = recipe::build_matched_reference(SEED, true).unwrap();
    let changes: Vec<_> = baseline.asset.weights().iter().zip(repaired.asset.weights())
        .enumerate().filter(|(_, (a,b))| a.to_bits() != b.to_bits()).map(|(i,_)|i).collect();
    assert_eq!(changes.len(), 6);
    assert!(changes.iter().all(|&i| baseline.asset.weights()[i] == 0.));
    assert_eq!(baseline.phenotype.neuron_dynamics(), repaired.phenotype.neuron_dynamics());
    assert_eq!(baseline.phenotype.sensor_encoder(), repaired.phenotype.sensor_encoder());
    assert_eq!(baseline.phenotype.candidate_decoder(), repaired.phenotype.candidate_decoder());
    let avoid_output: Vec<_> = repaired.phenotype.synapses().iter().enumerate()
        .filter(|(_,s)|s.kind()==CompiledSynapseKind::Recurrent && s.route_index()==5 && s.source()==911 && s.target()==1807)
        .map(|(i,_)|i).collect();
    assert_eq!(avoid_output.len(),1);
    assert!(changes.contains(&avoid_output[0]));
    let mut sham_weights=repaired.asset.weights().to_vec();
    sham_weights[avoid_output[0]]=baseline.asset.weights()[avoid_output[0]];
    let sham_asset=FoundationWeightAsset::from_trained_weights(&repaired.phenotype,sham_weights,TrainingStageManifest::bootstrap()).unwrap();
    // Sham is an explicitly edited fresh detached array, never acquired weights.
    let (sham_phenotype,_)=PhenotypeCompiler::compile_n2048_grounded_drive_basis_candidate(repaired.inputs.genome().clone(),repaired.inputs.development().clone(),sham_asset.clone()).unwrap();
    assert_eq!(sham_phenotype.neuron_dynamics(),repaired.phenotype.neuron_dynamics());
    assert_eq!(sham_phenotype.sensor_encoder(),repaired.phenotype.sensor_encoder());
    assert!(sham_phenotype.synapses().iter().zip(sham_asset.weights()).all(|(s,w)|s.genetic_weight().to_bits()==w.to_bits()));
    let arms=[("unchanged",&baseline.phenotype,&baseline.asset),
        ("matched_reference",&repaired.phenotype,&repaired.asset),
        ("avoid_reference_output_sham",&sham_phenotype,&sham_asset)];
    let mut results=vec![];
    for (arm,p,asset) in arms {
        emit(json!({"phase":"arm_admission_attempt","arm":arm,"attempted_iterations":attempted,"completed_iterations":completed,"phenotype_hash":p.phenotype_hash()}));
        let trainer=CpuFoundationTrainer::new(p.clone(),asset.clone(),StageTrainableMask::recurrent_only(p).unwrap(),AdamWConfig::default()).unwrap();
        let motor=p.candidate_decoder().motor_start();
        let (gain,threshold)=p.foundation_abi().fresh_dendritic_parameters();
        let dendrites=DendriticBranchSet::new((0..64).map(|i|DendriticBranch::new(motor+i,threshold,gain,vec![DendriticInputRef::new(motor+i,1.).unwrap(),DendriticInputRef::new(motor+i+1,1.).unwrap()]).unwrap()).collect()).unwrap();
        for hunger in [0_f32,0.9] { for fear in [0_f32,0.9] { for pain in [0_f32,0.9] {
            // Deliberately synthetic constant receptors, not a saved creature's context.
            // Same encoder arithmetic as input.rs, all unspecified raw lanes zero.
            let n=p.neuron_count() as usize;
            let mut encoded=vec![0_f32;n];
            for a in p.sensor_encoder().assignments() {
                let raw=if a.source_group()==SensorEncoderSourceGroup::Homeostasis {
                    match a.source_index() {0=>hunger,2=>fear,3=>pain,_=>0.}
                } else {0.};
                let source=if a.source_group()==SensorEncoderSourceGroup::Homeostasis {raw*0.5} else {raw};
                let (lo,hi)=a.clamp_range();
                encoded[a.target_neuron() as usize]+=(source*a.scale()+a.bias()).clamp(lo,hi);
            }
            for value in &mut encoded {*value=(*value*0.57-0.017728).clamp(-1.,1.);}
            let mut candidates=vec![];
            for family in [CandidateActionFamily::Avoid,CandidateActionFamily::Ingest,CandidateActionFamily::Contact] {
                for cue in [-0.85,0.,0.85] {let mut inputs=[0.;54];inputs[15]=cue;candidates.push(TrainingReplayCandidate{family,decoder_inputs:inputs,innate_bias:0.});}
            }
            let tick=TrainingReplayTick{encoded_inputs:encoded,projection_gain:0.5,local_threshold_shift:0.25,microstep_count:p.microstep_count().into(),enabled_routes:vec![true;p.projections().len()],effective_weight_offsets:vec![0.;p.synapses().len()],structural_synapses:vec![],candidates};
            let q=TrainingSequence{phenotype_hash:p.phenotype_hash(),initial:TrainingInitialState{activations:vec![0.;n],activity_ema:vec![0.;n],metabolic_load:vec![0.;n],dendrites:dendrites.clone()},ticks:vec![tick;48],burn_in_ticks:0,memory_candidate_gain:p.candidate_decoder().memory_channel().unwrap().max_candidate_gain()};
            attempted+=48;
            emit(json!({"phase":"context_attempt","arm":arm,"hunger":hunger,"fear":fear,"pain":pain,"attempted_iterations":attempted,"completed_iterations":completed}));
            let out=trainer.evaluate_replay(&q).unwrap();
            completed+=48;
            let mut samples=vec![];
            for i in [0,3,11,47] {
                let a=&out.final_activations[i];
                let contrast=a[1793]+a[1796]-2.*a[1807];
                let logits=&out.candidate_logits[i];
                assert!(logits.iter().all(|x|x.is_finite()));
                assert!(logits[1].abs()<1e-6);
                assert!((logits[0]+logits[2]).abs()<1e-5);
                assert!((logits[2]-(-10.*0.85*contrast)).abs()<1e-5);
                samples.push(json!({"iteration":i+1,"avoid_pain_motor":a[1793],"avoid_fear_motor":a[1796],"avoid_reference_motor":a[1807],"avoid_contrast":contrast,"logits":logits}));
            }
            let row=json!({"phase":"context_complete","arm":arm,"hunger":hunger,"fear":fear,"pain":pain,"samples":samples,"attempted_iterations":attempted,"completed_iterations":completed});
            emit(row.clone());
            results.push(row);
        }}}
    }
    emit(json!({"phase":"complete","attempted_iterations":attempted,"completed_iterations":completed}));
    eprintln!("AVOID_TRUTH_TABLE {}",json!({"schema":"detached-avoid-truth-table-v1","source_iterations":1152,"world_steps":0,"learning_updates":0,"cue_order":[-0.85,0.,0.85],"family_order":["Avoid","Ingest","Contact"],"scope":"synthetic fixed receptor and cue context; native source recurrence/dendrites; no biological or behavior inference","results":results}));
}
