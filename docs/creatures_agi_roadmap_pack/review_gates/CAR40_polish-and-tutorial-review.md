# CAR40 - Polish and tutorial review

## Type
Review gate / consultation checkpoint.

## Mode
Review Gate

## Review class
R3 — stop for user/ChatGPT consultation unless the verdict is clearly PASS and the manifest permits continuing.

## Reviews plans
CA37-CA40

## Objective
Review whether a first-time player can understand and enjoy a 5-10 minute session.

## Required audit

- Read all receipts and changed files from the reviewed tranche.
- Confirm scope compliance.
- Confirm global invariants.
- Confirm validation evidence.
- Confirm no overclaims.
- Confirm no hidden S12/G25/P37/release-tag creation.
- Confirm no artifacts tracked.
- Confirm `alife_core` dependency tree is clean.
- Review user-facing evidence if the tranche touched graphics/playability.
- Review safety boundaries if the tranche touched school/semantic/AGI research.
- Produce a verdict.

## Verdicts

- `PASS`
- `PASS_WITH_NOTES`
- `FIX_REQUIRED`
- `BLOCKER`

## Required report

Create:

`docs/creatures_agi_roadmap_pack/reviews/CAR40_REVIEW_REPORT.md`

Report fields:
- Scope reviewed
- Files inspected
- Commands run
- Findings by severity
- Invariant status
- User-facing status
- Evidence gaps
- Fix prompt if needed
- Next plan recommendation

## Validation

Run the standard validation protocol. Add focused validation for the reviewed tranche.

## Hard stop

Stop after this gate. Ask the user to paste the report into ChatGPT for review unless the user explicitly allowed autonomous continuation past this review gate.

## Receipt

```text
CAR40 review receipt
Verdict:
Files reviewed:
Commands run:
Results:
Findings:
Fix required:
Next plan:
Stopped for consultation: yes/no
```
