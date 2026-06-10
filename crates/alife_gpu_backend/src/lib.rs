//! v0 scaffold: wgpu backend contracts and placeholders only.

use alife_core::NeuralComputeBackend;

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
