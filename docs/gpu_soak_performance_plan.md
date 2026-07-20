# P36 GPU Soak And Performance Plan

Status: manual GPU release-gate plan.

Production neural causality is GPU-authoritative. GPU checks in this file are
behavioral, causal, and performance-report gates for machines with a local wgpu
adapter. If hardware is unavailable, record the neural gate as blocked or
unknown; do not substitute a CPU neural run or claim a neural tick occurred.

## Preconditions

- Run the full default validation suite first.
- Use a graphics/GPU-capable machine with current drivers.
- Keep active gameplay free of synchronous bulk neural, per-synapse, per-lobe,
  or weight readback.
- Write reports under `target/artifacts/`.

## Contract Commands

```powershell
cargo test -p alife_gpu_backend --test gpu_buffer_contracts
cargo test -p alife_gpu_backend --test static_forward_parity
cargo test -p alife_gpu_backend --test supertile_routing_masks
cargo test -p alife_gpu_backend --test recompaction_autophagy
cargo test -p alife_gpu_backend --test gpu_runtime_performance
cargo test -p alife_gpu_backend --test closed_loop_learning_buffers
cargo test -p alife_gpu_backend --test closed_loop_wgsl
```

Manual hardware-backed checks:

```powershell
cargo test -p alife_gpu_backend --features gpu-tests --test static_forward_parity -- --ignored --nocapture
cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_gpu_behavior -j 1 -- --nocapture
cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_fast_plasticity -j 1 -- --nocapture
```

## Runtime And Performance Report

Runtime report:

```powershell
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

Validated GPU report after hardware causal gates pass:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE='1'
$env:ALIFE_GPU_RUNTIME_VALIDATED='1'
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
Remove-Item Env:ALIFE_GPU_RUNTIME_AVAILABLE
Remove-Item Env:ALIFE_GPU_RUNTIME_VALIDATED
```

Manual full-tier report:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE='1'
$env:ALIFE_GPU_RUNTIME_VALIDATED='1'
cargo run -p alife_tools --bin benchmark_tiers -- --all --gpu-runtime
Remove-Item Env:ALIFE_GPU_RUNTIME_AVAILABLE
Remove-Item Env:ALIFE_GPU_RUNTIME_VALIDATED
```

Record:

- OS, GPU model, driver version, and adapter name when available.
- Feature flags and environment variables used.
- closed-loop selection, eligibility, fast-plasticity, and sleep evidence status.
- requested adapter and typed GPU-unavailable reason, if any.
- Tick time, GPU neural time, skipped tile counters, and 60 FPS target status
  exactly as reported.
- Bottlenecks and unknown values. Unknown is preferable to fabricated data.

## Manual Status For This Repository Snapshot

Headless contract checks may run without GPU hardware only when they explicitly
select `HeuristicBaseline`. Neural release and promotion gates require current
real-hardware evidence from the GPU-authoritative runtime.
