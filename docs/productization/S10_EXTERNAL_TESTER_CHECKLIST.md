# S10 External Tester Checklist

Use this checklist from a clean checkout. Record command output and manual
observations separately under `target/playtest_evidence/S10/` if you are doing a
local evidence capture pass. Do not commit those evidence files.

## Required Automated Checks

Run from the repository root:

```powershell
cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke
cargo run -p alife_game_app --bin alife_game_app -- release-candidate-smoke
cargo run -p alife_game_app --bin alife_game_app -- content-authoring-smoke
```

Expected result: every command exits successfully. If any command fails, capture
the full command and output.

## Optional Manual Graphics Check

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1
```

Pass criteria:

- an A-Life window opens
- a tiny world is visible
- a creature marker and food/hazard/resource markers are visible
- pause, step, run/resume, selection, and inspector/readout surfaces are usable
- the app exits cleanly

If no compatible graphics environment is available, record this as manual
evidence missing rather than a product pass.

## Optional Manual GPU Check

```powershell
ALIFE_GPU_RUNTIME_BACKEND=static ALIFE_GPU_RUNTIME_FEATURE=1 ALIFE_GPU_RUNTIME_AVAILABLE=1 ALIFE_GPU_RUNTIME_VALIDATED=1 cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```

Pass criteria: the report records a validated GPU runtime path with hardware
evidence. CPU fallback is acceptable behavior, but it is not GPU performance
evidence.

## Artifact Check

```powershell
git status --short
git ls-files target dist target/artifacts graphify-out
```

Pass criteria: generated artifacts are not tracked. Local evidence under
`target/playtest_evidence/` remains uncommitted.

## Release Boundary

S10 does not create a release tag. Any release/tag decision belongs to S11 and
requires explicit user approval.
