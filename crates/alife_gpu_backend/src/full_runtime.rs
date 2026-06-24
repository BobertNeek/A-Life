//! Optional product-facing GPU neural runtime bridge.
//!
//! The current live path supports CPU-shadow-guarded static GPU action scoring
//! with compact active-tick readback. Static-plastic shadow mode can apply
//! validated post-seal H_shadow deltas through the core-owned lifetime contract.
//! The combined static/plastic mode runs both in one CPU-shadow-guarded live
//! path; full action-authoritative static+routing+plastic runtime remains
//! unsupported.

use std::time::Instant;

use alife_core::{
    validate_finite, BrainClassSpec, BrainScaleTier, CooEntry, CooTile, ExperiencePatch,
    NeuralProjectionSchema, OjaUpdateConfig, PostSealHShadowDeltaTarget,
    PostSealLifetimeDeltaBatch, PostSealLifetimeDeltaRecord, PostSealLifetimeDeltaSourceKind,
    ProjectionTile, ScaffoldContractError, SparseTileCoord, SparseTilePayload, SynapseWeightSplit,
    Validate,
};

use crate::{
    run_plasticity_gpu_diagnostic_timed, run_static_forward_gpu_action_summary_timed,
    GpuActionSummaryStagingRecord, GpuFixedPointPolicy, GpuOjaFixedPointConfig, GpuPlasticityPlan,
    GpuRuntimeBackendConfig, GpuRuntimeBackendKind, GpuRuntimeBackendStatus,
    GpuRuntimeFallbackReason, GpuStaticActionSummaryConfig, GpuStaticForwardPlan, GpuUploadBuffers,
    GPU_ACTION_SUMMARY_RECORD_BYTES, P26_PLASTICITY_DIAGNOSTIC_WORDS,
};

pub const FULL_GPU_RUNTIME_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullGpuRuntimeMode {
    CpuReference,
    GpuStaticShadow,
    GpuStaticActionAuthoritative,
    GpuStaticPlasticShadow,
    GpuStaticPlasticCpuShadowGuarded,
    GpuFullShadow,
    GpuFullActionAuthoritative,
}

impl FullGpuRuntimeMode {
    pub const fn requested_backend(self) -> GpuRuntimeBackendKind {
        match self {
            Self::CpuReference => GpuRuntimeBackendKind::CpuReference,
            Self::GpuStaticShadow | Self::GpuStaticActionAuthoritative => {
                GpuRuntimeBackendKind::GpuStatic
            }
            Self::GpuStaticPlasticShadow | Self::GpuStaticPlasticCpuShadowGuarded => {
                GpuRuntimeBackendKind::GpuPlastic
            }
            Self::GpuFullShadow | Self::GpuFullActionAuthoritative => {
                GpuRuntimeBackendKind::GpuFull
            }
        }
    }

    pub const fn requests_plasticity(self) -> bool {
        matches!(
            self,
            Self::GpuStaticPlasticShadow
                | Self::GpuStaticPlasticCpuShadowGuarded
                | Self::GpuFullShadow
                | Self::GpuFullActionAuthoritative
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullGpuRuntimeProductClaim {
    None,
    ShadowOnly,
    CpuShadowGuarded,
    CpuShadowGuardedStaticPlusLiveHShadow,
    ActionAuthoritative,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FullGpuRuntimeStaticTickInput {
    pub action_ids: [u32; 4],
    pub food_salience: f32,
    pub hazard_salience: f32,
    pub inspect_salience: f32,
    pub idle_salience: f32,
    pub confidence: f32,
    pub drive_source_mask: u32,
}

impl FullGpuRuntimeStaticTickInput {
    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        for value in [
            self.food_salience,
            self.hazard_salience,
            self.inspect_salience,
            self.idle_salience,
            self.confidence,
        ] {
            validate_finite(value)?;
            if !(0.0..=1.0).contains(&value) {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        if self.action_ids.contains(&0) {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        Ok(())
    }

    pub fn saliences(self) -> [f32; 4] {
        [
            self.food_salience,
            self.hazard_salience,
            self.inspect_salience,
            self.idle_salience,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullGpuRuntimeRoutingReport {
    pub total_tiles: u32,
    pub active_tiles: u32,
    pub skipped_tiles: u32,
    pub active_synapses: u32,
    pub skipped_supertiles: u32,
    pub routing_descriptors_evaluated: u32,
    pub dispatch_level_culling_optimized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullGpuRuntimeReadbackReport {
    pub compact_readback_bytes: usize,
    pub action_summary_allowed: bool,
    pub bulk_neural_readback_forbidden: bool,
    pub per_synapse_readback_forbidden: bool,
    pub per_lobe_readback_forbidden: bool,
    pub weight_readback_forbidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FullGpuRuntimeTimingReport {
    pub upload_ms: f32,
    pub gpu_submit_poll_ms: f32,
    pub compact_readback_ms: f32,
    pub cpu_shadow_ms: f32,
    pub total_gpu_runtime_ms: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FullGpuRuntimeStaticTickReport {
    pub schema_version: u16,
    pub mode: FullGpuRuntimeMode,
    pub backend: GpuRuntimeBackendStatus,
    pub hardware_identifier: Option<String>,
    pub action_summary: Option<GpuActionSummaryStagingRecord>,
    pub cpu_shadow_action_summary: Option<GpuActionSummaryStagingRecord>,
    pub cpu_shadow_parity_passed: bool,
    pub routing: FullGpuRuntimeRoutingReport,
    pub readback: FullGpuRuntimeReadbackReport,
    pub timing: FullGpuRuntimeTimingReport,
    pub product_runtime_claim: FullGpuRuntimeProductClaim,
    pub fallback_note: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FullGpuRuntimePlasticityReport {
    pub schema_version: u16,
    pub diagnostic_only: bool,
    pub post_seal_only: bool,
    pub h_shadow_changed: bool,
    pub updated_values_count: u32,
    pub max_delta_q: i32,
    pub saturation_count: u32,
    pub nan_or_inf_rejected: bool,
    pub genetic_fixed_unchanged: bool,
    pub lifetime_consolidated_unchanged: bool,
    pub h_operational_unchanged: bool,
    pub cpu_shadow_parity_passed: bool,
    pub submit_poll_ms: f32,
    pub diagnostic_readback_ms: f32,
    pub diagnostic_readback_bytes: usize,
    pub h_shadow_delta_records: Vec<PostSealLifetimeDeltaRecord>,
    pub live_core_update_applied: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FullGpuRuntimeBackendReport {
    pub static_tick: FullGpuRuntimeStaticTickReport,
    pub plasticity: Option<FullGpuRuntimePlasticityReport>,
}

pub struct FullGpuRuntimeSession {
    mode: FullGpuRuntimeMode,
    execution: FullGpuRuntimeSessionExecution,
}

enum FullGpuRuntimeSessionExecution {
    Cpu {
        backend: GpuRuntimeBackendStatus,
        hardware_identifier: Option<String>,
        fallback_note: Option<String>,
    },
    Gpu {
        device: wgpu::Device,
        queue: wgpu::Queue,
        plan: GpuStaticForwardPlan,
        backend: GpuRuntimeBackendStatus,
        hardware_identifier: String,
    },
}

impl FullGpuRuntimeSession {
    pub fn new(mode: FullGpuRuntimeMode) -> Result<Self, ScaffoldContractError> {
        pollster::block_on(Self::new_async(mode))
    }

    pub fn run_static_tick(
        &self,
        input: FullGpuRuntimeStaticTickInput,
    ) -> Result<FullGpuRuntimeStaticTickReport, ScaffoldContractError> {
        pollster::block_on(self.run_static_tick_async(input))
    }

    pub fn run_post_seal_plasticity_diagnostic(
        &self,
        input: FullGpuRuntimeStaticTickInput,
    ) -> Result<FullGpuRuntimePlasticityReport, ScaffoldContractError> {
        pollster::block_on(self.run_post_seal_plasticity_diagnostic_async(input))
    }

    async fn new_async(mode: FullGpuRuntimeMode) -> Result<Self, ScaffoldContractError> {
        if mode == FullGpuRuntimeMode::CpuReference {
            return Ok(Self {
                mode,
                execution: FullGpuRuntimeSessionExecution::Cpu {
                    backend: GpuRuntimeBackendConfig::request(mode.requested_backend())
                        .with_gpu_feature_enabled(false)
                        .with_hardware_available(false)
                        .with_validation_passed(true)
                        .select_backend()?,
                    hardware_identifier: None,
                    fallback_note: Some("CPU reference mode requested".to_string()),
                },
            });
        }
        if env_flag_optional("ALIFE_GPU_RUNTIME_AVAILABLE") == Some(false) {
            return Ok(Self {
                mode,
                execution: FullGpuRuntimeSessionExecution::Cpu {
                    backend: GpuRuntimeBackendConfig::request(mode.requested_backend())
                        .with_gpu_feature_enabled(true)
                        .with_hardware_available(false)
                        .with_validation_passed(true)
                        .select_backend()?,
                    hardware_identifier: None,
                    fallback_note: Some(
                        "ALIFE_GPU_RUNTIME_AVAILABLE=0 forced CPU fallback".to_string(),
                    ),
                },
            });
        }
        if env_flag_optional("ALIFE_GPU_RUNTIME_VALIDATED") == Some(false) {
            return Ok(Self {
                mode,
                execution: FullGpuRuntimeSessionExecution::Cpu {
                    backend: GpuRuntimeBackendConfig::request(mode.requested_backend())
                        .with_gpu_feature_enabled(true)
                        .with_hardware_available(true)
                        .with_validation_passed(false)
                        .select_backend()?,
                    hardware_identifier: None,
                    fallback_note: Some(
                        "ALIFE_GPU_RUNTIME_VALIDATED=0 forced CPU fallback".to_string(),
                    ),
                },
            });
        }

        let instance = wgpu::Instance::default();
        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
        {
            Ok(adapter) => adapter,
            Err(error) => {
                return Ok(Self {
                    mode,
                    execution: FullGpuRuntimeSessionExecution::Cpu {
                        backend: GpuRuntimeBackendConfig::request(mode.requested_backend())
                            .with_gpu_feature_enabled(true)
                            .with_hardware_available(false)
                            .with_validation_passed(true)
                            .select_backend()?,
                        hardware_identifier: None,
                        fallback_note: Some(format!("wgpu adapter request failed: {error}")),
                    },
                });
            }
        };
        let info = adapter.get_info();
        let hardware_identifier = format!(
            "{} ({:?}, {:?}, {})",
            info.name, info.backend, info.device_type, info.driver_info
        );
        let mut required_limits = wgpu::Limits::downlevel_defaults();
        required_limits.max_storage_buffers_per_shader_stage =
            required_limits.max_storage_buffers_per_shader_stage.max(10);
        let (device, queue) = match adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("alife-full-gpu-runtime-session-device"),
                required_features: wgpu::Features::empty(),
                required_limits,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
        {
            Ok(device) => device,
            Err(error) => {
                return Ok(Self {
                    mode,
                    execution: FullGpuRuntimeSessionExecution::Cpu {
                        backend: GpuRuntimeBackendConfig::request(mode.requested_backend())
                            .with_gpu_feature_enabled(true)
                            .with_hardware_available(false)
                            .with_validation_passed(true)
                            .select_backend()?,
                        hardware_identifier: Some(hardware_identifier),
                        fallback_note: Some(format!("wgpu device request failed: {error}")),
                    },
                });
            }
        };

        let backend = GpuRuntimeBackendConfig::request(mode.requested_backend())
            .with_gpu_feature_enabled(true)
            .with_hardware_available(true)
            .with_validation_passed(true)
            .select_backend()?;
        if backend.selected == GpuRuntimeBackendKind::CpuReference {
            return Ok(Self {
                mode,
                execution: FullGpuRuntimeSessionExecution::Cpu {
                    backend,
                    hardware_identifier: Some(hardware_identifier),
                    fallback_note: Some(
                        "requested full static+routing+plasticity runtime is not currently supported; bounded static/plastic shadow evidence is available separately"
                            .to_string(),
                    ),
                },
            });
        }

        Ok(Self {
            mode,
            execution: FullGpuRuntimeSessionExecution::Gpu {
                device,
                queue,
                plan: live_static_plan()?,
                backend,
                hardware_identifier,
            },
        })
    }

    async fn run_static_tick_async(
        &self,
        input: FullGpuRuntimeStaticTickInput,
    ) -> Result<FullGpuRuntimeStaticTickReport, ScaffoldContractError> {
        input.validate()?;
        match &self.execution {
            FullGpuRuntimeSessionExecution::Cpu {
                backend,
                hardware_identifier,
                fallback_note,
            } => cpu_status_report(
                input,
                self.mode,
                *backend,
                hardware_identifier.clone(),
                fallback_note.clone(),
            ),
            FullGpuRuntimeSessionExecution::Gpu {
                device,
                queue,
                plan,
                backend,
                hardware_identifier,
            } => {
                let upload_start = Instant::now();
                let activation_q = live_activation_q(plan, input)?;
                let action_summary_config = action_summary_config(input)?;
                let upload_ms = elapsed_ms(upload_start);

                let cpu_shadow_start = Instant::now();
                let cpu = plan.execute_cpu_diagnostic(&activation_q)?;
                let cpu_summary = action_summary_config.cpu_action_summary(&cpu.activations_q)?;
                let cpu_shadow_ms = elapsed_ms(cpu_shadow_start);

                let gpu = run_static_forward_gpu_action_summary_timed(
                    device,
                    queue,
                    plan,
                    &activation_q,
                    action_summary_config,
                )
                .await?;
                let parity = gpu.action_summary == cpu_summary;
                let (backend, parity_fallback_note) = if parity {
                    (*backend, None)
                } else {
                    (
                        GpuRuntimeBackendConfig::request(self.mode.requested_backend())
                            .with_gpu_feature_enabled(true)
                            .with_hardware_available(true)
                            .with_validation_passed(false)
                            .select_backend()?,
                        Some(
                            "GPU compact action summary failed CPU shadow parity; active tick used CPU proposals"
                                .to_string(),
                        ),
                    )
                };
                let routing = routing_report(plan);
                let readback = FullGpuRuntimeReadbackReport {
                    compact_readback_bytes: gpu.compact_readback_bytes,
                    action_summary_allowed: gpu.compact_readback_bytes
                        == GPU_ACTION_SUMMARY_RECORD_BYTES,
                    bulk_neural_readback_forbidden: true,
                    per_synapse_readback_forbidden: true,
                    per_lobe_readback_forbidden: true,
                    weight_readback_forbidden: true,
                };
                let claim = if parity {
                    match self.mode {
                        FullGpuRuntimeMode::GpuStaticActionAuthoritative
                        | FullGpuRuntimeMode::GpuStaticPlasticCpuShadowGuarded
                        | FullGpuRuntimeMode::GpuFullActionAuthoritative => {
                            FullGpuRuntimeProductClaim::CpuShadowGuarded
                        }
                        FullGpuRuntimeMode::GpuStaticShadow
                        | FullGpuRuntimeMode::GpuStaticPlasticShadow
                        | FullGpuRuntimeMode::GpuFullShadow => {
                            FullGpuRuntimeProductClaim::ShadowOnly
                        }
                        FullGpuRuntimeMode::CpuReference => FullGpuRuntimeProductClaim::None,
                    }
                } else {
                    FullGpuRuntimeProductClaim::None
                };
                Ok(FullGpuRuntimeStaticTickReport {
                    schema_version: FULL_GPU_RUNTIME_SCHEMA_VERSION,
                    mode: self.mode,
                    backend,
                    hardware_identifier: Some(hardware_identifier.clone()),
                    action_summary: Some(gpu.action_summary),
                    cpu_shadow_action_summary: Some(cpu_summary),
                    cpu_shadow_parity_passed: parity,
                    routing,
                    readback,
                    timing: FullGpuRuntimeTimingReport {
                        upload_ms,
                        gpu_submit_poll_ms: gpu.timing.submit_poll_wall_ms,
                        compact_readback_ms: gpu.timing.compact_readback_wall_ms,
                        cpu_shadow_ms,
                        total_gpu_runtime_ms: upload_ms
                            + gpu.timing.submit_poll_wall_ms
                            + gpu.timing.compact_readback_wall_ms,
                    },
                    product_runtime_claim: claim,
                    fallback_note: parity_fallback_note,
                })
            }
        }
    }

    async fn run_post_seal_plasticity_diagnostic_async(
        &self,
        input: FullGpuRuntimeStaticTickInput,
    ) -> Result<FullGpuRuntimePlasticityReport, ScaffoldContractError> {
        input.validate()?;
        let FullGpuRuntimeSessionExecution::Gpu { device, queue, .. } = &self.execution else {
            return Ok(FullGpuRuntimePlasticityReport {
                schema_version: FULL_GPU_RUNTIME_SCHEMA_VERSION,
                diagnostic_only: true,
                post_seal_only: true,
                h_shadow_changed: false,
                updated_values_count: 0,
                max_delta_q: 0,
                saturation_count: 0,
                nan_or_inf_rejected: true,
                genetic_fixed_unchanged: true,
                lifetime_consolidated_unchanged: true,
                h_operational_unchanged: true,
                cpu_shadow_parity_passed: false,
                submit_poll_ms: 0.0,
                diagnostic_readback_ms: 0.0,
                diagnostic_readback_bytes: 0,
                h_shadow_delta_records: Vec::new(),
                live_core_update_applied: false,
            });
        };

        let static_plan = live_static_plan()?;
        let activation_q = live_activation_q(&static_plan, input)?;
        let cpu_static = static_plan.execute_cpu_diagnostic(&activation_q)?;
        let schema = live_static_schema(0.1)?;
        let plasticity_plan = live_plasticity_plan_from_schema(&schema)?;
        let cpu =
            plasticity_plan.execute_cpu_diagnostic(&activation_q, &cpu_static.activations_q)?;
        let gpu = run_plasticity_gpu_diagnostic_timed(
            device,
            queue,
            &plasticity_plan,
            &activation_q,
            &cpu_static.activations_q,
        )
        .await?;
        let mut updated_values_count = 0_u32;
        let mut max_delta_q = 0_i32;
        for (before, after) in plasticity_plan
            .h_shadow_initial_q
            .iter()
            .copied()
            .zip(gpu.result.h_shadow_q.iter().copied())
        {
            if before != after {
                updated_values_count = updated_values_count.saturating_add(1);
            }
            max_delta_q = max_delta_q.max((i32::from(after) - i32::from(before)).abs());
        }
        let h_shadow_delta_records = h_shadow_delta_records_from_schema(
            &schema,
            &plasticity_plan.h_shadow_initial_q,
            &gpu.result.h_shadow_q,
            plasticity_plan.policy,
            plasticity_plan.oja.to_oja_config(plasticity_plan.policy),
        )?;
        Ok(FullGpuRuntimePlasticityReport {
            schema_version: FULL_GPU_RUNTIME_SCHEMA_VERSION,
            diagnostic_only: false,
            post_seal_only: true,
            h_shadow_changed: gpu.result.h_shadow_q != plasticity_plan.h_shadow_initial_q,
            updated_values_count,
            max_delta_q,
            saturation_count: gpu.result.diagnostics.saturation_count,
            nan_or_inf_rejected: true,
            genetic_fixed_unchanged: gpu.result.genetic_fixed_q == plasticity_plan.genetic_fixed_q,
            lifetime_consolidated_unchanged: gpu.result.lifetime_consolidated_q
                == plasticity_plan.lifetime_consolidated_q,
            h_operational_unchanged: gpu.result.h_operational_q == plasticity_plan.h_operational_q,
            cpu_shadow_parity_passed: gpu.result.h_shadow_q == cpu.h_shadow_q
                && gpu.result.genetic_fixed_q == cpu.genetic_fixed_q
                && gpu.result.lifetime_consolidated_q == cpu.lifetime_consolidated_q
                && gpu.result.h_operational_q == cpu.h_operational_q
                && gpu.result.diagnostics == cpu.diagnostics,
            submit_poll_ms: gpu.timing.submit_poll_wall_ms,
            diagnostic_readback_ms: gpu.timing.readback_wall_ms,
            diagnostic_readback_bytes: plasticity_diagnostic_readback_bytes(&plasticity_plan),
            h_shadow_delta_records,
            live_core_update_applied: false,
        })
    }
}

pub fn run_full_gpu_runtime_static_tick(
    input: FullGpuRuntimeStaticTickInput,
    mode: FullGpuRuntimeMode,
) -> Result<FullGpuRuntimeStaticTickReport, ScaffoldContractError> {
    pollster::block_on(run_full_gpu_runtime_static_tick_async(input, mode))
}

pub fn run_full_gpu_runtime_post_seal_plasticity_diagnostic(
    input: FullGpuRuntimeStaticTickInput,
) -> Result<FullGpuRuntimePlasticityReport, ScaffoldContractError> {
    pollster::block_on(run_full_gpu_runtime_post_seal_plasticity_diagnostic_async(
        input,
    ))
}

pub fn full_gpu_runtime_live_plasticity_schema(
) -> Result<NeuralProjectionSchema, ScaffoldContractError> {
    live_static_schema(0.1)
}

pub fn post_seal_delta_batch_from_plasticity_report(
    patch: &ExperiencePatch,
    report: &FullGpuRuntimePlasticityReport,
) -> Result<PostSealLifetimeDeltaBatch, ScaffoldContractError> {
    patch.validate_contract()?;
    if !report.cpu_shadow_parity_passed
        || !report.genetic_fixed_unchanged
        || !report.lifetime_consolidated_unchanged
        || !report.h_operational_unchanged
        || report.h_shadow_delta_records.is_empty()
    {
        return Err(ScaffoldContractError::BackendParity);
    }
    PostSealLifetimeDeltaBatch::new(
        patch.header().organism_id,
        patch.pre_action().brain_class_id,
        patch.pre_action().brain_neuron_count,
        patch.pre_action().max_active_synapses,
        patch.header().world_tick,
        patch.header().sequence_id,
        PostSealLifetimeDeltaSourceKind::GpuCpuShadowGuarded,
        report.cpu_shadow_parity_passed,
        report.genetic_fixed_unchanged,
        report.lifetime_consolidated_unchanged,
        report.h_operational_unchanged,
        report.h_shadow_delta_records.clone(),
    )
}

async fn run_full_gpu_runtime_static_tick_async(
    input: FullGpuRuntimeStaticTickInput,
    mode: FullGpuRuntimeMode,
) -> Result<FullGpuRuntimeStaticTickReport, ScaffoldContractError> {
    input.validate()?;
    if mode == FullGpuRuntimeMode::CpuReference {
        return cpu_fallback_report(input, mode, GpuRuntimeFallbackReason::FeatureDisabled, None);
    }
    if env_flag_optional("ALIFE_GPU_RUNTIME_AVAILABLE") == Some(false) {
        return cpu_fallback_report(
            input,
            mode,
            GpuRuntimeFallbackReason::HardwareUnavailable,
            Some("ALIFE_GPU_RUNTIME_AVAILABLE=0 forced CPU fallback".to_string()),
        );
    }
    if env_flag_optional("ALIFE_GPU_RUNTIME_VALIDATED") == Some(false) {
        return cpu_fallback_report(
            input,
            mode,
            GpuRuntimeFallbackReason::ValidationFailed,
            Some("ALIFE_GPU_RUNTIME_VALIDATED=0 forced CPU fallback".to_string()),
        );
    }

    let instance = wgpu::Instance::default();
    let adapter = match instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
    {
        Ok(adapter) => adapter,
        Err(error) => {
            return cpu_fallback_report(
                input,
                mode,
                GpuRuntimeFallbackReason::HardwareUnavailable,
                Some(format!("wgpu adapter request failed: {error}")),
            );
        }
    };
    let info = adapter.get_info();
    let mut required_limits = wgpu::Limits::downlevel_defaults();
    required_limits.max_storage_buffers_per_shader_stage =
        required_limits.max_storage_buffers_per_shader_stage.max(10);
    let (device, queue) = match adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("alife-full-gpu-runtime-device"),
            required_features: wgpu::Features::empty(),
            required_limits,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        })
        .await
    {
        Ok(device) => device,
        Err(error) => {
            return cpu_fallback_report(
                input,
                mode,
                GpuRuntimeFallbackReason::HardwareUnavailable,
                Some(format!("wgpu device request failed: {error}")),
            );
        }
    };

    let mut backend = GpuRuntimeBackendConfig::request(mode.requested_backend())
        .with_gpu_feature_enabled(true)
        .with_hardware_available(true)
        .with_validation_passed(true)
        .select_backend()?;
    if backend.selected == GpuRuntimeBackendKind::CpuReference {
        return cpu_status_report(
            input,
            mode,
            backend,
            Some(format!(
                "{} ({:?}, {:?}, {})",
                info.name, info.backend, info.device_type, info.driver_info
            )),
            Some(
                "requested full static+routing+plasticity runtime is not currently supported; bounded static/plastic shadow evidence is available separately"
                    .to_string(),
            ),
        );
    }
    let upload_start = Instant::now();
    let plan = live_static_plan()?;
    let activation_q = live_activation_q(&plan, input)?;
    let action_summary_config = action_summary_config(input)?;
    let upload_ms = elapsed_ms(upload_start);

    let cpu_shadow_start = Instant::now();
    let cpu = plan.execute_cpu_diagnostic(&activation_q)?;
    let cpu_summary = action_summary_config.cpu_action_summary(&cpu.activations_q)?;
    let cpu_shadow_ms = elapsed_ms(cpu_shadow_start);

    let gpu = run_static_forward_gpu_action_summary_timed(
        &device,
        &queue,
        &plan,
        &activation_q,
        action_summary_config,
    )
    .await?;
    let parity = gpu.action_summary == cpu_summary;
    let parity_fallback_note = if parity {
        None
    } else {
        backend = GpuRuntimeBackendConfig::request(mode.requested_backend())
            .with_gpu_feature_enabled(true)
            .with_hardware_available(true)
            .with_validation_passed(false)
            .select_backend()?;
        Some(
            "GPU compact action summary failed CPU shadow parity; active tick used CPU proposals"
                .to_string(),
        )
    };
    let routing = routing_report(&plan);
    let readback = FullGpuRuntimeReadbackReport {
        compact_readback_bytes: gpu.compact_readback_bytes,
        action_summary_allowed: gpu.compact_readback_bytes == GPU_ACTION_SUMMARY_RECORD_BYTES,
        bulk_neural_readback_forbidden: true,
        per_synapse_readback_forbidden: true,
        per_lobe_readback_forbidden: true,
        weight_readback_forbidden: true,
    };
    let claim = if parity {
        match mode {
            FullGpuRuntimeMode::GpuStaticActionAuthoritative
            | FullGpuRuntimeMode::GpuStaticPlasticCpuShadowGuarded
            | FullGpuRuntimeMode::GpuFullActionAuthoritative => {
                FullGpuRuntimeProductClaim::CpuShadowGuarded
            }
            FullGpuRuntimeMode::GpuStaticShadow
            | FullGpuRuntimeMode::GpuStaticPlasticShadow
            | FullGpuRuntimeMode::GpuFullShadow => FullGpuRuntimeProductClaim::ShadowOnly,
            FullGpuRuntimeMode::CpuReference => FullGpuRuntimeProductClaim::None,
        }
    } else {
        FullGpuRuntimeProductClaim::None
    };
    Ok(FullGpuRuntimeStaticTickReport {
        schema_version: FULL_GPU_RUNTIME_SCHEMA_VERSION,
        mode,
        backend,
        hardware_identifier: Some(format!(
            "{} ({:?}, {:?}, {})",
            info.name, info.backend, info.device_type, info.driver_info
        )),
        action_summary: Some(gpu.action_summary),
        cpu_shadow_action_summary: Some(cpu_summary),
        cpu_shadow_parity_passed: parity,
        routing,
        readback,
        timing: FullGpuRuntimeTimingReport {
            upload_ms,
            gpu_submit_poll_ms: gpu.timing.submit_poll_wall_ms,
            compact_readback_ms: gpu.timing.compact_readback_wall_ms,
            cpu_shadow_ms,
            total_gpu_runtime_ms: upload_ms
                + gpu.timing.submit_poll_wall_ms
                + gpu.timing.compact_readback_wall_ms,
        },
        product_runtime_claim: claim,
        fallback_note: parity_fallback_note,
    })
}

async fn run_full_gpu_runtime_post_seal_plasticity_diagnostic_async(
    input: FullGpuRuntimeStaticTickInput,
) -> Result<FullGpuRuntimePlasticityReport, ScaffoldContractError> {
    input.validate()?;
    if env_flag_optional("ALIFE_GPU_RUNTIME_AVAILABLE") == Some(false) {
        return Ok(FullGpuRuntimePlasticityReport {
            schema_version: FULL_GPU_RUNTIME_SCHEMA_VERSION,
            diagnostic_only: true,
            post_seal_only: true,
            h_shadow_changed: false,
            updated_values_count: 0,
            max_delta_q: 0,
            saturation_count: 0,
            nan_or_inf_rejected: true,
            genetic_fixed_unchanged: true,
            lifetime_consolidated_unchanged: true,
            h_operational_unchanged: true,
            cpu_shadow_parity_passed: false,
            submit_poll_ms: 0.0,
            diagnostic_readback_ms: 0.0,
            diagnostic_readback_bytes: 0,
            h_shadow_delta_records: Vec::new(),
            live_core_update_applied: false,
        });
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
    let mut required_limits = wgpu::Limits::downlevel_defaults();
    required_limits.max_storage_buffers_per_shader_stage =
        required_limits.max_storage_buffers_per_shader_stage.max(10);
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("alife-full-gpu-runtime-plasticity-device"),
            required_features: wgpu::Features::empty(),
            required_limits,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|_| ScaffoldContractError::BackendParity)?;

    let static_plan = live_static_plan()?;
    let activation_q = live_activation_q(&static_plan, input)?;
    let cpu_static = static_plan.execute_cpu_diagnostic(&activation_q)?;
    let schema = live_static_schema(0.1)?;
    let plasticity_plan = live_plasticity_plan_from_schema(&schema)?;
    let cpu = plasticity_plan.execute_cpu_diagnostic(&activation_q, &cpu_static.activations_q)?;
    let gpu = run_plasticity_gpu_diagnostic_timed(
        &device,
        &queue,
        &plasticity_plan,
        &activation_q,
        &cpu_static.activations_q,
    )
    .await?;
    let mut updated_values_count = 0_u32;
    let mut max_delta_q = 0_i32;
    for (before, after) in plasticity_plan
        .h_shadow_initial_q
        .iter()
        .copied()
        .zip(gpu.result.h_shadow_q.iter().copied())
    {
        if before != after {
            updated_values_count = updated_values_count.saturating_add(1);
        }
        max_delta_q = max_delta_q.max((i32::from(after) - i32::from(before)).abs());
    }
    let h_shadow_delta_records = h_shadow_delta_records_from_schema(
        &schema,
        &plasticity_plan.h_shadow_initial_q,
        &gpu.result.h_shadow_q,
        plasticity_plan.policy,
        plasticity_plan.oja.to_oja_config(plasticity_plan.policy),
    )?;
    Ok(FullGpuRuntimePlasticityReport {
        schema_version: FULL_GPU_RUNTIME_SCHEMA_VERSION,
        diagnostic_only: false,
        post_seal_only: true,
        h_shadow_changed: gpu.result.h_shadow_q != plasticity_plan.h_shadow_initial_q,
        updated_values_count,
        max_delta_q,
        saturation_count: gpu.result.diagnostics.saturation_count,
        nan_or_inf_rejected: true,
        genetic_fixed_unchanged: gpu.result.genetic_fixed_q == plasticity_plan.genetic_fixed_q,
        lifetime_consolidated_unchanged: gpu.result.lifetime_consolidated_q
            == plasticity_plan.lifetime_consolidated_q,
        h_operational_unchanged: gpu.result.h_operational_q == plasticity_plan.h_operational_q,
        cpu_shadow_parity_passed: gpu.result.h_shadow_q == cpu.h_shadow_q
            && gpu.result.genetic_fixed_q == cpu.genetic_fixed_q
            && gpu.result.lifetime_consolidated_q == cpu.lifetime_consolidated_q
            && gpu.result.h_operational_q == cpu.h_operational_q
            && gpu.result.diagnostics == cpu.diagnostics,
        submit_poll_ms: gpu.timing.submit_poll_wall_ms,
        diagnostic_readback_ms: gpu.timing.readback_wall_ms,
        diagnostic_readback_bytes: plasticity_diagnostic_readback_bytes(&plasticity_plan),
        h_shadow_delta_records,
        live_core_update_applied: false,
    })
}

fn cpu_fallback_report(
    input: FullGpuRuntimeStaticTickInput,
    mode: FullGpuRuntimeMode,
    fallback_reason: GpuRuntimeFallbackReason,
    fallback_note: Option<String>,
) -> Result<FullGpuRuntimeStaticTickReport, ScaffoldContractError> {
    let backend = GpuRuntimeBackendConfig::request(mode.requested_backend())
        .with_gpu_feature_enabled(mode != FullGpuRuntimeMode::CpuReference)
        .with_hardware_available(false)
        .with_validation_passed(true)
        .select_backend()?;
    cpu_status_report(
        input,
        mode,
        backend,
        None,
        fallback_note.or_else(|| Some(format!("{fallback_reason:?}"))),
    )
}

fn cpu_status_report(
    input: FullGpuRuntimeStaticTickInput,
    mode: FullGpuRuntimeMode,
    backend: GpuRuntimeBackendStatus,
    hardware_identifier: Option<String>,
    fallback_note: Option<String>,
) -> Result<FullGpuRuntimeStaticTickReport, ScaffoldContractError> {
    let plan = live_static_plan()?;
    let activation_q = live_activation_q(&plan, input)?;
    let action_summary_config = action_summary_config(input)?;
    let cpu = plan.execute_cpu_diagnostic(&activation_q)?;
    let cpu_summary = action_summary_config.cpu_action_summary(&cpu.activations_q)?;
    Ok(FullGpuRuntimeStaticTickReport {
        schema_version: FULL_GPU_RUNTIME_SCHEMA_VERSION,
        mode,
        backend,
        hardware_identifier,
        action_summary: None,
        cpu_shadow_action_summary: Some(cpu_summary),
        cpu_shadow_parity_passed: false,
        routing: routing_report(&plan),
        readback: FullGpuRuntimeReadbackReport {
            compact_readback_bytes: 0,
            action_summary_allowed: true,
            bulk_neural_readback_forbidden: true,
            per_synapse_readback_forbidden: true,
            per_lobe_readback_forbidden: true,
            weight_readback_forbidden: true,
        },
        timing: FullGpuRuntimeTimingReport {
            upload_ms: 0.0,
            gpu_submit_poll_ms: 0.0,
            compact_readback_ms: 0.0,
            cpu_shadow_ms: 0.0,
            total_gpu_runtime_ms: 0.0,
        },
        product_runtime_claim: FullGpuRuntimeProductClaim::None,
        fallback_note,
    })
}

fn live_static_plan() -> Result<GpuStaticForwardPlan, ScaffoldContractError> {
    let policy = GpuFixedPointPolicy::reference();
    let upload = GpuUploadBuffers::from_cpu_schema(&live_static_schema(0.0)?, policy)?;
    GpuStaticForwardPlan::from_upload(&upload, policy)
}

fn live_plasticity_plan_from_schema(
    schema: &NeuralProjectionSchema,
) -> Result<GpuPlasticityPlan, ScaffoldContractError> {
    let policy = GpuFixedPointPolicy::reference();
    let upload = GpuUploadBuffers::from_cpu_schema(schema, policy)?;
    GpuPlasticityPlan::from_upload(
        &upload,
        policy,
        GpuOjaFixedPointConfig::from_oja_config(
            OjaUpdateConfig {
                learning_rate: 0.5,
                learning_rate_scale: 1.0,
                decay: 1.0,
                shadow_min: -1.0,
                shadow_max: 1.0,
            },
            policy,
            0xF00D,
        )?,
    )
}

fn plasticity_diagnostic_readback_bytes(plan: &GpuPlasticityPlan) -> usize {
    plan.h_shadow_initial_q
        .len()
        .saturating_mul(std::mem::size_of::<i32>())
        .saturating_add(
            (P26_PLASTICITY_DIAGNOSTIC_WORDS as usize).saturating_mul(std::mem::size_of::<u32>()),
        )
}

fn live_static_schema(h_shadow: f32) -> Result<NeuralProjectionSchema, ScaffoldContractError> {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&spec)?;
    schema.projections[0].tiles.push(ProjectionTile::new_coo(
        0,
        SparseTileCoord::new(0, 1)?,
        CooTile::new(vec![
            CooEntry::new(0, 0, weights(1.0, 0.0, 0.5, 0.0, h_shadow)?)?,
            CooEntry::new(1, 1, weights(1.0, 0.0, 0.5, 0.0, h_shadow)?)?,
            CooEntry::new(2, 2, weights(1.0, 0.0, 0.5, 0.0, h_shadow)?)?,
            CooEntry::new(3, 3, weights(1.0, 0.0, 0.5, 0.0, h_shadow)?)?,
        ])?,
    ));
    schema.rebuild_supertile_masks();
    Ok(schema)
}

fn live_activation_q(
    plan: &GpuStaticForwardPlan,
    input: FullGpuRuntimeStaticTickInput,
) -> Result<Vec<i32>, ScaffoldContractError> {
    let mut activations = vec![0.0; plan.header.neuron_count as usize];
    let saliences = input.saliences();
    activations[16..20].copy_from_slice(&saliences);
    plan.quantize_activations(&activations)
}

fn action_summary_config(
    input: FullGpuRuntimeStaticTickInput,
) -> Result<GpuStaticActionSummaryConfig, ScaffoldContractError> {
    input.validate()?;
    Ok(GpuStaticActionSummaryConfig {
        brain_slot: 0,
        action_count: 4,
        action_ids: input.action_ids,
        score_indices: [0, 1, 2, 3],
        confidence_q16: (input.confidence * u16::MAX as f32).round() as u32,
        drive_source_mask: input.drive_source_mask,
        motor_payload_ref: 0,
        flags: 0,
    })
}

fn routing_report(plan: &GpuStaticForwardPlan) -> FullGpuRuntimeRoutingReport {
    let counters = plan.routing_counters();
    FullGpuRuntimeRoutingReport {
        total_tiles: plan.tile_metadata.len() as u32,
        active_tiles: counters.active_tiles,
        skipped_tiles: counters.skipped_microtiles,
        active_synapses: counters.active_synapses,
        skipped_supertiles: counters.skipped_supertiles,
        routing_descriptors_evaluated: counters.routing_descriptors_evaluated,
        dispatch_level_culling_optimized: false,
    }
}

fn weights(
    genetic_fixed: f32,
    lifetime_consolidated: f32,
    alpha: f32,
    h_operational: f32,
    h_shadow: f32,
) -> Result<SynapseWeightSplit, ScaffoldContractError> {
    SynapseWeightSplit::new(
        genetic_fixed,
        lifetime_consolidated,
        alpha,
        h_operational,
        h_shadow,
    )
}

fn h_shadow_delta_records_from_schema(
    schema: &NeuralProjectionSchema,
    before_q: &[i16],
    after_q: &[i16],
    policy: GpuFixedPointPolicy,
    config: OjaUpdateConfig,
) -> Result<Vec<PostSealLifetimeDeltaRecord>, ScaffoldContractError> {
    if before_q.len() != after_q.len() {
        return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
    }
    let mut weight_index = 0_usize;
    let mut records = Vec::new();
    for projection in &schema.projections {
        for (tile_index, tile) in projection.tiles.iter().enumerate() {
            match &tile.payload {
                SparseTilePayload::Dense(dense) => {
                    for synapse_index in 0..dense.weights.len() {
                        push_delta_record(
                            &mut records,
                            projection.projection_index,
                            tile_index,
                            synapse_index,
                            weight_index,
                            before_q,
                            after_q,
                            policy,
                            config,
                        )?;
                        weight_index = weight_index.saturating_add(1);
                    }
                }
                SparseTilePayload::Coo(coo) => {
                    for synapse_index in 0..coo.entries.len() {
                        push_delta_record(
                            &mut records,
                            projection.projection_index,
                            tile_index,
                            synapse_index,
                            weight_index,
                            before_q,
                            after_q,
                            policy,
                            config,
                        )?;
                        weight_index = weight_index.saturating_add(1);
                    }
                }
                SparseTilePayload::RowRunUnsupported | SparseTilePayload::ColumnRunUnsupported => {
                    return Err(ScaffoldContractError::UnsupportedSparseTileFormat);
                }
            }
        }
    }
    if weight_index != before_q.len() {
        return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
    }
    Ok(records)
}

#[allow(clippy::too_many_arguments)]
fn push_delta_record(
    records: &mut Vec<PostSealLifetimeDeltaRecord>,
    projection_index: u32,
    tile_index: usize,
    synapse_index: usize,
    weight_index: usize,
    before_q: &[i16],
    after_q: &[i16],
    policy: GpuFixedPointPolicy,
    config: OjaUpdateConfig,
) -> Result<(), ScaffoldContractError> {
    let before = *before_q
        .get(weight_index)
        .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
    let after = *after_q
        .get(weight_index)
        .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
    if before == after {
        return Ok(());
    }
    records.push(PostSealLifetimeDeltaRecord::h_shadow(
        PostSealHShadowDeltaTarget::new(projection_index, tile_index as u32, synapse_index as u16),
        dequantize_weight(before, policy),
        dequantize_weight(after, policy),
        config.shadow_min,
        config.shadow_max,
    )?);
    Ok(())
}

fn dequantize_weight(value: i16, policy: GpuFixedPointPolicy) -> f32 {
    f32::from(value) / policy.weight_scale as f32
}

fn env_flag_optional(name: &str) -> Option<bool> {
    std::env::var(name).ok().map(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn elapsed_ms(start: Instant) -> f32 {
    start.elapsed().as_secs_f64().mul_add(1000.0, 0.0) as f32
}
