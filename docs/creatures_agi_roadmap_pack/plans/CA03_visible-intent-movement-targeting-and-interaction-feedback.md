# CA03 - Visible intent, movement, targeting, and interaction feedback

## Phase
A

## Mode
Full Spec Loop

## Review class
R2

## Objective
Make creature intent and world interaction visibly legible.

## Dependencies
CA02

## Owned scope
Bevy presentation and app summary; no core action semantics unless direct bug.

## Required work
- Show target line or intent marker from creature to food/hazard.
- Show action badge: approach/eat/flee/inspect/sleep/idle.
- Show food interaction cue and hazard cue.
- Use stable-ID driven presentation only.
- Event feed shows last 5 meaningful events.

## Forbidden scope
- Do not create S12/G25/P37. Do not tag a release. Do not weaken validation. Do not commit screenshots/logs/target artifacts. Do not leak Bevy/wgpu/GPU deps into alife_core. Do not claim full action-authoritative GPU runtime unless this exact plan owns and proves that claim.


## Required tests and evidence
- Add or update tests for the user-visible behavior touched by this plan.
- Preserve existing smoke tests.
- Record focused evidence in docs if this plan changes product behavior.

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
CA03 receipt
Plan: Visible intent, movement, targeting, and interaction feedback
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
Next plan(s): CA04
```
