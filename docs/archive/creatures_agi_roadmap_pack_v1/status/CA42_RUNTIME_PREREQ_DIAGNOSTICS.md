# CA42 - Runtime Prerequisite and GPU Diagnostics Launcher

## Summary

CA42 adds a launch preflight for the GPU-first graphical alpha. The preflight
probes the requested GPU runtime path when available, prints adapter/backend
diagnostics, records a log path, and makes CPU fallback or `RequireGpu` blocking
explicit before the graphical window opens.

The preflight is app/tooling behavior only. It does not change simulation
semantics, action authority, CPU shadow parity, fallback rules, or core
contracts.

## Commands

Run the preflight directly:

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- runtime-prereq-smoke --gpu-mode static-plastic-cpu-shadow-guarded --graphics-backend dx12 --log target/artifacts/ca42_runtime_prereq/runtime_prereq.log
```

Run the normal graphical launcher with preflight:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Run the explicit GPU-required path:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded -RequireGpu
```

Run the forced fallback path:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

## Behavior

- The repository launcher runs `runtime-prereq-smoke` before
  `graphical-playground`.
- The package-local runner runs the same preflight through `alife_game_app.exe`
  before opening the packaged graphical app.
- The repository preflight log path is
  `target/artifacts/ca42_runtime_prereq/runtime_prereq.log`.
- The packaged preflight log path is `diagnostics/runtime_prereq.log` inside
  the unpacked package.
- Missing or disabled GPU support reports a typed fallback reason.
- `-RequireGpu` turns fallback into a clear preflight failure.
- Without `-RequireGpu`, CPU fallback remains available but is labeled as a
  degraded/safety path.

## Invariants

- CPU fallback remains available.
- CPU shadow parity remains the correctness gate.
- Product runtime claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- Full action-authoritative GPU runtime is not claimed.
- No Bevy, wgpu, GPU, or model-runtime dependency is added to `alife_core`.
- No release tag is created.
- No screenshots, logs, target artifacts, model files, cache files, or
  generated captures are tracked.

## Focused Evidence

CA42 focused checks:

```powershell
cargo test -p alife_game_app --test app_shell ca42 -- --nocapture
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- runtime-prereq-smoke --gpu-mode static-plastic-cpu-shadow-guarded --graphics-backend dx12 --log target/artifacts/ca42_runtime_prereq/runtime_prereq.log
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- runtime-prereq-smoke --gpu-mode static-plastic-cpu-shadow-guarded --graphics-backend dx12 --require-gpu --log target/artifacts/ca42_runtime_prereq/runtime_prereq_require_gpu.log
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- runtime-prereq-smoke --gpu-mode static-plastic-cpu-shadow-guarded --graphics-backend dx12 --log target/artifacts/ca42_runtime_prereq/runtime_prereq_forced_fallback.log
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun -GpuMode static-plastic-cpu-shadow-guarded
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_windows_alpha_package.ps1 -DryRun -GpuMode static-plastic-cpu-shadow-guarded
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
```

Observed results before full validation:

- CA42 app-shell tests passed.
- Direct preflight with `--graphics-backend dx12` selected
  `NVIDIA GeForce RTX 3050 api=Dx12 driver=32.0.15.8180`.
- `--require-gpu` passed on this machine because the DX12 GPU probe succeeded.
- Forced `ALIFE_GPU_RUNTIME_AVAILABLE=0` selected `CpuReference` with
  `HardwareUnavailable`, `degraded_visible=true`, and no GPU performance claim.
- Repository launcher dry-run printed the preflight command and log path before
  the graphical command.
- Package runner dry-run printed the package-local preflight command and
  `diagnostics/runtime_prereq.log`.
- `platform-package-smoke` passed with eight commands including
  `ca42-runtime-prereq-smoke`.
- 30-second graphical smoke passed through the CA42 preflight.
- Forced-fallback 10-second graphical smoke passed and reported degraded CPU
  fallback.

## Known Limitations

- The preflight can report local hardware/driver state, but it does not replace
  graphical smoke or human tester evidence.
- CPU fallback is still a supported degraded path unless `-RequireGpu` is used.
- Local adapter details are machine-specific and must not be generalized as
  public GPU performance evidence.

## Receipt Fields

- Plan: CA42
- Branch: `codex/CA42-runtime-prerequisite-gpu-diagnostics-launcher`
- Runtime code changed: app/tooling runtime preflight only; no simulation
  semantics changed.
- Core APIs changed: no.
- Docs changed: yes.
- Public APIs changed: new app command `runtime-prereq-smoke`; launcher scripts
  now run preflight and accept `-RequireGpu`.
- Tests added/changed: CA42 app-shell preflight and script wiring tests.
- Focused evidence: CA42 app-shell tests, direct DX12 runtime preflight,
  `RequireGpu` positive preflight, forced CPU fallback preflight, launcher
  dry-runs, platform-package smoke, 30-second graphical smoke, and forced
  fallback graphical smoke passed.
- Validation results: full branch validation passed:
  `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`,
  `cargo test --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1`,
  `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`,
  `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`,
  `cargo tree -p alife_core`,
  `cargo check --workspace --all-features --all-targets`, and
  `cargo test --workspace --all-features --all-targets`.
- Invariant checks: no S12/G25/P37, no release tag, no tracked generated
  artifacts, no `alife_core` dependency leak, no full action-authoritative GPU
  claim, CPU fallback preserved, CPU shadow parity preserved.
- Main status: pending merge.
- Next plan: CA43.
