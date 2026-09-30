use alife_core::{FoundationWeightAsset, SensorProfile};
use alife_training::{
    Era1TrialRunner, N2048ActiveBatteryRunner, N2048EvolutionHardener, TrainingError,
};

fn assert_canonical_biology_block<T>(result: Result<T, TrainingError>, expected: &str) {
    match result {
        Err(error @ TrainingError::CanonicalBiologyUnavailable { pipeline }) => {
            assert_eq!(pipeline, expected);
            assert!(error.to_string().contains("receptor-gated learning"));
        }
        Err(other) => panic!("expected the biology integration blocker, got {other}"),
        Ok(_) => panic!("legacy evaluator must not run without canonical biology"),
    }
}

#[test]
fn legacy_evaluators_report_their_blocker_before_requiring_a_gpu() {
    // This gate runs in headless CI with no GPU. An adapter error would mean
    // the runner attempted hardware initialization before checking readiness.
    assert_canonical_biology_block(
        N2048ActiveBatteryRunner::new_required(),
        "N2048 active battery and reproduction intent",
    );
    assert_canonical_biology_block(Era1TrialRunner::new_required(), "Era 1 causal trial runner");
    let foundation =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    assert_canonical_biology_block(
        N2048EvolutionHardener::new_required(foundation),
        "N2048 foundation hardening",
    );
}
