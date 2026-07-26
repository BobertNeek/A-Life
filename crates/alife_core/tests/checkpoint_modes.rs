use alife_core::{BrainCheckpointMode, Validate};

#[test]
fn checkpoint_modes_round_trip_through_stable_slugs() {
    for mode in [
        BrainCheckpointMode::GeneticRebuild,
        BrainCheckpointMode::DurableLearnedFounder,
        BrainCheckpointMode::ExactResume,
    ] {
        mode.validate_contract().unwrap();
        assert_eq!(
            BrainCheckpointMode::try_from_slug(mode.slug()).unwrap(),
            mode
        );
    }
}
