# CA12 - Asset bundle automation and ingestion

## Phase
C

## Mode
Micro-Spec

## Review class
R1

## Objective
Make assets/shaders/config bundles discoverable and versioned for the app.

## Dependencies
CA11

## Owned scope
tools/app/docs/scripts.

## Required work
- Central app manifest.
- Bundle validation command.
- Shader asset discovery.
- Tiny placeholder art manifest.
- No large binary assets.

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
CA12 receipt
Plan: Asset bundle automation and ingestion
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
Next plan(s): CAR12
```
