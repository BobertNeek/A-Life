# S10 External Playtest Candidate Report

Status: implemented on `codex/S10-packaging-qa-external-playtest`.

S10 prepares the current A-Life build for a bounded external playtest. It does
not create a release tag, installer, signing pipeline, store package, or new
implementation chain. The candidate remains a local checkout and smoke-test
candidate for the supported headless CPU path plus optional/manual graphics and
GPU evidence paths.

## Candidate Scope

Supported path:

- headless CPU playground
- deterministic product smoke suite
- P34 fixture/config/asset validation
- tiny S09 tutorial/content pack validation
- local packaging and artifact discipline

Manual evidence paths:

- graphical playground launch on local graphics hardware
- GPU runtime/performance evidence when hardware and validation flags are set
- extended balance and soak runs

Unsupported in S10:

- release tagging
- store packaging
- signing automation
- committing generated packages, screenshots, logs, GPU captures, benchmark
  artifacts, or `target/` output

## Clean Checkout Runbook

From a fresh checkout on Windows:

```powershell
git clone https://github.com/BobertNeek/A-Life.git
cd A-Life
cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json
cargo run -p alife_game_app --bin alife_game_app -- release-candidate-smoke
cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
cargo run -p alife_game_app --bin alife_game_app -- content-authoring-smoke
```

Repository validation on Windows must use the PowerShell wrappers:

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

## Local Run Scripts

Headless dry-run and execution:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_headless_playground.ps1 -DryRun
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_headless_playground.ps1
```

Graphical dry-run and manual local graphical check:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
```

The graphical command is manual evidence. It should not be reported as passing
unless a tester actually sees the A-Life window, confirms the visible scene and
controls, and records the result.

## GPU Runtime Evidence

Use this only on hardware where GPU runtime validation is intentionally enabled:

```powershell
ALIFE_GPU_RUNTIME_BACKEND=static ALIFE_GPU_RUNTIME_FEATURE=1 ALIFE_GPU_RUNTIME_AVAILABLE=1 ALIFE_GPU_RUNTIME_VALIDATED=1 cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

If hardware support or validation flags are missing, the report may honestly
record CPU fallback. CPU fallback is not GPU performance evidence.

## Artifact Discipline

Generated outputs must remain untracked. S10 validates this with:

```powershell
git ls-files target dist target/artifacts graphify-out
```

Expected output is empty. Local package, screenshot, log, benchmark, and
Computer Use evidence may be created under `target/`, but must not be committed.

## Known Issues Summary

The current known issues source remains:

```text
docs/playable_sim_spec/known_issues.md
```

Current non-blocking limitations include:

- fast balance smoke includes scripted hazard contact for metric visibility
- extended balance is manual/ignored by default
- GPU hardware performance is manual unless actually measured
- graphical playground evidence depends on local graphics support

No release blockers are known when the S10 validation and focused smoke commands
pass.

## S10 Focused Evidence Commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke
cargo run -p alife_game_app --bin alife_game_app -- release-candidate-smoke
git ls-files target dist target/artifacts graphify-out
```

## Result

The external playtest candidate is a documentation and local-run candidate. It
is suitable for a tester who can run a checkout and report smoke/manual evidence.
It is not a signed release, store package, or GPU performance claim.

Next plan: S11 final playtest/release decision. No S12, G25, or P37 was created.
