//! Smallest v1.1 player-loop RED gate.
//!
//! The fixture reaches a grounded teacher token and the production GPU runtime
//! boundary. It stops at the first missing production seam instead of faking
//! the later attention, prediction, motor, learning, sleep, reproduction,
//! persistence, or presentation links.
#![cfg(feature = "gpu-runtime")]

use alife_core::{
    BrainCapacityClass, BrainScaleTier, FoundationGeneticIdentity, FoundationWeightAsset,
    OrganismId, PolicyBackend, SensorProfile, TeacherPerceptionChannel, Tick, Vec3f,
};
use alife_game_app::GpuLiveBrainRuntime;
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};
use alife_world::{
    Habitat, HabitatActor, HabitatAuthority, HabitatId, HabitatMode, HabitatOperation,
    HeadlessScenarioBuilder, WorldOrganismRecord,
};

#[test]
fn v11_player_loop_reaches_grounded_body_then_reds_at_next_production_boundary() {
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
    let foundation_asset = FoundationWeightAsset::builtin_nano512_v1(sensor_profile)
        .expect("checked Nano512 foundation asset");
    let foundation_manifest = foundation_asset.manifest();
    let foundation = FoundationGeneticIdentity::new(
        foundation_manifest.foundation_id().raw(),
        foundation_manifest.foundation_version().raw() as u16,
        foundation_manifest.compatibility_family_id().raw(),
        BrainCapacityClass::N512_ID,
    )
    .expect("valid Nano512 foundation identity");
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

    let mut runtime = GpuLiveBrainRuntime::new_profiled(
        backend,
        world,
        13_001,
        BrainScaleTier::Nano512,
        sensor_profile,
    )
    .expect("supported profiled Nano512 production runtime");

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

    assert!(
        false,
        "Task 13 RED at the next unavailable production seam: the runtime now returns a measured structured-education authority receipt, but focal attention and the later prediction, motor, outcome, learning, sleep, reproduction, persistence, and presentation links remain unavailable. Do not fake them."
    );
}
