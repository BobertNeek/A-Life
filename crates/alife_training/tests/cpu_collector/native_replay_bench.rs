//! Explicit cloud benchmark using native sampled decisions from fresh fixtures.
//! It measures replay learning, not trained competence or GPU throughput.
use super::*;
use alife_training::{PpoConfig, PpoValueHeadCheckpoint};
use sha2::{Digest, Sha256};
use std::time::Instant;

#[test]
#[ignore = "native CPU replay benchmark; explicitly run with a writable output directory"]
fn native_collected_replay_ledger() {
    let mut results = Vec::new();
    let mut checkpoints = Vec::new();
    for (name, profile, window_ticks, effective_batch_size) in [
        ("object-slots", SensorProfile::GroundedObjectSlotsV1, 32, 1),
        (
            "terrain-vision",
            SensorProfile::GroundedTerrainVisionV1,
            24,
            2,
        ),
    ] {
        let (game, mut trainer, _) = new_game_with_profile_fixture(None, profile);
        let mut initial_actor = trainer.checkpoint();
        initial_actor.optimizer_step = 16_777_220;
        initial_actor.stage_mask = StageTrainableMask::from_synapse_indices(
            trainer.phenotype(),
            &(0..trainer.weights().len() as u32)
                .filter(|i| i % 4 != 0)
                .collect::<Vec<_>>(),
        )
        .unwrap();
        for i in 0..initial_actor.weights.len() {
            initial_actor.update_ages[i] = [321, 19_800, 16_777_217, 29][i % 4];
            initial_actor.first_moment[i] = (i % 7) as f32 * 1e-7;
            initial_actor.second_moment[i] = (i % 11) as f32 * 1e-9;
        }
        trainer.restore_checkpoint(initial_actor.clone()).unwrap();
        let mut host = CpuCanonicalHost::new_cohort(game, &trainer, 42).unwrap();
        let collection_started = Instant::now();
        let mut decisions = Vec::new();
        let mut world_steps = 0;
        while decisions.len() < 65 && world_steps < 512 {
            decisions.extend(host.step(&trainer, 0.9).unwrap().decisions);
            world_steps += 1;
        }
        let collection_ms = collection_started.elapsed().as_secs_f64() * 1000.;
        assert_eq!(
            decisions.len(),
            65,
            "fixture must retain a real bootstrap action"
        );
        let organism = host.world().organism_registry().iter().next().unwrap();
        assert!(organism.lifecycle().is_alive());
        let phenotype = organism.phenotype().clone();
        let width = trainer.phenotype().neuron_count() as usize + 1;
        let mut parameters = vec![0.; width * 3];
        for i in 0..width {
            parameters[i] = (i % 7) as f32 * 0.0001;
            parameters[width + i] = (i % 5) as f32 * 1e-6;
            parameters[width * 2 + i] = (i % 11) as f32 * 1e-8;
        }
        let initial_value = PpoValueHeadCheckpoint {
            feature_count: trainer.phenotype().neuron_count(),
            optimizer_step: 19_800,
            parameters,
            last_updated_policy_version: Some(7399),
        };
        let value = CpuValueHead::from_checkpoint(initial_value.clone()).unwrap();
        let config = PpoConfig {
            temperature: 0.9,
            epochs: 2,
            ..Default::default()
        };
        let prepare_started = Instant::now();
        let windows = cpu_training_windows(
            &decisions,
            None,
            &trainer,
            &value,
            &phenotype,
            7400,
            config,
            window_ticks,
            8,
        )
        .unwrap();
        let preparation_ms = prepare_started.elapsed().as_secs_f64() * 1000.;
        let private = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
        let decisions_bytes = serde_json::to_vec(&decisions).unwrap();
        let mut samples = Vec::new();
        let mut retained_result = None;
        for run in 0..8 {
            trainer.restore_checkpoint(initial_actor.clone()).unwrap();
            let mut value = CpuValueHead::from_checkpoint(initial_value.clone()).unwrap();
            let started = Instant::now();
            let receipt = train_cpu_residual_cohort(
                &mut trainer,
                &mut value,
                windows.len(),
                effective_batch_size,
                |i| Ok(windows[i].clone()),
                config,
                7400,
            )
            .unwrap();
            let milliseconds = started.elapsed().as_secs_f64() * 1000.;
            if run > 0 {
                samples.push(milliseconds);
            }
            if run == 1 {
                retained_result = Some(serde_json::json!({"actor":trainer.checkpoint(),
                    "critic":value.checkpoint(),"receipt":receipt}));
            }
        }
        checkpoints.push(serde_json::json!({"case":name,"state":retained_result.unwrap()}));
        trainer.restore_checkpoint(initial_actor).unwrap();
        assert_eq!(
            private,
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
        );
        let candidate_counts = decisions
            .iter()
            .map(|d| d.replay_tick.candidates.len())
            .collect::<Vec<_>>();
        results.push(serde_json::json!({"case":name,"world_steps":world_steps,
            "native_decisions":decisions.len(),"loss_rows":decisions.len()-1,
            "window_rows":windows.iter().map(|w|w.actor.batch.len()).collect::<Vec<_>>(),
            "window_burn_in":windows.iter().map(|w|w.actor.sequence.burn_in_ticks).collect::<Vec<_>>(),
            "effective_batch_size":effective_batch_size,"config":config,
            "candidate_count_min":candidate_counts.iter().min(),"candidate_count_max":candidate_counts.iter().max(),
            "decisions_sha256":format!("{:x}",Sha256::digest(&decisions_bytes)),
            "private_sha256":format!("{:x}",Sha256::digest(&private)),
            "collection_once_ms":collection_ms,"window_preparation_once_ms":preparation_ms,
            "learner_ms":samples}));
    }
    let tag = std::env::var("ALIFE_NATIVE_BENCH_TAG").unwrap_or_else(|_| "candidate".into());
    let directory = std::path::PathBuf::from(
        std::env::var_os("ALIFE_BENCH_OUTPUT_DIR")
            .expect("set ALIFE_BENCH_OUTPUT_DIR for this explicit benchmark"),
    );
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join(format!("{tag}-native.json")),
        serde_json::to_vec_pretty(&results).unwrap(),
    )
    .unwrap();
    std::fs::write(
        directory.join(format!("{tag}-native-checkpoints.json")),
        serde_json::to_vec(&checkpoints).unwrap(),
    )
    .unwrap();
    eprintln!("NATIVE_REPLAY {}", serde_json::json!(results));
}
