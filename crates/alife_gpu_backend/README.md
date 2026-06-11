# alife_gpu_backend

wgpu/WebGPU backend boundary for sparse neural compute.

This crate owns GPU resource planning, buffer/shader manifests, dispatch interfaces, and later WGSL integration. It must not define cognitive truth that is absent from the CPU reference path, and it must not add neural runtime kernels before the gated GPU plans.
