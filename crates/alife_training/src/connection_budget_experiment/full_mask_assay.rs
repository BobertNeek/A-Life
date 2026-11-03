//! Full-mask preparation is compared with the actual native reference API.
//! No new validation cache or weight-bound view training interface is introduced.
use super::*;
use crate::cpu::CpuReplayExecution;
use std::hint::black_box;

fn collect(seed: u64, ticks: usize) -> (DisposableBudgetFounder, Vec<TrainingSequence>) {
    let mut e = DisposableBudgetFounder::new(seed).unwrap();
    let mut rows = Vec::new();
    for step in 0..ticks {
        for d in e.host.step(&e.trainer, 1.).unwrap().decisions {
            if step >= ticks / 2 {
                rows.push(TrainingSequence {
                    phenotype_hash: e.trainer.phenotype().phenotype_hash(),
                    initial: d.initial,
                    ticks: vec![d.replay_tick],
                    burn_in_ticks: 0,
                    memory_candidate_gain: e
                        .trainer
                        .phenotype()
                        .candidate_decoder()
                        .memory_channel()
                        .map_or(0., |m| m.max_candidate_gain()),
                });
            }
        }
    }
    assert!(!rows.is_empty());
    (e, rows)
}

#[test]
fn native_full_mask_cadence_retains_real_frame_bptt_and_adam() {
    let (mut reference, rows) = collect(70601, 8);
    let mut cadence = DisposableBudgetFounder::new(70601).unwrap();
    let mut retained = reference.trainer.checkpoint();
    retained.optimizer_step = 16_777_220;
    for i in 0..retained.weights.len() {
        retained.first_moment[i] = (i % 7) as f32 * 1e-7;
        retained.second_moment[i] = 1e-8 + (i % 5) as f32 * 1e-8;
        retained.update_ages[i] = [321, 19_800, 16_777_217, 29][i % 4];
    }
    reference
        .trainer
        .restore_checkpoint(retained.clone())
        .unwrap();
    cadence.trainer.restore_checkpoint(retained).unwrap();
    cadence
        .trainer
        .set_replay_execution(CpuReplayExecution::CadenceCsr);
    let plan = reference.capped_fan_in_plan(64, None).unwrap();
    let frozen = reference.compact_view(&plan).unwrap();
    for q in &rows {
        let a = reference.trainer.evaluate_replay(q).unwrap();
        let b = cadence.trainer.evaluate_replay(q).unwrap();
        assert_eq!(evaluation_bits(&[a]), evaluation_bits(&[b]));
        let adjoints = q
            .ticks
            .iter()
            .map(|t| vec![0.03; t.candidates.len()])
            .collect::<Vec<_>>();
        let a = reference
            .trainer
            .probe_replay_gradients(q, &adjoints)
            .unwrap();
        let b = cadence
            .trainer
            .probe_replay_gradients(q, &adjoints)
            .unwrap();
        assert_eq!(a.objective.to_bits(), b.objective.to_bits());
        assert_eq!(
            a.gradients.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            b.gradients.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
        reference.trainer.apply_gradients(&a.gradients).unwrap();
        cadence.trainer.apply_gradients(&b.gradients).unwrap();
        assert_eq!(
            identity(&reference.trainer.checkpoint()).unwrap(),
            identity(&cadence.trainer.checkpoint()).unwrap()
        );
    }
    assert_eq!(
        cadence.trainer.replay_execution(),
        CpuReplayExecution::CadenceCsr
    );
    // A bound detached view is deliberately invalid after Adam changes weights.
    assert!(reference.evaluate_compact(&frozen, &rows[0]).is_err());
}

#[test]
#[ignore = "bounded real-frame CPU component assay against genuine native reference"]
fn full_mask_components_and_native_reference() {
    let (mut e, rows) = collect(70602, 32);
    let retained = identity(&e.trainer.checkpoint()).unwrap();
    let plan = e.capped_fan_in_plan(64, None).unwrap();
    assert_eq!(plan.selected.iter().filter(|v| **v).count(), 24_576);
    let reused = e.compact_view(&plan).unwrap();
    let expected = rows
        .iter()
        .map(|q| e.trainer.evaluate_replay(q).unwrap())
        .collect::<Vec<_>>();
    let names = [
        "native_reference",
        "native_cadence",
        "full_cancellation",
        "compact_fresh",
        "compact_reused",
    ];
    let mut pairs = Vec::new();
    let mut components = Vec::new();
    for pair in 0..7 {
        let mut measured = serde_json::Map::new();
        let mut order = Vec::new();
        for offset in 0..5 {
            let which = (pair + offset) % 5;
            order.push(names[which]);
            let at = Instant::now();
            e.trainer.set_replay_execution(if which == 1 {
                CpuReplayExecution::CadenceCsr
            } else {
                CpuReplayExecution::Reference
            });
            let setup = at.elapsed().as_secs_f64();
            let at = Instant::now();
            let outputs = match which {
                0 | 1 => rows
                    .iter()
                    .map(|q| e.trainer.evaluate_replay(q).unwrap())
                    .collect::<Vec<_>>(),
                2 => rows
                    .iter()
                    .map(|q| {
                        e.trainer
                            .evaluate_replay(&e.intervene(&plan, q).unwrap())
                            .unwrap()
                    })
                    .collect(),
                3 => {
                    let view = e.compact_view(&plan).unwrap();
                    rows.iter()
                        .map(|q| e.evaluate_compact(&view, q).unwrap())
                        .collect()
                }
                _ => rows
                    .iter()
                    .map(|q| e.evaluate_compact(&reused, q).unwrap())
                    .collect(),
            };
            black_box(&outputs);
            let elapsed = at.elapsed().as_secs_f64();
            assert_eq!(evaluation_bits(&outputs), evaluation_bits(&expected));
            measured.insert(
                names[which].into(),
                serde_json::json!({"seconds":elapsed,"mode_setup_seconds":setup}),
            );
            // Returned output destruction and bit comparison stay outside timing.
        }
        pairs.push(serde_json::json!({"pair":pair,"order":order,"paths":measured}));
        e.trainer
            .set_replay_execution(CpuReplayExecution::Reference);
        let at = Instant::now();
        for q in &rows {
            q.validate_for(e.trainer.phenotype()).unwrap();
        }
        let validation = at.elapsed().as_secs_f64();
        let at = Instant::now();
        let copies = rows.clone();
        black_box(&copies);
        let clone = at.elapsed().as_secs_f64();
        let at = Instant::now();
        drop(copies);
        let clone_drop = at.elapsed().as_secs_f64();
        let at = Instant::now();
        for _ in &rows {
            e.validate_plan(&plan).unwrap();
        }
        let binding_per_frame = at.elapsed().as_secs_f64();
        let at = Instant::now();
        let view = e.compact_view(&plan).unwrap();
        black_box(&view);
        let aggregate_construction = at.elapsed().as_secs_f64();
        drop(view);
        let (view, construction) = e
            .trainer
            .measure_detached_construction(&plan.selected)
            .unwrap();
        assert_eq!(
            evaluation_bits(&[view.evaluate(&e.trainer, &rows[0]).unwrap()]),
            evaluation_bits(&expected[..1])
        );
        drop(view);
        let mut native = [0.; 3];
        for (q, expected) in rows.iter().zip(&expected) {
            let (out, times) = e.trainer.measure_replay_components(q).unwrap();
            assert_eq!(
                evaluation_bits(&[out]),
                evaluation_bits(std::slice::from_ref(expected))
            );
            for i in 0..3 {
                native[i] += times[i];
            }
        }
        components.push(serde_json::json!({"sample":pair,"sequence_validation_seconds":validation,"sequence_clone_seconds":clone,"sequence_clone_drop_seconds":clone_drop,
            "full_plan_sha_binding_per_frame_seconds":binding_per_frame,"aggregate_view_construction_seconds":aggregate_construction,
            "graph_construction_seconds":construction[0],"cadence_index_construction_seconds":construction[1],"weight_bit_snapshot_seconds":construction[2],
            "native_validation_seconds":native[0],"native_raw_forward_seconds":native[1],"native_tape_destruction_seconds":native[2]}));
    }
    assert_eq!(identity(&e.trainer.checkpoint()).unwrap(), retained);
    let result = serde_json::json!({"seed":70602,"world_ticks":32,"held_out_rows":rows.len(),"source_checkpoint_identity":retained,"frame_identity":identity(&rows).unwrap(),"pairs":pairs,"isolated_component_calls":components,
        "limits":["real baseline-world frames; forward-only timing; no experimental rollout or learning throughput",
            "all paths compared to native Reference output bits; full cancellation is not the genuine reference baseline",
            "mode setup recorded separately and included in overall assay wall; native cadence table is reused across all rows in its timed call",
            "components are separate actual primitive calls, not an additive decomposition of the separately timed public call",
            "aggregate construction includes SHA binding plus graph/index construction; inner component scopes overlap that aggregate",
            "output destruction and parity checks outside path timing; frequency/allocator/cache history uncontrolled",
            "no new prepared training API; existing native cadence handles full graph BPTT/Adam; no accepted brain or GPU"]});
    eprintln!("full_mask_native_reference={result}");
    if let Some(path) = std::env::var_os("ALIFE_FULL_MASK_ASSAY_DIR") {
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            std::path::Path::new(&path).join("trials.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
    }
}
