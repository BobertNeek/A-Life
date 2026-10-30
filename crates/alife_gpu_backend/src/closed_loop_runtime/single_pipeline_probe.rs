//! Original probes and an explicitly behavior-removing NON-PRODUCTION compile-only reduction.
use super::*;
use crate::closed_loop_pipeline::single_pipeline_probe::{compile_selected, Target};
fn one(target: Target) -> Result<(), Box<dyn std::error::Error>> {
    if !cfg!(feature = "training-rollout") {
        return Err("explicit training-rollout variant required".into());
    }
    eprintln!(
        "gpu_single_pipeline source_variant={} selected_target={target:?} no_dispatch=true",
        target.source_variant()
    );
    eprintln!("gpu_single_pipeline entry={} stage=raw_acquire event=begin training_rollout=true default_instance_flags=true selection=unchanged_PRIMARY_SECONDARY no_kernel_set=true",target.entry());
    let raw = acquire_required_gpu(&WgpuDeviceFactory)?;
    eprintln!("gpu_single_pipeline entry={} stage=raw_acquire event=ok hardware={} adapter={:?} features={:?} limits={:?}",target.entry(),serde_json::to_string(&raw.hardware)?,raw.adapter.get_info(),raw.device.features(),raw.device.limits());
    // No GpuClosedLoopKernelSet/new_required, timestamps, arena, brain or dispatch.
    let _pipeline = compile_selected(&raw.device, target)?;
    eprintln!(
        "gpu_single_pipeline entry={} stage=event_poll event=begin async_loss_not_attributed=true",
        target.entry()
    );
    let poll = raw.device.poll(wgpu::PollType::Poll);
    eprintln!(
        "gpu_single_pipeline event=poll_returned result={poll:?} lost={}",
        raw.lost.load(Ordering::Acquire)
    );
    poll?;
    if raw.lost.load(Ordering::Acquire) {
        return Err(ScaffoldContractError::NeuralBackendUnavailable.into());
    }
    eprintln!(
        "gpu_single_pipeline entry={} stage=empty_submit_wait event=begin",
        target.entry()
    );
    let index = raw.queue.submit(std::iter::empty());
    let wait = raw.device.poll(wgpu::PollType::Wait {
        submission_index: Some(index),
        timeout: None,
    });
    eprintln!(
        "gpu_single_pipeline event=wait_returned result={wait:?} lost={}",
        raw.lost.load(Ordering::Acquire)
    );
    wait?;
    if raw.lost.load(Ordering::Acquire) {
        return Err(ScaffoldContractError::NeuralBackendUnavailable.into());
    }
    eprintln!(
        "gpu_single_pipeline entry={} stage=complete no_dispatch=true",
        target.entry()
    );
    Ok(())
}
#[test]
#[ignore = "exclusive real GPU; original single pipeline only; fresh process"]
fn original_sleep_initialize_only() -> Result<(), Box<dyn std::error::Error>> {
    one(Target::Sleep)
}
#[test]
#[ignore = "exclusive real GPU; original single pipeline only; fresh process"]
fn original_eligibility_finalize_only() -> Result<(), Box<dyn std::error::Error>> {
    one(Target::Eligibility)
}
#[test]
#[ignore = "exclusive real GPU; original encode control only; fresh process"]
fn original_encode_control_only() -> Result<(), Box<dyn std::error::Error>> {
    one(Target::Encode)
}

#[test]
#[ignore = "NON-PRODUCTION behavior-removing compile-only reduction; NEVER dispatch; exclusive real GPU/fresh process"]
fn nonproduction_remove_initializer_barrier_only() -> Result<(), Box<dyn std::error::Error>> {
    one(Target::SleepNoInitializerBarrier)
}
