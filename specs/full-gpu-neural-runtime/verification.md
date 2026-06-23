# Full GPU Neural Runtime Verification

## Required Validation

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

## Focused Evidence

- `cargo run -p alife_game_app --bin alife_game_app -- full-gpu-runtime-smoke crates/alife_world/tests/fixtures/p34`
- `cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- full-gpu-runtime-smoke crates/alife_world/tests/fixtures/p34 --mode static-action-authoritative --ticks 3`
- `cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- full-gpu-runtime-smoke crates/alife_world/tests/fixtures/p34 --mode static-plastic-shadow --ticks 1`
- forced fallback with `ALIFE_GPU_RUNTIME_AVAILABLE=0` and an explicit GPU mode.
- Existing manual ignored GPU parity tests where available.
