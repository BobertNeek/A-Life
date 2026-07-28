# CA01 - GPU-first visible playable loop

## Phase
A

## Mode
Full Spec Loop

## Review class
R2

## Objective
Make Space/N visibly change the graphical game and reframe the default view as a GPU-first creature simulation.

## Dependencies
CA00

## Owned scope
alife_game_app Bevy shell, UI overlays, scripts, productization docs.

## Required work
- Default graphical alpha title/status says A-Life GPU Alpha Playground.
- GPU mode is primary; CPU fallback appears as degraded mode.
- Space/N visibly update state, action, event feed, and tick.
- Hide or explain TerminalInvalidState.
- Add clear player-facing status/event feed.

## Forbidden scope
- Do not create S12/G25/P37. Do not tag a release. Do not weaken validation. Do not commit screenshots/logs/target artifacts. Do not leak Bevy/wgpu/GPU deps into alife_core. Do not claim full action-authoritative GPU runtime unless this exact plan owns and proves that claim.


## Required tests and evidence
- 30s graphical GPU smoke
- forced fallback smoke
- graphical-controls-smoke

## Focused commands
- Use relevant existing smoke commands for this plan.
- If this plan changes graphical behavior, run graphical smoke and forced fallback.
- If this plan changes GPU behavior, run GPU runtime/timing commands where supported.

## Validation
Run full validation from VALIDATION_PROTOCOL.md. Also run plan-specific focused commands. Use Windows wrappers; never plain bash.

## Review checklist
- Scope matches this plan only.
- No forbidden new plan or release tag.
- No tracked artifacts.
- `alife_core` dependency tree is clean.
- Player-facing claims match evidence.
- GPU/CPU claims are honest.
- Tests cover the new behavior.
- Documentation updated when behavior or commands changed.

## Receipt
```text
CA01 receipt
Plan: GPU-first visible playable loop
Branch:
Files changed:
Runtime code changed:
Core APIs changed:
Docs changed:
Public APIs changed:
Tests added/changed:
Focused evidence:
Commands run:
Validation results:
Invariant checks:
Known limitations:
Artifacts tracked:
Release/tag status:
alife_core dependency status:
Main status:
Next plan(s): CA02
```
