# CA44 External Tester Intake Report

Use one copy of this template per independent external tester run. Store raw
screenshots, videos, terminal output, crash summaries, and filled local tester
forms outside git, preferably under:

```text
target/playtest_evidence/alpha/TESTER_OR_RUN_ID/
```

Do not commit media, logs, captures, package artifacts, model files, or local
hardware dumps.

## Tester And Machine

- Tester alias:
- Independent human tester: yes/no
- Date/time:
- Repo SHA:
- Package/build source:
- OS:
- CPU:
- GPU:
- GPU driver:
- RAM:
- Display resolution:
- Input devices:

## Commands Run

Record exact commands, exit status, and evidence path.

| Command | Exit Status | Evidence Path | Notes |
| --- | --- | --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -GpuMode static-plastic-cpu-shadow-guarded` |  |  |  |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded` |  |  |  |
| Package runner command, if testing a zip package |  |  |  |
| Forced fallback command, if tested |  |  |  |

## Playtest Checklist

| Check | Pass/Fail/Missing | Notes / Evidence Path |
| --- | --- | --- |
| Window opens and title contains `A-Life GPU Alpha Playground`. |  |  |
| Creature, food, hazard, obstacle, and terrain context are visible. |  |  |
| Tester can identify the selected creature. |  |  |
| Tester can pause/run with Space. |  |  |
| Tester can step once with `N`. |  |  |
| Tester can change speed with `1/2/3`. |  |  |
| Tester can follow selection with `F`. |  |  |
| Tester can save/load using the documented controls. |  |  |
| GPU status is visible or degraded CPU fallback is explicit. |  |  |
| No full action-authoritative GPU claim appears. |  |  |
| No Bevy Entity IDs appear in player-facing text. |  |  |
| Crash summary/feedback template appears if launch fails. |  |  |
| App exits cleanly without a leftover process. |  |  |

## Findings

Use these severities:

- `BLOCKER`: prevents launch, clean close, or the supported alpha path.
- `HIGH`: prevents a normal tester from treating it as a playable game.
- `MEDIUM`: playable but confusing, brittle, incomplete, or poorly explained.
- `LOW`: polish, wording, layout, or minor documentation issue.
- `MANUAL_EVIDENCE_MISSING`: cannot judge without tester evidence.

| ID | Severity | Area | Reproduction / Evidence | Recommended Fix Or Decision |
| --- | --- | --- | --- | --- |
| CA44-001 |  |  |  |  |

## Tester Recommendation

Choose one:

- Continue alpha playtests.
- Fix blockers/high findings first.
- Gather missing evidence.
- Consider beta criteria review.
- Defer release/tagging.

Notes:

