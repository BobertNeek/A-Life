# CA56 - Continual learning and non-catastrophic adaptation

## Phase
L

## Mode
Full Spec Loop

## Review class
R3

## Objective
Measure and improve continual learning over changing tasks.

## Dependencies
CA55

## Owned scope
core/tools/world.

## Required work
- Task sequences.
- Forgetting metrics.
- Adaptation metrics.
- Sleep/consolidation interventions.
- No hidden replay action forcing.

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
CA56 receipt
Plan: Continual learning and non-catastrophic adaptation
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
Next plan(s): CA57
```
