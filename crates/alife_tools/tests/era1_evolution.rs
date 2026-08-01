use alife_core::{
    BrainCapacityClass, CreatureGenome, EnvironmentalRegime, Era1Ability, Era1Control,
    Era1EvidencePartition, Era1TrialIdentity, Era1TrialReceipt, FoundationGeneticIdentity,
    LanguageTokenId, MetricReading, OrganismId, PassiveLifeEvent, PassiveLifeStatistics,
    PhenotypeHash, PolicyBackend, SensorProfile, Tick,
};
use alife_tools::{
    ei0_exit_gate::Ei0ExitGateReport,
    era1_evolution::{
        recompute_era1_selection_profile_from_receipt, run_era1_evolution, Era1BirthReceipt,
        Era1CandidateSelectionEvidence, Era1CandidateSelectionReceipt, Era1EcologyReceipt,
        Era1EvolutionConfig, Era1EvolutionError, Era1SelectionCandidateIdentity,
    },
};

mod common;

const SOURCE_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SOURCE_TREE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn candidate_identity(
    genome: &CreatureGenome,
    organism_id: OrganismId,
    generation: u32,
) -> Era1SelectionCandidateIdentity {
    Era1SelectionCandidateIdentity {
        organism_id,
        genome_id: genome.id,
        parent_genome_ids: genome.parent_genome_ids.clone(),
        lineage_id: genome.lineage_id,
        generation,
    }
}

fn complete_life_statistics(organism_id: OrganismId) -> PassiveLifeStatistics {
    life_statistics(organism_id, true, 49_151)
}

fn life_statistics(
    organism_id: OrganismId,
    successful: bool,
    energy_q16: u32,
) -> PassiveLifeStatistics {
    let mut statistics = PassiveLifeStatistics::new(organism_id, Tick::ZERO).unwrap();
    for (index, regime) in [
        EnvironmentalRegime::Temperate,
        EnvironmentalRegime::Scarcity,
        EnvironmentalRegime::Abundance,
        EnvironmentalRegime::Hazardous,
        EnvironmentalRegime::Social,
        EnvironmentalRegime::Novel,
    ]
    .into_iter()
    .enumerate()
    {
        statistics
            .observe(PassiveLifeEvent::SurvivalTick {
                tick: Tick::new(index as u64 + 1),
                regime,
                energy_q16,
                movement_distance_q16: 32_768,
                gpu_dispatched: true,
                gpu_throttled: false,
            })
            .unwrap();
    }
    for event in [
        PassiveLifeEvent::FoodOutcome {
            beneficial: successful,
        },
        PassiveLifeEvent::PoisonEncounter {
            avoided: successful,
        },
        PassiveLifeEvent::HazardEncounter {
            avoided: successful,
        },
        PassiveLifeEvent::Reproduction { successful },
        PassiveLifeEvent::SleepRetention { retained: true },
        PassiveLifeEvent::LearningProbe {
            improvement_q16: 45_875,
        },
        PassiveLifeEvent::ReversalRecovery {
            ticks_to_recover: 1,
        },
        PassiveLifeEvent::VocabularyGrounding { correct: true },
        PassiveLifeEvent::Comprehension {
            assisted: false,
            correct: true,
        },
        PassiveLifeEvent::Comprehension {
            assisted: true,
            correct: true,
        },
        PassiveLifeEvent::NarrationUtterance,
        PassiveLifeEvent::Narration { faithful: true },
        PassiveLifeEvent::PeerCommunication { successful: true },
        PassiveLifeEvent::DialectTransfer { successful: true },
        PassiveLifeEvent::DialectDivergence {
            distance_q16: 26_214,
        },
    ] {
        statistics.observe(event).unwrap();
    }
    statistics
        .finalize(Tick::new(7), "completed evaluation")
        .unwrap();
    statistics
}

fn bounded_window_life_statistics(organism_id: OrganismId) -> PassiveLifeStatistics {
    let regimes = [
        EnvironmentalRegime::Temperate,
        EnvironmentalRegime::Scarcity,
        EnvironmentalRegime::Abundance,
        EnvironmentalRegime::Hazardous,
        EnvironmentalRegime::Social,
        EnvironmentalRegime::Novel,
    ];
    let mut statistics = PassiveLifeStatistics::new(organism_id, Tick::ZERO).unwrap();
    for tick in 1..=220_u64 {
        statistics
            .observe(PassiveLifeEvent::SurvivalTick {
                tick: Tick::new(tick),
                regime: regimes[(tick as usize - 1) % regimes.len()],
                energy_q16: 33_801,
                movement_distance_q16: 0,
                gpu_dispatched: true,
                gpu_throttled: false,
            })
            .unwrap();
    }
    for event in [
        PassiveLifeEvent::FoodOutcome { beneficial: false },
        PassiveLifeEvent::PoisonEncounter { avoided: false },
        PassiveLifeEvent::HazardEncounter { avoided: false },
        PassiveLifeEvent::Reproduction { successful: true },
        PassiveLifeEvent::SleepRetention { retained: false },
        PassiveLifeEvent::LearningProbe { improvement_q16: 0 },
        PassiveLifeEvent::ReversalRecovery {
            ticks_to_recover: u32::MAX,
        },
        PassiveLifeEvent::VocabularyGrounding { correct: false },
        PassiveLifeEvent::Comprehension {
            assisted: false,
            correct: false,
        },
        PassiveLifeEvent::Comprehension {
            assisted: true,
            correct: false,
        },
        PassiveLifeEvent::NarrationUtterance,
        PassiveLifeEvent::Narration { faithful: false },
        PassiveLifeEvent::PeerCommunication { successful: false },
        PassiveLifeEvent::DialectTransfer { successful: false },
        PassiveLifeEvent::DialectDivergence { distance_q16: 0 },
    ] {
        statistics.observe(event).unwrap();
    }
    statistics
        .finalize(Tick::new(221), "completed bounded Era 1 window")
        .unwrap();
    statistics
}

fn complete_candidate_receipt(
    config: &Era1EvolutionConfig,
    genome: &CreatureGenome,
    organism_id: OrganismId,
    generation: u32,
) -> Era1CandidateSelectionReceipt {
    let identity = candidate_identity(genome, organism_id, generation);
    let mut trial_receipts = Vec::new();
    for &seed in &config.evaluation_seeds {
        for &world in &config.held_out_world_transforms {
            for ability in Era1Ability::ALL {
                for &control in &config.controls {
                    trial_receipts.push(Era1TrialReceipt {
                        schema_version: alife_core::ERA1_EVALUATION_SCHEMA_VERSION,
                        identity: Era1TrialIdentity {
                            seed,
                            organism_id,
                            genome_id: genome.id,
                            parent_genome_ids: genome.parent_genome_ids.clone(),
                            lineage_id: genome.lineage_id,
                            generation,
                            brain_class_id: BrainCapacityClass::N2048_ID,
                            world_family_id: alife_tools::era1_promotion::canonical_world_family_id(
                                ability,
                            ),
                            world_variant_id: world,
                        },
                        ability,
                        control,
                        partition: if generation == 0 {
                            Era1EvidencePartition::HeldOutTransfer
                        } else {
                            Era1EvidencePartition::ReproducedOffspring
                        },
                        score: MetricReading::Measured {
                            value_q16: 45_875 + u32::from(ability as u8) * 256,
                            exposures: 1,
                        },
                        phenotype_hash: PhenotypeHash([1, 2, 3, 4]),
                        foundation_id: genome.foundation.foundation_id,
                        foundation_version: u32::from(genome.foundation.version),
                        sensor_profile: SensorProfile::GroundedObjectSlotsV1,
                        policy_backend: PolicyBackend::NeuralClosedLoopGpu,
                        world_digest: [11, 12, 13, 14],
                        perception_digest: [21, 22, 23, 24],
                        sealed_evidence_digest: [31, 32, 33, 34],
                        assistance: Vec::new(),
                        adapter_name: "NVIDIA GeForce RTX 3050".to_string(),
                        backend_api: "vulkan".to_string(),
                        source_commit: SOURCE_COMMIT.to_string(),
                        source_tree: SOURCE_TREE.to_string(),
                    });
                }
            }
        }
    }
    let reproduction_partner = founder(genome.id.0 ^ 0xE101_0C01);
    let ecology_receipts = config
        .evaluation_seeds
        .iter()
        .flat_map(|seed| {
            let identity = identity.clone();
            config.held_out_world_transforms.iter().map({
                let reproduction_partner = reproduction_partner.clone();
                move |world| {
                    let reproduction_seed = *seed ^ *world ^ 0xE101_0C02;
                    let offspring =
                        CreatureGenome::reproduce(genome, &reproduction_partner, reproduction_seed)
                            .unwrap();
                    Era1EcologyReceipt {
                        schema_version:
                            alife_tools::era1_evolution::ERA1_ECOLOGY_RECEIPT_SCHEMA_VERSION,
                        identity: identity.clone(),
                        evaluation_seed: *seed,
                        world_variant_id: *world,
                        statistics: complete_life_statistics(organism_id),
                        trial_evidence_digest: format!("blake3-256:{}", "1".repeat(64)),
                        reproduction_partner: reproduction_partner.clone(),
                        reproduction_seed,
                        reproduction_offspring_genome_id: offspring.id,
                        source_commit: SOURCE_COMMIT.to_string(),
                        source_tree: SOURCE_TREE.to_string(),
                    }
                }
            })
        })
        .collect();
    Era1CandidateSelectionReceipt {
        schema_version: alife_tools::era1_evolution::ERA1_SELECTION_EVIDENCE_SCHEMA_VERSION,
        identity,
        trial_receipts,
        ecology_receipts,
    }
}

fn incomplete_candidate_evidence(
    config: &Era1EvolutionConfig,
    genome: &CreatureGenome,
    organism_id: OrganismId,
    generation: u32,
) -> Era1CandidateSelectionEvidence {
    let receipt = complete_candidate_receipt(config, genome, organism_id, generation);
    Era1CandidateSelectionEvidence {
        schema_version: receipt.schema_version,
        identity: receipt.identity,
        trial_evidence: Vec::new(),
        ecology_receipts: receipt.ecology_receipts,
    }
}

fn founder(seed: u64) -> CreatureGenome {
    let foundation = FoundationGeneticIdentity::new(
        0x4E32_3034_385F_5631,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    CreatureGenome::early_mammal_founder(seed, foundation).unwrap()
}

fn committed_ei0_exit_gate() -> Ei0ExitGateReport {
    serde_json::from_str(include_str!("../reports/ei0_exit_gate_report.json")).unwrap()
}

fn evidence_for_candidate(
    config: &Era1EvolutionConfig,
    generation: u32,
    candidate_index: usize,
    births: &[Era1BirthReceipt],
) -> Era1CandidateSelectionEvidence {
    let birth = &births[candidate_index];
    incomplete_candidate_evidence(config, &birth.genome, birth.organism_id, generation)
}

#[test]
fn bounded_default_runs_exact_seeded_two_generation_reproduction() {
    let run = common::validator_only_evolution_receipt();
    let repeated = common::validator_only_evolution_receipt();
    let founders = run.wild_reservoir.clone();
    let config = run.config.clone();

    assert_eq!(run, repeated);
    assert_eq!(run.wild_reservoir, founders);
    assert_eq!(config.lineage_count, 4);
    assert_eq!(config.evaluation_seeds.len(), 3);
    assert_eq!(config.held_out_world_transforms.len(), 2);
    assert_eq!(config.controls, Era1Control::ALL);
    assert_eq!(config.ordinary_birth_generations, 2);
    assert_eq!(run.generations.len(), 3);
    assert_eq!(run.lineages.len(), 4);
    assert!(run.generations.iter().all(|generation| {
        !generation.archives.is_empty()
            && !generation.portable_save.digest_hex.is_empty()
            && generation.preserved_wild_genome_ids
                == founders.iter().map(|genome| genome.id).collect::<Vec<_>>()
    }));

    for generation_index in 1..run.generations.len() {
        let parents = &run.generations[generation_index - 1].births;
        let generation = &run.generations[generation_index];
        let plan = generation.selection_plan.as_ref().unwrap();
        assert!(!plan.pairings.is_empty());
        assert_eq!(generation.habitat_breeding.len(), plan.pairings.len());
        assert_eq!(generation.births.len(), 4);
        for birth in &generation.births {
            assert!(birth.genome.provenance.ordinary_birth);
            assert_eq!(birth.genome.parent_genome_ids.len(), 2);
            let maternal = parents
                .iter()
                .find(|candidate| candidate.genome.id == birth.genome.parent_genome_ids[0])
                .unwrap();
            let paternal = parents
                .iter()
                .find(|candidate| candidate.genome.id == birth.genome.parent_genome_ids[1])
                .unwrap();
            assert_eq!(
                birth.genome,
                CreatureGenome::reproduce(
                    &maternal.genome,
                    &paternal.genome,
                    birth.genome.conception_seed,
                )
                .unwrap()
            );
        }
    }
}

#[test]
fn ordinary_children_inherit_only_dna_starter_words_and_empty_lifetime_state() {
    let run = common::validator_only_evolution_receipt();

    for birth in run
        .generations
        .iter()
        .skip(1)
        .flat_map(|generation| &generation.births)
    {
        assert!(birth.acquired_state.is_empty());
        let expressed = birth.genome.express().unwrap();
        assert_eq!(
            birth.inherited_starter_tokens,
            expressed.predisposition.starter_tokens
        );
        assert!(!birth.inherited_starter_tokens.is_empty());
        assert!(birth
            .inherited_starter_tokens
            .iter()
            .all(|token| *token != LanguageTokenId::new(0).unwrap()));
    }
}

#[test]
fn copied_learning_or_fabricated_inheritance_invalidates_evolution_receipts() {
    let run = common::validator_only_evolution_receipt();
    run.validate_contract().unwrap();

    let mut forged_profile = run.clone();
    forged_profile.selection_rounds[0].derived_profiles[0]
        .objectives
        .cognitive
        .value = Some(0.0);
    assert!(forged_profile.validate_contract().is_err());

    let mut copied_learning = run.clone();
    copied_learning.generations[1].births[0]
        .acquired_state
        .learned_vocabulary
        .push(LanguageTokenId::new(41).unwrap());
    assert!(copied_learning.validate_contract().is_err());

    let mut injected_silence = run.clone();
    injected_silence.generations[1].births[0]
        .inherited_starter_tokens
        .push(LanguageTokenId::new(0).unwrap());
    assert!(injected_silence.validate_contract().is_err());

    let mut fabricated_parent = run;
    fabricated_parent.generations[1].births[0]
        .genome
        .parent_genome_ids[0] = fabricated_parent.generations[0].births[2].genome.id;
    assert!(fabricated_parent.validate_contract().is_err());
}

#[test]
fn incomplete_trial_coverage_mismatched_identity_and_unknown_ecology_are_rejected() {
    let founders = vec![
        founder(54_001),
        founder(54_002),
        founder(54_003),
        founder(54_004),
    ];
    let config = Era1EvolutionConfig::bounded_default(0xE1_5004).unwrap();
    let gate = committed_ei0_exit_gate();
    let root = tempfile::tempdir().unwrap();

    let incomplete = run_era1_evolution(
        &config,
        Some(&gate),
        &founders,
        root.path().join("incomplete"),
        |generation, candidate_index, births| {
            Ok(evidence_for_candidate(
                &config,
                generation,
                candidate_index,
                births,
            ))
        },
    );
    assert!(
        matches!(
            &incomplete,
            Err(Era1EvolutionError::InvalidEvidence(
                "selection trial coverage is incomplete"
            ))
        ),
        "{incomplete:?}"
    );

    let mismatched = run_era1_evolution(
        &config,
        Some(&gate),
        &founders,
        root.path().join("mismatched"),
        |generation, candidate_index, births| {
            let mut evidence = evidence_for_candidate(&config, generation, candidate_index, births);
            evidence.identity.generation = generation + 1;
            Ok(evidence)
        },
    );
    assert!(
        matches!(
            &mismatched,
            Err(Era1EvolutionError::InvalidEvidence(
                "selection evidence does not match the stable parent identity"
            ))
        ),
        "{mismatched:?}"
    );
}

#[test]
fn validator_only_receipt_recomputes_managed_plans_and_probation_contracts() {
    let run = common::validator_only_evolution_receipt();
    run.validate_contract().unwrap();
    for generation in run.generations.iter().skip(1) {
        let plan = generation.selection_plan.as_ref().unwrap();
        assert!(!plan.pairings.is_empty());
        assert!(plan
            .offspring
            .iter()
            .filter(|offspring| offspring.probation.is_some())
            .all(|offspring| offspring
                .probation
                .as_ref()
                .is_some_and(|probation| !probation.sibling_controls.is_empty()
                    && !probation.population_controls.is_empty())));
    }
}

#[test]
fn missing_or_tampered_ei0_gate_prevents_all_evolution_artifacts() {
    let founders = vec![
        founder(57_001),
        founder(57_002),
        founder(57_003),
        founder(57_004),
    ];
    let config = Era1EvolutionConfig::bounded_default(0xE1_5007).unwrap();
    let root = tempfile::tempdir().unwrap();
    let missing_root = root.path().join("missing");
    let missing = run_era1_evolution(
        &config,
        None,
        &founders,
        &missing_root,
        |generation, candidate_index, births| {
            Ok(evidence_for_candidate(
                &config,
                generation,
                candidate_index,
                births,
            ))
        },
    );
    assert!(matches!(missing, Err(Era1EvolutionError::Ei0Gate(_))));
    assert!(!missing_root.exists());

    let mut tampered_gate = committed_ei0_exit_gate();
    tampered_gate.verdict.era0_exit_gate_passed = false;
    let tampered_root = root.path().join("tampered");
    let tampered = run_era1_evolution(
        &config,
        Some(&tampered_gate),
        &founders,
        &tampered_root,
        |generation, candidate_index, births| {
            Ok(evidence_for_candidate(
                &config,
                generation,
                candidate_index,
                births,
            ))
        },
    );
    assert!(matches!(tampered, Err(Era1EvolutionError::Ei0Gate(_))));
    assert!(!tampered_root.exists());
}

#[test]
fn selection_profiles_are_derived_from_complete_receipts_and_unknown_ecology_blocks() {
    let genome = founder(56_001);
    let config = Era1EvolutionConfig::bounded_default(0xE1_5006).unwrap();
    let organism_id = OrganismId(20_001);
    let evidence = complete_candidate_receipt(&config, &genome, organism_id, 0);

    let derived =
        recompute_era1_selection_profile_from_receipt(&config, &genome, &evidence, 4, &[]).unwrap();
    assert_eq!(
        derived.identity,
        candidate_identity(&genome, organism_id, 0)
    );
    assert!(derived.objectives.all_known());

    let mut duplicated_world_cell = evidence.clone();
    duplicated_world_cell.trial_receipts[0].identity.seed = config.evaluation_seeds[1];
    assert!(matches!(
        recompute_era1_selection_profile_from_receipt(
            &config,
            &genome,
            &duplicated_world_cell,
            4,
            &[]
        ),
        Err(Era1EvolutionError::InvalidEvidence(
            "selection trial coverage contains duplicates"
        ))
    ));

    let mut mismatched_source = evidence.clone();
    mismatched_source.ecology_receipts[0].source_tree =
        "cccccccccccccccccccccccccccccccccccccccc".to_string();
    assert!(matches!(
        recompute_era1_selection_profile_from_receipt(&config, &genome, &mismatched_source, 4, &[]),
        Err(Era1EvolutionError::InvalidEvidence(
            "ecology receipt has mismatched or incomplete provenance"
        ))
    ));

    let mut unknown = evidence;
    let mut incomplete = PassiveLifeStatistics::new(organism_id, Tick::ZERO).unwrap();
    incomplete
        .finalize(Tick::ZERO, "completed without ecological exposure")
        .unwrap();
    unknown.ecology_receipts[0].statistics = incomplete;
    assert!(matches!(
        recompute_era1_selection_profile_from_receipt(
            &config,
            &genome,
            &unknown,
            4,
            &[]
        ),
        Err(Era1EvolutionError::UnknownSelectionObjective(id)) if id == genome.id
    ));
}

#[test]
fn descendant_selection_rejects_founder_only_evidence_partition() {
    let maternal = founder(56_002);
    let paternal = founder(56_003);
    let genome = CreatureGenome::reproduce(&maternal, &paternal, 0xE1_5007).unwrap();
    let config = Era1EvolutionConfig::bounded_default(0xE1_5007).unwrap();
    let organism_id = OrganismId(20_002);
    let mut evidence = complete_candidate_receipt(&config, &genome, organism_id, 1);
    evidence.trial_receipts[0].partition = Era1EvidencePartition::HeldOutTransfer;

    let result = recompute_era1_selection_profile_from_receipt(&config, &genome, &evidence, 4, &[]);
    let expected = matches!(
        &result,
        Err(Era1EvolutionError::InvalidEvidence(
            "selection trial receipt has mismatched or incomplete provenance"
        ))
    );
    assert!(expected, "{result:?}");
}

#[test]
fn complete_bounded_survival_window_is_scored_as_full_survival() {
    let config = Era1EvolutionConfig::bounded_default(0xE1_5006).unwrap();
    let genome = founder(56_001);
    let organism_id = OrganismId(56_001);
    let mut evidence = complete_candidate_receipt(&config, &genome, organism_id, 0);
    for ecology in &mut evidence.ecology_receipts {
        ecology.statistics = bounded_window_life_statistics(organism_id);
    }

    let profile =
        recompute_era1_selection_profile_from_receipt(&config, &genome, &evidence, 4, &[]).unwrap();
    let ecological = profile.objectives.ecological.value.unwrap();

    assert!((0.419..0.420).contains(&ecological), "{ecological}");
}
