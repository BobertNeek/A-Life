//! v0 scaffold: wgpu backend contracts and placeholders only.

use alife_core::NeuralComputeBackend;

pub mod buffers;
pub mod shader_contract;

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
pub use shader_contract::{GpuShaderPass, P24_WGSL_CONTRACT_STUB};

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
}

#[derive(Debug, Default)]
pub struct WgpuScaffoldBackend;

impl NeuralComputeBackend for WgpuScaffoldBackend {
    fn backend_name(&self) -> &'static str {
        "wgpu-scaffold"
    }
}

pub type WgpuLimits = wgpu::Limits;
