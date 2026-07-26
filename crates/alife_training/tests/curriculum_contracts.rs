use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, FoundationWeightAsset, NormalizedScalar,
    PhenotypeCompiler, SensorProfile, Tick,
};
use alife_training::{
    wilson_lower_bound_85, CurriculumSplit, FoundationCurriculumStage, N2048CurriculumV1,
    StageEvaluation, StageGatePolicy,
};

fn phenotype() -> alife_core::BrainPhenotype {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0xC011_1C01, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let foundation =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    PhenotypeCompiler::compile_from_foundation_asset(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        &foundation,
    )
    .unwrap()
}

#[test]
fn shipped_grounded_foundation_completed_the_nine_stage_curriculum() {
    let foundation =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    assert_eq!(
        foundation
            .manifest()
            .training_stage()
            .completed_stage_count(),
        FoundationCurriculumStage::ALL.len() as u16
    );
}

#[test]
fn curriculum_v1_freezes_all_nine_grounded_slm_free_stages() {
    let curriculum = N2048CurriculumV1::new();
    assert_eq!(FoundationCurriculumStage::ALL.len(), 9);
    for (index, stage) in FoundationCurriculumStage::ALL.into_iter().enumerate() {
        let spec = curriculum.stage(stage);
        assert_eq!(spec.ordinal(), index as u16 + 1);
        assert!(!spec.uses_privileged_semantic_labels());
        assert!(!spec.uses_slm_assistance());
        assert_eq!(spec.held_out_episode_count(), 256);
        assert!(!curriculum
            .stage_mask(&phenotype(), stage)
            .unwrap()
            .is_empty());
    }
}

#[test]
fn held_out_sequences_are_deterministic_disjoint_and_language_surface_randomized() {
    let phenotype = phenotype();
    let curriculum = N2048CurriculumV1::new();
    let stage = FoundationCurriculumStage::SpeechMechanics;
    let training = curriculum
        .sequence(&phenotype, stage, CurriculumSplit::Training, 77)
        .unwrap();
    let replay = curriculum
        .sequence(&phenotype, stage, CurriculumSplit::Training, 77)
        .unwrap();
    let held_out = curriculum
        .sequence(&phenotype, stage, CurriculumSplit::HeldOut, 77)
        .unwrap();
    let alternate_surface = curriculum
        .sequence(&phenotype, stage, CurriculumSplit::HeldOut, 78)
        .unwrap();
    assert_eq!(training, replay);
    assert_ne!(training, held_out);
    assert_ne!(held_out, alternate_surface);
}

#[test]
fn stage_gate_enforces_real_episode_count_confidence_and_regression_limits() {
    let policy = StageGatePolicy::default();
    assert!(wilson_lower_bound_85(240, 256).unwrap() >= 0.90);
    assert!(wilson_lower_bound_85(230, 256).unwrap() < 0.90);

    let passing = StageEvaluation::try_new(256, 240, 0.91, 0.02, true, None).unwrap();
    policy.validate(&passing).unwrap();

    assert!(StageEvaluation::try_new(255, 255, 0.0, 0.0, true, None).is_err());
    let regression = StageEvaluation::try_new(256, 250, 0.2, 0.021, true, None).unwrap();
    assert!(policy.validate(&regression).is_err());
    let changed_frozen = StageEvaluation::try_new(256, 250, 0.2, 0.0, false, None).unwrap();
    assert!(policy.validate(&changed_frozen).is_err());
}
