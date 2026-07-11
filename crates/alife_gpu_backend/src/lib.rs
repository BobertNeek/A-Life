//! v0 scaffold: wgpu backend contracts and placeholders only.

use alife_core::NeuralComputeBackend;

pub mod buffers;
pub mod closed_loop_buffers;
pub mod full_runtime;
pub mod plasticity;
pub mod recompaction;
pub mod routing_masks;
pub mod runtime;
pub mod shader_contract;
pub mod static_forward;
pub mod timing;

pub use buffers::{
    GpuAccumulatorLayout, GpuActionSummaryStagingRecord, GpuActivationPingPongViews,
    GpuBufferContractHeader, GpuBufferView, GpuDiagnosticCountersRecord, GpuFixedPointPolicy,
    GpuPackedSynapseIndexRecord, GpuReadbackClass, GpuReadbackPolicy, GpuRoutingDescriptorRecord,
    GpuSupertileMaskRecord, GpuTileMetadataRecord, GpuUploadBuffers, GpuWeightBufferViews,
    WeightBufferFormat, GPU_ACTION_SUMMARY_RECORD_BYTES, GPU_BUFFER_CONTRACT_SCHEMA_VERSION,
    GPU_DIAGNOSTIC_COUNTER_BYTES, GPU_HEADER_BYTES, GPU_PACKED_SYNAPSE_INDEX_BYTES,
    GPU_ROUTING_DESCRIPTOR_BYTES, GPU_SERIALIZATION_ENDIANNESS, GPU_SUPERTILE_MASK_BYTES,
    GPU_TILE_METADATA_BYTES,
};
pub use closed_loop_buffers::*;
pub use full_runtime::{
    full_gpu_runtime_live_plasticity_schema, post_seal_delta_batch_from_plasticity_report,
    run_full_gpu_runtime_post_seal_plasticity_diagnostic, run_full_gpu_runtime_static_tick,
    FullGpuRuntimeBackendReport, FullGpuRuntimeMode, FullGpuRuntimePlasticityReport,
    FullGpuRuntimeProductClaim, FullGpuRuntimeReadbackReport, FullGpuRuntimeRoutingReport,
    FullGpuRuntimeSession, FullGpuRuntimeStaticTickInput, FullGpuRuntimeStaticTickReport,
    FullGpuRuntimeTimingReport, FULL_GPU_RUNTIME_SCHEMA_VERSION,
};
pub use plasticity::{
    run_plasticity_gpu_diagnostic, run_plasticity_gpu_diagnostic_timed, GpuOjaFixedPointConfig,
    GpuPlasticityDiagnostics, GpuPlasticityDispatch, GpuPlasticityPlan, GpuPlasticityResult,
    GpuPlasticityTimedResult, GpuPlasticityTiming, P26_PLASTICITY_DIAGNOSTIC_WORDS,
    P26_PLASTICITY_TOLERANCE_Q, P26_PLASTICITY_WORKGROUP_SIZE, P26_WGSL_PLASTICITY,
};
pub use recompaction::{
    GpuAffectedTileRef, GpuAutophagyMarker, GpuAutophagyMarkerKind, GpuAutophagyPolicy,
    GpuBufferReplacement, GpuLogicalBufferRef, GpuRecompactionDiagnostics, GpuRecompactionOutput,
    GpuRecompactionPlan, GpuRecompactionRemapTable, GpuRecompactionSwapState,
    GpuRecompactionValidationStatus, GpuRoutingMaskPreservation, GpuStructuralEditPlanEntry,
    GpuStructuralEditStatus, GPU_RECOMPACTION_SCHEMA_VERSION, P28_WGSL_RECOMPACTION_AUTOPHAGY,
};
pub use routing_masks::{
    p27_routing_counters, p27_tile_is_active, GpuActiveTileMaskConfig, GpuRoutingCounters,
    GpuRoutingMaskPlan, GpuSupertileIndex, GpuSupertileMaskWords, P27_MICROTILE_EDGE,
    P27_PLASTICITY_STORAGE_BINDINGS, P27_STATIC_FORWARD_STORAGE_BINDINGS, P27_SUPERTILE_EDGE,
    P27_SUPERTILE_MASK_WORDS, P27_SUPERTILE_MICROTILES, P27_WGSL_SUPERTILE_ROUTING,
};
pub use runtime::{
    probe_local_wgpu_runtime, probe_local_wgpu_runtime_for_graphics_backend,
    probe_local_wgpu_runtime_with_backends, required_storage_buffers, GpuPerformanceTargetStatus,
    GpuRuntimeBackendConfig, GpuRuntimeBackendKind, GpuRuntimeBackendStatus, GpuRuntimeBoundary,
    GpuRuntimeCapabilityManifest, GpuRuntimeDiagnosticExport, GpuRuntimeFallbackReason,
    GpuRuntimeHardwareProbe, GpuRuntimeReadbackGuard, GpuRuntimeThrottleDecision,
    GpuRuntimeThrottlingPolicy, GpuRuntimeTimingBudget, GpuRuntimeTimingSample, GpuThrottleLevel,
    GpuThrottleReason, GpuTierMeasurement, GpuTierPerformanceReport, GpuTierPopulation,
    P29_RUNTIME_SCHEMA_VERSION,
};
pub use shader_contract::{GpuShaderPass, P24_WGSL_CONTRACT_STUB};
pub use static_forward::{
    finalize_static_forward_accumulators_for_diagnostics,
    run_static_forward_gpu_action_summary_timed, run_static_forward_gpu_diagnostic,
    run_static_forward_gpu_diagnostic_timed, GpuStaticActionSummaryConfig,
    GpuStaticActionSummaryTimedResult, GpuStaticActionSummaryTiming, GpuStaticForwardDiagnostics,
    GpuStaticForwardDispatch, GpuStaticForwardPlan, GpuStaticForwardResult,
    GpuStaticForwardTimedResult, GpuStaticForwardTiming, P25_DIAGNOSTIC_COUNTER_WORDS,
    P25_STATIC_FORWARD_TOLERANCE_ABS, P25_STATIC_FORWARD_WORKGROUP_SIZE, P25_WGSL_ACTION_SUMMARY,
    P25_WGSL_STATIC_FORWARD,
};
pub use timing::{
    run_local_gpu_diagnostic_timing, GpuDiagnosticProductRuntimeClaim, GpuDiagnosticTimingKind,
    GpuDiagnosticTimingReport, GpuDiagnosticWorkloadTiming, GpuTimingTargetStatus,
    GPU_DIAGNOSTIC_TIMING_SCHEMA_VERSION,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderSourceLanguage {
    Wgsl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuBackendManifest {
    pub shader_language: ShaderSourceLanguage,
    pub runtime_neural_kernels_implemented: bool,
}

impl GpuBackendManifest {
    pub const SCAFFOLD: Self = Self {
        shader_language: ShaderSourceLanguage::Wgsl,
        runtime_neural_kernels_implemented: false,
    };

    pub const STATIC_FORWARD_PARITY: Self = Self {
        shader_language: ShaderSourceLanguage::Wgsl,
        runtime_neural_kernels_implemented: true,
    };
}

#[derive(Debug, Default)]
pub struct WgpuScaffoldBackend;

impl NeuralComputeBackend for WgpuScaffoldBackend {
    fn backend_name(&self) -> &'static str {
        "wgpu-scaffold"
    }
}

pub type WgpuLimits = wgpu::Limits;
