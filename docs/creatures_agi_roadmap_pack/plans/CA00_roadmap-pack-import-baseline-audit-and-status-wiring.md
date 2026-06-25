# CA00 - Roadmap pack import, baseline audit, and status wiring

## Phase
A

## Mode
Micro-Spec

## Review class
R0

## Objective
Import this pack and verify the current repo baseline without implementing game features.

## Dependencies
None

## Owned scope
Docs only.

## Required work
- Copy pack under docs/creatures_agi_roadmap_pack.
- Create status/progress placeholder if absent.
- Verify current main and productization docs.
- Record baseline: GPU graphical alpha, current claim, known UX gaps.
- Stop before CA01.

## Forbidden scope
- Do not create S12/G25/P37. Do not tag a release. Do not weaken validation. Do not commit screenshots/logs/target artifacts. Do not leak Bevy/wgpu/GPU deps into alife_core. Do not claim full action-authoritative GPU runtime unless this exact plan owns and proves that claim.


## Required tests and evidence
- docs_check, standard validation, cargo tree -p alife_core

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
CA00 receipt
Plan: Roadmap pack import, baseline audit, and status wiring
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
Next plan(s): CA01
```
