# First Graphical Alpha Playtest Checklist

Use this checklist for the first external tester pass on the graphical A-Life
GPU alpha playground. This is not release approval and does not create a tag.

## Recommended Command

Run from the repository root on Windows:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

For a persistent manual session, omit `-SmokeSeconds 30`.

## Tester Checks

Record yes/no plus notes for each item.

| Check | Result | Notes / Evidence |
| --- | --- | --- |
| Window opens with title containing `A-Life GPU Alpha Playground`. |  |  |
| Creature marker is visible and distinguishable. |  |  |
| Food marker is visible and distinguishable. |  |  |
| Hazard and obstacle markers are visible in the GPU alpha fixture, or explicit guide text explains any fallback fixture. |  |  |
| Selected creature stable ID is visible. |  |  |
| Space toggles pause/run. |  |  |
| `N` steps once. |  |  |
| `1/2/3` speed controls are visible or testable. |  |  |
| `F` follow is visible or testable. |  |  |
| `Esc` closes the app cleanly. |  |  |
| GPU status shows selected GPU mode or clear CPU fallback. |  |  |
| Inspector is readable and read-only. |  |  |
| Inspector updates after at least one tick. |  |  |
| Product claim does not say full action-authoritative runtime. |  |  |
| No Bevy Entity IDs appear in player-facing text. |  |  |
| App closes without leaving a process running. |  |  |

## Evidence To Capture

- Screenshot or short video of the initial window.
- Screenshot or short video after pause/run or one step.
- Screenshot of the GPU/fallback status and read-only inspector.
- Terminal output from the launcher command.
- Hardware notes: OS, CPU, GPU, driver if known, display resolution.
- Any rendering glitches, confusing text, or controls that did not work.

## Local Control Evidence Fallback

If keyboard injection or desktop automation cannot verify controls, run this
deterministic command from the repository root and record its result separately
from human tester evidence:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- graphical-controls-smoke crates/alife_world/tests/fixtures/p34
cargo run -p alife_game_app --bin alife_game_app -- graphical-controls-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

This command verifies the Space/N/1/2/3/F/Esc-equivalent control semantics
through the same app control surface without requiring foreground key input.

## Finding Severity

- `BLOCKER`: window cannot launch, cannot close, or supported smoke fails.
- `HIGH`: a first tester cannot identify the world, creature, controls, or
  runtime status.
- `MEDIUM`: visible but confusing, cluttered, or missing expected explanation.
- `LOW`: wording, polish, layout, or documentation issue.
- `MANUAL_EVIDENCE_MISSING`: cannot judge without local graphics/GPU evidence.

## Boundaries

- The player-facing alpha target is GPU-first.
- CPU fallback is allowed as a degraded safety mode and must be visible.
- CPU fallback is not GPU performance evidence.
- CPU shadow remains the gate for the current GPU runtime claim.
- Do not create S12, G25, P37, or a release tag from this checklist.
