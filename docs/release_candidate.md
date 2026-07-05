# G23 Playable Release Candidate

Candidate: `playable-sim-rc1`

Status: feature-complete candidate for the supported headless CPU playable path, pending the live G23 validation run and R23 review. The graphical and GPU hardware paths remain manual unless measured on local hardware.

## Supported Playable Path

The supported candidate path is the headless CPU playground:

```powershell
cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json
```

This path exercises P34 config/assets, the tiny world save, CPU backend selection, sealed patch collection, school verifier smoke, and the P35 playground manifest without requiring GPU or graphics hardware.

## Automated Gates

Run the full validation set before accepting this candidate:

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

Focused candidate commands:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- release-candidate-smoke
cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke
cargo run -p alife_game_app --bin alife_game_app -- save-load-ux-smoke crates/alife_world/tests/fixtures/p34
cargo test -p alife_world --test headless_soak fast_headless_soak_preserves_release_gate_invariants
cargo run -p alife_game_app --bin alife_game_app -- longrun-balance-smoke
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
```

## Manual Gates

Graphical playtest:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
```

The dry-run checks command wiring only. A real graphical playtest depends on local graphics support and should record the hardware/display context.

GPU hardware diagnostics:

```powershell
ALIFE_GPU_RUNTIME_BACKEND=static ALIFE_GPU_RUNTIME_FEATURE=1 ALIFE_GPU_RUNTIME_AVAILABLE=1 ALIFE_GPU_RUNTIME_VALIDATED=1 cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

If hardware support or validation flags are not set, the report may honestly record CPU fallback rather than GPU performance. CPU fallback is not GPU performance.

## Known Limitations

- The fast balance smoke proves bounded deterministic loops, not full ecosystem fun across every possible ecology.
- The extended balance run is manual:

```powershell
cargo test -p alife_game_app --test app_shell g19_manual_extended_balance_run -- --ignored --nocapture
```

- GPU performance is unknown unless the manual GPU command records hardware-validated measurements.
- Graphical playtest evidence is manual unless local graphics support is available.

Additional known issues are tracked in `docs/playable_sim_spec/known_issues.md`.

## Tag Proposal

No release tag was created by G23. If R23 passes and the user explicitly requests tagging later, the proposed command is:

```powershell
git tag -a playable-sim-rc1 <validated-main-sha> -m "A-Life playable sim RC1"
```

Do not tag automatically during G23.
