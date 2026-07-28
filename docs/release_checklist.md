# A-Life P36 Release Checklist

Status: P36 release-gate checklist.

This checklist is a release-candidate gate, not a new feature plan. Do not tag a
release until every required gate has either passed with command evidence or is
recorded as a blocker with an exact reproduction.

## Required Local Gates

Run these from the repository root on Windows:

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

On non-Windows systems, the shell scripts may be run through the platform shell.
On Windows, use the PowerShell wrappers or an explicit Git Bash executable path.

## Focused Release Gates

| Gate | Command | Required status |
|---|---|---|
| Golden trace replay | `cargo test -p alife_world --test golden_traces_determinism` | Required |
| Scenario suite | `cargo test -p alife_world --test scenario_suite` | Required |
| Fast headless soak | `cargo test -p alife_world --test headless_soak` | Required |
| Extended headless soak | `cargo test -p alife_world --test headless_soak -- --ignored --nocapture` | Manual |
| Save/load round trip | `cargo test -p alife_world --test save_load_roundtrip` | Required |
| Config/asset manifest validation | `cargo test -p alife_tools --test save_load_configs_assets` | Required |
| P35 playground smoke | `cargo test -p alife_tools --test playground_examples` | Required |
| Benchmark smoke | `cargo test -p alife_tools --test benchmark_tiers benchmark_tiers_smoke_runs_tier_1_and_10_without_bevy_or_gpu` | Required |
| Benchmark report artifact | `cargo run -p alife_tools --bin benchmark_tiers` | Required smoke artifact under `target/artifacts/` |
| P35 headless demo | `cargo run -p alife_tools --bin p35_playground -- run-headless crates/alife_world/tests/fixtures/p34` | Required |
| P35 all-demo smoke | `cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json` | Required |
| School/teacher verifier smoke | `cargo run -p alife_tools --bin p35_playground -- school-demo` | Required |
| Semantic fake-provider smoke | `cargo run -p alife_tools --features semantic-demo --bin p35_playground -- semantic-demo` | Required if feature build is available |
| Offline tools smoke | `cargo run -p alife_tools --bin p30_offline -- --help` | Manual/help smoke |
| ETF/NC tooling smoke | `cargo run -p alife_tools --bin p31_offline -- --help` | Manual/help smoke |
| Weight asset tooling smoke | `cargo run -p alife_tools --bin p32_weights -- --help` | Manual/help smoke |
| Genome lab tooling smoke | `cargo run -p alife_tools --bin p33_genome_lab -- --help` | Manual/help smoke |
| Persistence tooling smoke | `cargo run -p alife_tools --bin p34_persistence -- --help` | Manual/help smoke |

## GPU And Graphics Manual Gates

Run these only on machines with supported hardware and graphics/runtime support.
If unavailable, record the gate as manual/unknown; do not fabricate results.

| Gate | Command | Notes |
|---|---|---|
| P25 GPU static parity | `cargo test -p alife_gpu_backend --features gpu-tests --test static_forward_parity -- --ignored --nocapture` | Requires a local wgpu adapter |
| GPU closed-loop causal behavior | `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_gpu_behavior -j 1 -- --nocapture` | Requires a local wgpu adapter |
| GPU sealed-outcome fast plasticity | `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_fast_plasticity -j 1 -- --nocapture` | Requires a local wgpu adapter |
| GPU runtime performance report | `cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime` | Records typed unavailable status when required hardware is absent |
| Full benchmark tiers | `cargo run -p alife_tools --bin benchmark_tiers -- --all --gpu-runtime` | Manual expected-slow |
| Bevy adapter smoke | `cargo run -p alife_bevy_adapter --example minimal_adapter` | Requires graphics-capable environment |

## Artifact Policy

- Reports must be written under `target/artifacts/`.
- Do not commit large logs, generated tensors, GPU captures, or benchmark
  output artifacts.
- Tiny fixtures under `crates/*/tests/fixtures/` are allowed only when they are
  required for deterministic tests.
- Golden traces must not be overwritten unless the behavior change is intended
  and reviewed.

## Release Candidate Review

Before marking a candidate ready:

- Confirm `alife_core` has no Bevy, Avian, wgpu, renderer, ECS, OS-windowing,
  Python runtime, Unity, C#, or HLSL dependency leak.
- Confirm save/load uses stable IDs and versioned schemas.
- Confirm neural mode is GPU-required, and GPU failure stops learned actions with a typed unavailable result.
- Confirm active gameplay does not require synchronous neural readback.
- Confirm performance misses and unknown GPU hardware status are documented in
  `docs/final_status_report.md`.
- Confirm all known limitations are explicit.
