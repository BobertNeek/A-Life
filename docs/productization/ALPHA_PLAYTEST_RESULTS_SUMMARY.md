# A-Life Alpha Playtest Results Summary

Status: local alpha evidence setup pass.

This summary records the repository-side preparation for external alpha
playtests. It is not a public release claim and does not approve a release tag.

## Current Classification

Current classification remains: alpha / external playtest candidate.

Release tag status: deferred. No release tag was created.

Next implementation plan: None. Future work requires explicit user instruction.
This pass did not create S12, G25, P37, or any hidden continuation chain.

## Evidence Package

Added docs:

- `docs/productization/ALPHA_PLAYTEST_RUNBOOK.md`
- `docs/productization/ALPHA_PLAYTEST_EVIDENCE_TEMPLATE.md`
- `docs/productization/ALPHA_PLAYTEST_RESULTS_SUMMARY.md`

Local generated evidence should be stored outside git under:

```text
target/playtest_evidence/alpha/
```

## Local Validation Results

Run date: 2026-06-22.

Repository SHA at start of evidence setup: `700167f`.

Repository-side validation for this setup pass completed successfully:

- default formatting, check, test, clippy, docs, and boundary commands passed
- `alife_core` dependency tree remained engine-independent
- release-candidate, product-QA, platform-package, and P35 playground smoke
  commands passed
- GPU runtime benchmark command completed with CPU fallback because hardware
  GPU runtime validation was unavailable
- graphical dry-run command passed
- bounded graphical smoke opened the Bevy path and exited after the configured
  timeout
- default graphical command opened a responding `A-Life Graphical Playground`
  window and was closed after confirming persistence

Local command output from this setup run was stored under `target/` only and is
not part of the committed evidence package.

## Local Smoke Commands For This Pass

Required validation:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
```

Focused smoke:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- release-candidate-smoke
cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun
```

Manual graphical command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1
```

## Known Limitations

- Dry-run output proves command wiring only; it is not graphical playtest
  evidence.
- CPU fallback is acceptable runtime behavior, but it is not GPU performance
  evidence.
- The default graphical window was confirmed locally, but external tester
  screenshots, videos, and observations have not been collected yet.
- A public release/tag decision still requires explicit user approval.
- GPU hardware performance remains unknown until measured on validated GPU
  hardware without CPU fallback.

## Recommended Next User Decision

Run one or more external alpha playtests using the runbook and evidence
template, then decide whether to fix findings, collect more manual evidence, or
authorize a separate alpha tag review.
