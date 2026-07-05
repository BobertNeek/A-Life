# G22 Product QA Hardening

G22 is a bug-bash and QA evidence layer. It does not add gameplay features or
change runtime contracts. It checks the existing playable-sim surfaces for
crash resilience, invalid inputs, UI transitions, optional feature boundaries,
manual gates, and known limitations before G23.

## CI-safe smoke

Run the product QA aggregation smoke from the repository root:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke
```

The smoke reuses existing headless app, survival, live brain, save/load,
school, semantic, GPU fallback, long-run balance, onboarding, and packaging
checks. It keeps the CPU/headless path as the default correctness route.

## Required validation wrappers

On Windows, use the PowerShell wrappers. Do not invoke `check.sh` through an
ambiguous plain `bash` command.

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

## Focused QA commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- headless-smoke crates/alife_world/tests/fixtures/p34
cargo run -p alife_game_app --bin alife_game_app -- playable-survival-loop-smoke
cargo run -p alife_game_app --bin alife_game_app -- live-brain-tick-smoke crates/alife_world/tests/fixtures/p34
cargo run -p alife_game_app --bin alife_game_app -- save-load-ux-smoke crates/alife_world/tests/fixtures/p34
cargo run -p alife_game_app --bin alife_game_app -- school-mode-smoke
cargo run -p alife_game_app --bin alife_game_app -- semantic-provider-smoke
cargo run -p alife_game_app --bin alife_game_app -- gpu-product-smoke
cargo run -p alife_game_app --bin alife_game_app -- longrun-balance-smoke
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
cargo test -p alife_world --test headless_soak fast_headless_soak_preserves_release_gate_invariants
cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json
```

## Manual and extended gates

Extended balance remains manual because larger or longer runs should not become
normal CI load:

```powershell
cargo test -p alife_game_app --test app_shell g19_manual_extended_balance_run -- --ignored --nocapture
```

GPU hardware performance evidence is manual. This command only becomes GPU
evidence when the local machine has supported hardware and the runtime flags are
set. If the flags or validation evidence are missing, reports may honestly
record CPU fallback instead of GPU performance.

```powershell
ALIFE_GPU_RUNTIME_BACKEND=static ALIFE_GPU_RUNTIME_FEATURE=1 ALIFE_GPU_RUNTIME_AVAILABLE=1 ALIFE_GPU_RUNTIME_VALIDATED=1 cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

Graphical smoke remains manual and feature-gated:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
```

## Known issue handling

Release blockers must be fixed or documented with exact reproduction steps.
Non-blocking limitations are recorded in
`docs/playable_sim_spec/known_issues.md`. G22 does not hide missed metrics or
convert fallback results into product GPU claims.
