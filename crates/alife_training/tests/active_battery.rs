#[cfg(feature = "gpu-tests")]
use alife_core::OrganismId;
use alife_core::{ActiveChallengeKind, ACTIVE_CHALLENGE_COUNT};
use alife_training::ActiveBatteryChallengeSpec;
#[cfg(feature = "gpu-tests")]
use alife_training::N2048ActiveBatteryRunner;

#[test]
fn every_active_challenge_has_a_bounded_production_world_spec() {
    let specs = ActiveBatteryChallengeSpec::all();
    assert_eq!(specs.len(), ACTIVE_CHALLENGE_COUNT);
    for (expected, spec) in ActiveChallengeKind::ALL.into_iter().zip(specs) {
        assert_eq!(spec.kind, expected);
        assert!(spec.tick_budget > 0 && spec.tick_budget <= 64);
        assert!(spec.world_object_count >= 2);
        assert!(spec.uses_grounded_sensing);
        assert!(!spec.slm_enabled);
    }
}

#[cfg(feature = "gpu-tests")]
#[test]
fn real_gpu_active_battery_measures_all_fifteen_challenges() {
    let mut runner = N2048ActiveBatteryRunner::new_required().unwrap();
    let evidence = runner
        .run_genetic_founder(OrganismId(7), 0xA11F_E4404)
        .unwrap();

    assert_eq!(evidence.receipt.completed_count(), ACTIVE_CHALLENGE_COUNT);
    assert_eq!(evidence.challenge_worlds, ACTIVE_CHALLENGE_COUNT as u32);
    assert!(evidence.gpu_dispatches >= ACTIVE_CHALLENGE_COUNT as u64);
    assert_eq!(evidence.gpu_dispatches, evidence.sealed_outcomes);
    assert!(!evidence.slm_enabled);
    assert!(!evidence.adapter_name.trim().is_empty());
    assert!(!evidence.backend_api.trim().is_empty());
}
