//! Smallest v1.1 player-loop RED gate.
//!
//! The fixture reaches a grounded teacher token and one coherent production
//! GPU tick with the canonical organism archived before GPU admission. It
//! stops at the first later unavailable lifecycle seam instead of faking
//! sleep, reproduction, persistence, or presentation links.
#![cfg(feature = "gpu-runtime")]

use std::fs;

use alife_archive::LineageLibraryConfig;
use alife_core::{
    ArchiveLearnedCapturePolicy, BrainCapacityClass, BrainScaleTier, FoundationGeneticIdentity,
    FoundationWeightAsset, OrganismId, PolicyBackend, SensorProfile, TeacherPerceptionChannel,
    Tick, Vec3f,
};
use alife_game_app::GpuLiveBrainRuntime;
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};
use alife_world::{
    Habitat, HabitatActor, HabitatAuthority, HabitatId, HabitatMode, HabitatOperation,
    HeadlessScenarioBuilder, WorldOrganismRecord,
};

#[test]
fn v11_player_loop_reaches_one_coherent_gpu_tick_then_reds_at_next_lifecycle_boundary() {
    let organism_id = OrganismId(1);
    let school_id = HabitatId::new(2).expect("non-zero school habitat id");
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())
        .expect("Task 13 requires the production GPU backend");
    let mut world = HeadlessScenarioBuilder::new(13_001)
        .agent("learner", organism_id, Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
        .teacher_token(
            "teacher-word",
            Vec3f::new(0.5, 0.0, 0.0),
            77,
            TeacherPerceptionChannel::Hearing,
        )
        .build()
        .expect("bounded grounded player-loop world");
    let world_entity_id = world
        .organism_entity_ids()
        .into_iter()
        .find(|(candidate, _)| *candidate == organism_id)
        .map(|(_, entity)| entity)
        .expect("learner world entity");
    let sensor_profile = SensorProfile::GroundedObjectSlotsV1;
    let foundation_asset = FoundationWeightAsset::builtin_n2048_v1(sensor_profile)
        .expect("checked N2048 foundation asset");
    let foundation_manifest = foundation_asset.manifest();
    let foundation = FoundationGeneticIdentity::new(
        foundation_manifest.foundation_id().raw(),
        foundation_manifest.foundation_version().raw() as u16,
        foundation_manifest.compatibility_family_id().raw(),
        BrainCapacityClass::N2048_ID,
    )
    .expect("valid N2048 foundation identity");
    let genome = alife_core::CreatureGenome::early_mammal_founder(13_001, foundation)
        .expect("valid learner genome");
    let phenotype = genome.express().expect("valid learner phenotype");
    let biochemistry = alife_core::BiochemistryState::new(&phenotype, Tick::ZERO)
        .expect("valid learner biochemistry");
    world
        .register_organism_record(
            WorldOrganismRecord::new(
                organism_id,
                world_entity_id,
                genome,
                phenotype,
                biochemistry,
                Tick::ZERO,
            )
            .expect("valid learner world-organism record"),
        )
        .expect("register learner world-organism record");
    let mut authority = HabitatAuthority::new(vec![
        Habitat::new(HabitatId::DEFAULT_WILD, "Wild", HabitatMode::Wild)
            .expect("valid wild habitat"),
        Habitat::new(school_id, "Nursery School", HabitatMode::School)
            .expect("valid school habitat"),
    ])
    .expect("valid habitat authority");
    authority
        .register_creature(organism_id, school_id, Tick::ZERO)
        .expect("school membership for the learner");
    world
        .replace_habitat_authority(authority)
        .expect("world-owned school authority");

    let archive_root =
        std::env::temp_dir().join(format!("alife-v11-player-loop-{}", std::process::id()));
    let _ = fs::remove_dir_all(&archive_root);
    let mut runtime = GpuLiveBrainRuntime::new_profiled_archived(
        backend,
        world,
        13_001,
        BrainScaleTier::Standard2048,
        sensor_profile,
        LineageLibraryConfig::profile_default(&archive_root),
        "task-13-v11-player-loop",
        ArchiveLearnedCapturePolicy::GeneticOnly,
    )
    .expect("supported profiled Standard2048 production runtime");

    let birth_manifest_digest = runtime
        .archive_birth_manifest(organism_id)
        .expect("production admission must commit the learner genetic archive");
    assert_eq!(
        runtime
            .world_snapshot()
            .organism_registry()
            .get(organism_id)
            .and_then(|record| record.archive().birth_manifest_digest()),
        Some(birth_manifest_digest),
        "the canonical world record must consume the committed birth archive"
    );
    assert_eq!(
        runtime
            .lineage_archive_manifest_count()
            .expect("lineage archive manifest count"),
        Some(1)
    );

    let grounded = runtime.world_snapshot();
    assert!(
        grounded
            .object_snapshots()
            .iter()
            .any(|object| object.label == "teacher-word"),
        "grounded observation must contain the teacher object"
    );
    assert!(
        grounded
            .presentation_snapshot()
            .organisms
            .iter()
            .any(|organism| organism.organism_id == organism_id),
        "grounded body state must contain the focal organism by stable ID"
    );

    let education = runtime
        .authorize_structured_education(organism_id, school_id, HabitatActor::Teacher)
        .expect("teacher structured education must return an authority receipt");
    assert_eq!(education.habitat_id, school_id);
    assert_eq!(education.organism_id, organism_id);
    assert_eq!(education.operation, HabitatOperation::StructuredEducation);
    assert_eq!(education.actor, HabitatActor::Teacher);
    assert_eq!(education.tick, Tick::ZERO);
    assert_eq!(
        education.cognition_policy,
        PolicyBackend::NeuralClosedLoopGpu
    );

    let summaries = runtime
        .tick()
        .expect("production GPU runtime must complete one coherent causal tick");
    assert_eq!(summaries.len(), 1);
    let summary = &summaries[0];
    assert_eq!(summary.organism_id, organism_id);
    assert_eq!(summary.tick_before, Tick::ZERO);
    assert_eq!(summary.tick_after, Tick::new(1));
    assert_eq!(summary.world_tick_before, Tick::ZERO);
    assert_eq!(summary.world_tick_after, Tick::new(1));
    assert!(summary.patch_sealed);
    assert_eq!(summary.learning_updates, 1);
    assert_eq!(summary.memory_updates, 1);
    assert_eq!(summary.topology_updates, 1);
    assert_eq!(runtime.world_snapshot().tick(), Tick::new(1));

    let patch = runtime
        .sealed_patches()
        .last()
        .expect("the GPU-selected outcome must survive in a sealed ExperiencePatch");
    assert!(patch.selected_bundle().is_some());
    assert!(patch.prediction_target().is_some());
    assert!(patch.cognitive_work().is_some());
    let joint = patch
        .outcome()
        .joint
        .as_ref()
        .expect("the sealed patch must contain the measured joint outcome");
    assert_eq!(joint.execution, patch.outcome().physical);
    assert!(!joint.channel_observations.is_empty());
    assert!(joint
        .channel_observations
        .iter()
        .any(|observation| observation.executed));

    assert!(!runtime.last_learning_receipts().is_empty());
    assert!(!runtime.last_activity_work_receipts().is_empty());
    assert!(!runtime.last_cognitive_work_receipts().is_empty());
    assert!(!runtime.last_memory_recall_receipts().is_empty());
    assert!(!runtime.last_memory_update_receipts().is_empty());
    assert!(!runtime.last_cognitive_context_digests().is_empty());
    assert!(!runtime.last_topology_observations().is_empty());
    assert!(runtime.last_memory_preparation_errors().is_empty());
    assert!(runtime.last_memory_observation_errors().is_empty());
    assert!(runtime.last_pre_seal_discard_failures().is_empty());
    assert!(runtime.last_post_seal_learning_failures().is_empty());

    let next_lifecycle_error = match runtime.capture_portable_checkpoint() {
        Ok(_) => "persistence unexpectedly succeeded without a durable save boundary".to_string(),
        Err(error) => error.to_string(),
    };
    drop(runtime);
    fs::remove_dir_all(&archive_root).expect("remove temporary player-loop archive");
    assert!(
        false,
        "Task 13 RED at the next unavailable lifecycle seam: the archived one-tick production path now reaches persistence, which stops at `{next_lifecycle_error}`. Do not fake the remaining lifecycle links."
    );
}
