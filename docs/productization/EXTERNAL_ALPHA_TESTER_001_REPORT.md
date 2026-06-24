# External Alpha Tester 001 Report

Status: local Codex Computer Use tester evidence captured. This is a real
computer-use playtest on the local Windows machine, not an independent human
external tester pass.

Evidence date: 2026-06-24

## Scope

This report records a first graphical alpha tester pass using the existing
checklist. It does not implement features, create S12/G25/P37, create a release
tag, or change runtime behavior.

## Tester Machine

- Tester: Codex Computer Use local tester
- Machine: `DESKTOP-K7EDOMI`
- OS: Microsoft Windows 10 Home 10.0.19045 build 19045
- CPU: Intel(R) Core(TM) i7-3770K CPU @ 3.50GHz
- RAM: 32 GB
- GPU: NVIDIA GeForce RTX 3050
- GPU driver: 32.0.15.8180
- Display evidence: primary display captured at window size 1134x738

## Command

Primary local evidence command:

```powershell
target\debug\alife_game_app.exe graphical-playground crates/alife_world/tests/fixtures/p34 --gpu-mode static-plastic-cpu-shadow-guarded --smoke-seconds 60
```

The documented launcher command was also exercised during setup:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -GpuMode static-plastic-cpu-shadow-guarded
```

The direct executable smoke was used for the final captured window evidence
because it produced a stable targetable `A-Life Alpha Playground - smoke 60s`
window for Computer Use inspection.

## Evidence Paths

Local evidence files were kept untracked under:

```text
target/playtest_evidence/external_alpha_001/
```

Captured screenshot reference, not committed:

```text
target/playtest_evidence/external_alpha_001/screenshots/001_alife_alpha_window.png
```

Raw command output references, not committed:

```text
target/playtest_evidence/external_alpha_001/direct_smoke60_stdout.log
target/playtest_evidence/external_alpha_001/direct_smoke60_stderr.log
```

## Runtime Summary

The 60-second graphical smoke reported:

- Window title: `A-Life Alpha Playground - smoke 60s`
- GPU mode requested: `static-plastic-cpu-shadow-guarded`
- Selected runtime: `GpuPlastic`
- Product claim: `CpuShadowGuardedStaticPlusLiveHShadow`
- GPU scores used for proposals: `true`
- CPU shadow parity: `true`
- H_shadow applications: 1
- Fallback: `None`
- Objects: 2
- Creatures: 1
- Food: 1
- Stable IDs: `true`
- Mind tick: 19
- World tick: 18
- Action: `Idle`
- Sealed patches: 16
- Packed logs: 16
- App close behavior: bounded smoke timeout exited cleanly

The stderr log contained Vulkan loader warnings about a missing validation layer
and a deprecated GOG Galaxy overlay layer manifest. They did not block the
smoke run or prevent GPU selection.

## Checklist Results

| Check | Result | Evidence |
| --- | --- | --- |
| Window opened with title containing `A-Life Alpha Playground`. | Pass | Computer Use found the window and the screenshot shows the title. |
| Creature marker visible and distinguishable. | Pass | Screenshot shows `[@] creature stable:1` and a creature marker. |
| Food marker visible and distinguishable. | Pass | Screenshot shows `[+] food stable:2 berry nutrition=0.75` and a food marker. |
| Hazard marker or guide visible and distinguishable. | Partial | The P34 fixture screenshot shows creature and food only; no separate hazard object was visually confirmed. |
| Selected creature stable ID visible. | Pass | Inspector and marker text show stable ID `1`. |
| Space toggles pause/run. | Manual evidence missing | Computer Use key input failed with `foreground window did not report a process id`. |
| `N` steps once. | Manual evidence missing | Computer Use key input failed with the same process-id error. |
| `1/2/3` speed controls visible or testable. | Partial | Controls text is visible; key interaction was not verified. |
| `F` follow visible or testable. | Partial | Controls text is visible; key interaction was not verified. |
| `Esc` closes the app cleanly. | Manual evidence missing | Smoke timeout closed cleanly; Esc key path was not verified. |
| GPU status shows selected GPU mode or clear CPU fallback. | Pass | Overlay and summary show `Selected: GpuPlastic fallback=none`. |
| Inspector is readable and read-only. | Partial | Inspector is visible and read-only, but the right panel is horizontally clipped at the captured window size. |
| Inspector updates after at least one tick. | Pass | Screenshot/log show mind tick 19, world tick 18, sealed patch true, and 16 patches/logs. |
| Product claim does not say full action-authoritative runtime. | Pass | Overlay states CPU shadow remains the gate and the claim is `CpuShadowGuardedStaticPlusLiveHShadow`. |
| No Bevy Entity IDs appear in player-facing text. | Pass | Observed text uses stable IDs and does not expose Bevy Entity IDs. |
| App closes without leaving a process running. | Pass | The smoke timeout exited and no `alife_game_app` process remained afterward. |

## Findings

### BLOCKER

- None found.

### HIGH

- None found.

### MEDIUM

- The right read-only inspector/GPU runtime panel is horizontally clipped in the
  captured 1134x738 window. A first alpha tester can see the section, but not
  all field values.
- Manual controls were not verified through Computer Use because key injection
  failed with `foreground window did not report a process id`. Controls are
  visible in-window, but human tester interaction evidence is still needed.
- The P34 fixture did not show a distinct hazard object in the captured world
  view. The first-tester checklist should continue treating hazard visibility
  as partial unless a richer fixture or clearer hazard guide is used.

### LOW

- Local Vulkan loader warnings mention a missing validation layer and a
  deprecated GOG Galaxy overlay layer manifest. They did not block the run.

### MANUAL_EVIDENCE_MISSING

- Independent human external tester evidence.
- Manual Space/N/1/2/3/F/Esc interaction evidence.
- Video evidence of a tester using controls.
- Full readability confirmation at additional screen sizes.

## Release/Tag Recommendation

Do not tag a release from this evidence. The current evidence supports a small
alpha tester loop with targeted UX follow-up. It does not support public release
readiness and does not change the GPU runtime claim.

## Next Recommendation

Fix or improve the clipped inspector layout before broadening the alpha tester
pool, then collect one independent human tester pass that manually exercises
pause/run, step, speed, follow, and quit controls.
