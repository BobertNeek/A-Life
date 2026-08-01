use alife_core::{
    BrainCapacityClass, Era1Ability, Era1AssistanceKind, Era1Control, Era1EvidencePartition,
    Era1PlateauWindow, Era1TrialIdentity, Era1TrialReceipt, MetricReading, OrganismId,
    PhenotypeHash, PolicyBackend, SensorProfile, ERA1_EVALUATION_SCHEMA_VERSION,
};
use alife_tools::era1_evolution::Era1EvolutionReceipt;
use alife_tools::era1_promotion::{
    assess_era1_plateau, canonical_world_family_id, derive_era1_promotion,
    validate_committed_era1_promotion_report, Era1CommittedPromotionReport, Era1ComparisonStatus,
    Era1EvidenceStatus, Era1HardwareCost, Era1PlateauStatus, Era1PromotionVerdict,
    ERA1_COMMITTED_PROMOTION_REPORT_SCHEMA_VERSION,
};
use std::{fs, path::PathBuf};

mod common;

const SOURCE_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const SOURCE_TREE: &str = "89abcdef0123456789abcdef0123456789abcdef";
const ADAPTER: &str = "NVIDIA GeForce RTX 3050";

fn evolution() -> Era1EvolutionReceipt {
    common::validator_only_evolution_receipt()
}

fn measured(value_q16: u32) -> MetricReading {
    MetricReading::Measured {
        value_q16,
        exposures: 4,
    }
}

fn complete_trials(evolution: &Era1EvolutionReceipt) -> Vec<Era1TrialReceipt> {
    let mut receipts = Vec::new();
    for generation in evolution.generations.iter().skip(1) {
        for birth in &generation.births {
            for &seed in &evolution.config.evaluation_seeds {
                for &world_variant_id in &evolution.config.held_out_world_transforms {
                    for ability in Era1Ability::ALL {
                        for control in Era1Control::ALL {
                            let control_code = control as u64;
                            receipts.push(Era1TrialReceipt {
                                schema_version: ERA1_EVALUATION_SCHEMA_VERSION,
                                identity: Era1TrialIdentity {
                                    seed,
                                    organism_id: OrganismId(birth.genome.id.0),
                                    genome_id: birth.genome.id,
                                    parent_genome_ids: birth.genome.parent_genome_ids.clone(),
                                    lineage_id: birth.genome.lineage_id,
                                    generation: generation.generation,
                                    brain_class_id: BrainCapacityClass::N2048_ID,
                                    world_family_id: canonical_world_family_id(ability),
                                    world_variant_id,
                                },
                                ability,
                                control,
                                partition: Era1EvidencePartition::ReproducedOffspring,
                                score: measured(if control == Era1Control::Intact {
                                    50_000
                                } else {
                                    45_000
                                }),
                                phenotype_hash: PhenotypeHash([
                                    birth.genome.id.0,
                                    u64::from(generation.generation),
                                    birth.lineage_slot as u64 + 1,
                                    1,
                                ]),
                                foundation_id: birth.genome.foundation.foundation_id,
                                foundation_version: u32::from(birth.genome.foundation.version),
                                sensor_profile: SensorProfile::GroundedObjectSlotsV1,
                                policy_backend: PolicyBackend::NeuralClosedLoopGpu,
                                world_digest: [world_variant_id, seed, ability as u64, 1],
                                perception_digest: [birth.genome.id.0, seed, control_code + 1, 1],
                                sealed_evidence_digest: [
                                    birth.genome.id.0,
                                    world_variant_id,
                                    ability as u64,
                                    control_code + 1,
                                ],
                                assistance: Vec::new(),
                                adapter_name: ADAPTER.to_string(),
                                backend_api: "vulkan".to_string(),
                                source_commit: SOURCE_COMMIT.to_string(),
                                source_tree: SOURCE_TREE.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    receipts
}

fn hardware(receipt_count: usize) -> Era1HardwareCost {
    Era1HardwareCost {
        adapter_name: ADAPTER.to_string(),
        backend_api: "vulkan".to_string(),
        trial_receipts: u32::try_from(receipt_count).unwrap(),
        gpu_dispatches: 47_520,
        elapsed_ns: 9_876_543_210,
        peak_vram_bytes: 1_610_612_736,
    }
}

fn plateau_windows() -> Vec<Era1PlateauWindow> {
    vec![
        Era1PlateauWindow {
            first_generation: 0,
            last_generation: 2,
            improvement_q16: 400,
            complete_cells: 528,
            ecological_regression: false,
            diversity_regression: false,
        },
        Era1PlateauWindow {
            first_generation: 1,
            last_generation: 3,
            improvement_q16: 500,
            complete_cells: 528,
            ecological_regression: false,
            diversity_regression: false,
        },
        Era1PlateauWindow {
            first_generation: 2,
            last_generation: 4,
            improvement_q16: 300,
            complete_cells: 528,
            ecological_regression: false,
            diversity_regression: false,
        },
    ]
}

#[test]
fn complete_literal_matrix_passes_without_averaging_subgroups() {
    let evolution = evolution();
    let trials = complete_trials(&evolution);
    let report = derive_era1_promotion(
        &evolution,
        &trials,
        hardware(trials.len()),
        &plateau_windows(),
    )
    .unwrap();

    assert_eq!(trials.len(), 2_640);
    assert_eq!(report.verdict, Era1PromotionVerdict::Pass);
    assert_eq!(report.subgroup_scores.len(), 528);
    assert!(report
        .subgroup_scores
        .iter()
        .all(|group| group.intact_q16 == Some(50_000)));
    assert_eq!(report.control_comparisons.len(), 44);
    assert!(report.control_comparisons.iter().all(|comparison| {
        comparison.status == Era1ComparisonStatus::Measured
            && comparison.matched_cells == 48
            && comparison.intact_mean_q16 == Some(50_000)
            && comparison.control_mean_q16 == Some(45_000)
            && comparison.margin_q16 == Some(5_000)
            && comparison.passes_minimum_margin
    }));
    assert_eq!(report.hardware, hardware(2_640));
    assert_eq!(report.plateau.status, Era1PlateauStatus::Measured);
    assert!(report.plateau.review_eligible);
    assert!(!report.plateau.brain_class_change_authorized);
}

#[test]
fn unknown_missing_and_nonpositive_cells_block_promotion() {
    let evolution = evolution();
    let trials = complete_trials(&evolution);

    let mut unknown = trials.clone();
    unknown[0].score = MetricReading::Unknown;
    let report = derive_era1_promotion(
        &evolution,
        &unknown,
        hardware(unknown.len()),
        &plateau_windows(),
    )
    .unwrap();
    assert_eq!(report.verdict, Era1PromotionVerdict::Blocked);

    let mut missing = trials.clone();
    missing.pop();
    let report = derive_era1_promotion(
        &evolution,
        &missing,
        hardware(missing.len()),
        &plateau_windows(),
    )
    .unwrap();
    assert_eq!(report.verdict, Era1PromotionVerdict::Blocked);
    assert!(report
        .control_comparisons
        .iter()
        .any(|comparison| comparison.status == Era1ComparisonStatus::Unknown));

    let mut nonpositive = trials;
    let intact = nonpositive
        .iter_mut()
        .find(|receipt| receipt.control == Era1Control::Intact)
        .unwrap();
    intact.score = measured(0);
    let report = derive_era1_promotion(
        &evolution,
        &nonpositive,
        hardware(nonpositive.len()),
        &plateau_windows(),
    )
    .unwrap();
    assert_eq!(report.verdict, Era1PromotionVerdict::Blocked);
    assert!(report
        .subgroup_scores
        .iter()
        .any(|group| group.intact_q16 == Some(0)));
}

#[test]
fn weak_or_contaminated_controls_cannot_pass_on_an_aggregate() {
    let evolution = evolution();
    let mut trials = complete_trials(&evolution);
    let contaminated = trials
        .iter_mut()
        .find(|receipt| receipt.control == Era1Control::MemoryDisabled)
        .unwrap();
    contaminated.score = measured(50_001);

    let report = derive_era1_promotion(
        &evolution,
        &trials,
        hardware(trials.len()),
        &plateau_windows(),
    )
    .unwrap();

    assert_eq!(report.verdict, Era1PromotionVerdict::Blocked);
    assert!(report.control_comparisons.iter().any(|comparison| {
        comparison.control == Era1Control::MemoryDisabled && !comparison.passes_minimum_margin
    }));
}

#[test]
fn assistance_source_mismatch_and_fabricated_provenance_are_rejected() {
    let evolution = evolution();
    let trials = complete_trials(&evolution);

    let mut assisted = trials.clone();
    assisted[0].assistance.push(Era1AssistanceKind::Teacher);
    assert!(derive_era1_promotion(
        &evolution,
        &assisted,
        hardware(assisted.len()),
        &plateau_windows(),
    )
    .is_err());

    let mut source_mismatch = trials.clone();
    source_mismatch[0].source_tree = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
    assert!(derive_era1_promotion(
        &evolution,
        &source_mismatch,
        hardware(source_mismatch.len()),
        &plateau_windows(),
    )
    .is_err());

    let mut fabricated = trials;
    fabricated[0].identity.parent_genome_ids = evolution.generations[0].births[2]
        .genome
        .parent_genome_ids
        .clone();
    assert!(derive_era1_promotion(
        &evolution,
        &fabricated,
        hardware(fabricated.len()),
        &plateau_windows(),
    )
    .is_err());
}

#[test]
fn every_ability_rejects_a_structurally_consistent_wrong_world_label() {
    let evolution = evolution();
    let trials = complete_trials(&evolution);
    for ability in Era1Ability::ALL {
        let mut relabelled = trials.clone();
        let wrong = if canonical_world_family_id(ability) == 1 {
            2
        } else {
            1
        };
        for trial in relabelled
            .iter_mut()
            .filter(|trial| trial.ability == ability)
        {
            trial.identity.world_family_id = wrong;
        }
        assert!(derive_era1_promotion(
            &evolution,
            &relabelled,
            hardware(relabelled.len()),
            &plateau_windows(),
        )
        .is_err());
    }
}

#[test]
fn plateau_needs_three_consecutive_low_gain_nonregressing_windows() {
    let measured = assess_era1_plateau(&plateau_windows()).unwrap();
    assert_eq!(measured.status, Era1PlateauStatus::Measured);
    assert!(measured.review_eligible);
    assert!(!measured.brain_class_change_authorized);

    let unknown = assess_era1_plateau(&plateau_windows()[..2]).unwrap();
    assert_eq!(unknown.status, Era1PlateauStatus::Unknown);
    assert!(!unknown.review_eligible);

    let mut high_gain = plateau_windows();
    high_gain[1].improvement_q16 = 656;
    assert!(!assess_era1_plateau(&high_gain).unwrap().review_eligible);

    let mut ecological_regression = plateau_windows();
    ecological_regression[2].ecological_regression = true;
    assert!(
        !assess_era1_plateau(&ecological_regression)
            .unwrap()
            .review_eligible
    );

    let mut nonconsecutive = plateau_windows();
    nonconsecutive[2].first_generation = 3;
    nonconsecutive[2].last_generation = 5;
    assert!(
        !assess_era1_plateau(&nonconsecutive)
            .unwrap()
            .review_eligible
    );
}

#[test]
fn committed_report_recomputes_source_receipts_and_unknown_gate_outcome() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("reports")
        .join("era1_promotion_report.json");
    let encoded = fs::read_to_string(path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    if value["schema_version"].as_u64()
        != Some(u64::from(ERA1_COMMITTED_PROMOTION_REPORT_SCHEMA_VERSION))
    {
        assert!(serde_json::from_str::<Era1CommittedPromotionReport>(&encoded).is_err());
        return;
    }
    let report: Era1CommittedPromotionReport = serde_json::from_str(&encoded).unwrap();

    validate_committed_era1_promotion_report(&report).unwrap();
    assert_eq!(
        report.schema_version,
        ERA1_COMMITTED_PROMOTION_REPORT_SCHEMA_VERSION
    );
    assert_eq!(report.artifact_binding.adapter_name, ADAPTER);
    assert_eq!(report.artifact_binding.backend_api, "vulkan");
    assert_eq!(report.matrix_coverage.len(), 55);
    assert!(report
        .matrix_coverage
        .iter()
        .all(|cell| cell.status == Era1EvidenceStatus::Unknown));
    assert_eq!(report.promotion.verdict, Era1PromotionVerdict::Blocked);
    assert_eq!(report.promotion.plateau.status, Era1PlateauStatus::Unknown);
    assert!(!report.boundaries.assistance_present);
    assert!(!report.boundaries.hidden_policy_present);
    assert!(!report.boundaries.brain_class_scaling_performed);
    assert_eq!(report.boundaries.era2_status, "OUT_OF_SCOPE");

    let mut tampered = report.clone();
    tampered.trial_receipts[0].world_digest[0] ^= 1;
    assert!(validate_committed_era1_promotion_report(&tampered).is_err());

    let mut hand_authored = report.clone();
    hand_authored.trial_evidence[0].steps[0].world_after_action_digest[0] ^= 1;
    hand_authored.trial_receipts[0] = hand_authored.trial_evidence[0].receipt.clone();
    assert!(validate_committed_era1_promotion_report(&hand_authored).is_err());

    let mut wrong_source = report;
    wrong_source.artifact_binding.source_contract_digest =
        "blake3-256:0000000000000000000000000000000000000000000000000000000000000000".to_string();
    assert!(validate_committed_era1_promotion_report(&wrong_source).is_err());
}
