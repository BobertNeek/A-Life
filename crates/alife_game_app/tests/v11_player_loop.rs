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
use alife_runtime::GpuDurableSaveManifest;
use alife_world::{
    persistence::{
        AssetManifest, CreatureMindSaveSummary, CreatureSaveState, LearningTraceSaveSummary,
        PortableAssetDigest, PortableSaveFile, RuntimeConfig, WeightLayerSaveSummary,
    },
    CreatureAppearanceGenome, Habitat, HabitatActor, HabitatAuthority, HabitatId, HabitatMode,
    HabitatOperation, HeadlessScenarioBuilder, WorldOrganismRecord,
};

fn player_loop_base_save(
    runtime: &GpuLiveBrainRuntime,
    organism_id: OrganismId,
) -> PortableSaveFile {
    let world = runtime.world_snapshot();
    let record = world
        .organism_registry()
        .get(organism_id)
        .expect("canonical player-loop organism record");
    let biochemistry = record.biochemistry().clone();
    let genetic_fixed_digest = PortableAssetDigest::for_bytes(
        &serde_json::to_vec(record.phenotype()).expect("canonical phenotype serializes"),
    )
    .0;
    let creature = CreatureSaveState {
        organism_id,
        genome_id: record.genome().id,
        brain_class: BrainScaleTier::Standard2048,
        development_tick: biochemistry.development.last_update_tick,
        appearance: CreatureAppearanceGenome::default(),
        mind: CreatureMindSaveSummary {
            tick: biochemistry.tick,
            homeostasis: biochemistry.homeostasis,
            memory_record_count: 0,
            memory_source_ids: Vec::new(),
            concept_count: 0,
            edge_count: 0,
            simplex_count: 0,
            unresolved_gap_count: 0,
            sleep_state_label: "awake".to_string(),
            diagnostics: Vec::new(),
        },
        weights: WeightLayerSaveSummary {
            generated_weight_asset_id: None,
            genetic_fixed_digest,
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
        composite_genetics: None,
        lifetime_state_asset: None,
        gpu_brain: None,
    };
    let mut config = RuntimeConfig::deterministic_default(13_001, BrainScaleTier::Standard2048);
    config.features.gpu_backend_enabled = true;
    PortableSaveFile::from_headless_world(
        "task-13-v11-player-loop",
        &world,
        config,
        AssetManifest::empty(),
        vec![creature],
    )
    .expect("canonical player-loop base save")
}

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

    let birth_manifest_digest = runtime
        .world_snapshot()
        .organism_registry()
        .get(organism_id)
        .and_then(|record| record.archive().birth_manifest_digest())
        .expect("canonical world retains its birth archive identity");
    let before_replace_signature = runtime
        .world_snapshot()
        .canonical_signature_digest()
        .expect("canonical world signature before durable replace");
    let durable_root = archive_root.join("player-loop-durable");
    let asset_root = durable_root.join("assets");
    let save_path = durable_root.join("player-loop.json");
    fs::create_dir_all(&asset_root).expect("create durable checkpoint asset root");
    let base_save = player_loop_base_save(&runtime, organism_id);
    runtime
        .attach_durable_checkpoint_boundary(&save_path, &asset_root, base_save)
        .expect("attach the runtime-owned durable checkpoint boundary");

    let durable = GpuDurableSaveManifest::open(&save_path, &asset_root)
        .expect("open the published player-loop checkpoint");
    let loaded = durable
        .load()
        .expect("load the published player-loop checkpoint");
    assert_eq!(loaded.save.world.tick, Tick::new(1));
    assert!(loaded
        .save
        .creatures
        .iter()
        .find(|creature| creature.organism_id == organism_id)
        .and_then(|creature| creature.gpu_brain.as_ref())
        .is_none());
    let checkpointed = runtime
        .capture_portable_checkpoint()
        .expect("capture the exact live GPU checkpoint");
    assert_eq!(checkpointed.world.tick, Tick::new(1));
    assert!(checkpointed
        .creatures
        .iter()
        .find(|creature| creature.organism_id == organism_id)
        .and_then(|creature| creature.gpu_brain.as_ref())
        .is_some());
    GpuDurableSaveManifest::publish_snapshot(&save_path, &asset_root, &checkpointed)
        .expect("publish the captured player-loop checkpoint");
    let durable = GpuDurableSaveManifest::open(&save_path, &asset_root)
        .expect("reopen the captured player-loop checkpoint");
    let loaded = durable
        .load()
        .expect("reload the captured player-loop checkpoint");
    assert!(loaded
        .save
        .creatures
        .iter()
        .find(|creature| creature.organism_id == organism_id)
        .and_then(|creature| creature.gpu_brain.as_ref())
        .is_some());
    assert_eq!(
        loaded
            .save
            .restore_headless_world()
            .expect("restore the saved canonical world")
            .organism_registry()
            .get(organism_id)
            .and_then(|record| record.archive().birth_manifest_digest()),
        Some(birth_manifest_digest)
    );

    runtime
        .replace_from_durable_save(
            runtime
                .new_staging_like_live()
                .expect("same-adapter staging backend for durable replace"),
            durable,
        )
        .expect("replace the live runtime from its durable checkpoint");
    assert_eq!(
        runtime
            .world_snapshot()
            .canonical_signature_digest()
            .expect("canonical world signature after durable replace"),
        before_replace_signature
    );
    assert_eq!(
        runtime
            .world_snapshot()
            .organism_registry()
            .get(organism_id)
            .and_then(|record| record.archive().birth_manifest_digest()),
        Some(birth_manifest_digest)
    );
    assert_eq!(
        runtime
            .lineage_archive_manifest_count()
            .expect("archive count after replace"),
        Some(1)
    );

    drop(runtime);
    fs::remove_dir_all(&archive_root).expect("remove temporary player-loop archive");
    assert!(
        false,
        "Task 13 RED at the next unavailable lifecycle seam: durable save/load/replace now preserves the canonical world and archive identity, but the production voxel presentation link remains unproven. Do not fake the remaining lifecycle links."
    );
}
