# Validation Protocol

Run these for every implementation branch before merge and again on merged `main`.

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
```

Run all-features validation for any plan touching features, app, Bevy, GPU, semantic, school, packaging, or release gates:

```powershell
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

Run focused commands listed in the plan.

## Visual commands

Graphical smoke:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Forced fallback smoke:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

## GPU manual evidence

Use only when the plan asks:

```powershell
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime --measure-gpu
cargo test -p alife_gpu_backend --features gpu-tests --test static_forward_parity -- --ignored --nocapture
cargo test -p alife_gpu_backend --features gpu-tests --test plasticity_oja_parity -- --ignored --nocapture
```

## Result discipline

- A command not run is `not run`.
- A dry-run is not graphical evidence.
- CPU fallback is not GPU evidence.
- Local RTX 3050/Vulkan timing is local evidence only.
- Manual/hardware unavailable must be recorded honestly.
