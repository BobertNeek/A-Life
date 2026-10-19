//! Opt-in real-hardware qualification. Never run this on the user's PC without
//! coordination; cloud CPU checks do not substitute for this test.
#![cfg(all(feature = "gpu-tests", feature = "foundation-training"))]
#[test]
#[ignore = "requires an explicitly coordinated GPU and admitted native source/history"]
fn fifty_private_organisms_complete_native_training_and_optimizer_handoff() {
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
            known_exclusions: false,
            replay_device_limit_bytes: 512 * 1024 * 1024,
            replay_host_limit_bytes: 8 * 1024 * 1024 * 1024,
            sampling_temperature: 1.0,
        })
        .unwrap();
    assert_eq!(receipt["complete"], true);
    assert_eq!(receipt["population"], 50);
    assert_eq!(receipt["max_collector_batch_rows"], 50);
    assert_eq!(receipt["episodes"].as_array().unwrap().len(), 50);
    assert!(receipt["distinct_initial_sensory_states"].as_u64().unwrap() > 1);
    assert!(receipt["verified_replay_boundaries"].as_u64().unwrap() >= 50);
    assert!(receipt["actor_optimizer_updates"].as_u64().unwrap() > 0);
    assert!(receipt["value_optimizer_updates"].as_u64().unwrap() > 0);
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
