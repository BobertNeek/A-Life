# Local GPU Bring-up Report

Status: local GPU adapter/device probe working; diagnostic GPU timing is recorded separately in `docs/productization/LOCAL_GPU_TIMING_EVIDENCE_REPORT.md`.

Branch: `codex/local-gpu-bringup`

## Scope

This pass diagnosed the local Windows GPU path for A-Life without changing the
default headless CPU path, requiring GPU in CI, tagging a release, or creating a
new roadmap plan. The CPU reference remains the correctness oracle and GPU
runtime evidence remains diagnostics/performance-report scoped.

## Local Hardware Summary

| Field | Result |
| --- | --- |
| OS | Windows 10 Home 10.0.19045, 64-bit |
| CPU | Intel Core i7-3770K, 4 cores / 8 logical processors |
| RAM | About 32 GiB visible to Windows |
| Display adapter | NVIDIA GeForce RTX 3050 |
| Windows driver | 32.0.15.8180 |
| NVIDIA-SMI | 581.80, CUDA 13.0, WDDM |
| DirectX | DirectX 12 reported by `dxdiag` |
| Vulkan | Vulkan instance 1.4.309; adapter API 1.4.312 |
| Vulkan driver | NVIDIA proprietary, driver info 581.80 |
| Hybrid GPU | `dxdiag` reports Microsoft Graphics Hybrid: Not Supported |

Local environment artifacts were written under `target/gpu_bringup/` and were
not committed.

## Runtime Path Findings

Before this bring-up, `benchmark_tiers --gpu-runtime` did not perform hardware
probing. It selected `GpuStatic` only when `ALIFE_GPU_RUNTIME_AVAILABLE=1` and
`ALIFE_GPU_RUNTIME_VALIDATED=1` were set, while the generated report still
listed `Hardware: unknown`. That meant environment flags could simulate backend
selection and were not valid GPU hardware evidence.

This pass adds a narrow local wgpu adapter/device probe in `alife_gpu_backend`.
The benchmark CLI now uses that probe for `--gpu-runtime` reports:

- adapter request uses high-performance, surface-free wgpu selection.
- device request validates the storage-buffer limit needed by the requested
  A-Life GPU backend tier.
- env flags may still request backend kind or explicitly disable/force fallback
  behavior, but they no longer prove hardware availability by themselves.
- the report records adapter name, backend API, adapter type, driver info, and
  storage-buffer limit evidence when available.

## Commands And Results

### Baseline Validation

All baseline validation commands passed before the probe change:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo test --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`
- `cargo tree -p alife_core`
- `cargo check --workspace --all-features --all-targets`
- `cargo test --workspace --all-features --all-targets`

### Existing GPU Commands Before Fix

Default command:

```powershell
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

Result before fix: exited successfully but selected `CpuReference` with
`HardwareUnavailable`; hardware was reported as `unknown`.

Env-flag command:

```powershell
$env:ALIFE_GPU_RUNTIME_BACKEND="static"
$env:ALIFE_GPU_RUNTIME_FEATURE="1"
$env:ALIFE_GPU_RUNTIME_AVAILABLE="1"
$env:ALIFE_GPU_RUNTIME_VALIDATED="1"
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

Result before fix: selected `GpuStatic`, but hardware was still reported as
`unknown`. This was env-flag simulation, not real adapter evidence.

### Existing Manual GPU Parity Tests

These ignored/manual tests passed on the local RTX 3050:

```powershell
cargo test -p alife_gpu_backend --features gpu-tests -- --ignored --nocapture
cargo test -p alife_gpu_backend --features gpu-tests --test static_forward_parity -- --ignored --nocapture
cargo test -p alife_gpu_backend --features gpu-tests --test plasticity_oja_parity -- --ignored --nocapture
```

The static forward and plasticity diagnostics requested a real wgpu adapter and
device and matched their CPU diagnostic fixtures.

### GPU Runtime Command After Fix

```powershell
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

Result after fix:

- backend requested: `GpuStatic`
- backend selected: `GpuStatic`
- fallback reason: `None`
- hardware: `NVIDIA GeForce RTX 3050 (Vulkan, DiscreteGpu, 581.80)`
- storage buffers per shader stage: `524288`
- no active gameplay neural readback: `true`
- GPU neural time: `unknown`

The report still copies P20 CPU smoke timings for population tiers 1 and 10.
Those timings are not GPU neural performance measurements.

## Actual GPU Hardware Used

Yes for adapter/device bring-up and ignored diagnostic parity tests.

Yes for bounded diagnostic GPU timing in the post-bring-up timing report.

No for product gameplay GPU performance timing. The P25/P26 timing report uses
manual diagnostic fixtures, so the 60 FPS gameplay target remains unclaimed.

## CPU Fallback Status

CPU fallback remains available:

- the default non-`--gpu-runtime` benchmark path remains CPU-only.
- `alife_core` remains dependency-clean and does not depend on wgpu.
- GPU runtime selection remains optional and report-scoped.
- setting unavailable/failed validation conditions still falls back to
  `CpuReference`; `ALIFE_GPU_RUNTIME_AVAILABLE=0` was verified to force
  `HardwareUnavailable` even when the local adapter probe succeeds.

## Blockers Resolved

- `benchmark_tiers --gpu-runtime` no longer reports GPU selection based only on
  environment flags.
- the report now records real local adapter/backend evidence.

## Remaining Limitations

- Product gameplay GPU neural runtime timing is still unknown.
- Local P25/P26 diagnostic GPU timing is now recorded separately in
  `docs/productization/LOCAL_GPU_TIMING_EVIDENCE_REPORT.md`.
- P25/P26 ignored GPU parity tests are diagnostic/manual evidence and do not
  prove product WebGPU portability.
- The benchmark report still reuses CPU smoke metrics for tier rows.
- Full product GPU performance and 60 FPS evidence remain future manual
  measurement work.

## Recommendation

Use `cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime` as the
local GPU adapter bring-up command. Treat `GpuStatic` selection plus adapter
identity as local GPU availability evidence, not as GPU performance evidence.
Run the ignored P25/P26 parity tests when validating shader diagnostics on this
machine.
