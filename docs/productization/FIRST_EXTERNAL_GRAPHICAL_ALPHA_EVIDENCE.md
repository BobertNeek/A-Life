# First External Graphical Alpha Evidence

Superseding note: the local evidence below preserves pre-GPU-first window
titles and P34 fixture observations. Current first-player graphical alpha
testing should use `A-Life GPU Alpha Playground` with
`crates/alife_world/tests/fixtures/gpu_alpha`.

Status: local graphical alpha rehearsal complete; local Codex Computer Use
tester evidence captured. Independent human external tester evidence is still
`MANUAL_EVIDENCE_MISSING`.

Evidence date: 2026-06-24

## Scope

This pass collects first external alpha evidence using the existing graphical
alpha checklist. It does not implement features, create S12/G25/P37, create a
release tag, or change runtime behavior.

## Tester Machine

Local rehearsal machine:

- Machine: `DESKTOP-K7EDOMI`
- OS: Microsoft Windows 10 Home 10.0.19045 build 19045
- CPU: Intel(R) Core(TM) i7-3770K CPU @ 3.50GHz
- RAM: 32 GB
- GPU: NVIDIA GeForce RTX 3050
- GPU driver: 32.0.15.8180
- GPU status: OK

External tester:

- Availability: no independent human external tester evidence was captured in
  this pass.
- Local computer-use tester: captured in
  `docs/productization/EXTERNAL_ALPHA_TESTER_001_REPORT.md`.
- Classification: local computer-use evidence captured; independent human
  evidence remains `MANUAL_EVIDENCE_MISSING`.

## External Tester 001 Update

`docs/productization/EXTERNAL_ALPHA_TESTER_001_REPORT.md` records a local Codex
Computer Use tester pass. The captured window opened as `A-Life Alpha Playground
- smoke 60s`, selected `GpuPlastic`, showed the
`CpuShadowGuardedStaticPlusLiveHShadow` claim, displayed stable-ID creature and
food markers, showed read-only inspector/GPU status, sealed 16 patches, and
closed cleanly through the smoke timeout.

This pass is not an independent human external alpha. Computer Use key input
could not verify Space/N/1/2/3/F/Esc interactions because the target window
reported `foreground window did not report a process id`. A local screenshot
was captured under `target/playtest_evidence/external_alpha_001/screenshots/`
and remains untracked. The main findings are medium-severity inspector clipping,
unverified manual controls, and partial hazard visibility in the P34 fixture.

## Launch Command Used

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

The command is the current recommended first-tester smoke. It requests the
optional GPU mode and exits through the bounded smoke timeout.

## Local Rehearsal Result

Command result: pass, exit code 0.

Observed launcher/runtime summary:

- Window title: `A-Life Alpha Playground - smoke 30s`
- GPU mode requested: `static-plastic-cpu-shadow-guarded`
- Selected GPU runtime: `GpuPlastic`
- Product claim: `CpuShadowGuardedStaticPlusLiveHShadow`
- GPU scores used for proposals: `true`
- CPU shadow parity: `true`
- H_shadow applications: 1
- Fallback: `None`
- Objects: 2
- Creatures: 1
- Food: 1
- Stable IDs: `true`
- Sealed patches: 16
- Packed logs: 16
- App close behavior: bounded smoke exited cleanly

The local Vulkan loader emitted warnings about a missing validation layer and a
deprecated GOG Galaxy overlay layer manifest. The command still completed
successfully and selected the real GPU-backed path. These warnings are local
environment noise, not release-wide GPU evidence.

## Forced Fallback Rehearsal

Command:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Command result: pass, exit code 0.

Observed launcher/runtime summary:

- Window title: `A-Life Alpha Playground - smoke 10s`
- GPU mode requested: `static-plastic-cpu-shadow-guarded`
- Selected runtime: `CpuReference`
- Product claim: `None`
- GPU scores used for proposals: `false`
- CPU shadow parity: `false`
- H_shadow applications: 0
- Fallback: `HardwareUnavailable`
- Objects: 2
- Creatures: 1
- Food: 1
- Stable IDs: `true`
- Sealed patches: 10
- Packed logs: 10
- App close behavior: bounded smoke exited cleanly

This confirms forced CPU fallback remains available and does not claim GPU work.

## Checklist Evidence

| Check | Evidence status | Notes |
| --- | --- | --- |
| Window opened | Pass, local rehearsal | The bounded smoke path launched the graphical window and exited cleanly. |
| Creature marker visible | Pass by local smoke summary | The smoke reported one creature and stable-ID-backed visible objects. External screenshot evidence is still missing. |
| Food marker visible | Pass by local smoke summary | The smoke reported one food object. External screenshot evidence is still missing. |
| Hazard marker or guide visible | Manual evidence missing | The P34 fixture path uses the alpha hazard guide unless a richer fixture is loaded; this needs tester screenshot/video evidence. |
| Pause/run/step controls worked | Manual evidence missing | The command prints controls, but no external tester manually exercised Space or `N` in this pass. |
| GPU mode or CPU fallback visible | Pass, local rehearsal | GPU mode and fallback status were reported in both normal and forced-fallback smoke summaries. External screenshot evidence is still missing. |
| Inspector readable | Manual evidence missing | The current app includes the read-only inspector, but this pass did not capture external tester readability evidence. |
| App closed cleanly | Pass, local rehearsal | Both bounded smoke runs exited with code 0. |
| Screenshot/video evidence path | Manual evidence missing | No media was captured or committed in this pass. |

## Findings

### BLOCKER

- None found in local rehearsal.

### HIGH

- None found in local rehearsal.

### MEDIUM

- External tester evidence is still missing, including screenshots/video and
  manual confirmation that pause/run/step controls are understandable.
- Hazard evidence remains guide-based for the P34 tiny fixture until a tester
  captures the visible guide or a richer fixture is used.

### LOW

- Local Vulkan loader warnings mention a missing validation layer and a
  deprecated GOG Galaxy overlay Vulkan layer manifest. They did not block the
  smoke run, but testers should record whether the same warnings appear.

### MANUAL_EVIDENCE_MISSING

- External tester machine evidence.
- External screenshot/video evidence.
- Manual pause/run/step/follow/quit interaction evidence.
- External inspector readability evidence.

## Release/Tag Recommendation

Defer release and tagging unless explicitly approved later. The current evidence
supports moving to a small external alpha tester loop, not public release
readiness. The GPU runtime claim remains `CpuShadowGuardedStaticPlusLiveHShadow`;
this is not full action-authoritative GPU runtime.

## Next Recommendation

Run the same checklist with one or more external alpha testers. Ask testers to
capture the initial window, the GPU/fallback panel, the read-only inspector, and
at least one pause/run/step interaction, then record findings using
`docs/productization/FIRST_GRAPHICAL_ALPHA_PLAYTEST_CHECKLIST.md`.
