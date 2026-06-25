# CA02 - GPU alpha fixture with creature, food, hazard, and obstacle

## Phase
A

## Mode
Micro-Spec

## Review class
R1

## Objective
Add or select a tiny deterministic alpha fixture that contains a real hazard and obstacle, not just a hazard guide.

## Dependencies
CA01

## Owned scope
alife_world fixtures/persistence, alife_game_app launcher/config/tests.

## Required work
- Create versioned alpha fixture or scenario manifest.
- Include creature, food, hazard, obstacle, stable IDs, deterministic seed.
- Keep P34 compatibility.
- Graphical default may use alpha fixture; old P34 tests remain.
- Document fixture semantics.

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
CA02 receipt
Plan: GPU alpha fixture with creature, food, hazard, and obstacle
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
Next plan(s): CA03
```
