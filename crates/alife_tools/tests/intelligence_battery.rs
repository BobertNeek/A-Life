use alife_core::{
    BrainScaleTier, GenomeId, LineageId, PackedExperienceFrame, PackedExperienceRecord,
    PackedSideBufferSpans, PackedSideBuffers, PACKED_FLAG_SUCCESS,
};
use alife_tools::p33_evaluation::{
    evaluate_battery, AssistanceKind, BatteryLayer, BatterySuite, BatteryTrial, ComputeProvenance,
    EvaluationError, EvaluationFlag, EvaluationProvenance, LineageProvenance, TeamMode,
    TrialDomain, TrialPhase, TrialTrace, EI0_EVALUATION_SCHEMA_VERSION,
};

#[test]
fn hidden_promotion_requires_complete_provenance() {
    let mut suite = BatterySuite {
        schema_version: EI0_EVALUATION_SCHEMA_VERSION,
        suite_id: "hidden-provenance".to_string(),
        trials: vec![trial(
            "hidden-learning",
            BatteryLayer::HiddenPromotion,
            TrialDomain::Learning,
            TeamMode::Individual,
            41,
            "hidden-a",
            vec![
                trace(TrialPhase::Baseline, &[(false, -0.2, 0.8, -0.2)]),
                trace(TrialPhase::Acquisition, &[(true, 0.8, 0.1, -0.1)]),
            ],
        )],
    };
    suite.trials[0].provenance.compute.adapter.clear();

    let error = evaluate_battery(&suite).unwrap_err();

    assert!(matches!(
        error,
        EvaluationError::MissingPromotionProvenance { test_id }
            if test_id == "hidden-learning"
    ));
}

#[test]
fn hidden_promotion_rejects_assistance_or_prior_exposure() {
    let mut assisted = trial(
        "assisted-hidden",
        BatteryLayer::HiddenPromotion,
        TrialDomain::Transfer,
        TeamMode::Individual,
        42,
        "hidden-b",
        vec![trace(TrialPhase::Transfer, &[(true, 0.8, 0.1, -0.1)])],
    );
    assisted.provenance.assistance = vec![AssistanceKind::Teacher];
    assisted.provenance.exposure_count = 1;
    let suite = BatterySuite {
        schema_version: EI0_EVALUATION_SCHEMA_VERSION,
        suite_id: "assisted-hidden".to_string(),
        trials: vec![assisted],
    };

    assert!(matches!(
        evaluate_battery(&suite),
        Err(EvaluationError::ContaminatedPromotionEvidence { .. })
    ));
}

#[test]
fn unexposed_domains_remain_unknown_instead_of_zero() {
    let suite = BatterySuite {
        schema_version: EI0_EVALUATION_SCHEMA_VERSION,
        suite_id: "ecology-only".to_string(),
        trials: vec![trial(
            "foraging-anchor",
            BatteryLayer::PermanentAnchor,
            TrialDomain::Ecology,
            TeamMode::Individual,
            51,
            "meadow-a",
            vec![trace(
                TrialPhase::Acquisition,
                &[(true, 0.7, 0.1, -0.1), (true, 0.6, 0.2, -0.1)],
            )],
        )],
    };

    let report = evaluate_battery(&suite).unwrap();

    assert!(report.objectives.ecological.value.is_some());
    assert!(report.measures.learning.value.is_none());
    assert!(report.objectives.cognitive.value.is_none());
    assert_eq!(report.measures.learning.samples, 0);
}

#[test]
fn battery_measures_learning_transfer_reversal_memory_abstraction_social_and_compute() {
    let suite = broad_suite();

    let report = evaluate_battery(&suite).unwrap();

    assert!(report.measures.learning.value.unwrap() > 0.5);
    assert!(report.measures.transfer.value.unwrap() > 0.5);
    assert!(report.measures.reversal.value.unwrap() > 0.5);
    assert!(report.measures.delayed_memory.value.unwrap() > 0.5);
    assert!(report.measures.abstraction.value.unwrap() > 0.5);
    assert!(report.measures.social_contribution.value.unwrap() > 0.0);
    assert!(report.objectives.ecological.value.is_some());
    assert!(report.objectives.cognitive.value.is_some());
    assert!(report.objectives.social.value.is_some());
    assert!(report.objectives.group.value.is_some());
    assert!(report.objectives.stability.value.is_some());
    assert!(report.objectives.efficiency.value.unwrap() < 1.0);
    assert!(report.objectives.diversity.value.is_some());
    assert_eq!(report.packed_record_count, 24);
    assert!(!report.promotion_eligible);
}

#[test]
fn fixed_answer_reuse_across_variants_and_seeds_is_flagged() {
    let mut first = trial(
        "fixed-a",
        BatteryLayer::ProceduralBreeding,
        TrialDomain::Transfer,
        TeamMode::Individual,
        61,
        "layout-a",
        vec![trace(TrialPhase::Transfer, &[(true, 0.8, 0.1, -0.1)])],
    );
    first.answer_fingerprint = Some("answer-7".to_string());
    let mut second = trial(
        "fixed-b",
        BatteryLayer::ProceduralBreeding,
        TrialDomain::Transfer,
        TeamMode::Individual,
        62,
        "layout-b",
        vec![trace(TrialPhase::Transfer, &[(true, 0.8, 0.1, -0.1)])],
    );
    second.answer_fingerprint = Some("answer-7".to_string());
    let suite = BatterySuite {
        schema_version: EI0_EVALUATION_SCHEMA_VERSION,
        suite_id: "fixed-answer".to_string(),
        trials: vec![first, second],
    };

    let report = evaluate_battery(&suite).unwrap();

    assert!(report.flags.contains(&EvaluationFlag::FixedAnswerOverfit));
    assert!(!report.promotion_eligible);
}

#[test]
fn anchor_only_performance_gap_is_flagged_as_benchmark_gaming() {
    let suite = BatterySuite {
        schema_version: EI0_EVALUATION_SCHEMA_VERSION,
        suite_id: "anchor-gap".to_string(),
        trials: vec![
            trial(
                "known-anchor",
                BatteryLayer::PermanentAnchor,
                TrialDomain::Transfer,
                TeamMode::Individual,
                71,
                "known-layout",
                vec![trace(
                    TrialPhase::Transfer,
                    &[(true, 1.0, 0.0, 0.0), (true, 1.0, 0.0, 0.0)],
                )],
            ),
            trial(
                "rotated-procedural",
                BatteryLayer::ProceduralBreeding,
                TrialDomain::Transfer,
                TeamMode::Individual,
                72,
                "rotated-layout",
                vec![trace(
                    TrialPhase::Transfer,
                    &[(false, -1.0, 1.0, -1.0), (false, -1.0, 1.0, -1.0)],
                )],
            ),
        ],
    };

    let report = evaluate_battery(&suite).unwrap();

    assert!(report.flags.contains(&EvaluationFlag::AnchorProceduralGap));
}

#[test]
fn group_free_riding_is_flagged_for_persistent_and_randomized_teams() {
    let suite = BatterySuite {
        schema_version: EI0_EVALUATION_SCHEMA_VERSION,
        suite_id: "free-rider".to_string(),
        trials: vec![
            social_trial("pack-free-rider", TeamMode::PersistentPack, 81, false),
            social_trial("random-free-rider", TeamMode::RandomizedTeam, 82, false),
        ],
    };

    let report = evaluate_battery(&suite).unwrap();

    assert!(report.flags.contains(&EvaluationFlag::GroupFreeRider));
    assert_eq!(report.measures.social_contribution.value, Some(0.0));
    assert_eq!(report.objectives.group.value, Some(0.0));
}

fn broad_suite() -> BatterySuite {
    BatterySuite {
        schema_version: EI0_EVALUATION_SCHEMA_VERSION,
        suite_id: "broad-procedural".to_string(),
        trials: vec![
            trial(
                "ecology",
                BatteryLayer::PermanentAnchor,
                TrialDomain::Ecology,
                TeamMode::Individual,
                101,
                "meadow-a",
                vec![trace(
                    TrialPhase::Acquisition,
                    &[(true, 0.7, 0.2, -0.1), (true, 0.6, 0.2, -0.2)],
                )],
            ),
            trial(
                "learning",
                BatteryLayer::ProceduralBreeding,
                TrialDomain::Learning,
                TeamMode::Individual,
                102,
                "maze-a",
                vec![
                    trace(TrialPhase::Baseline, &[(false, -0.4, 0.9, -0.5)]),
                    trace(
                        TrialPhase::Acquisition,
                        &[
                            (false, -0.1, 0.6, -0.3),
                            (true, 0.5, 0.3, -0.2),
                            (true, 0.8, 0.1, -0.1),
                        ],
                    ),
                ],
            ),
            trial(
                "transfer",
                BatteryLayer::ProceduralBreeding,
                TrialDomain::Transfer,
                TeamMode::Individual,
                103,
                "maze-b",
                vec![trace(
                    TrialPhase::Transfer,
                    &[(true, 0.7, 0.2, -0.1), (true, 0.8, 0.1, -0.1)],
                )],
            ),
            trial(
                "reversal",
                BatteryLayer::ProceduralBreeding,
                TrialDomain::Reversal,
                TeamMode::Individual,
                104,
                "reward-swap-a",
                vec![trace(
                    TrialPhase::Reversal,
                    &[
                        (false, -0.3, 0.8, -0.3),
                        (false, -0.1, 0.6, -0.2),
                        (true, 0.6, 0.2, -0.1),
                        (true, 0.8, 0.1, -0.1),
                    ],
                )],
            ),
            trial(
                "delayed-memory",
                BatteryLayer::ProceduralBreeding,
                TrialDomain::DelayedMemory,
                TeamMode::Individual,
                105,
                "delay-a",
                vec![trace(
                    TrialPhase::DelayRecall,
                    &[(true, 0.8, 0.1, -0.1), (true, 0.6, 0.2, -0.1)],
                )],
            ),
            trial(
                "abstraction-a",
                BatteryLayer::ProceduralBreeding,
                TrialDomain::Abstraction,
                TeamMode::Individual,
                106,
                "shape-a",
                vec![trace(TrialPhase::Transfer, &[(true, 0.7, 0.2, -0.1)])],
            ),
            trial(
                "abstraction-b",
                BatteryLayer::ProceduralBreeding,
                TrialDomain::Abstraction,
                TeamMode::Individual,
                107,
                "shape-b",
                vec![trace(TrialPhase::Transfer, &[(true, 0.8, 0.1, -0.1)])],
            ),
            social_trial(
                "persistent-contributor",
                TeamMode::PersistentPack,
                108,
                true,
            ),
            social_trial("random-contributor", TeamMode::RandomizedTeam, 109, true),
        ],
    }
}

fn social_trial(test_id: &str, team_mode: TeamMode, seed: u64, contributes: bool) -> BatteryTrial {
    let (active, removed) = if contributes {
        (
            vec![(true, 0.8, 0.1, -0.1), (true, 0.7, 0.2, -0.1)],
            vec![(false, -0.2, 0.7, -0.3)],
        )
    } else {
        (
            vec![(false, -0.3, 0.8, -0.3)],
            vec![(true, 0.8, 0.1, -0.1), (true, 0.7, 0.2, -0.1)],
        )
    };
    trial(
        test_id,
        BatteryLayer::ProceduralBreeding,
        TrialDomain::SocialContribution,
        team_mode,
        seed,
        test_id,
        vec![
            trace(TrialPhase::ActiveGroup, &active),
            trace(TrialPhase::MemberRemoved, &removed),
            trace(TrialPhase::Replacement, &[(true, 0.3, 0.4, -0.2)]),
        ],
    )
}

fn trial(
    test_id: &str,
    layer: BatteryLayer,
    domain: TrialDomain,
    team_mode: TeamMode,
    seed: u64,
    variant_id: &str,
    traces: Vec<TrialTrace>,
) -> BatteryTrial {
    BatteryTrial {
        test_id: test_id.to_string(),
        layer,
        domain,
        team_mode,
        seed,
        variant_id: variant_id.to_string(),
        answer_fingerprint: None,
        hidden_set_id: (layer == BatteryLayer::HiddenPromotion)
            .then(|| "promotion-set-alpha".to_string()),
        focal_organism_id: 1,
        provenance: EvaluationProvenance {
            source_run_id: format!("run-{seed}"),
            foundation_id: "n2048-foundation-v1".to_string(),
            foundation_version: 1,
            exposure_count: 0,
            assistance: Vec::new(),
            compute: ComputeProvenance {
                adapter: "test-adapter".to_string(),
                backend: "NeuralClosedLoopGpu".to_string(),
                dispatches: 4,
                neural_ticks: 4,
                elapsed_micros: 100,
                energy_milliunits: 25,
                budget_units: 100,
            },
            lineage: LineageProvenance {
                lineage_id: LineageId(700 + seed),
                genome_id: GenomeId(900 + seed),
                ancestor_genome_ids: Vec::new(),
                population_share: 0.2,
                genome_novelty: 0.8,
            },
        },
        traces,
    }
}

fn trace(phase: TrialPhase, values: &[(bool, f32, f32, f32)]) -> TrialTrace {
    TrialTrace {
        phase,
        records: values
            .iter()
            .enumerate()
            .map(
                |(index, &(success, reward, prediction_error, energy_delta))| {
                    packed_record(
                        index as u64 + 1,
                        success,
                        reward,
                        prediction_error,
                        energy_delta,
                    )
                },
            )
            .collect(),
    }
}

fn packed_record(
    sequence: u64,
    success: bool,
    reward_valence: f32,
    prediction_error: f32,
    energy_delta: f32,
) -> PackedExperienceRecord {
    PackedExperienceRecord {
        frame: PackedExperienceFrame {
            schema_version: PackedExperienceFrame::SCHEMA_VERSION,
            experience_schema_version: alife_core::SchemaVersions::CURRENT.experience.raw(),
            sensory_abi_version: alife_core::SchemaVersions::CURRENT.sensory_abi.raw(),
            action_abi_version: alife_core::SchemaVersions::CURRENT.action_abi.raw(),
            flags: if success { PACKED_FLAG_SUCCESS } else { 0 },
            reserved_header: 0,
            organism_id: 1,
            sequence_id: sequence,
            pre_action_tick: sequence,
            decision_tick: sequence,
            outcome_tick: sequence + 1,
            brain_class_id: BrainScaleTier::Standard2048.default_class_id().raw(),
            brain_scale_tier_code: 3,
            selected_action_kind_code: 1,
            reserved_kind: 0,
            selected_action_id: 1,
            action_duration_ticks: 1,
            action_source_mask: 1,
            target_entity_id: 0,
            position: [0.0, 0.0, 0.0],
            heading_quat: [0.0, 0.0, 0.0, 1.0],
            target_position: [0.0, 0.0, 0.0],
            drive_summary: [0.2; alife_core::PACKED_DRIVE_SUMMARY_CHANNELS],
            hormone_summary: [0.2; alife_core::PACKED_HORMONE_SUMMARY_CHANNELS],
            action_intensity: 0.5,
            action_confidence: 0.8,
            decision_confidence: 0.8,
            reward_valence,
            frustration_delta: if success { 0.0 } else { 0.2 },
            pain_delta: if success { 0.0 } else { 0.2 },
            energy_delta,
            prediction_error,
            salience_summary: 0.5,
            memory_expected_valence: 0.0,
            memory_salience_hint: 0.2,
            side_buffer_spans: PackedSideBufferSpans::EMPTY,
            reserved: [0; alife_core::PACKED_EXPERIENCE_FRAME_RESERVED_U32S],
        },
        side_buffers: PackedSideBuffers::from_records(Vec::new()).unwrap(),
    }
}
