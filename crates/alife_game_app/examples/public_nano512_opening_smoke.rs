//! Public, opt-in software correctness example, not production-hardware admission.
//! One canonical uncoached Nano512 opening. No checkpoint, replay, model override,
//! forced action or fallback. Build the example only with the stock gpu-tests feature.
#[cfg(feature = "gpu-tests")]
use alife_core::{
    BrainCapacityClass, BrainScaleTier, CanonicalDigestBuilder, DecisionEvidence, FoundationWeightAsset,
    N512FounderFoundationProjection, OrganismId, SensorProfile, SleepPhase, Tick,
};
#[cfg(feature = "gpu-tests")]
use alife_game_app::{GpuLiveBrainRuntime, GpuLiveTickOutcome};
#[cfg(feature = "gpu-tests")]
use alife_gpu_backend::{GpuAuthorityCommitEvidenceV1, GpuClosedLoopBackend, GpuRuntimeProfile};
#[cfg(feature = "gpu-tests")]
use alife_runtime::GpuSessionConsumerKind;
#[cfg(feature = "gpu-tests")]
use alife_world::{CanonicalNewGameConfig, create_canonical_new_game};
#[cfg(feature = "gpu-tests")]
use std::io::{self, Write};
#[cfg(feature = "gpu-tests")]
use std::process::ExitCode;

#[cfg(feature = "gpu-tests")]
const SEED: u64 = 31_117;
#[cfg(feature = "gpu-tests")]
const OWNER: OrganismId = OrganismId(1);
#[cfg(feature = "gpu-tests")]
const CLASS: BrainScaleTier = BrainScaleTier::Nano512;
#[cfg(feature = "gpu-tests")]
const SENSOR: SensorProfile = SensorProfile::GroundedObjectSlotsV1;

#[cfg(feature = "gpu-tests")]
fn mark(phase: &str) {
    println!("NANO512_OPENING_STAGE {phase}");
    io::stdout().flush().unwrap();
}

#[cfg(feature = "gpu-tests")]
fn main() -> ExitCode {
    // Explicit opt-in for this one nonproduction example; no defaults or
    // command line source/output paths and no mutation to the inherited process.
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    assert_eq!(args.len(), 1);
    assert_eq!(args[0], "--one-awake-opening");
    assert_eq!(std::env::var("QUALIA_PUBLIC_NANO512_WITNESS").as_deref(), Ok("1"));
    for key in ["ALIFE_SLM_PRIOR", "ALIFE_SLM_PRIOR_BACKEND"] {
        assert_eq!(std::env::var(key).as_deref(), Ok("off"));
    }
    for forbidden in [
        "ALIFE_SLM_PRIOR_BANK", "ALIFE_EXPLICIT_FIXED_HOST_PRESSURE_NS",
    ] {
        assert!(std::env::var_os(forbidden).is_none(), "unexpected {forbidden}");
    }
    mark("admitted-example-args");
    let asset = FoundationWeightAsset::builtin_nano512_v1(SENSOR).unwrap();
    assert_eq!(asset.manifest().sensor_profile(), SENSOR);
    let config = CanonicalNewGameConfig::phase3(SEED, 1).unwrap();
    assert_eq!((config.brain_class, config.sensor_profile), (CLASS, SENSOR));

    // The same public canonical builtin New Game fixture path. Keep its
    // actual stock ecology and terrain, without opening a save or injecting
    // target, teacher, drive, food, movement, utterance, or outcome.
    let mut birth = create_canonical_new_game(&config, &asset).unwrap();
    let [founder] = birth.receipt.founders.as_slice() else { panic!("not one fresh founder"); };
    assert_eq!(founder.organism_id, OWNER);
    let body = founder.world_entity_id;
    let resident = birth.world.organism_registry().get(OWNER).unwrap();
    assert_eq!(resident.birth_tick(), Tick::ZERO);
    assert!(resident.lifecycle().is_alive() && !resident.biochemistry().body.sleeping);
    // Ordinary builtin Nano512 compiles this expressed record through the
    // stock canonical foundation projection. CreaturePhenotype is not the
    // compiled GPU BrainPhenotype and must never be compared by a guessed hash.
    let projection = N512FounderFoundationProjection::compile(resident.phenotype(), SENSOR, &asset)
        .unwrap();
    let compiled = projection.compiled_phenotype();
    assert_eq!(compiled.brain_class_id(), BrainCapacityClass::N512_ID);
    let brain = compiled.phenotype_hash();
    drop(projection);
    birth.world.enable_island_for_new_game().unwrap();
    let world = birth.world;
    assert_eq!(world.tick(), Tick::ZERO);
    assert!(world.audible_utterances().is_empty());
    assert!(world.tracked_objects().save_state(OWNER).unwrap().records.is_empty());
    mark("fresh-fixture-ready");

    // This is not the software TRAINING constructor (forbidden by live-action
    // admission), not a policy fallback, not a hardware-qualification test.
    let backend = GpuClosedLoopBackend::new_software_vulkan_runtime_test(
        GpuRuntimeProfile::production_v1(),
    ).unwrap();
    assert!(backend.is_software_runtime_test() && !backend.is_software_training_test());
    let info = backend.software_runtime_test_adapter_info().unwrap();
    assert_eq!(format!("{:?}", info.device_type), "Cpu");
    assert_eq!(format!("{:?}", info.backend), "Vulkan");
    assert!(info.name.to_ascii_lowercase().contains("llvmpipe"));
    assert_eq!(info.driver, "llvmpipe");
    assert!(info.driver_info.contains("Mesa"));
    mark("explicit-named-software-runtime-device-ready");

    let mut rt = GpuLiveBrainRuntime::new_profiled(backend, world, SEED, CLASS, SENSOR).unwrap();
    assert_eq!(rt.session_authority().consumer(), GpuSessionConsumerKind::Gameplay);
    let prior = rt.semantic_prior_metrics().unwrap();
    assert_eq!(prior.backend, "off");
    assert!(prior.recorded_bank_digest.is_none() && prior.decision_frames == 0);
    assert!(rt.persistence_idle_for_shutdown_for_test());
    assert_eq!(rt.sleep_state_for_test(OWNER).unwrap().phase, SleepPhase::Awake);
    let r = rt.residency_summary();
    assert_eq!((r.handle_count, r.resident_count, r.memory_sidecar_count), (1, 1, 1));
    assert!(rt.sealed_patches().is_empty() && rt.last_learning_receipts().is_empty()
        && rt.last_gpu_authority_receipts().is_empty());
    mark("one-fresh-nano512-awake-resident-ready");

    // The only submitted interval. On no progress, unsupported scheduler,
    // hardware or authority mismatch the harness stops; no retry.
    let GpuLiveTickOutcome::Progressed(rows) = rt.tick_outcome().unwrap() else {
        panic!("ordinary interval not accepted");
    };
    let [row] = rows.as_slice() else { panic!("not exactly one accepted owner row"); };
    assert_eq!((row.organism_id, row.world_tick_before, row.world_tick_after),
        (OWNER, Tick::ZERO, Tick(1)));
    assert!(row.patch_sealed);
    assert_eq!(row.patch_sequence_id, Some(1));
    let motors = row.motor_execution.as_ref().expect("no ordinary settled motor evidence");
    assert!(!motors.channel_receipts.is_empty());
    assert!(rt.persistence_idle_for_shutdown_for_test());
    let end_world = rt.world_snapshot();
    let end_body = end_world.organism_registry().get(OWNER).unwrap();
    assert_eq!((end_world.tick(), end_body.birth_tick(), end_body.world_entity_id(),
                end_body.biochemistry().tick), (Tick(1), Tick::ZERO, body, Tick(1)));
    assert!(end_body.lifecycle().is_alive());
    let [patch] = rt.sealed_patches() else { panic!("not one committed fresh patch"); };
    let frame = patch.pre_action().perception();
    frame.validate().unwrap();
    assert_eq!((frame.organism_id(), frame.tick(), frame.sensor_profile()),
        (OWNER, Tick::ZERO, SENSOR));
    assert_eq!((patch.header().organism_id, patch.header().sequence_id.raw(),
        patch.decision().decision_tick, patch.outcome().outcome_tick),
        (OWNER, 1, Tick::ZERO, Tick(1)));
    assert!(patch.selected_bundle().is_some() && patch.outcome().joint.is_some());
    assert!(patch.outcome().teacher_feedback.is_none() && frame.sensory().semantic_context.is_none());
    let DecisionEvidence::NeuralClosedLoopGpu(e) = &patch.decision().evidence else {
        panic!("no real device-origin chosen candidate");
    };
    assert_eq!((e.phenotype_hash, e.base_digest, e.frame_digest),
        (brain, frame.base_digest(), frame.frame_digest()));
    assert!(e.dispatch_generation > 0 && e.logit.is_finite());
    assert_eq!(row.selected_action_id, Some(e.action_id));
    let candidate = frame.candidates().get(usize::from(e.candidate_index)).unwrap();
    assert_eq!((candidate.candidate_index, candidate.action_id), (e.candidate_index, e.action_id));

    // Public ordinary learning receipt exposes the trusted real handle, never
    // infer a slot/generation from singleton state. Join the separate compact
    // authority to that handle AND to the accepted semantic patch.
    let [learning] = rt.last_learning_receipts() else { panic!("a commit required, not retained"); };
    let [authority] = rt.last_gpu_authority_receipts() else { panic!("missing compact authority"); };
    authority.validate().unwrap();
    assert_eq!(authority.schema_version(), 1);
    let h = learning.handle;
    assert_eq!((h.organism_id(), h.phenotype_hash(), h.class_id()),
        (OWNER, brain, BrainCapacityClass::N512_ID));
    assert_eq!((learning.sequence_id, learning.dispatch_generation, learning.active_activation_side),
        (patch.header().sequence_id, e.dispatch_generation, e.active_activation_side));
    assert_eq!(authority.evidence(), GpuAuthorityCommitEvidenceV1::LearningCommitted {
        input_fast_generation: learning.input_fast_generation,
        output_fast_generation: learning.output_fast_generation,
        output_eligibility_generation: learning.output_eligibility_generation,
        replay_journal_generation: learning.replay_journal_generation,
        transaction_generation: learning.transaction_generation,
        hardware_receipt_generation: learning.hardware_receipt_generation,
    });
    let patch_digest = patch.causal_digest().unwrap();
    let mut d = CanonicalDigestBuilder::new(b"alife.gpu.authority-receipt.v1");
    d.write_u16(authority.schema_version());
    d.write_u32(u32::from(h.class_id().raw()));
    d.write_u32(h.slot());
    d.write_u32(h.generation());
    d.write_u64(OWNER.raw());
    for w in brain.0 { d.write_u64(w); }
    d.write_u64(patch.header().sequence_id.raw());
    d.write_u64(patch.decision().decision_tick.raw());
    d.write_u64(patch.outcome().outcome_tick.raw());
    d.write_u64(e.dispatch_generation);
    d.write_u8(e.active_activation_side);
    for w in patch_digest { d.write_u64(w); }
    d.write_u8(1);
    for w in [learning.input_fast_generation, learning.output_fast_generation,
        learning.output_eligibility_generation, learning.replay_journal_generation,
        learning.transaction_generation, learning.hardware_receipt_generation] { d.write_u64(w); }
    assert_eq!(d.finish256(), authority.receipt_digest());

    // Compact values only: no full model, heap, replay spans or record export.
    // A valid neural decision can legally choose Rest, decline fruit, or fail.
    // The actual accepted native settlement must have incurred a positive
    // physical fee for the narrower paid-act pass. Otherwise retain the
    // qualified neural result as a paid-act null; never try another tick.
    let mut settled_channels = Vec::new();
    let mut paid = false;
    for ch in &motors.channel_receipts {
        let fee = ch.physical.energy_cost.raw();
        assert!(fee.is_finite() && fee >= 0.0);
        paid |= fee > 0.0;
        settled_channels.push(serde_json::json!({
            "phase": format!("{:?}", ch.phase),
            "energy_fee_f32_bits": fee.to_bits(),
            "contact": format!("{:?}", ch.physical.contact),
        }));
    }
    println!("{}", serde_json::json!({
        "schema": "nano512-ordinary-offline-software-opening-v1",
        "adapter": info.name, "driver_info": info.driver_info,
        "same_owner": OWNER.raw(), "decision_tick": 0, "outcome_tick": 1,
        "world_seed": SEED, "neural_action": e.action_id.raw(), "candidate_index": e.candidate_index,
        "logit_f32_bits": e.logit.to_bits(), "real_channel_count": motors.channel_receipts.len(),
        "actual_patch_success": row.patch_success, "body_entity": body.raw(),
        "settled_channels": settled_channels, "accepted_nonzero_paid_channel": paid,
        "joint_patch_digest": patch_digest,
        "authority_digest": authority.receipt_digest(),
        "slot": h.slot(), "slot_generation": h.generation(),
        "dispatch_generation": e.dispatch_generation, "phenotype_hash": brain.0,
        "hardware_gpu_qualification": false, "save_or_replay_used": false,
    }));
    io::stdout().flush().unwrap();
    if paid { ExitCode::SUCCESS } else { ExitCode::from(3) }
}

#[cfg(not(feature = "gpu-tests"))]
fn main() -> std::process::ExitCode {
    eprintln!("public_nano512_opening_smoke requires --features gpu-tests");
    std::process::ExitCode::from(78)
}
