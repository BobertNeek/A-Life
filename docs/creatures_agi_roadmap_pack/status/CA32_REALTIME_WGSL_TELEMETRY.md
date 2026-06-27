# CA32 Real-time WGSL Telemetry

## Status

Complete on branch `codex/CA32-real-time-wgsl-telemetry-in-app`.

CA32 adds a compact app-visible telemetry surface for active WGSL GPU ticks. It
does not change action authority, fallback policy, or `alife_core` boundaries.

## What Changed

- Added `alife.ca32.realtime_wgsl_telemetry.v1` as the app schema marker for
  active WGSL telemetry summaries.
- Added `RealtimeWgslTelemetrySummary` to the graphical GPU telemetry surface.
- Added a compact UI/inspector/profiler line with:
  - tick marker,
  - upload timing,
  - GPU submit/poll compute timing,
  - compact readback timing,
  - CPU shadow timing,
  - active/skipped tile counters,
  - active synapse counters,
  - compact readback bytes.
- Added `realtime-wgsl-telemetry-smoke <fixture-root>` as a product/developer
  smoke command.
- Extended the CA30 neural activity profiler to display the WGSL timing split
  and route counters without adding raw neural readback.

## Evidence Boundary

The timing is host-observed runtime telemetry derived from the existing GPU
runtime reports. It is local/manual hardware evidence when a GPU is selected and
explicitly unavailable/fallback evidence when the GPU path cannot run.

CA32 does not introduce timestamp-query timing, full action-authoritative GPU
runtime, or bulk neural tensor readback.

## Invariants

- CPU shadow parity remains the correctness gate.
- CPU fallback remains available and explicitly reported.
- The product runtime claim remains bounded; CA32 does not claim full
  action-authoritative GPU runtime.
- Telemetry is summary-only and nonblocking. If timing is unavailable, the UI
  reports the unavailable reason instead of blocking the hot path.
- `alife_core` remains engine-independent and receives no Bevy, wgpu, renderer,
  model-runtime, or app dependency.
- No screenshots, logs, target artifacts, model files, `S12`, `G25`, `P37`, or
  release tag are created by this plan.

## Focused Evidence

Planned/required CA32 focused commands:

```powershell
cargo test -p alife_game_app --test app_shell ca32_realtime_wgsl_telemetry_exposes_timing_split_and_routing_counters -- --nocapture
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- realtime-wgsl-telemetry-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime --measure-gpu
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

## Known Limitations

- Timing is host-observed split telemetry, not hardware timestamp-query timing.
- Local GPU timing remains local evidence for this machine only.
- CA32 surfaces per-tick telemetry for the existing combined static/plastic CPU
  shadow guarded path; it does not batch multiple creatures or graduate CPU
  shadow gating. Those are later Phase H plans.
