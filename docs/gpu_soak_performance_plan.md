# P36 GPU Soak And Performance Plan

Status: manual GPU release-gate plan.

The CPU reference path remains the correctness oracle. GPU checks in this file
are diagnostics, parity, and performance-report gates for machines with a local
wgpu adapter. If hardware is unavailable, record the GPU status as manual or
unknown. Do not infer GPU product readiness from CPU fallback reports.

## Preconditions

- Run the full default validation suite first.
- Use a graphics/GPU-capable machine with current drivers.
- Keep active gameplay free of synchronous bulk neural, per-synapse, per-lobe,
  or weight readback.
- Write reports under `target/artifacts/`.

## Parity Commands

```powershell
cargo test -p alife_gpu_backend --test gpu_buffer_contracts
cargo test -p alife_gpu_backend --test static_forward_parity
cargo test -p alife_gpu_backend --test plasticity_oja_parity
cargo test -p alife_gpu_backend --test supertile_routing_masks
cargo test -p alife_gpu_backend --test recompaction_autophagy
cargo test -p alife_gpu_backend --test gpu_runtime_performance
```

Manual hardware-backed checks:

```powershell
cargo test -p alife_gpu_backend --features gpu-tests --test static_forward_parity -- --ignored --nocapture
cargo test -p alife_gpu_backend --features gpu-tests --test plasticity_oja_parity -- --ignored --nocapture
cargo test -p alife_gpu_backend --test plasticity_oja_parity -- --ignored --nocapture
```

## Runtime And Performance Report

Default CPU-fallback report:

```powershell
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

Validated GPU report after hardware parity passes:

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
- P25/P26/P27/P28 parity status.
- P29 backend selected and fallback reason, if any.
- Tick time, GPU neural time, skipped tile counters, and 60 FPS target status
  exactly as reported.
- Bottlenecks and unknown values. Unknown is preferable to fabricated data.

## Manual Status For This Repository Snapshot

The CI/default release gate does not require GPU hardware. GPU runtime maturity
is limited to schema, parity, diagnostic, and fallback contracts unless manual
hardware evidence is recorded for the current release candidate.
