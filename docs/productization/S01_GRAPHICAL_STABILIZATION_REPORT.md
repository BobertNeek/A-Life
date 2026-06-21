# S01 Graphical Stabilization Report

Status: implemented and locally smoke-tested.

## Launch Commands

Persistent interactive window:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1
```

Bounded smoke window:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 5
```

Dry run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun
```

The script now launches the feature-gated Bevy app path instead of running the
old `visible-world-smoke` command and exiting immediately.

## What Is Visible

The local graphical run opened a persistent window titled:

```text
A-Life Graphical Playground
```

The window displayed the P34 tiny world fixture with:

- a visible creature placeholder labeled `stable:1 agent`
- a visible food/resource placeholder labeled `stable:2 berry`
- a diagnostic overlay showing fixture, deterministic seed, mode, and backend
- explicit `Backend: CPU Reference fallback` status
- instructions to close the window

This is the smallest real graphical shell for the product. It is not a polished
gameplay UI.

## Local Evidence

Graphics actually ran on this Windows machine.

Captured evidence is local and intentionally untracked:

```text
target/playtest_evidence/S01/screenshots/001_graphical_playground_window.png
target/playtest_evidence/S01/logs/graphical_persistent_stderr.log
target/playtest_evidence/S01/logs/graphical_persistent_stdout.log
```

Computer Use detected the target window:

```text
process:...\target\debug\alife_game_app.exe
title: A-Life Graphical Playground
```

Native Computer Use window screenshot capture still failed on this Windows 10
host with:

```text
SetIsBorderRequired failed: No such interface supported (0x80004002)
```

The active-window `Alt+Print` fallback captured a bounded app screenshot at
`1402x914`, not the full multi-monitor desktop.

## Runtime Notes

The graphical launch path produced Vulkan/wgpu environment warnings related to
missing validation layers and a GOG Galaxy overlay layer manifest. The window
still launched and rendered successfully. These warnings are local environment
diagnostics, not a GPU acceleration claim.

The graphical shell does not require the A-Life GPU neural runtime. The displayed
backend status is CPU reference fallback.

## What Remains Missing

This S01 slice intentionally stops at a persistent visible window. The following
normal-player surfaces remain incomplete or not yet exposed as graphical UI:

- camera pan/zoom/orbit controls
- creature selection interaction
- inspector/debug panels beyond the static overlay
- pause/step/resume UI controls
- save/load UI
- school/teacher UI
- semantic provider UI
- GPU diagnostics/status panel beyond CPU fallback text
- full gameplay feedback loop without reading logs

## Known Limitations

- Bevy/graphics remains optional and feature-gated.
- Headless CPU validation remains the default correctness path.
- GPU performance remains manual/unknown unless measured by the P29/P36 evidence
  commands on supported hardware.
- Computer Use native screenshot capture remains unavailable on this Windows 10
  build; the reliable evidence path is the active-window `Alt+Print` clipboard
  fallback.
- S01 does not claim release readiness.

## Next Recommended Stage

The next productization stage should be graphical interaction and UX controls:

1. camera movement
2. pause/step/run
3. creature selection
4. inspector/debug overlay controls
5. save/load entry point

That stage should still keep the headless CPU path as the oracle and preserve
feature-gated graphics.
