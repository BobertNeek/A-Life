# A-Life Alpha Playtest Evidence Template

Use this template for each external alpha playtest. Store screenshots, videos,
logs, benchmark artifacts, and hardware dumps outside git, preferably under
`target/playtest_evidence/alpha/TESTER_OR_RUN_ID/`.

Do not use this evidence form as release approval. A release tag still requires
an explicit user decision after reviewing the collected evidence.

## Tester And Environment

- Tester name or alias:
- Date and local time:
- Time zone:
- Repo SHA tested:
- Branch tested:
- OS and version:
- CPU:
- GPU/display adapter:
- RAM:
- Display resolution and monitor count:
- Input devices used:
- Notes about graphics/GPU availability:

## Command Results

Record every command exactly as run, its exit status, and the relevant summary
line or error.

| Command | Exit Status | Evidence Path | Notes |
| --- | --- | --- | --- |
| `cargo fmt --all -- --check` |  |  |  |
| `cargo check --workspace --all-targets` |  |  |  |
| `cargo test --workspace --all-targets` |  |  |  |
| `cargo clippy --workspace --all-targets -- -D warnings` |  |  |  |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1` |  |  |  |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1` |  |  |  |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1` |  |  |  |
| `cargo tree -p alife_core` |  |  |  |
| `cargo run -p alife_game_app --bin alife_game_app -- release-candidate-smoke` |  |  |  |
| `cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke` |  |  |  |
| `cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke` |  |  |  |
| `cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json` |  |  |  |
| `cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime` |  |  |  |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun` |  |  |  |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1` |  |  |  |

## Graphical Launch Result

- Did a window open and remain interactive:
- Window title:
- Was a tiny world visible:
- Creature marker or placeholder visible:
- Food/hazard/resource marker visible:
- Stable ID/debug text visible:
- CPU fallback/backend status visible:
- Pause/step/run controls usable:
- Selection/inspector usable:
- Save/load UX visible or reachable:
- Did the app exit cleanly:
- Screenshot/video references:
- Exact failure if graphical launch did not run:

Dry-run output is command wiring evidence only. It is not real graphical
playtest evidence.

## GPU Runtime Result

- Command used:
- `ALIFE_GPU_RUNTIME_BACKEND` value:
- `ALIFE_GPU_RUNTIME_FEATURE` value:
- `ALIFE_GPU_RUNTIME_AVAILABLE` value:
- `ALIFE_GPU_RUNTIME_VALIDATED` value:
- Backend selected:
- CPU fallback status:
- Hardware identifier if available:
- Frame rate or timing report if measured:
- Subjective smoothness if measured:
- Evidence path:

CPU fallback is acceptable runtime behavior, but it is not GPU performance
evidence.

## Usability Observations

- First thing the tester understood:
- First confusion point:
- Controls discovered without help:
- Controls missed or misunderstood:
- Did the tester understand what happened without reading logs:
- Crashes or hangs:
- Save/load result:
- Overall playability rating:
  - 1 = cannot play
  - 2 = launches but confusing or mostly tooling
  - 3 = alpha-playable with guidance
  - 4 = playable with polish gaps
  - 5 = release-candidate feel

## Findings

Classify each finding with one of:

- `BLOCKER`: prevents launch or makes the supported product path unusable.
- `HIGH`: prevents a normal external tester from treating it as a game.
- `MEDIUM`: usable but confusing, brittle, incomplete, or poorly explained.
- `LOW`: polish, wording, docs, or minor UX issue.
- `MANUAL_EVIDENCE_MISSING`: cannot judge without hardware, screenshots, or
  playtest evidence.

| ID | Severity | Area | Reproduction / Evidence | Recommended Fix Or Decision |
| --- | --- | --- | --- | --- |
| APL-001 |  |  |  |  |

## Tester Recommendation

Choose one:

- Continue external alpha playtests.
- Fix blockers/high findings first.
- Gather missing graphical/GPU evidence.
- Consider alpha tag review after validation is rerun on the exact SHA.
- Defer release/tagging.

Notes:
