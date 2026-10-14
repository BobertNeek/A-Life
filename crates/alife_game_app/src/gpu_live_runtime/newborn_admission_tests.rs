//! Natural reproduction fixtures shared by CPU transaction and GPU admission checks.
use super::*;
use alife_core::{ContinuousLocus, CreatureGenome};
use alife_world::HeadlessScenarioBuilder;
#[cfg(feature = "gpu-tests")]
use std::fs;

const PARENTS: [OrganismId; 2] = [OrganismId(1), OrganismId(2)];
const CHILD: OrganismId = OrganismId(3);
const SEED: u64 = 0x43B1_0001;

fn ready_parent_world() -> HeadlessWorld {
    let mut world = HeadlessScenarioBuilder::new(SEED)
        .agent("parent-a", PARENTS[0], Vec3f::ZERO)
        .agent("parent-b", PARENTS[1], Vec3f::new(0.5, 0.0, 0.0))
        .build()
        .unwrap();
    let identity = FoundationGeneticIdentity::new(
        FoundationId::N512_V1.raw(),
        FoundationVersion::V1.raw() as u16,
        FoundationCompatibilityFamilyId::N512_FOUNDATION.raw(),
        BrainCapacityClass::N512_ID,
    )
    .unwrap();
    let mut puberty = 0;
    for (parent, seed) in PARENTS.into_iter().zip([0xE10_43A1, 0xE10_43B3]) {
        let mut genome = CreatureGenome::early_mammal_founder(seed, identity).unwrap();
        // A deterministic test pair; every other inherited parameter is retained.
        genome.reproduction.fertility = ContinuousLocus::mean(1.0, 1.0).unwrap();
        let phenotype = genome.express().unwrap();
        puberty = puberty.max(phenotype.development.puberty_tick.raw());
        let body = world
            .organism_entity_ids()
            .into_iter()
            .find_map(|(id, body)| (id == parent).then_some(body))
            .unwrap();
        world
            .register_organism_record(
                WorldOrganismRecord::newborn(parent, body, genome, phenotype, Tick::ZERO).unwrap(),
            )
            .unwrap();
    }
    let mut habitats = world.habitat_authority().clone();
    for parent in PARENTS {
        habitats
            .register_creature(parent, HabitatId::DEFAULT_WILD, Tick::ZERO)
            .unwrap();
    }
    world.replace_habitat_authority(habitats).unwrap();
    let period = u64::from(alife_core::BiochemistryCadence::early_mammal().reproduction_ticks);
    let boundary = puberty.div_ceil(period) * period;
    // Mature through the inherited body/chemistry step with ordinary rest input.
    // Without recovery, neural ATP drops below the real reproduction threshold.
    let rest = parent_rest_events();
    while world.tick().raw() + 2 < boundary {
        world.try_advance_tick_with_body_events(&rest).unwrap();
        assert_eq!(
            world.organism_registry().iter().count(),
            2,
            "premature birth"
        );
    }
    // Wake one interval before the reproduction boundary without changing drives.
    world.try_advance_tick().unwrap();
    assert_eq!(world.tick().raw() + 1, boundary);
    for record in parent_records(&world) {
        assert!(!record.biochemistry().body.sleeping);
        assert!(record.biochemistry().homeostasis.drives.brain_atp >= 0.25);
    }
    world
}

fn parent_rest_events() -> BTreeMap<u64, BodyEventDelta> {
    BTreeMap::from(PARENTS.map(|id| {
        (
            id.raw(),
            BodyEventDelta {
                sleep_recovery: 1.0,
                ..BodyEventDelta::zero()
            },
        )
    }))
}

fn parent_records(world: &HeadlessWorld) -> Vec<WorldOrganismRecord> {
    PARENTS
        .into_iter()
        .map(|id| world.organism_registry().get(id).unwrap().clone())
        .collect()
}

fn assert_newborn(world: &HeadlessWorld, birth_tick: Tick) {
    assert_eq!(world.tick(), birth_tick);
    assert_eq!(
        world.organism_registry().iter().count(),
        3,
        "parent readiness: {:?}",
        parent_records(world)
            .iter()
            .map(|r| (
                r.organism_id(),
                r.biochemistry().body.energy,
                r.biochemistry().reproduction,
                r.biochemistry().homeostasis.drives.reproductive_drive,
                r.biochemistry().homeostasis.hormones.developmental_hormone,
                r.phenotype().chemistry.reproductive_threshold
            ))
            .collect::<Vec<_>>()
    );
    let child = world.organism_registry().get(CHILD).unwrap();
    assert!(child.lifecycle().is_alive());
    assert_eq!(child.age_at(birth_tick).unwrap(), Tick::ZERO);
    assert_eq!(child.biochemistry().tick, birth_tick);
    child.validate_contract().unwrap();
    assert!(world
        .organism_entity_ids()
        .contains(&(CHILD, child.world_entity_id())));
    assert!(world.habitat_authority().membership(CHILD).is_some());
}

#[test]
fn newborn_fixture_reaches_natural_birth_at_the_passive_boundary() {
    let mut world = ready_parent_world();
    let before = parent_records(&world);
    let biology = *world
        .organism_registry()
        .get(PARENTS[0])
        .unwrap()
        .biochemistry();
    let frame = world
        .perception_frame(
            PARENTS[0],
            world.tick(),
            SensorProfile::PrivilegedAffordanceV1,
            biology.homeostasis,
        )
        .unwrap();
    assert!(frame
        .candidates()
        .iter()
        .any(|c| c.family == alife_core::CandidateActionFamily::Idle));
    let birth_tick = Tick(world.tick().raw() + 1);
    world.try_advance_tick().unwrap();
    assert_newborn(&world, birth_tick);
    for parent in before {
        assert!(
            world
                .organism_registry()
                .get(parent.organism_id())
                .unwrap()
                .biochemistry()
                .body
                .energy
                < parent.biochemistry().body.energy,
            "actual parental reserve transfer"
        );
    }
}

struct CpuAuthority {
    world: HeadlessWorld,
    residents: BTreeMap<u64, ResidentCognition>,
}

impl LiveAuthorityOwner for CpuAuthority {
    fn world_and_residents(
        &mut self,
    ) -> (&mut HeadlessWorld, &mut BTreeMap<u64, ResidentCognition>) {
        (&mut self.world, &mut self.residents)
    }
}

#[test]
fn newborn_fixture_failure_rolls_back_parents_clock_allocators_and_registry() {
    let world = ready_parent_world();
    let residents = PARENTS
        .into_iter()
        .map(|id| {
            (
                id.raw(),
                GpuLiveBrainRuntime::compile_birth(
                    &world,
                    BrainScaleTier::Nano512,
                    SensorProfile::PrivilegedAffordanceV1,
                    id,
                )
                .unwrap()
                .1,
            )
        })
        .collect();
    let mut owner = CpuAuthority { world, residents };
    let before = owner.world.canonical_signature_digest().unwrap();
    let before_parents = parent_records(&owner.world);
    let before_residents = serde_json::to_vec(&owner.residents).unwrap();
    let birth_tick = Tick(owner.world.tick().raw() + 1);
    let mut control = owner.world.clone();
    // Match the recovery events used by the GPU fixture's passive sleep driver.
    let body_events = parent_rest_events();
    control
        .try_advance_tick_with_body_events(&body_events)
        .unwrap();
    assert_newborn(&control, birth_tick);
    let (failed, _) = tick_with_sleep_progress_inner(&mut owner, false, |candidate| {
        advance_and_synchronize_authority(
            &mut candidate.world,
            &mut candidate.residents,
            birth_tick,
            &body_events,
        )?;
        assert_newborn(&candidate.world, birth_tick);
        assert_ne!(
            candidate.world.canonical_signature_digest().unwrap(),
            before
        );
        Err::<(), ScaffoldContractError>(ScaffoldContractError::NeuralBackendUnavailable)
    });
    assert_eq!(failed, Err(ScaffoldContractError::NeuralBackendUnavailable));
    // The canonical digest includes all four ID/sequence allocators and habitat state.
    assert_eq!(owner.world.canonical_signature_digest().unwrap(), before);
    assert_eq!(parent_records(&owner.world), before_parents);
    assert_eq!(
        serde_json::to_vec(&owner.residents).unwrap(),
        before_residents
    );
    assert!(owner.world.organism_registry().get(CHILD).is_none());
    let (retried, _) = tick_with_sleep_progress_inner(&mut owner, false, |candidate| {
        advance_and_synchronize_authority(
            &mut candidate.world,
            &mut candidate.residents,
            birth_tick,
            &body_events,
        )
    });
    retried.unwrap();
    assert_eq!(
        owner.world.canonical_signature_digest().unwrap(),
        control.canonical_signature_digest().unwrap()
    );
    assert_newborn(&owner.world, birth_tick);
}

#[cfg(feature = "gpu-tests")]
struct PassiveSleepDriver;

#[cfg(feature = "gpu-tests")]
impl GpuSleepConsolidationDriver for PassiveSleepDriver {
    fn progress(
        &mut self,
        _: OrganismId,
        _: SleepState,
        _: Option<ConsolidationIntent>,
    ) -> SleepProgressResult {
        Ok(None)
    }
}

#[cfg(feature = "gpu-tests")]
fn archived_newborn_runtime(label: &str) -> (GpuLiveBrainRuntime, PathBuf) {
    let world = ready_parent_world();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let archive_root = std::env::temp_dir().join(format!(
        "alife-gpu-newborn-{label}-{}-{nonce}",
        std::process::id()
    ));
    let backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("in-process GPU backend");
    let mut runtime = GpuLiveBrainRuntime::new_profiled_archived(
        backend,
        world,
        SEED,
        BrainScaleTier::Nano512,
        SensorProfile::PrivilegedAffordanceV1,
        LineageLibraryConfig::profile_default(&archive_root),
        "task-4.3b1-newborn",
        ArchiveLearnedCapturePolicy::GeneticOnly,
    )
    .unwrap();
    // Keep parent geometry/readiness independent of a neural action winner.
    // The normal staged tick still advances canonical passive biology and admits birth.
    for parent in PARENTS {
        runtime.request_recovery_sleep(parent).unwrap();
    }
    (runtime, archive_root)
}

#[cfg(feature = "gpu-tests")]
fn assert_admitted_newborn(runtime: &GpuLiveBrainRuntime, birth_tick: Tick) -> Blake3Digest {
    assert_newborn(&runtime.world, birth_tick);
    let record = runtime.world.organism_registry().get(CHILD).unwrap();
    let digest = runtime
        .archive_birth_manifest(CHILD)
        .expect("newborn archive manifest");
    assert_eq!(record.archive().birth_manifest_digest(), Some(digest));
    let handle = runtime.handle_for(CHILD).expect("newborn GPU handle");
    assert_eq!(handle.organism_id(), CHILD);
    assert_eq!(
        runtime.residents[&CHILD.raw()].phenotype.phenotype_hash(),
        handle.phenotype_hash()
    );
    assert_eq!(
        runtime.residents[&CHILD.raw()].homeostasis,
        record.biochemistry().homeostasis
    );
    assert_eq!(runtime.memories[&CHILD.raw()].bank().lifetime_len(), 0);
    assert_eq!(runtime.topologies[&CHILD.raw()].organism_id(), CHILD);
    assert_eq!(runtime.backend.admission_receipt().live_brains, 3);
    for keys in [
        runtime.handles.keys().copied().collect::<Vec<_>>(),
        runtime.residents.keys().copied().collect(),
        runtime.memories.keys().copied().collect(),
        runtime.topologies.keys().copied().collect(),
    ] {
        assert_eq!(keys, vec![1, 2, 3]);
    }
    digest
}

#[cfg(feature = "gpu-tests")]
#[test]
fn newborn_is_archived_linked_and_admitted_before_tick_returns() {
    let (mut runtime, archive_root) = archived_newborn_runtime("success");
    let birth_tick = Tick(runtime.world.tick().raw() + 1);
    runtime
        .tick_with_sleep_driver(&mut PassiveSleepDriver)
        .unwrap();
    assert_admitted_newborn(&runtime, birth_tick);
    drop(runtime);
    fs::remove_dir_all(archive_root).unwrap();
}

#[cfg(feature = "gpu-tests")]
#[test]
fn failed_newborn_admission_publishes_nothing_and_retry_reuses_manifest() {
    let (mut runtime, archive_root) = archived_newborn_runtime("retry");
    let birth_tick = Tick(runtime.world.tick().raw() + 1);
    let before = runtime.world.canonical_signature_digest().unwrap();
    let before_parents = parent_records(&runtime.world);
    let before_residents = serde_json::to_vec(&runtime.residents).unwrap();
    let before_memories = runtime.memories.clone();
    let before_topologies = runtime.topologies.clone();
    let before_handles = runtime.handles.clone();
    let before_manifests = runtime.archive_birth_manifests.clone();
    let archive_count_before = runtime.lineage_archive_manifest_count().unwrap().unwrap();
    runtime.backend.force_admission_failures_for_test(1);
    assert!(runtime
        .tick_with_sleep_driver(&mut PassiveSleepDriver)
        .is_err());
    assert_eq!(runtime.world.canonical_signature_digest().unwrap(), before);
    assert_eq!(parent_records(&runtime.world), before_parents);
    assert_eq!(
        serde_json::to_vec(&runtime.residents).unwrap(),
        before_residents
    );
    assert_eq!(runtime.memories, before_memories);
    assert_eq!(runtime.topologies, before_topologies);
    assert_eq!(runtime.handles, before_handles);
    assert_eq!(runtime.archive_birth_manifests, before_manifests);
    assert_eq!(runtime.backend.admission_receipt().live_brains, 2);
    assert!(runtime.world.organism_registry().get(CHILD).is_none());
    assert!(runtime
        .world
        .organism_entity_ids()
        .iter()
        .all(|(id, _)| *id != CHILD));
    let library = runtime.lineage_library.as_ref().unwrap();
    let retained = library
        .latest_manifest_for(runtime.lineage_run_id.as_deref().unwrap(), CHILD)
        .unwrap()
        .expect("birth was archived before injected GPU admission failure");
    assert_eq!(
        runtime.lineage_archive_manifest_count().unwrap(),
        Some(archive_count_before + 1)
    );
    // Reconcile alone cannot recreate a rolled-back birth; retry the whole tick.
    runtime
        .tick_with_sleep_driver(&mut PassiveSleepDriver)
        .unwrap();
    assert_eq!(assert_admitted_newborn(&runtime, birth_tick), retained);
    assert_eq!(
        runtime.lineage_archive_manifest_count().unwrap(),
        Some(archive_count_before + 1)
    );
    drop(runtime);
    fs::remove_dir_all(archive_root).unwrap();
}
