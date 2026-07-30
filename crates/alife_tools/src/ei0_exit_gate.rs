//! Reproducible Era 0 lifecycle-gate evidence.
//!
//! This module composes existing production authorities. It does not score
//! brains, choose neural actions, or redefine any simulation contract.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use alife_archive::{GeneticArchiveInput, LineageLibrary, LineageLibraryConfig};
use alife_core::{
    ActiveChallengeResult, BrainCapacityClass, BrainScaleTier, CreatureGenome,
    FoundationGeneticIdentity, FoundationWeightAsset, GeneticLineageProvenance, GenomeId,
    HomeostaticSnapshot, LineageId, OrganismId, PhenotypeCompiler, PhenotypeHash, PolicyBackend,
    ScaffoldContractError, SensorProfile, Tick, Validate, Vec3f, ACTIVE_CHALLENGE_COUNT,
};
use alife_training::{ActiveBatteryEvidence, N2048ActiveBatteryRunner};
use alife_world::{
    persistence::{
        AssetManifest, CreatureMindSaveSummary, CreatureSaveState, LearningTraceSaveSummary,
        PortableSaveFile, RuntimeConfig, WeightLayerSaveSummary,
    },
    Habitat, HabitatActor, HabitatAuthority, HabitatAuthorityError, HabitatBreedingKind,
    HabitatBreedingRequest, HabitatId, HabitatMode, HeadlessScenarioBuilder,
};
use serde::{Deserialize, Serialize};

pub const EI0_EXIT_GATE_SCHEMA_VERSION: u16 = 1;
const WORLD_SEED: u64 = 0xE10_0A11;
const WILD_HABITAT_RAW: u64 = 11;
const MANAGED_HABITAT_RAW: u64 = 12;
const FOUNDATION_ID_RAW: u64 = 0x4E32_3034_385F_5631;
const FOUNDATION_FAMILY_RAW: u64 = 0x4E32_3034_385F_FA11;

#[derive(Debug, thiserror::Error)]
pub enum Ei0ExitGateError {
    #[error("core contract failed: {0}")]
    Core(#[from] ScaffoldContractError),
    #[error("habitat authority failed: {0}")]
    Habitat(#[from] HabitatAuthorityError),
    #[error("portable save failed: {0}")]
    Persistence(#[from] alife_world::persistence::PersistenceError),
    #[error("lineage archive failed: {0}")]
    Archive(#[from] alife_archive::ArchiveError),
    #[error("GPU active battery failed: {0}")]
    Training(#[from] alife_training::TrainingError),
    #[error("gate report I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("gate report JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("gate evidence is inconsistent: {0}")]
    Evidence(&'static str),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ei0BirthReceipt {
    pub organism_id: OrganismId,
    pub genome_id: GenomeId,
    pub lineage_id: LineageId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub generation: u32,
    pub conception_seed: u64,
    pub ordinary_birth: bool,
    pub provenance: GeneticLineageProvenance,
    pub foundation_id: u64,
    pub foundation_version: u16,
    pub compatibility_family_id: u64,
    pub breeding_kind: HabitatBreedingKind,
    pub actor: HabitatActor,
    pub cognition_policy: PolicyBackend,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ei0LaneReceipt {
    pub mode: HabitatMode,
    pub habitat_id: HabitatId,
    pub births: Vec<Ei0BirthReceipt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ei0LifecycleGateReport {
    pub schema_version: u16,
    pub founder_count: usize,
    pub live_population_count: usize,
    pub generation_count: u32,
    pub run_observed: bool,
    pub portable_save_round_trip: bool,
    pub tampered_save_rejected: bool,
    pub tampered_provenance_rejected: bool,
    pub restored_population_count: usize,
    pub archive_birth_manifest_count: u64,
    pub lineage_compare_passed: bool,
    pub no_lifetime_state_inherited: bool,
    pub player_directed_wild_breeding_rejected: bool,
    pub creature_directed_managed_breeding_rejected: bool,
    pub lanes: Vec<Ei0LaneReceipt>,
    pub population_genomes: Vec<CreatureGenome>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ei0LifecycleEvidence {
    pub report: Ei0LifecycleGateReport,
    pub final_generation_genomes: Vec<CreatureGenome>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ei0GpuBatteryReceipt {
    pub organism_id: OrganismId,
    pub source_creature_genome_id: GenomeId,
    pub brain_genome_id: GenomeId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub lineage_id: LineageId,
    pub phenotype_hash: PhenotypeHash,
    pub foundation_id: u64,
    pub foundation_version: u32,
    pub compatibility_family_id: u64,
    pub policy_backend: PolicyBackend,
    pub completed_challenges: usize,
    pub challenge_results: Vec<ActiveChallengeResult>,
    pub challenge_worlds: u32,
    pub gpu_dispatches: u64,
    pub sealed_outcomes: u64,
    pub sleep_consolidations: u32,
    pub slm_enabled: bool,
    pub adapter_name: String,
    pub backend_api: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ei0HeuristicBaselineBoundary {
    pub source_backend: String,
    pub promotion_eligible: bool,
    pub hidden_promotion_trials: u64,
    pub unknown_measures: Vec<String>,
    pub unknown_measures_preserved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ei0ExitClauses {
    pub run: bool,
    pub observe: bool,
    pub save_load: bool,
    pub wild_breed: bool,
    pub managed_breed: bool,
    pub test: bool,
    pub archive: bool,
    pub compare: bool,
    pub stable_multi_generation_population: bool,
    pub gpu_policy_identity: bool,
    pub no_hidden_policy_control: bool,
}

impl Ei0ExitClauses {
    pub const fn all_passed(self) -> bool {
        self.run
            && self.observe
            && self.save_load
            && self.wild_breed
            && self.managed_breed
            && self.test
            && self.archive
            && self.compare
            && self.stable_multi_generation_population
            && self.gpu_policy_identity
            && self.no_hidden_policy_control
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ei0ExitVerdict {
    pub era0_exit_gate_passed: bool,
    pub era1_promotion_evaluated: bool,
    pub era1_status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ei0ExitGateReport {
    pub schema_version: u16,
    pub verdict: Ei0ExitVerdict,
    pub clauses: Ei0ExitClauses,
    pub lifecycle: Ei0LifecycleGateReport,
    pub gpu_tests: Vec<Ei0GpuBatteryReceipt>,
    pub heuristic_baseline: Ei0HeuristicBaselineBoundary,
}

#[derive(Debug, Deserialize)]
struct BaselineFixture {
    source_backends: Vec<String>,
    layer_counts: BaselineLayerCounts,
    evidence_scope: BaselineEvidenceScope,
    measures: BTreeMap<String, BaselineReading>,
    objectives: BTreeMap<String, BaselineReading>,
    promotion_eligible: bool,
}

#[derive(Debug, Deserialize)]
struct BaselineLayerCounts {
    hidden_promotion: u64,
}

#[derive(Debug, Deserialize)]
struct BaselineEvidenceScope {
    promotion_backend_eligible: bool,
    unsupported_measures: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct BaselineReading {
    value: Option<f64>,
    samples: u64,
}

#[derive(Debug, Clone)]
struct PopulationMember {
    organism_id: OrganismId,
    genome: CreatureGenome,
    generation: u32,
    habitat_id: HabitatId,
}

pub fn run_ei0_lifecycle_gate(
    evidence_root: impl AsRef<Path>,
) -> Result<Ei0LifecycleEvidence, Ei0ExitGateError> {
    let evidence_root = evidence_root.as_ref();
    std::fs::create_dir_all(evidence_root).map_err(alife_archive::ArchiveError::from)?;

    let wild_id = HabitatId::new(WILD_HABITAT_RAW).ok_or(Ei0ExitGateError::Evidence(
        "wild habitat id must be nonzero",
    ))?;
    let managed_id = HabitatId::new(MANAGED_HABITAT_RAW).ok_or(Ei0ExitGateError::Evidence(
        "managed habitat id must be nonzero",
    ))?;
    let mut authority = HabitatAuthority::new(vec![
        Habitat::new(wild_id, "Era 0 Wild", HabitatMode::Wild)?,
        Habitat::new(managed_id, "Era 0 Managed", HabitatMode::Managed)?,
    ])?;

    let mut world_builder = HeadlessScenarioBuilder::new(WORLD_SEED)
        .food("era0-food", Vec3f::new(3.0, 0.0, 0.0), 0.8)
        .hazard("era0-hazard", Vec3f::new(-3.0, 0.0, 0.0), 0.4);
    let mut members = Vec::with_capacity(14);
    for (lane_index, habitat_id) in [wild_id, managed_id].into_iter().enumerate() {
        for founder_index in 0..4_u64 {
            let organism_id = OrganismId(1_000 + lane_index as u64 * 1_000 + founder_index + 1);
            let genome = founder_genome(WORLD_SEED + lane_index as u64 * 100 + founder_index + 1)?;
            let label = format!("era0-founder-{lane_index}-{founder_index}");
            world_builder = world_builder.social_agent(
                &label,
                organism_id,
                Vec3f::new(founder_index as f32, lane_index as f32 * 2.0, 0.0),
                0.5,
            );
            authority.register_creature(organism_id, habitat_id, Tick::ZERO)?;
            members.push(PopulationMember {
                organism_id,
                genome,
                generation: 0,
                habitat_id,
            });
        }
    }
    let mut world = world_builder.build()?;
    world.replace_habitat_authority(authority.clone())?;

    let player_directed_wild_breeding_rejected = authority
        .authorize_breeding(HabitatBreedingRequest {
            habitat_id: wild_id,
            first_parent: members[0].organism_id,
            second_parent: members[1].organism_id,
            kind: HabitatBreedingKind::Explicit,
            actor: HabitatActor::Player,
            tick: Tick::ZERO,
        })
        .is_err();
    let creature_directed_managed_breeding_rejected = authority
        .authorize_breeding(HabitatBreedingRequest {
            habitat_id: managed_id,
            first_parent: members[4].organism_id,
            second_parent: members[5].organism_id,
            kind: HabitatBreedingKind::CreatureChosen,
            actor: HabitatActor::Organism(members[4].organism_id),
            tick: Tick::ZERO,
        })
        .is_err();

    let mut lanes = Vec::with_capacity(2);
    let mut final_generation_genomes = Vec::with_capacity(2);
    let mut next_organism_raw = 3_001_u64;
    for (mode, habitat_id, founder_offset) in [
        (HabitatMode::Wild, wild_id, 0_usize),
        (HabitatMode::Managed, managed_id, 4_usize),
    ] {
        let kind = if mode == HabitatMode::Wild {
            HabitatBreedingKind::CreatureChosen
        } else {
            HabitatBreedingKind::Explicit
        };
        let (first, first_receipt) = breed_member(
            &mut world,
            &mut authority,
            &members[founder_offset],
            &members[founder_offset + 1],
            next_organism_raw,
            habitat_id,
            kind,
            0xE10_1000 + next_organism_raw,
        )?;
        next_organism_raw += 1;
        let (second, second_receipt) = breed_member(
            &mut world,
            &mut authority,
            &members[founder_offset + 2],
            &members[founder_offset + 3],
            next_organism_raw,
            habitat_id,
            kind,
            0xE10_1000 + next_organism_raw,
        )?;
        next_organism_raw += 1;
        let (final_member, final_receipt) = breed_member(
            &mut world,
            &mut authority,
            &first,
            &second,
            next_organism_raw,
            habitat_id,
            kind,
            0xE10_1000 + next_organism_raw,
        )?;
        next_organism_raw += 1;
        final_generation_genomes.push(final_member.genome.clone());
        members.extend([first, second, final_member]);
        lanes.push(Ei0LaneReceipt {
            mode,
            habitat_id,
            births: vec![first_receipt, second_receipt, final_receipt],
        });
    }
    world.replace_habitat_authority(authority)?;

    let run_observed = observe_population(&mut world, &members)?;
    world.advance_tick();
    let creature_saves = members
        .iter()
        .map(|member| creature_save(member, world.tick()))
        .collect::<Vec<_>>();
    let save = PortableSaveFile::from_headless_world(
        "ei0-multi-generation-gate",
        &world,
        RuntimeConfig::deterministic_default(WORLD_SEED, BrainScaleTier::Standard2048),
        AssetManifest::empty(),
        creature_saves,
    )?;
    let save_path = evidence_root.join("ei0-multi-generation.alife.json");
    save.to_json_file(&save_path)?;
    let restored_save = PortableSaveFile::from_json_file(&save_path)?;
    let restored_world = restored_save.restore_headless_world()?;
    let restored_population_count = restored_world.organism_entity_ids().len();
    let portable_save_round_trip = restored_population_count == members.len()
        && restored_world.habitat_authority() == world.habitat_authority()
        && restored_world.tick() == world.tick();
    let mut tampered_save = restored_save.clone();
    tampered_save.deterministic_seed = tampered_save.deterministic_seed.wrapping_add(1);
    let tampered_save_rejected = tampered_save
        .validate_with_asset_root(evidence_root)
        .is_err();

    let archive_root = evidence_root.join("lineage-library");
    let mut library = LineageLibrary::open(LineageLibraryConfig::profile_default(&archive_root))?;
    let foundation = FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)?;
    let foundation_bytes = foundation.encode_canonical()?;
    for member in &members {
        let expressed = member.genome.express()?;
        let capacity = BrainCapacityClass::production_for_id(expressed.foundation.brain_class_id)?;
        let mature_tick = Tick::new(u64::from(expressed.development.maturation_duration_ticks));
        let development = expressed.development_state_at(mature_tick)?;
        let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
            &expressed.brain_genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
            &foundation,
        )?;
        library.archive_birth(GeneticArchiveInput {
            source_run_id: "ei0-exit-gate",
            organism_id: member.organism_id,
            birth_tick: Tick::new(u64::from(member.generation)),
            genome: &expressed.brain_genome,
            phenotype: &phenotype,
            foundation_asset_bytes: Some(&foundation_bytes),
        })?;
    }
    let archive_birth_manifest_count = library.manifest_count()?;
    let lineage_compare_passed = final_generation_genomes.iter().all(|genome| {
        let organism_id = members
            .iter()
            .find(|member| member.genome.id == genome.id)
            .map(|member| member.organism_id);
        let Some(organism_id) = organism_id else {
            return false;
        };
        let Ok(Some(digest)) = library.latest_manifest_for("ei0-exit-gate", organism_id) else {
            return false;
        };
        let Ok(manifest) = library.load_manifest(digest) else {
            return false;
        };
        let Ok(archived) = library.load_brain_genome(&manifest) else {
            return false;
        };
        archived.id == genome.id
            && archived.parent_genome_ids == genome.parent_genome_ids
            && archived.lineage_id == Some(genome.lineage_id)
            && manifest
                .genetic
                .foundation_id
                .is_some_and(|id| id.raw() == genome.foundation.foundation_id)
            && manifest
                .genetic
                .foundation_version
                .is_some_and(|version| version.raw() == u32::from(genome.foundation.version))
            && manifest
                .genetic
                .compatibility_family_id
                .is_some_and(|family| family.raw() == genome.foundation.compatibility_family_id)
    });

    let no_lifetime_state_inherited = lanes.iter().flat_map(|lane| &lane.births).all(|birth| {
        birth.ordinary_birth
            && birth.provenance.ordinary_birth
            && birth.provenance.conception_seed == birth.conception_seed
    });
    let tampered_provenance_rejected = final_generation_genomes.first().is_some_and(|genome| {
        let mut tampered = genome.clone();
        tampered.provenance.conception_seed = tampered.provenance.conception_seed.wrapping_add(1);
        tampered.validate_contract().is_err()
    });
    let population_genomes = members.into_iter().map(|member| member.genome).collect();
    let report = Ei0LifecycleGateReport {
        schema_version: EI0_EXIT_GATE_SCHEMA_VERSION,
        founder_count: 8,
        live_population_count: restored_population_count,
        generation_count: 3,
        run_observed,
        portable_save_round_trip,
        tampered_save_rejected,
        tampered_provenance_rejected,
        restored_population_count,
        archive_birth_manifest_count,
        lineage_compare_passed,
        no_lifetime_state_inherited,
        player_directed_wild_breeding_rejected,
        creature_directed_managed_breeding_rejected,
        lanes,
        population_genomes,
    };
    Ok(Ei0LifecycleEvidence {
        report,
        final_generation_genomes,
    })
}

fn founder_genome(seed: u64) -> Result<CreatureGenome, ScaffoldContractError> {
    CreatureGenome::early_mammal_founder(
        seed,
        FoundationGeneticIdentity::new(
            FOUNDATION_ID_RAW,
            1,
            FOUNDATION_FAMILY_RAW,
            BrainCapacityClass::N2048_ID,
        )?,
    )
}

#[allow(clippy::too_many_arguments)]
fn breed_member(
    world: &mut alife_world::HeadlessWorld,
    authority: &mut HabitatAuthority,
    first: &PopulationMember,
    second: &PopulationMember,
    child_organism_raw: u64,
    habitat_id: HabitatId,
    kind: HabitatBreedingKind,
    conception_seed: u64,
) -> Result<(PopulationMember, Ei0BirthReceipt), Ei0ExitGateError> {
    if first.habitat_id != habitat_id || second.habitat_id != habitat_id {
        return Err(Ei0ExitGateError::Evidence(
            "parents must occupy the breeding habitat",
        ));
    }
    let actor = match kind {
        HabitatBreedingKind::CreatureChosen => HabitatActor::Organism(first.organism_id),
        HabitatBreedingKind::Explicit => HabitatActor::Player,
    };
    let permission = authority.authorize_breeding(HabitatBreedingRequest {
        habitat_id,
        first_parent: first.organism_id,
        second_parent: second.organism_id,
        kind,
        actor,
        tick: world.tick(),
    })?;
    let genome = CreatureGenome::reproduce(&first.genome, &second.genome, conception_seed)?;
    genome.validate_contract()?;
    let organism_id = OrganismId(child_organism_raw);
    world.spawn_social_agent(
        &format!("era0-offspring-{child_organism_raw}"),
        organism_id,
        Vec3f::new(child_organism_raw as f32 * 0.001, 1.0, 0.0),
        0.5,
    )?;
    authority.register_creature(organism_id, habitat_id, world.tick())?;
    let generation = first.generation.max(second.generation) + 1;
    let receipt = Ei0BirthReceipt {
        organism_id,
        genome_id: genome.id,
        lineage_id: genome.lineage_id,
        parent_genome_ids: genome.parent_genome_ids.clone(),
        generation,
        conception_seed: genome.conception_seed,
        ordinary_birth: genome.provenance.ordinary_birth,
        provenance: genome.provenance.clone(),
        foundation_id: genome.foundation.foundation_id,
        foundation_version: genome.foundation.version,
        compatibility_family_id: genome.foundation.compatibility_family_id,
        breeding_kind: permission.kind,
        actor: permission.actor,
        cognition_policy: permission.cognition_policy,
    };
    Ok((
        PopulationMember {
            organism_id,
            genome,
            generation,
            habitat_id,
        },
        receipt,
    ))
}

fn observe_population(
    world: &mut alife_world::HeadlessWorld,
    members: &[PopulationMember],
) -> Result<bool, ScaffoldContractError> {
    let mut observed = 0_usize;
    for member in members {
        let frame = world.perception_frame(
            member.organism_id,
            world.tick(),
            SensorProfile::GroundedObjectSlotsV1,
            HomeostaticSnapshot::baseline(world.tick()),
        )?;
        if !frame.candidates().is_empty() {
            observed += 1;
        }
    }
    Ok(observed == members.len())
}

fn creature_save(member: &PopulationMember, tick: Tick) -> CreatureSaveState {
    CreatureSaveState {
        organism_id: member.organism_id,
        genome_id: member.genome.id,
        brain_class: BrainScaleTier::Standard2048,
        development_tick: tick,
        appearance: alife_world::CreatureAppearanceGenome::default(),
        mind: CreatureMindSaveSummary {
            tick,
            homeostasis: HomeostaticSnapshot::baseline(tick),
            memory_record_count: 0,
            memory_source_ids: Vec::new(),
            concept_count: 0,
            edge_count: 0,
            simplex_count: 0,
            unresolved_gap_count: 0,
            sleep_state_label: "awake".to_string(),
            diagnostics: vec!["ei0 multi-generation gate".to_string()],
        },
        weights: WeightLayerSaveSummary {
            generated_weight_asset_id: None,
            genetic_fixed_digest: format!("fnv1a64:{:016x}", member.genome.id.0),
            genetic_layer_mutable: false,
            lifetime_consolidated_entries: 0,
            h_operational_entries: 0,
            h_shadow_entries: 0,
        },
        learning: LearningTraceSaveSummary {
            lifetime_learning_enabled: true,
            lamarckian_mode_enabled: false,
            last_consolidated_tick: None,
        },
        gpu_brain: None,
    }
}

pub fn run_ei0_exit_gate(
    evidence_root: impl AsRef<Path>,
) -> Result<Ei0ExitGateReport, Ei0ExitGateError> {
    let evidence_root = evidence_root.as_ref();
    let lifecycle_evidence = run_ei0_lifecycle_gate(evidence_root.join("lifecycle"))?;
    let mut runner = N2048ActiveBatteryRunner::new_required()?;
    let mut gpu_tests = Vec::with_capacity(lifecycle_evidence.final_generation_genomes.len());
    for (lane, genome) in lifecycle_evidence
        .report
        .lanes
        .iter()
        .zip(&lifecycle_evidence.final_generation_genomes)
    {
        let final_birth = lane.births.last().ok_or(Ei0ExitGateError::Evidence(
            "lane is missing its final birth",
        ))?;
        if final_birth.genome_id != genome.id {
            return Err(Ei0ExitGateError::Evidence(
                "lane final birth does not match the GPU test genome",
            ));
        }
        let evidence = runner.run_creature_genome(final_birth.organism_id, genome)?;
        gpu_tests.push(gpu_receipt(evidence)?);
    }

    let heuristic_baseline = load_heuristic_baseline_boundary()?;
    let lifecycle = lifecycle_evidence.report;
    let clauses = evaluate_clauses(&lifecycle, &gpu_tests, &heuristic_baseline);
    let verdict = Ei0ExitVerdict {
        era0_exit_gate_passed: clauses.all_passed(),
        era1_promotion_evaluated: false,
        era1_status: "OUT_OF_SCOPE".to_string(),
    };
    Ok(Ei0ExitGateReport {
        schema_version: EI0_EXIT_GATE_SCHEMA_VERSION,
        verdict,
        clauses,
        lifecycle,
        gpu_tests,
        heuristic_baseline,
    })
}

pub fn write_ei0_exit_gate_report(
    path: impl AsRef<Path>,
    report: &Ei0ExitGateReport,
) -> Result<(), Ei0ExitGateError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(report)?)?;
    Ok(())
}

fn gpu_receipt(evidence: ActiveBatteryEvidence) -> Result<Ei0GpuBatteryReceipt, Ei0ExitGateError> {
    let source_creature_genome_id =
        evidence
            .source_creature_genome_id
            .ok_or(Ei0ExitGateError::Evidence(
                "GPU receipt is missing the source creature genome",
            ))?;
    let lineage_id = evidence.lineage_id.ok_or(Ei0ExitGateError::Evidence(
        "GPU receipt is missing lineage identity",
    ))?;
    Ok(Ei0GpuBatteryReceipt {
        organism_id: evidence.receipt.organism_id,
        source_creature_genome_id,
        brain_genome_id: evidence.brain_genome_id,
        parent_genome_ids: evidence.parent_genome_ids,
        lineage_id,
        phenotype_hash: evidence.phenotype_hash,
        foundation_id: evidence.foundation_id,
        foundation_version: evidence.foundation_version,
        compatibility_family_id: evidence.compatibility_family_id,
        policy_backend: PolicyBackend::NeuralClosedLoopGpu,
        completed_challenges: evidence.receipt.completed_count(),
        challenge_results: evidence.receipt.results,
        challenge_worlds: evidence.challenge_worlds,
        gpu_dispatches: evidence.gpu_dispatches,
        sealed_outcomes: evidence.sealed_outcomes,
        sleep_consolidations: evidence.sleep_consolidations,
        slm_enabled: evidence.slm_enabled,
        adapter_name: evidence.adapter_name,
        backend_api: evidence.backend_api,
    })
}

fn load_heuristic_baseline_boundary() -> Result<Ei0HeuristicBaselineBoundary, Ei0ExitGateError> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("reports")
        .join("ei0_real_fixture_report.json");
    let fixture: BaselineFixture = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let unknown_measures = fixture.evidence_scope.unsupported_measures.clone();
    let unknown_measures_preserved = unknown_measures.len() == 9
        && unknown_measures.iter().all(|name| {
            let reading = match name.as_str() {
                "cognitive_objective" => fixture.objectives.get("cognitive"),
                "social_objective" => fixture.objectives.get("social"),
                "group_objective" => fixture.objectives.get("group"),
                other => fixture.measures.get(other),
            };
            reading.is_some_and(|reading| reading.value.is_none() && reading.samples == 0)
        });
    Ok(Ei0HeuristicBaselineBoundary {
        source_backend: fixture.source_backends.join(","),
        promotion_eligible: fixture.promotion_eligible,
        hidden_promotion_trials: fixture.layer_counts.hidden_promotion,
        unknown_measures,
        unknown_measures_preserved: unknown_measures_preserved
            && !fixture.evidence_scope.promotion_backend_eligible,
    })
}

fn evaluate_clauses(
    lifecycle: &Ei0LifecycleGateReport,
    gpu_tests: &[Ei0GpuBatteryReceipt],
    baseline: &Ei0HeuristicBaselineBoundary,
) -> Ei0ExitClauses {
    let wild_lane = lifecycle
        .lanes
        .iter()
        .find(|lane| lane.mode == HabitatMode::Wild);
    let managed_lane = lifecycle
        .lanes
        .iter()
        .find(|lane| lane.mode == HabitatMode::Managed);
    let wild_breed = lifecycle.player_directed_wild_breeding_rejected
        && wild_lane.is_some_and(|lane| {
            lane.births.len() == 3
                && lane.births.iter().all(|birth| {
                    birth.breeding_kind == HabitatBreedingKind::CreatureChosen
                        && matches!(birth.actor, HabitatActor::Organism(_))
                        && birth.cognition_policy == PolicyBackend::NeuralClosedLoopGpu
                })
        });
    let managed_breed = lifecycle.creature_directed_managed_breeding_rejected
        && managed_lane.is_some_and(|lane| {
            lane.births.len() == 3
                && lane.births.iter().all(|birth| {
                    birth.breeding_kind == HabitatBreedingKind::Explicit
                        && birth.actor == HabitatActor::Player
                        && birth.cognition_policy == PolicyBackend::NeuralClosedLoopGpu
                })
        });
    let test = gpu_tests.len() == 2
        && gpu_tests.iter().all(|receipt| {
            receipt.completed_challenges == ACTIVE_CHALLENGE_COUNT
                && receipt.challenge_worlds == ACTIVE_CHALLENGE_COUNT as u32
                && receipt.gpu_dispatches == receipt.sealed_outcomes
                && receipt.gpu_dispatches >= ACTIVE_CHALLENGE_COUNT as u64
                && receipt.sleep_consolidations >= 1
                && !receipt.slm_enabled
                && !receipt.adapter_name.trim().is_empty()
                && !receipt.backend_api.trim().is_empty()
        });
    let gpu_policy_identity = test
        && lifecycle.lanes.iter().zip(gpu_tests).all(|(lane, gpu)| {
            lane.births.last().is_some_and(|birth| {
                gpu.organism_id == birth.organism_id
                    && gpu.source_creature_genome_id == birth.genome_id
                    && gpu.brain_genome_id == birth.genome_id
                    && gpu.parent_genome_ids == birth.parent_genome_ids
                    && gpu.lineage_id == birth.lineage_id
                    && gpu.foundation_id == birth.foundation_id
                    && gpu.foundation_version == u32::from(birth.foundation_version)
                    && gpu.compatibility_family_id == birth.compatibility_family_id
                    && gpu.policy_backend == PolicyBackend::NeuralClosedLoopGpu
            })
        });
    let no_hidden_policy_control = gpu_policy_identity
        && baseline.source_backend == "HeuristicBaseline"
        && !baseline.promotion_eligible
        && baseline.hidden_promotion_trials == 0
        && baseline.unknown_measures_preserved;
    Ei0ExitClauses {
        run: lifecycle.founder_count == 8
            && lifecycle.live_population_count == 14
            && lifecycle.generation_count == 3,
        observe: lifecycle.run_observed,
        save_load: lifecycle.portable_save_round_trip && lifecycle.tampered_save_rejected,
        wild_breed,
        managed_breed,
        test,
        archive: lifecycle.archive_birth_manifest_count == 14,
        compare: lifecycle.lineage_compare_passed && lifecycle.tampered_provenance_rejected,
        stable_multi_generation_population: lifecycle.restored_population_count == 14
            && lifecycle.no_lifetime_state_inherited
            && lifecycle.population_genomes.len() == 14,
        gpu_policy_identity,
        no_hidden_policy_control,
    }
}

pub fn default_report_path(root: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join("ei0_exit_gate_report.json")
}
