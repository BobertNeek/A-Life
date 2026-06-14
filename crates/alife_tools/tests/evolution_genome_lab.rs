use std::process::Command;

use alife_core::{
    BrainClassSpec, BrainGenome, BrainScaleTier, LobeRatioPlan, NormalizedScalar,
    PackedExperienceFrame, PackedExperienceRecord, PackedSideBufferKind, PackedSideBufferRecord,
    PackedSideBufferSpans, PackedSideBuffers, SideBufferSpan, Validate, PACKED_FLAG_SUCCESS,
};
use alife_tools::p33_evolution::{
    crossover_genomes, mutate_genome, run_selection_generation, BirthWeightInitializerRef,
    CrossoverConfig, EvolutionLabConfig, FitnessSummary, MutationConfig, MutationField,
    SelectionCandidate,
};

fn standard_spec() -> BrainClassSpec {
    BrainClassSpec::for_tier(BrainScaleTier::Standard2048)
}

#[test]
fn mutation_touches_all_genome_field_families_and_keeps_child_valid() {
    let spec = standard_spec();
    let parent = BrainGenome::scaffold(42, spec.id);

    let mutation = mutate_genome(
        &parent,
        &spec,
        MutationConfig {
            seed: 0xA11F_3301,
            generation: 3,
            intensity: NormalizedScalar(0.75),
        },
    )
    .expect("mutation produces valid child");

    mutation.child.validate_contract().unwrap();
    assert_ne!(mutation.child.id, parent.id);
    assert_eq!(mutation.child.parent_genome_ids, vec![parent.id]);
    assert_eq!(mutation.child.brain_class_id, spec.id);
    assert_eq!(
        mutation.touched_fields,
        vec![
            MutationField::LobeRatios,
            MutationField::MacroConnectomeMasks,
            MutationField::SparseDensityPriors,
            MutationField::AlphaMask,
            MutationField::EndocrineConstants,
            MutationField::DriveThresholds,
            MutationField::SensorLayout,
            MutationField::MotorAffordances,
            MutationField::MutationRates,
            MutationField::DevelopmentalSchedule,
        ]
    );

    let LobeRatioPlan::InlineOverrides(overrides) = &mutation.child.lobe_ratios else {
        panic!("mutation should materialize bounded lobe-ratio overrides");
    };
    assert!(!overrides.is_empty());
    let ratio_sum: f32 = overrides.iter().map(|entry| entry.ratio.raw()).sum();
    assert!((ratio_sum - 1.0).abs() <= 0.001);
    let min_ratio = 16.0 / spec.neuron_count as f32;
    assert!(overrides
        .iter()
        .all(|entry| (min_ratio..=0.60).contains(&entry.ratio.raw())));
    assert_eq!(
        mutation.child.alpha_mask.storage_policy,
        alife_core::AlphaStoragePolicy::HierarchicalSparse
    );
    assert!(!mutation.child.alpha_mask.dense_reference_opt_in);
    assert!(
        mutation.child.genetic_prior_seed != parent.genetic_prior_seed,
        "mutation may reseed inherited birth priors but must not use lifetime state"
    );
}

#[test]
fn crossover_records_lineage_and_rejects_incompatible_classes() {
    let spec = standard_spec();
    let parent_a = BrainGenome::scaffold(11, spec.id);
    let parent_b = BrainGenome::scaffold(12, spec.id);

    let offspring = crossover_genomes(
        &parent_a,
        &parent_b,
        &spec,
        CrossoverConfig {
            seed: 0xA11F_3302,
            generation: 9,
        },
    )
    .expect("compatible crossover succeeds");

    offspring.child.validate_contract().unwrap();
    assert_eq!(
        offspring.child.parent_genome_ids,
        vec![parent_a.id, parent_b.id]
    );
    assert_eq!(
        offspring.lineage.parent_genome_ids,
        vec![parent_a.id, parent_b.id]
    );
    assert_eq!(offspring.lineage.generation, 9);
    assert_eq!(offspring.lineage.random_seed, 0xA11F_3302);
    assert!(offspring.lineage.compatible);

    let json = serde_json::to_string(&offspring.lineage).unwrap();
    let decoded: alife_tools::p33_evolution::CrossoverLineageRecord =
        serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, offspring.lineage);

    let other_spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let other_parent = BrainGenome::scaffold(13, other_spec.id);
    let err = crossover_genomes(
        &parent_a,
        &other_parent,
        &spec,
        CrossoverConfig {
            seed: 0xA11F_3303,
            generation: 10,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("incompatible parent genomes"));
}

#[test]
fn fitness_summary_uses_packed_logs_and_optional_teacher_verifier_score() {
    let records = vec![
        packed_record(0, 10, 0.20, 0.0, 0.12, true, Some(0.80)),
        packed_record(10, 20, 0.10, 0.2, 0.08, true, Some(0.90)),
        packed_record(20, 30, -0.05, 0.0, 0.04, false, None),
    ];

    let summary = FitnessSummary::from_packed_records(&records).unwrap();

    assert_eq!(summary.survival_ticks, 30);
    assert!(summary.energy_stability > 0.80);
    assert!(summary.food_success > 0.60);
    assert!(summary.pain_avoidance > 0.90);
    assert!(summary.curiosity_resolution > 0.80);
    assert!(summary.social_word_task_score > 0.0);
    assert_eq!(summary.teacher_verifier_score, Some(0.85));
    assert!(summary.composite_score > 0.60);
}

#[test]
fn selection_lab_is_deterministic_and_keeps_weight_assets_birth_only() {
    let spec = standard_spec();
    let parent_a = BrainGenome::scaffold(101, spec.id);
    let parent_b = BrainGenome::scaffold(102, spec.id);
    let parent_c = BrainGenome::scaffold(103, spec.id);
    let candidates = vec![
        SelectionCandidate::new(parent_a.clone(), FitnessSummary::synthetic(0.75).unwrap()),
        SelectionCandidate::new(parent_b.clone(), FitnessSummary::synthetic(0.40).unwrap()),
        SelectionCandidate::new(parent_c.clone(), FitnessSummary::synthetic(0.95).unwrap()),
    ];
    let config = EvolutionLabConfig {
        seed: 0xA11F_3304,
        generation: 4,
        survivor_count: 2,
        offspring_count: 3,
        mutation_intensity: NormalizedScalar(0.55),
        birth_weight_initializer: Some(BirthWeightInitializerRef {
            asset_id: "p32://generated-weight-template/demo".to_string(),
            asset_schema_version: 1,
            birth_only: true,
        }),
    };

    let first = run_selection_generation(&candidates, &spec, config.clone()).unwrap();
    let second = run_selection_generation(&candidates, &spec, config).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.survivors.len(), 2);
    assert_eq!(first.offspring.len(), 3);
    assert_eq!(first.survivors[0].genome.id, parent_c.id);
    for child in &first.offspring {
        child.child.validate_contract().unwrap();
        assert!(
            child
                .lineage
                .birth_weight_initializer
                .as_ref()
                .unwrap()
                .birth_only
        );
        assert!(
            !child.lineage.lifetime_state_inherited,
            "default P33 selection must not leak lifetime state into inherited genetic baseline"
        );
    }
}

#[test]
fn tiny_generation_smoke_binary_writes_json_report() {
    let out = std::env::temp_dir().join(format!("alife_p33_smoke_{}.json", std::process::id()));
    let bin = std::env::var("CARGO_BIN_EXE_p33_genome_lab")
        .expect("Cargo should expose the p33_genome_lab test binary path");
    let status = Command::new(bin)
        .args([
            "smoke",
            "--seed",
            "43981",
            "--generations",
            "1",
            "--out",
            out.to_str().unwrap(),
        ])
        .status()
        .expect("p33 smoke binary runs");
    assert!(status.success());

    let report = std::fs::read_to_string(&out).unwrap();
    assert!(report.contains("\"schema_version\""));
    assert!(report.contains("\"offspring\""));
    let _ = std::fs::remove_file(out);
}

fn packed_record(
    start_tick: u64,
    outcome_tick: u64,
    energy_delta: f32,
    pain_delta: f32,
    prediction_error: f32,
    success: bool,
    teacher_score: Option<f32>,
) -> PackedExperienceRecord {
    let mut side_records = vec![PackedSideBufferRecord::new(
        PackedSideBufferKind::HeardToken,
        7,
        0,
        [1.0, 0.0, 0.0, 0.9],
        0,
    )
    .unwrap()];
    let teacher_span = if let Some(score) = teacher_score {
        let offset = side_records.len() as u32;
        side_records.push(
            PackedSideBufferRecord::new(
                PackedSideBufferKind::TeacherSchoolRef,
                1,
                99,
                [score, 1.0, 0.0, 0.0],
                0,
            )
            .unwrap(),
        );
        SideBufferSpan::new(offset, 1)
    } else {
        SideBufferSpan::new(side_records.len() as u32, 0)
    };
    let after_heard = 1;
    let after_teacher = teacher_span.offset + teacher_span.count;

    let frame = PackedExperienceFrame {
        schema_version: PackedExperienceFrame::SCHEMA_VERSION,
        experience_schema_version: alife_core::SchemaVersions::CURRENT.experience.raw(),
        sensory_abi_version: alife_core::SchemaVersions::CURRENT.sensory_abi.raw(),
        action_abi_version: alife_core::SchemaVersions::CURRENT.action_abi.raw(),
        flags: if success { PACKED_FLAG_SUCCESS } else { 0 },
        reserved_header: 0,
        organism_id: 1,
        sequence_id: start_tick,
        pre_action_tick: start_tick,
        decision_tick: start_tick,
        outcome_tick,
        brain_class_id: BrainScaleTier::Standard2048.default_class_id().raw(),
        brain_scale_tier_code: 3,
        selected_action_kind_code: 200,
        reserved_kind: 0,
        selected_action_id: 200,
        action_duration_ticks: 1,
        action_source_mask: 1,
        target_entity_id: 0,
        position: [0.0, 0.0, 0.0],
        heading_quat: [0.0, 0.0, 0.0, 1.0],
        target_position: [0.0, 0.0, 0.0],
        drive_summary: [0.3; alife_core::PACKED_DRIVE_SUMMARY_CHANNELS],
        hormone_summary: [0.3; alife_core::PACKED_HORMONE_SUMMARY_CHANNELS],
        action_intensity: 0.5,
        action_confidence: 0.8,
        decision_confidence: 0.8,
        reward_valence: if success { 0.7 } else { -0.1 },
        frustration_delta: if success { 0.0 } else { 0.2 },
        pain_delta,
        energy_delta,
        prediction_error,
        salience_summary: 0.5,
        memory_expected_valence: 0.1,
        memory_salience_hint: 0.2,
        side_buffer_spans: PackedSideBufferSpans {
            visible_entities: SideBufferSpan::new(0, 0),
            touched_entities: SideBufferSpan::new(0, 0),
            heard_tokens: SideBufferSpan::new(0, 1),
            salience_clusters: SideBufferSpan::new(after_heard, 0),
            memory_links: SideBufferSpan::new(after_heard, 0),
            concept_links: SideBufferSpan::new(after_heard, 0),
            ranked_action_proposals: SideBufferSpan::new(after_heard, 0),
            arbitration_details: SideBufferSpan::new(after_heard, 0),
            semantic_codes: SideBufferSpan::new(after_heard, 0),
            gaussian_refs: SideBufferSpan::new(after_heard, 0),
            teacher_school_refs: teacher_span,
            diagnostic_extras: SideBufferSpan::new(after_teacher, 0),
        },
        reserved: [0; alife_core::PACKED_EXPERIENCE_FRAME_RESERVED_U32S],
    };
    PackedExperienceRecord {
        frame,
        side_buffers: PackedSideBuffers::from_records(side_records).unwrap(),
    }
}
