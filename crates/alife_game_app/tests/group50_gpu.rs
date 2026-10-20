//! Opt-in real-hardware qualification. Never run this on the user's PC without
//! coordination; cloud CPU checks do not substitute for this test.
#![cfg(all(feature = "gpu-tests", feature = "foundation-training"))]
#[test]
#[ignore = "requires an explicitly coordinated GPU and admitted native source/history"]
fn fifty_private_organisms_complete_native_training_and_optimizer_handoff() {
    // Explicit fixture selection, subject to the same immutable history check as
    // the CLI. Missing configuration stays strict; rejection never falls back.
    let known_exclusions = match std::env::var("ALIFE_GROUP50_SEED_ADMISSION") {
        Err(std::env::VarError::NotPresent) => false,
        Ok(policy) if policy == "strict" => false,
        Ok(policy) if policy == "known-exclusions" => true,
        _ => panic!("ALIFE_GROUP50_SEED_ADMISSION must be strict or known-exclusions"),
    };
    let expected_admission = serde_json::json!({
        "policy": if known_exclusions { "known-exclusions" } else { "strict" },
        "historical_freshness": if known_exclusions { "unknown" } else { "operator-attested-complete" },
    });
    let source = std::path::PathBuf::from(
        std::env::var_os("ALIFE_GROUP50_SOURCE").expect("set ALIFE_GROUP50_SOURCE"),
    );
    let history = std::path::PathBuf::from(
        std::env::var_os("ALIFE_GROUP50_HISTORY").expect("set ALIFE_GROUP50_HISTORY"),
    );
    let output = std::path::PathBuf::from(
        std::env::var_os("ALIFE_GROUP50_OUTPUT")
            .expect("set a new ALIFE_GROUP50_OUTPUT in the admitted lane"),
    );
    let seed = std::env::var("ALIFE_GROUP50_SEED")
        .expect("set ALIFE_GROUP50_SEED")
        .parse()
        .unwrap();
    let receipt =
        alife_game_app::run_foundation_training_group(alife_game_app::FoundationGroupRequest {
            previous: &source,
            output: &output,
            seed_history: &history,
            seed,
            decisions_per_organism: 16,
            known_exclusions,
            replay_device_limit_bytes: 512 * 1024 * 1024,
            replay_host_limit_bytes: 8 * 1024 * 1024 * 1024,
            sampling_temperature: 1.0,
        })
        .unwrap();
    assert_eq!(receipt["complete"], true);
    assert_eq!(receipt["population"], 50);
    assert_eq!(receipt["max_collector_batch_rows"], 50);
    assert_eq!(receipt["episodes"].as_array().unwrap().len(), 50);
    assert_eq!(
        receipt["seed_provenance"]["seed_admission"],
        expected_admission
    );
    for episode in receipt["episodes"].as_array().unwrap() {
        assert_eq!(episode["seed_admission"], expected_admission);
    }
    assert!(receipt["distinct_initial_sensory_states"].as_u64().unwrap() > 1);
    assert!(receipt["verified_replay_boundaries"].as_u64().unwrap() >= 50);
    assert!(receipt["actor_optimizer_updates"].as_u64().unwrap() > 0);
    assert!(receipt["value_optimizer_updates"].as_u64().unwrap() > 0);
    assert_eq!(receipt["replay_persistence"]["data_sync_calls"], 1);
    assert_eq!(receipt["replay_persistence"]["durable_cohort_commits"], 1);
    let captured = receipt["episodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["captured_rows"].as_u64().unwrap())
        .sum::<u64>();
    assert_eq!(
        receipt["replay_persistence"]["records"].as_u64(),
        Some(captured)
    );
    assert!(
        receipt["training_capture"]["captured_rows"]
            .as_u64()
            .unwrap()
            >= 2 * captured
    );
    assert!(
        receipt["training_capture"]["readback_submissions"]
            .as_u64()
            .unwrap()
            < receipt["training_capture"]["captured_rows"]
                .as_u64()
                .unwrap()
    );
    assert!(output.join("replay/cohort-replay.bundle").is_file());
    assert!(!output.join("replay/cohort-replay.pending").exists());
    let replay_source = serde_json::from_value(receipt["source"].clone()).unwrap();
    let recovered =
        alife_game_app::inspect_foundation_cohort_replay(&output.join("replay"), &replay_source)
            .unwrap();
    assert!(
        matches!(recovered,alife_game_app::FoundationCohortReplayRecovery::Committed{records,..}
        if records as u64 == captured)
    );
    assert!(receipt["next_cohort_optimizer_rebound"].as_bool().unwrap());
    assert!(receipt["peak_replay_device_bound_bytes"].as_u64().unwrap() <= 512 * 1024 * 1024);
    assert!(
        receipt["valid_learning_decisions_per_total_second"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    let readmission = alife_game_app::validate_foundation_training_group_source(&output).unwrap();
    assert_eq!(readmission["native_checkpoint_valid"], true);
    assert_eq!(readmission["seed_reserved"], false);
    assert_eq!(
        readmission["actor_optimizer_step"],
        receipt["actor_optimizer_step"]
    );
    assert_eq!(
        readmission["value_optimizer_step"],
        receipt["value_optimizer_step"]
    );
}
