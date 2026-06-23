# Local GPU Timing Evidence Report

Status: local diagnostic GPU timing recorded for P25/P26 parity workloads; a separate full GPU runtime smoke now records CPU-shadow-guarded static product timing, while full plastic gameplay GPU timing remains unclaimed.

Branch: `codex/local-gpu-timing-evidence`

## Scope

This pass added and ran a bounded diagnostic timing path for existing GPU
parity workloads. It does not make GPU mandatory, does not change gameplay
behavior, does not add active gameplay neural readback, and does not move GPU
dependencies into `alife_core`.

The evidence is diagnostic/manual GPU timing, not a release or marketing claim
about product gameplay frame rate.

## Local Hardware

| Field | Result |
| --- | --- |
| Adapter | NVIDIA GeForce RTX 3050 |
| Backend API | Vulkan |
| Adapter type | DiscreteGpu |
| Driver | 581.80 |
| Timestamp query support | Supported by adapter, not used by this timing path |
| Timing method used | Host-observed diagnostic submit/poll and readback wall timing |

## Command

```powershell
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime --measure-gpu
```

Output artifacts were written under `target/artifacts/` and were not committed:

- `target/artifacts/benchmark_tiers.md`
- `target/artifacts/gpu_runtime_performance.md`
- `target/artifacts/local_gpu_timing_evidence.md`

## Measured Workloads

| Workload | Dimensions | Warmup | Measured | CPU mean ms | GPU submit/poll mean ms | Readback mean ms | GPU total mean ms | Parity | 60 FPS target |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| P25 static forward diagnostic fixture | neurons=512, tiles=2, synapses=258, dispatch=(9,5,8) | 3 | 10 | 0.0808 | 1.1844 | 1.2950 | 2.4794 | pass | Not applicable |
| P26 plasticity/Oja diagnostic fixture | neurons=512, tiles=1, synapses=1, dispatch=1 | 3 | 10 | 0.0120 | 0.9010 | 0.6352 | 1.5362 | pass | Not applicable |

## Evidence Boundary

- Timing kind: `HostObservedDiagnostic`.
- Product runtime claim: `None`.
- Per-workload claim: `DiagnosticOnly`.
- Diagnostic readback timing is reported separately from submit/poll timing.
- The readback is manual parity evidence and is not exposed as an active
  gameplay tick API.
- The 60 FPS gameplay target is not applicable to these tiny diagnostic
  fixtures.
- CPU fallback remains available and default/headless paths do not require GPU.

## Interpretation

The local RTX 3050/Vulkan path can execute the existing P25 static forward and
P26 plasticity diagnostic workloads on real hardware and match their CPU
diagnostic references. This improves local GPU evidence from adapter/device
bring-up to measured diagnostic workload timing.

This does not prove full plastic gameplay GPU performance, full WebGPU
portability, or a 60 FPS gameplay target. `FULL_GPU_NEURAL_RUNTIME_REPORT.md`
adds live-tick static action-score timing with compact readback; live H_shadow
application remains a gap until a future core-owned post-seal lifetime-state
hook exists.

## Next Recommendation

Keep using:

```powershell
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime --measure-gpu
```

for local diagnostic GPU timing evidence. Treat the generated timing report as
manual diagnostic evidence; use `FULL_GPU_NEURAL_RUNTIME_REPORT.md` for the
separate static product-smoke timing evidence.
