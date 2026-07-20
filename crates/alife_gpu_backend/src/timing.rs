//! Local GPU diagnostic timing evidence for manual productization reports.
//!
//! This module measures the bounded legacy P25 static-forward fixture only. It is not an
//! active gameplay runtime path and all GPU readback remains manual diagnostic
//! evidence scoped.

use std::time::Instant;

use alife_core::{
    validate_finite, BrainClassSpec, BrainScaleTier, CooEntry, CooTile, DenseTile,
    NeuralProjectionSchema, ProjectionTile, ScaffoldContractError, SparseTileCoord,
    SynapseWeightSplit, MICROTILE_CELLS, MICROTILE_EDGE,
};

use crate::{
    run_static_forward_gpu_diagnostic_timed, GpuFixedPointPolicy, GpuRuntimeBackendKind,
    GpuStaticForwardPlan, GpuUploadBuffers,
};

pub const GPU_DIAGNOSTIC_TIMING_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuDiagnosticTimingKind {
    HostObservedDiagnostic,
    TimestampQuery,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuDiagnosticProductRuntimeClaim {
    None,
    DiagnosticOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuTimingTargetStatus {
    Met,
    Missed,
    NotApplicable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuDiagnosticWorkloadTiming {
    pub schema_version: u16,
    pub workload_name: String,
    pub fixture_dimensions: String,
    pub warmup_iterations: u32,
    pub measured_iterations: u32,
    pub cpu_reference_mean_ms: Option<f32>,
    pub gpu_submit_poll_mean_ms: Option<f32>,
    pub readback_mean_ms: Option<f32>,
    pub gpu_total_mean_ms: Option<f32>,
    pub parity_passed: bool,
    pub no_active_gameplay_readback: bool,
    pub timing_kind: GpuDiagnosticTimingKind,
    pub product_runtime_claim: GpuDiagnosticProductRuntimeClaim,
    pub target_60_fps: GpuTimingTargetStatus,
    pub notes: String,
}

impl GpuDiagnosticWorkloadTiming {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != GPU_DIAGNOSTIC_TIMING_SCHEMA_VERSION
            || self.workload_name.trim().is_empty()
            || self.fixture_dimensions.trim().is_empty()
            || self.measured_iterations == 0
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        for value in [
            self.cpu_reference_mean_ms,
            self.gpu_submit_poll_mean_ms,
            self.readback_mean_ms,
            self.gpu_total_mean_ms,
        ]
        .into_iter()
        .flatten()
        {
            validate_finite(value)?;
            if value < 0.0 {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        match self.timing_kind {
            GpuDiagnosticTimingKind::HostObservedDiagnostic
            | GpuDiagnosticTimingKind::TimestampQuery => {
                if !self.no_active_gameplay_readback
                    || self.product_runtime_claim
                        != GpuDiagnosticProductRuntimeClaim::DiagnosticOnly
                    || !self.parity_passed
                    || self.gpu_submit_poll_mean_ms.unwrap_or(0.0) <= 0.0
                    || self.gpu_total_mean_ms.unwrap_or(0.0) <= 0.0
                {
                    return Err(ScaffoldContractError::BackendParity);
                }
            }
            GpuDiagnosticTimingKind::Unavailable => {
                if self.gpu_submit_poll_mean_ms.is_some()
                    || self.readback_mean_ms.is_some()
                    || self.gpu_total_mean_ms.is_some()
                {
                    return Err(ScaffoldContractError::BackendParity);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuDiagnosticTimingReport {
    pub schema_version: u16,
    pub adapter_identifier: String,
    pub adapter_name: String,
    pub backend_api: String,
    pub driver_info: String,
    pub timestamp_query_supported: bool,
    pub requested_backend: GpuRuntimeBackendKind,
    pub product_gameplay_timing_claim: GpuDiagnosticProductRuntimeClaim,
    pub workloads: Vec<GpuDiagnosticWorkloadTiming>,
}

impl GpuDiagnosticTimingReport {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != GPU_DIAGNOSTIC_TIMING_SCHEMA_VERSION
            || self.adapter_identifier.trim().is_empty()
            || self.workloads.is_empty()
            || self.product_gameplay_timing_claim != GpuDiagnosticProductRuntimeClaim::None
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        for workload in &self.workloads {
            workload.validate()?;
        }
        Ok(())
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Local GPU timing evidence\n\n");
        out.push_str(&format!(
            "- Schema version: {}\n- Adapter: {}\n- Backend API: {}\n- Driver: {}\n- Requested backend: {:?}\n- Timestamp query supported: {}\n- Product gameplay timing claim: {:?}\n\n",
            self.schema_version,
            self.adapter_identifier,
            self.backend_api,
            self.driver_info,
            self.requested_backend,
            self.timestamp_query_supported,
            self.product_gameplay_timing_claim,
        ));
        out.push_str("| Workload | Dimensions | Warmup | Measured | CPU mean ms | GPU submit/poll mean ms | Readback mean ms | GPU total mean ms | Parity | Timing kind | 60 FPS target | Claim | Notes |\n");
        out.push_str("|---|---|---:|---:|---:|---:|---:|---:|---|---|---|---|---|\n");
        for workload in &self.workloads {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {:?} | {:?} | {:?} | {} |\n",
                workload.workload_name,
                workload.fixture_dimensions,
                workload.warmup_iterations,
                workload.measured_iterations,
                optional_ms(workload.cpu_reference_mean_ms),
                optional_ms(workload.gpu_submit_poll_mean_ms),
                optional_ms(workload.readback_mean_ms),
                optional_ms(workload.gpu_total_mean_ms),
                workload.parity_passed,
                workload.timing_kind,
                workload.target_60_fps,
                workload.product_runtime_claim,
                workload.notes,
            ));
        }
        out.push_str("\n## Evidence boundary\n\n");
        out.push_str("- These measurements are diagnostic/manual GPU workloads, not active gameplay runtime timing.\n");
        out.push_str("- Diagnostic readback is separated from submit/poll timing and is not exposed as an active tick API.\n");
        out.push_str(
            "- Required GPU unavailability is typed and stops learned actions; explicit headless baselines do not require GPU.\n",
        );
        out
    }
}

pub fn run_local_gpu_diagnostic_timing(
    warmup_iterations: u32,
    measured_iterations: u32,
) -> Result<GpuDiagnosticTimingReport, ScaffoldContractError> {
    pollster::block_on(run_local_gpu_diagnostic_timing_async(
        warmup_iterations,
        measured_iterations,
    ))
}

async fn run_local_gpu_diagnostic_timing_async(
    warmup_iterations: u32,
    measured_iterations: u32,
) -> Result<GpuDiagnosticTimingReport, ScaffoldContractError> {
    if measured_iterations == 0 {
        return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
    }
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .map_err(|_| ScaffoldContractError::BackendParity)?;
    let info = adapter.get_info();
    let timestamp_query_supported = adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY);
    let mut required_limits = wgpu::Limits::downlevel_defaults();
    required_limits.max_storage_buffers_per_shader_stage =
        required_limits.max_storage_buffers_per_shader_stage.max(10);
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("alife-local-gpu-diagnostic-timing-device"),
            required_features: wgpu::Features::empty(),
            required_limits,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|_| ScaffoldContractError::BackendParity)?;

    let static_workload =
        measure_static_forward(&device, &queue, warmup_iterations, measured_iterations).await?;
    let adapter_identifier = format!(
        "{} ({:?}, {:?}, {})",
        info.name, info.backend, info.device_type, info.driver_info
    );
    let report = GpuDiagnosticTimingReport {
        schema_version: GPU_DIAGNOSTIC_TIMING_SCHEMA_VERSION,
        adapter_identifier,
        adapter_name: info.name,
        backend_api: format!("{:?}", info.backend),
        driver_info: info.driver_info,
        timestamp_query_supported,
        requested_backend: GpuRuntimeBackendKind::GpuAuthoritative,
        product_gameplay_timing_claim: GpuDiagnosticProductRuntimeClaim::None,
        workloads: vec![static_workload],
    };
    report.validate()?;
    Ok(report)
}

async fn measure_static_forward(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    warmup_iterations: u32,
    measured_iterations: u32,
) -> Result<GpuDiagnosticWorkloadTiming, ScaffoldContractError> {
    let plan = static_forward_fixture_plan()?;
    let activations = static_forward_activation_fixture();
    let activation_q = plan.quantize_activations(&activations)?;
    let cpu_start = Instant::now();
    let cpu = plan.execute_cpu_diagnostic(&activation_q)?;
    let cpu_reference_mean_ms = elapsed_ms(cpu_start);

    for _ in 0..warmup_iterations {
        run_static_forward_gpu_diagnostic_timed(device, queue, &plan, &activation_q).await?;
    }

    let mut submit_poll_ms = 0.0_f32;
    let mut readback_ms = 0.0_f32;
    let mut parity_passed = true;
    for _ in 0..measured_iterations {
        let timed =
            run_static_forward_gpu_diagnostic_timed(device, queue, &plan, &activation_q).await?;
        submit_poll_ms += timed.timing.submit_poll_wall_ms;
        readback_ms += timed.timing.readback_wall_ms;
        parity_passed &= timed.result.activations_q == cpu.activations_q
            && timed.result.accumulators_q == cpu.accumulators_q
            && timed.result.diagnostics == cpu.diagnostics;
    }

    let submit_poll_mean = submit_poll_ms / measured_iterations as f32;
    let readback_mean = readback_ms / measured_iterations as f32;
    Ok(GpuDiagnosticWorkloadTiming {
        schema_version: GPU_DIAGNOSTIC_TIMING_SCHEMA_VERSION,
        workload_name: "P25 static forward diagnostic fixture".to_string(),
        fixture_dimensions: format!(
            "neurons={}, tiles={}, synapses={}, dispatch=({},{},{})",
            plan.header.neuron_count,
            plan.tile_metadata.len(),
            plan.packed_indices.len(),
            plan.dispatch.pass0_workgroups,
            plan.dispatch.pass1_workgroups,
            plan.dispatch.pass2_workgroups
        ),
        warmup_iterations,
        measured_iterations,
        cpu_reference_mean_ms: Some(cpu_reference_mean_ms),
        gpu_submit_poll_mean_ms: Some(submit_poll_mean),
        readback_mean_ms: Some(readback_mean),
        gpu_total_mean_ms: Some(submit_poll_mean + readback_mean),
        parity_passed,
        no_active_gameplay_readback: true,
        timing_kind: GpuDiagnosticTimingKind::HostObservedDiagnostic,
        product_runtime_claim: GpuDiagnosticProductRuntimeClaim::DiagnosticOnly,
        target_60_fps: GpuTimingTargetStatus::NotApplicable,
        notes: "Host-observed diagnostic timing; readback is manual parity evidence, not gameplay."
            .to_string(),
    })
}

fn static_forward_fixture_plan() -> Result<GpuStaticForwardPlan, ScaffoldContractError> {
    let policy = GpuFixedPointPolicy::reference();
    let schema = static_forward_schema()?;
    let upload = GpuUploadBuffers::from_cpu_schema(&schema, policy)?;
    GpuStaticForwardPlan::from_upload(&upload, policy)
}

fn static_forward_schema() -> Result<NeuralProjectionSchema, ScaffoldContractError> {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&spec)?;
    schema.projections[0].tiles.push(ProjectionTile::new_coo(
        0,
        SparseTileCoord::new(0, 0)?,
        CooTile::new(vec![
            CooEntry::new(0, 0, weights(0.25, 0.125, 0.5, 0.25, 0.0)?)?,
            CooEntry::new(1, 1, weights(-0.25, 0.5, 0.25, 1.0, 0.0)?)?,
        ])?,
    ));

    let mut dense = vec![SynapseWeightSplit::zero(); MICROTILE_CELLS];
    dense[3] = weights(0.5, 0.0, 1.0, 0.5, 0.0)?;
    dense[MICROTILE_EDGE as usize + 4] = weights(-0.25, 0.0, 1.0, -0.25, 0.0)?;
    schema.projections[0].tiles.push(ProjectionTile::new_dense(
        0,
        SparseTileCoord::new(1, 0)?,
        DenseTile::new(dense)?,
    ));
    schema.rebuild_supertile_masks();
    Ok(schema)
}

fn static_forward_activation_fixture() -> Vec<f32> {
    let mut activations = vec![0.0; 512];
    activations[0] = 0.5;
    activations[1] = -0.25;
    activations[3] = 0.75;
    activations[4] = -0.5;
    activations
}

fn weights(
    genetic: f32,
    lifetime: f32,
    alpha: f32,
    h: f32,
    h_shadow: f32,
) -> Result<SynapseWeightSplit, ScaffoldContractError> {
    SynapseWeightSplit::new(genetic, lifetime, alpha, h, h_shadow)
}

fn elapsed_ms(start: Instant) -> f32 {
    start.elapsed().as_secs_f64().mul_add(1000.0, 0.0) as f32
}

fn optional_ms(value: Option<f32>) -> String {
    value.map_or_else(|| "unknown".to_string(), |value| format!("{value:.4}"))
}
