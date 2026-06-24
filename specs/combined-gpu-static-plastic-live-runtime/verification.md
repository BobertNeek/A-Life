# Verification

Focused evidence captured:

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- full-gpu-runtime-smoke crates/alife_world/tests/fixtures/p34 --mode static-plastic-cpu-shadow-guarded --ticks 3
```

Result on local machine:

- Adapter: NVIDIA GeForce RTX 3050
- Backend/API: Vulkan
- Selected backend: `GpuPlastic`
- GPU proposal scores used: true
- CPU shadow parity: true
- Sealed patches: 3
- Post-seal H_shadow records applied: 2
- Active-tick timing: upload 0.1996 ms, GPU submit/poll 1.3207 ms, compact
  action-summary readback 0.8925 ms, CPU shadow 0.0238 ms, total GPU runtime
  2.4128 ms
- Post-seal diagnostic H_shadow readback: 48 bytes, 1.2890 ms,
  boundary-scoped after patch sealing
- Product runtime claim: `CpuShadowGuardedStaticPlusLiveHShadow`
- Unsupported full action-authoritative gap remains: true

Focused degradation evidence:

```powershell
$env:ALIFE_GPU_PLASTICITY_DIAGNOSTIC_AVAILABLE='0'
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- full-gpu-runtime-smoke crates/alife_world/tests/fixtures/p34 --mode static-plastic-cpu-shadow-guarded --ticks 3
Remove-Item Env:\ALIFE_GPU_PLASTICITY_DIAGNOSTIC_AVAILABLE -ErrorAction SilentlyContinue
```

Result: static GPU proposal scoring still selected `GpuPlastic` on RTX
3050/Vulkan and used CPU-shadow-verified scores for proposals, but H_shadow
deltas were not applied and the product claim degraded to `CpuShadowGuarded`.

Full validation passed:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

R2 re-review result: `not_blocked`, no findings.
