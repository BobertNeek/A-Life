//! Fresh public native data only. Full fixtures stay in an explicit local output.
use super::*;
use std::{collections::BTreeMap, time::Instant};

#[test]
#[ignore = "explicit disposable native-window export; requires fresh local output"]
fn export_fresh_native_packed_block_fixture() {
    let directory = std::path::PathBuf::from(
        std::env::var_os("ALIFE_PACKED_OUTPUT_DIR").expect("set local output directory"),
    );
    std::fs::create_dir_all(&directory).unwrap();
    let output = directory.join("fresh-native-block-fixture.json");
    assert!(!output.exists(), "never overwrite an acquired fixture");
    let batch = std::env::var("ALIFE_PACKED_BRIDGE_BATCH")
        .ok()
        .map(|s| s.parse::<usize>().unwrap())
        .unwrap_or(8);
    assert!((1..=64).contains(&batch));
    let started = Instant::now();
    let (game, trainer) = new_game_fixture(None);
    let p = trainer.phenotype();
    let checkpoint = trainer.checkpoint();
    let original_actor_bytes = serde_json::to_vec(&checkpoint).unwrap();
    let source = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .genome()
        .n2048_foundation_candidate
        .as_ref()
        .unwrap()
        .clone();
    let mut groups = BTreeMap::<(u16, u32, u32), Vec<(usize, usize)>>::new();
    let mut recurrent_ids = Vec::new();
    for (id, s) in p.synapses().iter().enumerate() {
        if s.kind() == CompiledSynapseKind::Recurrent {
            recurrent_ids.push(id);
            groups
                .entry((s.route_index(), s.source() / 16, s.target() / 16))
                .or_default()
                .push((id, ((s.target() % 16) * 16 + s.source() % 16) as usize));
        }
    }
    let blocks = groups
        .into_iter()
        .map(|((route, source, target), entries)| {
            assert_eq!(entries.len(), 256);
            let mut ids = vec![usize::MAX; 256];
            for (id, slot) in entries {
                assert_eq!(ids[slot], usize::MAX);
                ids[slot] = id;
            }
            assert!(!ids.contains(&usize::MAX));
            serde_json::json!({"source_start":source*16,"target_start":target*16,"route":route,
            "cadence":p.projections()[route as usize].update_cadence().raw(),"ids":ids})
        })
        .collect::<Vec<_>>();
    assert_eq!(blocks.len(), 96);
    assert_eq!(recurrent_ids.len(), 24576);
    let config = alife_training::PpoConfig {
        epochs: 1,
        temperature: 0.9,
        ..Default::default()
    };
    let mut samples = Vec::new();
    let mut receipts = Vec::new();
    let mut observed_offset_bits = std::collections::BTreeSet::new();
    let collect_at = Instant::now();
    let mut capture_ms = Vec::new();
    let mut admission_ms = Vec::new();
    for index in 0..batch {
        let at = Instant::now();
        let mut host =
            CpuCanonicalHost::new_cohort(game.clone(), &trainer, 73101 + index as u32).unwrap();
        let mut decisions = Vec::new();
        for _ in 0..512 {
            if decisions.len() >= 17 {
                break;
            }
            decisions.extend(host.step(&trainer, 0.9).unwrap().decisions);
        }
        assert_eq!(decisions.len(), 17);
        capture_ms.push(at.elapsed().as_secs_f64() * 1000.);
        let original_private = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
        let phenotype = host
            .world()
            .organism_registry()
            .iter()
            .next()
            .unwrap()
            .phenotype();
        let value = CpuValueHead::from_checkpoint(alife_training::PpoValueHeadCheckpoint {
            feature_count: p.neuron_count(),
            optimizer_step: 0,
            parameters: vec![0.; (p.neuron_count() as usize + 1) * 3],
            last_updated_policy_version: None,
        })
        .unwrap();
        let windows = cpu_training_windows(
            &decisions, None, &trainer, &value, phenotype, 7400, config, 16, 4,
        )
        .unwrap();
        assert_eq!(
            windows.len(),
            1,
            "batch consists of independent native trajectories, not duplicated rows"
        );
        let mut actor_copy =
            CpuFoundationTrainer::from_checkpoint(p.clone(), source.clone(), checkpoint.clone())
                .unwrap();
        let mut value_copy = CpuValueHead::from_checkpoint(value.checkpoint()).unwrap();
        let admission = Instant::now();
        // The ordinary frozen-feature and old-policy probability admission stays intact.
        let receipt = train_cpu_residual_cohort(
            &mut actor_copy,
            &mut value_copy,
            1,
            1,
            |i| Ok(windows[i].clone()),
            config,
            7400,
        )
        .unwrap();
        admission_ms.push(admission.elapsed().as_secs_f64() * 1000.);
        receipts.push(receipt);
        assert_eq!(trainer.checkpoint(), checkpoint);
        assert_eq!(
            serde_json::to_vec(&trainer.checkpoint()).unwrap(),
            original_actor_bytes
        );
        assert_eq!(
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
            original_private
        );
        let d = &decisions[8];
        let tick = &d.replay_tick;
        let a = &d.initial.activations;
        assert!(
            tick.microstep_count > 0,
            "sample must be an actually executed neural microstep"
        );
        let mut expected = vec![0_f32; p.neuron_count() as usize];
        for (id, s) in p.synapses().iter().enumerate() {
            if s.kind() == CompiledSynapseKind::Recurrent
                && tick.enabled_routes[s.route_index() as usize]
            {
                // Every supported cadence fires at local step zero.
                expected[s.target() as usize] += a[s.source() as usize]
                    * (trainer.weights()[id] + tick.effective_weight_offsets[id]);
            }
        }
        observed_offset_bits.insert(
            tick.effective_weight_offsets
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
        );
        samples.push(serde_json::json!({"window":index,"sampling_seed":73101+index,"native_decisions":17,
            "captured_tick":8,"microstep":0,"activations":a,"effective_weight_offsets":tick.effective_weight_offsets,
            "enabled_routes":tick.enabled_routes,"expected_inputs":expected}));
    }
    assert!(
        batch == 1 || observed_offset_bits.len() > 1,
        "per-window private offsets must actually differ"
    );
    let fixture = serde_json::json!({"schema_version":1,"provenance":"fresh-public-native-independent-collector-windows",
        "neurons":p.neuron_count(),"synapse_count":p.synapses().len(),"recurrent_edge_count":recurrent_ids.len(),
        "recurrent_ids":recurrent_ids,"weights":trainer.weights(),"blocks":blocks,"samples":samples});
    let bytes = serde_json::to_vec(&fixture).unwrap();
    std::fs::write(&output, bytes).unwrap();
    let result = serde_json::json!({"batch":batch,"actual_native_decisions":batch*17,"actual_windows":batch,
        "window_loss_rows":16,"window_burn_in":0,"blocks":96,"recurrent_edges":24576,
        "unique_private_offset_vectors":observed_offset_bits.len(),"capture_ms":capture_ms,"native_admission_and_update_ms":admission_ms,
        "collection_and_admission_seconds":collect_at.elapsed().as_secs_f64(),"total_bridge_seconds":started.elapsed().as_secs_f64(),
        "source_and_private_noop":true,"native_old_probability_admission":"passed without changed tolerances",
        "receipts":receipts,"tensor_core_execution":false});
    std::fs::write(
        directory.join("native-block-bridge-ledger.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    eprintln!("NATIVE_BLOCK_BRIDGE {result}");
}
