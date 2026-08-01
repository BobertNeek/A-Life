use alife_core::{
    BrainCapacityClass, CreatureGenome, EnvironmentalRegime, Era1Ability, Era1EvidencePartition,
    Era1TrialIdentity, Era1TrialReceipt, FoundationGeneticIdentity, GenomeId, MetricReading,
    OrganismId, PassiveLifeEvent, PassiveLifeStatistics, PhenotypeHash, PolicyBackend,
    SensorProfile, Tick,
};
use alife_tools::{
    ei0_exit_gate::Ei0ExitGateReport,
    era1_evolution::{
        recompute_era1_selection_profile_from_receipt, Era1AcquiredStateEvidence,
        Era1ArchiveReceipt, Era1BirthReceipt, Era1CandidateSelectionReceipt, Era1EcologyReceipt,
        Era1EvolutionConfig, Era1EvolutionReceipt, Era1GenerationReceipt, Era1LineageReceipt,
        Era1PortableSaveReceipt, Era1SelectionCandidateIdentity, Era1SelectionRoundReceipt,
        ERA1_ECOLOGY_RECEIPT_SCHEMA_VERSION, ERA1_EVOLUTION_SCHEMA_VERSION,
        ERA1_SELECTION_EVIDENCE_SCHEMA_VERSION,
    },
    era1_promotion::canonical_world_family_id,
    p33_evaluation::{ObjectiveVector, ScoreEstimate},
    p33_selection::{
        run_managed_selection, ManagedSelectionConfig, PopulationLane, SelectionCandidate,
    },
};
use alife_world::{
    HabitatActor, HabitatBreedingKind, HabitatBreedingReceipt, HabitatId, HabitatMode,
};

const SOURCE_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const SOURCE_TREE: &str = "89abcdef0123456789abcdef0123456789abcdef";

pub fn validator_only_evolution_receipt() -> Era1EvolutionReceipt {
    let config = Era1EvolutionConfig::bounded_default(0xE1_6001).unwrap();
    let wild_reservoir = [61_001, 61_002, 61_003, 61_004]
        .into_iter()
        .map(founder)
        .collect::<Vec<_>>();
    let wild_ids = wild_reservoir
        .iter()
        .map(|genome| genome.id)
        .collect::<Vec<_>>();
    let founder_births = wild_reservoir
        .iter()
        .cloned()
        .enumerate()
        .map(|(slot, genome)| birth(0, slot, genome))
        .collect::<Vec<_>>();
    let mut generations = vec![generation_receipt(
        0,
        founder_births,
        &wild_ids,
        None,
        Vec::new(),
    )];
    let mut selection_rounds = Vec::new();

    for generation in 1..=config.ordinary_birth_generations {
        let parent_generation = generation - 1;
        let parents = generations.last().unwrap().births.clone();
        let evidence = parents
            .iter()
            .enumerate()
            .map(|(index, parent)| {
                selection_receipt(
                    &config,
                    parent,
                    &parents[(index + 1) % parents.len()].genome,
                )
            })
            .collect::<Vec<_>>();
        let profiles = parents
            .iter()
            .zip(&evidence)
            .map(|(parent, receipt)| {
                recompute_era1_selection_profile_from_receipt(
                    &config,
                    &parent.genome,
                    receipt,
                    parents.len(),
                    &parent.genome.parent_genome_ids,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let candidates = selection_candidates(&wild_reservoir, &parents, &profiles);
        let plan =
            run_managed_selection(&candidates, &selection_config(&config, generation)).unwrap();
        let births = materialize_births(&config, generation, &parents, &plan);
        let habitat_breeding = plan
            .pairings
            .iter()
            .map(|pairing| {
                let maternal = parents
                    .iter()
                    .find(|birth| birth.genome.id == pairing.maternal_genome_id)
                    .unwrap();
                let paternal = parents
                    .iter()
                    .find(|birth| birth.genome.id == pairing.paternal_genome_id)
                    .unwrap();
                HabitatBreedingReceipt {
                    habitat_id: HabitatId::new(2).unwrap(),
                    first_parent: maternal.organism_id,
                    second_parent: paternal.organism_id,
                    mode: HabitatMode::Managed,
                    kind: HabitatBreedingKind::Explicit,
                    actor: HabitatActor::WorldAuthority,
                    tick: Tick::new(u64::from(generation)),
                    cognition_policy: PolicyBackend::NeuralClosedLoopGpu,
                }
            })
            .collect();
        generations.push(generation_receipt(
            generation,
            births,
            &wild_ids,
            Some(plan),
            habitat_breeding,
        ));
        selection_rounds.push(Era1SelectionRoundReceipt {
            parent_generation,
            evidence,
            derived_profiles: profiles,
        });
    }

    let lineages = (0..config.lineage_count)
        .map(|slot| Era1LineageReceipt {
            lineage_slot: slot,
            founder_genome_id: wild_reservoir[slot].id,
            genome_ids: generations
                .iter()
                .map(|generation| generation.births[slot].genome.id)
                .collect(),
        })
        .collect();
    let receipt = Era1EvolutionReceipt {
        schema_version: ERA1_EVOLUTION_SCHEMA_VERSION,
        config,
        baseline_ei0_exit_gate: committed_ei0_exit_gate(),
        wild_reservoir,
        selection_rounds,
        generations,
        lineages,
    };
    receipt.validate_contract().unwrap();
    receipt
}

pub fn committed_ei0_exit_gate() -> Ei0ExitGateReport {
    serde_json::from_str(include_str!("../../reports/ei0_exit_gate_report.json")).unwrap()
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

fn birth(generation: u32, slot: usize, genome: CreatureGenome) -> Era1BirthReceipt {
    let inherited_starter_tokens = genome.express().unwrap().predisposition.starter_tokens;
    Era1BirthReceipt {
        generation,
        lineage_slot: slot,
        organism_id: OrganismId(20_000 + u64::from(generation) * 100 + slot as u64 + 1),
        genome,
        inherited_starter_tokens,
        acquired_state: Era1AcquiredStateEvidence::default(),
    }
}

fn selection_receipt(
    config: &Era1EvolutionConfig,
    parent: &Era1BirthReceipt,
    reproduction_partner: &CreatureGenome,
) -> Era1CandidateSelectionReceipt {
    let identity = Era1SelectionCandidateIdentity {
        organism_id: parent.organism_id,
        genome_id: parent.genome.id,
        parent_genome_ids: parent.genome.parent_genome_ids.clone(),
        lineage_id: parent.genome.lineage_id,
        generation: parent.generation,
    };
    let mut trial_receipts = Vec::new();
    for &seed in &config.evaluation_seeds {
        for &world in &config.held_out_world_transforms {
            for ability in Era1Ability::ALL {
                for &control in &config.controls {
                    let domain = u64::from(ability as u8) ^ u64::from(control as u8).rotate_left(9);
                    trial_receipts.push(Era1TrialReceipt {
                        schema_version: alife_core::ERA1_EVALUATION_SCHEMA_VERSION,
                        identity: Era1TrialIdentity {
                            seed,
                            organism_id: parent.organism_id,
                            genome_id: parent.genome.id,
                            parent_genome_ids: parent.genome.parent_genome_ids.clone(),
                            lineage_id: parent.genome.lineage_id,
                            generation: parent.generation,
                            brain_class_id: BrainCapacityClass::N2048_ID,
                            world_family_id: canonical_world_family_id(ability),
                            world_variant_id: world,
                        },
                        ability,
                        control,
                        partition: if parent.generation == 0 {
                            Era1EvidencePartition::HeldOutTransfer
                        } else {
                            Era1EvidencePartition::ReproducedOffspring
                        },
                        score: MetricReading::Measured {
                            value_q16: 49_151,
                            exposures: 4,
                        },
                        phenotype_hash: PhenotypeHash([parent.genome.id.0, 1, 2, 3]),
                        foundation_id: parent.genome.foundation.foundation_id,
                        foundation_version: u32::from(parent.genome.foundation.version),
                        sensor_profile: SensorProfile::GroundedObjectSlotsV1,
                        policy_backend: PolicyBackend::NeuralClosedLoopGpu,
                        world_digest: [seed, world, domain, 1],
                        perception_digest: [seed, world, domain, 2],
                        sealed_evidence_digest: [seed, world, domain, 3],
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
    let ecology_receipts = config
        .evaluation_seeds
        .iter()
        .flat_map(|seed| {
            let identity = identity.clone();
            config.held_out_world_transforms.iter().map(move |world| {
                let reproduction_seed = *seed ^ *world ^ parent.genome.id.0.rotate_left(7);
                let offspring = CreatureGenome::reproduce(
                    &parent.genome,
                    reproduction_partner,
                    reproduction_seed,
                )
                .unwrap();
                Era1EcologyReceipt {
                    schema_version: ERA1_ECOLOGY_RECEIPT_SCHEMA_VERSION,
                    identity: identity.clone(),
                    evaluation_seed: *seed,
                    world_variant_id: *world,
                    statistics: complete_life_statistics(parent.organism_id),
                    trial_evidence_digest: format!("blake3-256:{}", "3".repeat(64)),
                    reproduction_partner: reproduction_partner.clone(),
                    reproduction_seed,
                    reproduction_offspring_genome_id: offspring.id,
                    source_commit: SOURCE_COMMIT.to_string(),
                    source_tree: SOURCE_TREE.to_string(),
                }
            })
        })
        .collect();
    Era1CandidateSelectionReceipt {
        schema_version: ERA1_SELECTION_EVIDENCE_SCHEMA_VERSION,
        identity,
        trial_receipts,
        ecology_receipts,
    }
}

fn complete_life_statistics(organism_id: OrganismId) -> PassiveLifeStatistics {
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
                energy_q16: 58_982,
                movement_distance_q16: 39_321,
                gpu_dispatched: true,
                gpu_throttled: false,
            })
            .unwrap();
    }
    for event in [
        PassiveLifeEvent::FoodOutcome { beneficial: true },
        PassiveLifeEvent::PoisonEncounter { avoided: true },
        PassiveLifeEvent::HazardEncounter { avoided: true },
        PassiveLifeEvent::Reproduction { successful: true },
        PassiveLifeEvent::SleepRetention { retained: true },
        PassiveLifeEvent::LearningProbe {
            improvement_q16: 49_151,
        },
        PassiveLifeEvent::ReversalRecovery {
            ticks_to_recover: 1,
        },
        PassiveLifeEvent::VocabularyGrounding { correct: true },
        PassiveLifeEvent::Comprehension {
            assisted: false,
            correct: true,
        },
        PassiveLifeEvent::PeerCommunication { successful: true },
        PassiveLifeEvent::DialectTransfer { successful: true },
        PassiveLifeEvent::DialectDivergence {
            distance_q16: 26_214,
        },
    ] {
        statistics.observe(event).unwrap();
    }
    statistics
        .finalize(Tick::new(7), "validator-only completed ecology")
        .unwrap();
    statistics
}

fn selection_candidates(
    wild: &[CreatureGenome],
    parents: &[Era1BirthReceipt],
    profiles: &[alife_tools::era1_evolution::Era1SelectionProfile],
) -> Vec<SelectionCandidate> {
    let mut candidates = wild
        .iter()
        .cloned()
        .map(|genome| SelectionCandidate {
            genome,
            objectives: unknown_objectives(),
            known_ancestor_genome_ids: Vec::new(),
            population_share: 1.0,
            lane: PopulationLane::Wild,
            specialist_roles: Vec::new(),
        })
        .collect::<Vec<_>>();
    for (parent, profile) in parents.iter().zip(profiles) {
        let mut ancestors = profile.known_ancestor_genome_ids.clone();
        ancestors.extend(parent.genome.parent_genome_ids.iter().copied());
        ancestors.sort_by_key(|id| id.0);
        ancestors.dedup();
        candidates.push(SelectionCandidate {
            genome: parent.genome.clone(),
            objectives: profile.objectives.clone(),
            known_ancestor_genome_ids: ancestors,
            population_share: profile.population_share,
            lane: PopulationLane::Managed,
            specialist_roles: profile.specialist_roles.clone(),
        });
    }
    candidates
}

fn materialize_births(
    config: &Era1EvolutionConfig,
    generation: u32,
    parents: &[Era1BirthReceipt],
    plan: &alife_tools::p33_selection::ManagedBreedingPlan,
) -> Vec<Era1BirthReceipt> {
    let parent_by_id = parents
        .iter()
        .map(|parent| (parent.genome.id.0, parent))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut genomes = plan
        .offspring
        .iter()
        .map(|offspring| offspring.genome.clone())
        .collect::<Vec<_>>();
    let mut sibling_round = 1_u64;
    while genomes.len() < config.lineage_count {
        let mut order = (0..plan.pairings.len()).collect::<Vec<_>>();
        order.sort_by_key(|index| (plan.pairings[*index].offspring_genome_ids.len(), *index));
        for pairing_index in order {
            if genomes.len() == config.lineage_count {
                break;
            }
            let pairing = &plan.pairings[pairing_index];
            let maternal = &parent_by_id[&pairing.maternal_genome_id.0].genome;
            let paternal = &parent_by_id[&pairing.paternal_genome_id.0].genome;
            let seed = derived_seed(
                config.evolution_seed
                    ^ maternal.id.0
                    ^ paternal.id.0.rotate_left(23)
                    ^ sibling_round.rotate_left(7),
                u64::from(generation),
                pairing_index as u64,
            );
            let child = CreatureGenome::reproduce(maternal, paternal, seed).unwrap();
            if !genomes.iter().any(|genome| genome.id == child.id) {
                genomes.push(child);
            }
        }
        sibling_round += 1;
    }
    genomes.truncate(config.lineage_count);
    genomes
        .into_iter()
        .enumerate()
        .map(|(slot, genome)| birth(generation, slot, genome))
        .collect()
}

fn generation_receipt(
    generation: u32,
    births: Vec<Era1BirthReceipt>,
    wild_ids: &[GenomeId],
    selection_plan: Option<alife_tools::p33_selection::ManagedBreedingPlan>,
    habitat_breeding: Vec<HabitatBreedingReceipt>,
) -> Era1GenerationReceipt {
    let organism_ids = births.iter().map(|birth| birth.organism_id).collect();
    let genome_ids = births.iter().map(|birth| birth.genome.id).collect();
    let archives = births
        .iter()
        .map(|birth| Era1ArchiveReceipt {
            generation,
            organism_id: birth.organism_id,
            genome_id: birth.genome.id,
            manifest_digest_hex: format!("blake3-256:{}", "4".repeat(64)),
        })
        .collect();
    Era1GenerationReceipt {
        generation,
        births,
        preserved_wild_genome_ids: wild_ids.to_vec(),
        selection_plan,
        habitat_breeding,
        archives,
        portable_save: Era1PortableSaveReceipt {
            generation,
            relative_path: format!("generation-{generation}.json"),
            digest_hex: format!("blake3-256:{}", "5".repeat(64)),
            organism_ids,
            genome_ids,
        },
    }
}

fn selection_config(config: &Era1EvolutionConfig, generation: u32) -> ManagedSelectionConfig {
    ManagedSelectionConfig {
        selection_seed: derived_seed(config.evolution_seed, 0xE1A1_5000, u64::from(generation)),
        max_pairings: config.lineage_count,
        minority_lineage_share_max: 0.25,
        fragile_ecology_max: 0.30,
        high_cognition_min: 0.75,
        robust_ecology_min: 0.65,
        introgression_sibling_count: 2,
    }
}

fn unknown_objectives() -> ObjectiveVector {
    ObjectiveVector {
        ecological: ScoreEstimate::UNKNOWN,
        cognitive: ScoreEstimate::UNKNOWN,
        social: ScoreEstimate::UNKNOWN,
        group: ScoreEstimate::UNKNOWN,
        stability: ScoreEstimate::UNKNOWN,
        efficiency: ScoreEstimate::UNKNOWN,
        diversity: ScoreEstimate::UNKNOWN,
    }
}

fn derived_seed(root: u64, domain: u64, index: u64) -> u64 {
    let mut value = root ^ domain.rotate_left(17) ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    value.max(1)
}
