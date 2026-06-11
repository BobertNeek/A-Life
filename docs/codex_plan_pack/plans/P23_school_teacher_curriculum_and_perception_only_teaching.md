# P23 - School/Teacher curriculum and perception-only teaching

Group: Group 2 - School parallel

Branch: `codex/P23-school-teacher`

Prerequisites: P09, P10, P12, P13

Concurrency: Yes. Can run with P17/P21 after core contracts.

Next plan(s): P18, P35

## Purpose

Implement school/teacher systems that teach through perception and feedback. This enforces the no-hidden-vector/no-direct-motor-bypass boundary.

## Owned scope

- `alife_school` crate and tests; optional hooks into headless harness.

## Required implementation steps

1. Define teacher roles, lesson IDs, curriculum steps, prompts/cues, expected observations, verifier checks, reward/feedback events, and lesson response channels.
2. Implement teacher inputs strictly as perception/context events: spoken tokens, gestures, object highlighting, social feedback, reward/punishment signals visible to the creature. No hidden vector injection and no direct action command injection.
3. Define `LessonResponse` and optional metadata used by `ActionCommand` without bypassing arbitration.
4. Implement simple curriculum runner for headless harness: name object, offer food, discourage poison, request approach/grab/vocalize, verify observed behavior through sealed patches.
5. Implement verifier interfaces that inspect ExperiencePatch logs and topology/memory summaries.
6. Add school/teacher schema versioning and docs.
7. Add tests proving teacher cannot directly select action and hidden vector injection flag is false/absent.
8. Update traceability for teacher boundary.

## Required tests and validation

- Tests for curriculum progression, perception event injection, verifier pass/fail, no direct motor bypass, lesson response metadata in action decisions, and interaction with memory/topology.
- Workspace tests and core boundary script.

## Acceptance criteria

- School can teach through ordinary perception and feedback.
- Verifier can grade behavior using sealed patches.
- Teacher boundary is enforced by tests.

## Failure handling

- If lessons require mechanics not in P17/P18, add fake harness events or mark those lessons pending. Do not change core arbitration to satisfy a lesson.
- If SLM is desired, leave it for P31/P35 optional integration; this plan is not an LLM integration plan.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P23 - School/Teacher curriculum and perception-only teaching
Branch: codex/P23-school-teacher
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P18, P35
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
