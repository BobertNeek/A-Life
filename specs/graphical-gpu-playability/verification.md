# Graphical GPU Playability Verification

Planned commands:

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
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-sustained-learning-soak crates/alife_world/tests/fixtures/p34 --ticks 1000 --report-every 100
cargo run -p alife_game_app --features "bevy-app gpu-runtime" --bin alife_game_app -- graphical-playground crates/alife_world/tests/fixtures/p34 --gpu-mode static-plastic-cpu-shadow-guarded --smoke-seconds 20
```

Results:

- `cargo fmt --all -- --check`: pass.
- `cargo check --workspace --all-targets`: pass.
- `cargo test --workspace --all-targets`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1`: pass.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`: pass.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`: pass.
- `cargo tree -p alife_core`: pass; no Bevy, wgpu, GPU, renderer, or game-app dependency leak.
- `cargo check --workspace --all-features --all-targets`: pass.
- `cargo test --workspace --all-features --all-targets`: pass.
- `cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-sustained-learning-soak crates/alife_world/tests/fixtures/p34 --ticks 1000 --report-every 100`: pass; selected RTX 3050/Vulkan `GpuPlastic`, 1000/1000 ticks, 0 parity failures, 1000 sealed patches.
- `cargo run -p alife_game_app --features "bevy-app gpu-runtime" --bin alife_game_app -- graphical-playground crates/alife_world/tests/fixtures/p34 --gpu-mode static-plastic-cpu-shadow-guarded --smoke-seconds 20`: pass; selected `GpuPlastic`, fallback `None`, GPU scores used, CPU shadow parity true, H_shadow applications visible.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 5`: pass; script launches bounded graphical GPU smoke and exits cleanly.
- R2 review found a copy-paste issue in dry-run command rendering and a defensive claim-acceptance tightening point. Both were fixed before merge: dry-run now quotes `--features 'bevy-app gpu-runtime'`, and graphical GPU proposal use accepts only CPU-shadow-guarded product claims.
- Post-review `cargo test -p alife_game_app --test app_shell graphical_gpu -- --nocapture`: pass.
- Post-review `cargo test -p alife_game_app s01_graphical_launcher_script_uses_persistent_window_commands -- --nocapture`: pass.
- Post-review `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun`: pass; printed copy-paste-safe quoted feature string.
- Post-review `cargo run -p alife_game_app --features "bevy-app gpu-runtime" --bin alife_game_app -- graphical-playground crates/alife_world/tests/fixtures/p34 --gpu-mode static-plastic-cpu-shadow-guarded --smoke-seconds 20`: pass; selected `GpuPlastic`, fallback `None`, GPU scores used, CPU shadow parity true, H_shadow applications visible.
- Post-review forced fallback graphical smoke with `ALIFE_GPU_RUNTIME_AVAILABLE=0`: pass; selected `CpuReference`, fallback `HardwareUnavailable`, no GPU-score claim, CPU patches sealed.

Known local graphics warnings: wgpu/Vulkan reports missing validation layer and a deprecated GOG Galaxy overlay layer manifest, but the graphical GPU smoke exits successfully and selects the RTX 3050/Vulkan backend.
