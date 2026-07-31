use alife_core::{
    BrainCapacityClass, BrainClassId, Era1Ability, Era1AssistanceKind, Era1Control,
    Era1EvidencePartition, Era1MatchedComparison, Era1PlateauWindow, Era1TrialIdentity,
    Era1TrialReceipt, GenomeId, LineageId, MetricReading, OrganismId, PhenotypeHash, PolicyBackend,
    SensorProfile, Validate, ERA1_ABILITY_COUNT, ERA1_EVALUATION_SCHEMA_VERSION,
};

const HASH_A: [u64; 4] = [11, 12, 13, 14];
const HASH_B: [u64; 4] = [21, 22, 23, 24];
const SOURCE_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const SOURCE_TREE: &str = "89abcdef0123456789abcdef0123456789abcdef";

fn offspring_identity() -> Era1TrialIdentity {
    Era1TrialIdentity {
        seed: 41,
        organism_id: OrganismId(501),
        genome_id: GenomeId(601),
        parent_genome_ids: vec![GenomeId(101), GenomeId(102)],
        lineage_id: LineageId(701),
        generation: 2,
        brain_class_id: BrainCapacityClass::N2048_ID,
        world_family_id: 801,
        world_variant_id: 901,
    }
}

fn valid_receipt() -> Era1TrialReceipt {
    Era1TrialReceipt {
        schema_version: ERA1_EVALUATION_SCHEMA_VERSION,
        identity: offspring_identity(),
        ability: Era1Ability::ObjectTransfer,
        control: Era1Control::Intact,
        partition: Era1EvidencePartition::ReproducedOffspring,
        score: MetricReading::Measured {
            value_q16: 49_151,
            exposures: 32,
        },
        phenotype_hash: PhenotypeHash(HASH_A),
        foundation_id: 1_001,
        foundation_version: 1,
        sensor_profile: SensorProfile::GroundedObjectSlotsV1,
        policy_backend: PolicyBackend::NeuralClosedLoopGpu,
        world_digest: HASH_A,
        perception_digest: HASH_B,
        sealed_evidence_digest: [31, 32, 33, 34],
        assistance: Vec::new(),
        adapter_name: "NVIDIA GeForce RTX 3050".to_string(),
        backend_api: "vulkan".to_string(),
        source_commit: SOURCE_COMMIT.to_string(),
        source_tree: SOURCE_TREE.to_string(),
    }
}

#[test]
fn era1_ability_contract_names_every_required_target_once() {
    assert_eq!(ERA1_ABILITY_COUNT, 11);
    assert_eq!(
        Era1Ability::ALL,
        [
            Era1Ability::FlexibleForaging,
            Era1Ability::HazardAvoidance,
            Era1Ability::SpatialMemory,
            Era1Ability::DelayedChoice,
            Era1Ability::RewardReversal,
            Era1Ability::ObjectTransfer,
            Era1Ability::MultiStepProblem,
            Era1Ability::IndividualRecognition,
            Era1Ability::Imitation,
            Era1Ability::GroundedLanguage,
            Era1Ability::PostSleepRetention,
        ]
    );
}

#[test]
fn exact_gpu_offspring_receipt_validates_and_unknown_stays_unknown() {
    valid_receipt().validate_contract().unwrap();

    let mut unknown = valid_receipt();
    unknown.score = MetricReading::Unknown;
    assert_eq!(unknown.score, MetricReading::Unknown);
    unknown.validate_contract().unwrap();
}

#[test]
fn receipt_rejects_wrong_authority_identity_and_fabricated_measurement() {
    let mutations: Vec<Box<dyn Fn(&mut Era1TrialReceipt)>> = vec![
        Box::new(|receipt| receipt.identity.seed = 0),
        Box::new(|receipt| receipt.identity.organism_id = OrganismId::INVALID),
        Box::new(|receipt| receipt.identity.brain_class_id = BrainClassId(2)),
        Box::new(|receipt| receipt.identity.parent_genome_ids.clear()),
        Box::new(|receipt| receipt.identity.parent_genome_ids[1] = GenomeId(101)),
        Box::new(|receipt| receipt.policy_backend = PolicyBackend::HeuristicBaseline),
        Box::new(|receipt| receipt.sensor_profile = SensorProfile::PrivilegedAffordanceV1),
        Box::new(|receipt| receipt.world_digest = [0; 4]),
        Box::new(|receipt| receipt.perception_digest = [0; 4]),
        Box::new(|receipt| receipt.sealed_evidence_digest = [0; 4]),
        Box::new(|receipt| receipt.foundation_id = 0),
        Box::new(|receipt| receipt.foundation_version = 0),
        Box::new(|receipt| receipt.adapter_name.clear()),
        Box::new(|receipt| receipt.backend_api = "dx12".to_string()),
        Box::new(|receipt| receipt.source_commit = "not-a-git-object".to_string()),
        Box::new(|receipt| {
            receipt.score = MetricReading::Measured {
                value_q16: 65_536,
                exposures: 1,
            }
        }),
        Box::new(|receipt| {
            receipt.score = MetricReading::Measured {
                value_q16: 1,
                exposures: 0,
            }
        }),
    ];

    for mutate in mutations {
        let mut receipt = valid_receipt();
        mutate(&mut receipt);
        assert!(
            receipt.validate_contract().is_err(),
            "mutation was accepted"
        );
    }
}

#[test]
fn hidden_and_held_out_evidence_rejects_assistance() {
    for assistance in [
        Era1AssistanceKind::Teacher,
        Era1AssistanceKind::SemanticPrior,
        Era1AssistanceKind::Translation,
        Era1AssistanceKind::Player,
        Era1AssistanceKind::Possession,
    ] {
        let mut receipt = valid_receipt();
        receipt.assistance.push(assistance);
        assert!(receipt.validate_contract().is_err());
    }
}

#[test]
fn founder_and_offspring_parent_contracts_are_distinct() {
    let mut founder = valid_receipt();
    founder.identity.generation = 0;
    founder.identity.parent_genome_ids.clear();
    founder.partition = Era1EvidencePartition::Acquisition;
    founder.validate_contract().unwrap();

    founder.identity.parent_genome_ids.push(GenomeId(101));
    assert!(founder.validate_contract().is_err());

    let mut impossible_offspring = valid_receipt();
    impossible_offspring.identity.generation = 0;
    assert!(impossible_offspring.validate_contract().is_err());
}

#[test]
fn comparison_and_plateau_contracts_reject_invalid_evidence_shapes() {
    let comparison = Era1MatchedComparison {
        ability: Era1Ability::SpatialMemory,
        control: Era1Control::MemoryDisabled,
        intact_mean_q16: 48_000,
        control_mean_q16: 36_000,
        margin_q16: 12_000,
        matched_cells: 12,
    };
    comparison.validate_contract().unwrap();

    let mut self_comparison = comparison;
    self_comparison.control = Era1Control::Intact;
    assert!(self_comparison.validate_contract().is_err());

    let mut wrong_margin = comparison;
    wrong_margin.margin_q16 = 11_999;
    assert!(wrong_margin.validate_contract().is_err());

    let plateau = Era1PlateauWindow {
        first_generation: 3,
        last_generation: 5,
        improvement_q16: 500,
        complete_cells: 24,
        ecological_regression: false,
        diversity_regression: false,
    };
    plateau.validate_contract().unwrap();

    let mut empty = plateau;
    empty.complete_cells = 0;
    assert!(empty.validate_contract().is_err());
}
