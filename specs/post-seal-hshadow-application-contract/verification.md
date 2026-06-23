# Post-Seal H_shadow Application Verification

Verification commands are recorded here as they run.

## Focused Checks

- `cargo test -p alife_core --test post_seal_lifetime_deltas -- --nocapture`
  - Result: pass, 11 tests, including direct unsealed-patch rejection.
- `cargo test -p alife_game_app --test app_shell full_gpu -- --nocapture`
  - Result: pass, 3 focused tests.
- `cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- full-gpu-runtime-smoke crates/alife_world/tests/fixtures/p34 --mode static-action-authoritative --ticks 3`
  - Result: pass; selected `GpuStatic` on NVIDIA GeForce RTX 3050/Vulkan, fallback `None`, CPU shadow parity true, compact readback 64 bytes.
- `cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- full-gpu-runtime-smoke crates/alife_world/tests/fixtures/p34 --mode static-plastic-shadow --ticks 3`
  - Result: pass; selected `GpuPlastic`, applied 2 post-seal H_shadow delta records, preserved `W_genetic_fixed`, lifetime-consolidated weights, and H_operational.
- Forced fallback with `ALIFE_GPU_RUNTIME_AVAILABLE=0` and static-plastic shadow smoke.
  - Result: pass; selected `CpuReference`, fallback `HardwareUnavailable`, sealed patches still produced.

## Full Validation

- `cargo fmt --all -- --check`
  - Result: pass after final code edit.
- `cargo check --workspace --all-targets`
  - Result: pass after final code edit.
- `cargo test --workspace --all-targets`
  - Result: pass after final code edit.
- `cargo clippy --workspace --all-targets -- -D warnings`
  - Result: pass after replacing a single-element loop in summary validation.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1`
  - Result: pass; wrapper used `C:\Program Files\Git\bin\bash.exe`.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`
  - Result: pass; wrapper used `C:\Program Files\Git\bin\bash.exe`.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`
  - Result: pass; wrapper used `C:\Program Files\Git\bin\bash.exe`.
- `cargo tree -p alife_core`
  - Result: pass; no Bevy/wgpu/GPU/tooling dependency leak.
- `cargo check --workspace --all-features --all-targets`
  - Result: pass.
- `cargo test --workspace --all-features --all-targets`
  - Result: pass after rerun by itself with a longer timeout. A first parallel
    invocation timed out while competing for Cargo locks and was not counted as
    a passing result.

## Existing GPU Evidence

- `cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime`
  - Result: pass; wrote untracked target artifacts.
- `cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime --measure-gpu`
  - Result: pass; wrote untracked target artifacts.
- `cargo test -p alife_gpu_backend --features gpu-tests --test static_forward_parity -- --ignored --nocapture`
  - Result: pass, 2 ignored/manual GPU tests.
- `cargo test -p alife_gpu_backend --features gpu-tests --test plasticity_oja_parity -- --ignored --nocapture`
  - Result: pass, 2 ignored/manual GPU tests.

## R2 Review

- Review class: R2, separate agent review.
- Result: not blocked.
- Findings fixed:
  - Updated stale `full_runtime.rs` module comment that still described
    plasticity as diagnostic-only.
  - Added direct unsealed-patch rejection regression coverage.
- Remaining findings: none blocking.
