//! Prospective ordinary physiology/CPU-host assay. Default prints the plan only.
//! No life/seed is executed unless an explicit bounded run option is supplied.
#[allow(dead_code)]
#[path = "support/motor_balance_assay.rs"]
mod assay;
#[allow(dead_code)]
#[path = "support/dedicated_fatigue.rs"]
mod dedicated;
#[path = "support/physiology_host.rs"]
mod host_fixture;
#[path = "support/motor_operating_point.rs"]
mod readout;
use alife_core::*;
use alife_training::{cpu::*, TrainingSequence};
use host_fixture::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const PROPOSED_WORLD_SEED: u64 = 0xB001_1108;
const TRACE: [usize; 25] = [
    256, 259, 262, 383, 1952, 1955, 1958, 1967, 896, 897, 899, 900, 902, 1747, 1759, 1766, 1775,
    1776, 1779, 1791, 1793, 1814, 1823, 1824, 1839,
];
fn digest<T: serde::Serialize>(v: &T) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(v)?)))
}
fn plan() -> Value {
    json!({"schema":"ordinary_physiology_b_protocol_v1","status":"source_only_unexecuted","founder_dependency":"a2424a76 bounded tag9; founder qualification passed; this assay unexecuted","arms":arms(),"cases":CASES,"factor_count":8,"case_count":10,"shared_controls":"full differential references, B1 Contact prior, all candidate availability and actual physiology","fixture_seed":7,"fixture_ticks_per_case":1,"fixture_max_world_ticks":80,"proposed_world_seed":PROPOSED_WORLD_SEED,"proposed_world_ticks_per_case":8,"proposed_max_world_ticks":640,"training_optimizer_steps":0,"normal_somatic_learning_retained":true,"scope":"CPU host only; no default GPU runtime activation, no evolutionary or learned-success claim","expansion_axes_deferred":["hot/cold","social","reproduction","unbound extension"],"tonic_warning":"383 receives the real regional gate offset; its fixed-context silence was not biological silence"})
}
/// Scalar metadata only. Never emit checkpoint banks, latent vectors or private
/// learned weights. Physical successes and representative episode coverage differ.
fn resident_summary(cp: &CpuCanonicalCheckpoint) -> Value {
    let r = &cp.residents[0];
    let mut families = std::collections::BTreeMap::<u16, usize>::new();
    let mut kinds = std::collections::BTreeMap::<u8, usize>::new();
    let mut observations = 0u64;
    let mut sequences = std::collections::BTreeSet::new();
    for record in &r.memory.records {
        *families.entry(record.family_raw).or_default() += 1;
        *kinds.entry(record.action_kind_raw).or_default() += 1;
        observations += u64::from(record.observation_count);
        sequences.insert(record.sealed_sequence_id_raw);
    }
    let stats = |values: &[f32]| {
        json!({
            "count": values.len(), "nonzero_count": values.iter().filter(|&&x|x!=0.).count(),
            "absolute_sum": values.iter().map(|x|f64::from(x.abs())).sum::<f64>(),
            "min": values.iter().copied().reduce(f32::min),
            "max": values.iter().copied().reduce(f32::max)
        })
    };
    let branches = r.neural.neural.dendrites.branches();
    let gains = branches.iter().map(|b| b.output_gain).collect::<Vec<_>>();
    let thresholds = branches.iter().map(|b| b.threshold).collect::<Vec<_>>();
    json!({"memory_records":r.memory.records.len(),"memory_generation":r.memory.generation,
        "memory_family_counts":families,"memory_action_kind_counts":kinds,
        "memory_observation_count":observations,"unique_stored_source_sequences":sequences.len(),
        "last_observed_sequence":r.memory.last_observed_sequence_id_raw,
        "sealed_decision_count":r.next_sequence.saturating_sub(1),
        "memory_merge_count":r.memory.merge_count,"memory_eviction_count":r.memory.eviction_count,
        "staged_memory_records":r.staged_memory.as_ref().map_or(0,|m|m.records.len()),
        "learned_weight_generation":r.neural.weight_generation,
        "lifetime_weights":stats(&r.neural.lifetime),"fast_weights":stats(&r.neural.fast),
        "dendritic_gains":stats(&gains),"dendritic_thresholds":stats(&thresholds),
        "sleep_state":r.sleep.state(),"scalar_metadata_only":true})
}
fn trial(seed: u64, arm: Arm, case: Case, ticks: usize) -> Result<Value> {
    if ![1, 8].contains(&ticks) {
        return Err("bounded protocol permits only one or eight ticks".into());
    }
    let Fixture {
        mut host,
        trainer,
        organism,
        recipe,
    } = fixture(seed, arm, case)?;
    let p = trainer.phenotype();
    let sentinel = digest(&trainer.checkpoint())?;
    let mut rows = vec![];
    let mut stopped = None;
    let mut first_input_signature = None;
    for _ in 0..ticks {
        let checkpoint = host.checkpoint(&trainer)?;
        let before = host
            .world()
            .organism_registry()
            .get(organism)
            .ok_or("learner missing")?
            .clone();
        if !before.lifecycle().is_alive()
            || before.biochemistry().body.health < 0.5
            || before.biochemistry().body.energy < 0.05
        {
            stopped = Some("bounded welfare/energy stop");
            break;
        }
        let biology = *before.biochemistry();
        let receptors = biology.neural_receptor_frame(before.phenotype())?;
        let effects =
            NeuralReceptorEffects::from_frame(&receptors, &NeuralReceptorPhenotype::compile(p)?)?;
        let before_cognitive = *before.cognitive_work();
        let pre_position = host
            .world()
            .entity(before.world_entity_id())
            .ok_or("body missing")?
            .position;
        let step = host.step(&trainer, 1.)?;
        let mut decisions = vec![];
        for row in step.decisions {
            let frame = row.patch.pre_action().perception();
            assert_eq!(frame.homeostasis(), &biology.homeostasis);
            if first_input_signature.is_none() {
                first_input_signature = Some(digest(&json!({
                    "encoded":row.replay_tick.encoded_inputs,
                    "candidates":row.replay_tick.candidates,
                    "projection_gain":row.replay_tick.projection_gain,
                    "threshold":row.replay_tick.local_threshold_shift,
                    "microsteps":row.replay_tick.microstep_count,
                    "routes":row.replay_tick.enabled_routes,
                    "biology":biology
                }))?);
            }
            assert_eq!(row.replay_tick.projection_gain, effects.projection_gain);
            assert_eq!(
                row.replay_tick.local_threshold_shift,
                effects.local_threshold_shift
            );
            let offset = ((effects.structural_growth_gate
                + effects.sleep_gate
                + effects.consolidation_gate)
                / 3.
                - 0.5)
                * 0.05;
            assert!(!p
                .sensor_encoder()
                .assignments()
                .iter()
                .any(|a| a.target_neuron() == 383));
            assert_eq!(row.replay_tick.encoded_inputs[383], offset.clamp(-1., 1.));
            let resident = &checkpoint.residents[0];
            let budget = cpu_activity_budget(
                p,
                CpuActivityAnchor {
                    organism_id: organism,
                    tick: frame.tick(),
                    sequence_cursor: resident.next_sequence,
                    previous_dispatch_generation: resident.neural.dispatch_generation,
                    source_frame_digest: resident.activity.source_frame_digest,
                    frame_digest: frame.frame_digest().0,
                },
                biology.homeostasis.drives.brain_atp,
                frame.candidates().len() as u32,
                row.replay_tick.candidates.len() as u32,
            )?;
            assert_eq!(
                row.replay_tick.microstep_count,
                u32::from(budget.microsteps)
            );
            assert_eq!(row.replay_tick.enabled_routes, budget.enabled_routes);
            assert_eq!(row.brain_work, budget.work);
            assert!(budget.affordable);
            let evaluation = trainer.evaluate_replay(&TrainingSequence {
                phenotype_hash: p.phenotype_hash(),
                initial: row.initial.clone(),
                ticks: vec![row.replay_tick.clone()],
                burn_in_ticks: 0,
                memory_candidate_gain: p
                    .candidate_decoder()
                    .memory_channel()
                    .map_or(0., |m| m.max_candidate_gain()),
            })?;
            assert_eq!(evaluation.final_activations[0], row.final_features);
            let candidates=frame.candidates().iter().zip(&row.replay_tick.candidates).enumerate().map(|(i,(c,r))|{
                let mut neural=0.;
                for (id,s) in p.synapses().iter().enumerate(){if let CompiledSynapseKind::Decoder(coord)=s.kind(){if coord.head()==DecoderHeadKind::ActionCandidate && coord.family()==c.family {neural+=row.final_features[s.source() as usize]*(r.decoder_inputs[coord.input_lane() as usize]*(trainer.weights()[id]+row.replay_tick.effective_weight_offsets[id]));}}}
                json!({"index":i,"kind":c.kind,"family":c.family,"features":c.features,"decoder_inputs":r.decoder_inputs.as_slice(),"innate_bias":r.innate_bias,"neural_action_readout":neural,"total_logit":evaluation.candidate_logits[0][i],"candidate":c})
            }).collect::<Vec<_>>();
            let path=TRACE.map(|id|json!({"id":id,"encoded_input":row.replay_tick.encoded_inputs[id],"activation":row.final_features[id]}));
            let tonic_effective = [7679,7841,7842,7844,7845,7847].map(|id|json!({
                "index":id,"source":p.synapses()[id].source(),"target":p.synapses()[id].target(),
                "founder_weight":trainer.weights()[id],"somatic_offset":row.replay_tick.effective_weight_offsets[id],
                "effective_weight":trainer.weights()[id]+row.replay_tick.effective_weight_offsets[id]
            }));
            decisions.push(json!({"sampling_counter":row.sampling_counter,"receptor_effects":effects,"receptors":receptors,"region_gate_offset":offset,"projection_gain":row.replay_tick.projection_gain,"threshold_shift":row.replay_tick.local_threshold_shift,"microsteps":row.replay_tick.microstep_count,"enabled_routes":row.replay_tick.enabled_routes,"brain_work":row.brain_work,"available_atp_q16":budget.available_atp_q16,"debit_atp_q16":budget.debit_atp_q16,"path":path,"tonic_effective_weights":tonic_effective,"candidates":candidates,"action":row.action,"selected_bundle":row.patch.selected_bundle(),"outcome":row.patch.outcome(),"biological_reward":row.biological_reward,"production_replay_exact":true}));
        }
        let after = host
            .world()
            .organism_registry()
            .get(organism)
            .ok_or("learner missing after step")?;
        let after_summary = resident_summary(&host.checkpoint(&trainer)?);
        rows.push(json!({"resident_before":resident_summary(&checkpoint),"resident_after":after_summary,"tick_before":step.tick_before,"tick_after":step.tick_after,"biology_before":biology,"biology_after":after.biochemistry(),"cognitive_before":before_cognitive,"cognitive_after":after.cognitive_work(),"position_before":pre_position,"position_after":host.world().entity(after.world_entity_id()).ok_or("body missing after step")?.position,"sleeping_organisms":step.sleeping_organisms,"sleep_consolidations":step.sleep_consolidations,"decisions":decisions}));
    }
    assert_eq!(sentinel, digest(&trainer.checkpoint())?);
    Ok(
        json!({"recipe":recipe,"rows":rows,"first_input_signature":first_input_signature,"early_stop":stopped,"founder_optimizer_unchanged":true,"resident_learning_not_disabled":true}),
    )
}
fn run() -> Result<Value> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args == ["--plan"] {
        return Ok(plan());
    }
    let (seed, ticks, receipt_dir) =
        match args.as_slice() {
            [mode, dir] if mode == "--run-first-tick-fixtures" => (7, 1, dir),
            [mode, dir] if mode == "--run-eight-tick-scenes" => (PROPOSED_WORLD_SEED, 8, dir),
            _ => return Err(
                "expected --plan or --run-first-tick-fixtures DIR or --run-eight-tick-scenes DIR"
                    .into(),
            ),
        };
    std::fs::create_dir(receipt_dir)?;
    std::fs::write(
        std::path::Path::new(receipt_dir).join("protocol.json"),
        serde_json::to_vec_pretty(&plan())?,
    )?;
    let mut trials = vec![];
    let mut matched = std::collections::BTreeMap::new();
    for arm in arms() {
        for case in CASES {
            let result = trial(seed, arm, case, ticks)?;
            std::fs::write(
                std::path::Path::new(receipt_dir).join(format!("case-{:02}.json", trials.len())),
                serde_json::to_vec_pretty(&result)?,
            )?;
            let signature = result["first_input_signature"].as_str().ok_or("case had no awake first dispatch; keep receipt and review rather than claim a motor test")?.to_string();
            if let Some(previous) = matched.get(case.name) {
                assert_eq!(
                    previous, &signature,
                    "first production inputs differed across neural factors"
                );
            } else {
                matched.insert(case.name, signature);
            }
            trials.push(result);
            eprintln!("completed {arm:?} {} ticks={ticks}", case.name);
        }
    }
    Ok(
        json!({"schema":"ordinary_physiology_b_results_v1","backend":"native_cpu_canonical_host","seed":seed,"ticks_per_case":ticks,"plan":plan(),"first_inputs_exact_across_arms":true,"trials":trials}),
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
    fn prospective_counts_do_not_launch_a_world() {
        assert_eq!(arms().len() * CASES.len(), 80);
        assert_eq!(plan()["proposed_max_world_ticks"], 640);
    }
    #[test]
    fn newborn_profiles_derive_real_drives_and_do_not_change_genes() {
        let arm = Arm {
            bounded: true,
            dedicated_b: true,
            tonic_bridges: true,
        };
        let f = fixture(7, arm, CASES[0]).unwrap();
        let record = f.host.world().organism_registry().get(f.organism).unwrap();
        let old = *record.biochemistry();
        let changed = chemistry(record.phenotype(), old, CASES[1], 0.75).unwrap();
        assert!(changed.homeostasis.drives.fatigue > old.homeostasis.drives.fatigue);
        assert_eq!(
            changed.homeostasis.drives.hunger,
            old.homeostasis.drives.hunger
        );
        assert_eq!(changed.homeostasis.hormones, old.homeostasis.hormones);
        assert_eq!(
            changed.homeostasis.drives.reproductive_drive,
            old.homeostasis.drives.reproductive_drive
        );
        assert_eq!(
            changed.homeostasis.drives.extension[1],
            old.homeostasis.drives.extension[1]
        );
        assert_eq!(f.host.world().tick(), Tick::ZERO);
        let cp = f.host.checkpoint(&f.trainer).unwrap();
        let restored = CpuCanonicalHost::from_checkpoint(cp.clone(), &f.trainer).unwrap();
        assert_eq!(
            digest(&cp).unwrap(),
            digest(&restored.checkpoint(&f.trainer).unwrap()).unwrap()
        );
    }
    #[test]
    fn tonic_five_edges_only_and_branch_controls_keep_fixed_coordinates() {
        let a = fixture(
            7,
            Arm {
                bounded: false,
                dedicated_b: true,
                tonic_bridges: true,
            },
            CASES[0],
        )
        .unwrap();
        let b = fixture(
            7,
            Arm {
                bounded: true,
                dedicated_b: true,
                tonic_bridges: true,
            },
            CASES[0],
        )
        .unwrap();
        assert_eq!(a.trainer.weights(), b.trainer.weights());
        assert_eq!(
            a.host
                .world()
                .organism_registry()
                .get(a.organism)
                .unwrap()
                .biochemistry(),
            b.host
                .world()
                .organism_registry()
                .get(b.organism)
                .unwrap()
                .biochemistry()
        );
        assert_eq!(
            a.trainer.phenotype().sensor_encoder(),
            b.trainer.phenotype().sensor_encoder()
        );
        for (x, y) in a
            .trainer
            .phenotype()
            .synapses()
            .iter()
            .zip(b.trainer.phenotype().synapses())
        {
            assert_eq!(
                (
                    x.source(),
                    x.target(),
                    x.kind(),
                    x.alpha(),
                    x.receptor_index()
                ),
                (
                    y.source(),
                    y.target(),
                    y.kind(),
                    y.alpha(),
                    y.receptor_index()
                )
            );
        }
        let (off, _) = tonic_factor(b.trainer.phenotype(), b.trainer.weights(), false).unwrap();
        assert_eq!(
            off.iter()
                .zip(b.trainer.weights())
                .filter(|(x, y)| x != y)
                .count(),
            5
        );
        let old = a.host.checkpoint(&a.trainer).unwrap();
        let new = b.host.checkpoint(&b.trainer).unwrap();
        assert_ne!(
            old.residents[0].neural.neural.dendrites,
            new.residents[0].neural.neural.dendrites
        );
    }
    #[test]
    fn actual_atp_budget_not_fixed_four_microsteps() {
        let f = fixture(
            7,
            Arm {
                bounded: true,
                dedicated_b: true,
                tonic_bridges: true,
            },
            CASES[0],
        )
        .unwrap();
        let p = f.trainer.phenotype();
        let anchor = CpuActivityAnchor {
            organism_id: f.organism,
            tick: Tick::ZERO,
            sequence_cursor: 1,
            previous_dispatch_generation: 0,
            source_frame_digest: [0; 4],
            frame_digest: p.phenotype_hash().0,
        };
        let high = cpu_activity_budget(p, anchor, 0.75, 8, 8).unwrap();
        let low = cpu_activity_budget(p, anchor, 0.05, 8, 8).unwrap();
        assert!(low.microsteps < high.microsteps);
        assert!(low.available_atp_q16 < high.available_atp_q16);
        assert!(high.debit_atp_q16 > 0);
    }
}
