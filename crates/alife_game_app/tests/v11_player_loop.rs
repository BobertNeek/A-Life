//! Smallest v1.1 player-loop RED gate.
//!
//! The fixture reaches a grounded teacher token and the production GPU runtime
//! boundary. It stops at the first missing production seam instead of faking
//! the later attention, prediction, motor, learning, sleep, reproduction,
//! persistence, or presentation links.
#![cfg(feature = "gpu-runtime")]

use alife_core::{BrainScaleTier, OrganismId, TeacherPerceptionChannel, Vec3f};
use alife_game_app::GpuLiveBrainRuntime;
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};
use alife_world::HeadlessScenarioBuilder;

#[test]
fn v11_player_loop_reaches_grounded_body_then_reds_at_school_authority_boundary() {
    let organism_id = OrganismId(1);
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())
        .expect("Task 13 requires the production GPU backend");
    let world = HeadlessScenarioBuilder::new(13_001)
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

    let mut runtime = GpuLiveBrainRuntime::new(
        backend,
        world,
        13_001,
        BrainScaleTier::Nano512,
    )
    .expect("supported Nano512 production runtime");

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

    assert!(
        false,
        "Task 13 RED at the first unavailable production seam: GpuLiveBrainRuntime has no school/teacher-authority action entry point. The existing live speech entry point is player-only, while the world teacher-token mutation cannot be applied through the runtime-owned world. Do not fake focal attention, grounded prediction, factorized motor selection, authoritative outcome, sealed learning, due sleep/consolidation, changed behavior, reproduction, child inspection, durable candidate replacement, restore, or presentation until that bridge exists."
    );
}
