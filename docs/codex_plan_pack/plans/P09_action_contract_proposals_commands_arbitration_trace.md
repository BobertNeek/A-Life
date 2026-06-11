# P09 - Action contract, proposals, commands, arbitration trace

Group: Group 1 - Parallel core contracts

Branch: `codex/P09-action-arbitration`

Prerequisites: P04

Concurrency: Yes. Can run with P05, P06, P07, P08 after P04.

Next plan(s): P10, P15, P21, P23

## Purpose

Upgrade motor output from thin tokens to structured decisions. The core brain must preserve target, intensity, duration, confidence, source drives, and arbitration evidence.

## Owned scope

- `alife_core` action module and tests.

## Required implementation steps

1. Define `ActionKind`/`ActionId` registry semantics without assuming one-byte output as the core contract. The GPU motor ring may emit compact tokens later, but core must expose structured commands.
2. Define `ActionProposal`: action ID/kind, score, confidence, source lobe, drive-source mask, target candidate, salience, inhibition neighborhood/ring index if applicable, and rationale/debug fields behind optional feature if needed.
3. Define `ActionCommand`: action ID, optional target entity ID, optional target position, intensity, duration ticks, confidence, source mask, optional teacher/lesson response channel, optional speech/writing/vocal motor payload reference, and ABI version.
4. Define `ActionDecision`: selected command, rejected top proposal, ranked top proposals, fallback reason, and validation status.
5. Define `ActionArbitrationTrace`: reciprocal inhibition inputs/outputs, WTA result, thresholds, ties, suppressed proposals, and deterministic tie-breaking seed/index.
6. Implement a CPU reference arbitration function: bounded scores, reciprocal inhibition or simple WTA baseline, deterministic tie break, fallback idle/inspect/rest action if no proposal passes threshold.
7. Ensure memory expectancy can bias proposal scores later without injecting raw replay actions.
8. Update traceability rows for structured action and arbitration.

## Required tests and validation

- Tests for structured command fields, target position/intensity/duration/confidence validation, deterministic WTA, tie-breaking, fallback action, invalid target handling, and source mask preservation.
- Tests that teacher/lesson metadata does not bypass selection.
- Workspace tests and boundary script.

## Acceptance criteria

- Action output is expressive enough for Bevy/Avian adapter and logs.
- A compact GPU token can later be decoded into the structured command rather than replacing it.
- Arbitration is deterministic and testable.

## Failure handling

- If existing code has a thin `ActionCommand`, migrate it without breaking old tests; add compatibility constructors if necessary.
- If action taxonomy is incomplete, define a minimal registry and leave specific gameplay verbs to P17/P18.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P09 - Action contract, proposals, commands, arbitration trace
Branch: codex/P09-action-arbitration
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P10, P15, P21, P23
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
