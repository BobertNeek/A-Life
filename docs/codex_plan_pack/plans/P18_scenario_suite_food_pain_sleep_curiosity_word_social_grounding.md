# P18 - Scenario suite: food, pain, sleep, curiosity, word, social grounding

Group: Group 2 - Harness serial after P17

Branch: `codex/P18-scenario-suite`

Prerequisites: P17, P12, P13, P16

Concurrency: Can branch after P17 but likely easier serial before P19/P20.

Next plan(s): P19, P20, P23

## Purpose

Turn isolated systems into meaningful behavioral scenarios that exercise perception, action, outcome, memory, topology, sleep, language, and social signals.

## Owned scope

- Scenario tests/fixtures in `alife_world` or integration tests.

## Required implementation steps

1. Build a scenario fixture library with named deterministic scenarios: food-seeking, poison/pain avoidance, obstacle frustration, fatigue/sleep, curiosity from contradiction, word-token grounding, simple social trust/fear, and teacher perception event.
2. Each scenario must define initial world state, creature genome/config, sensory timeline or object layout, expected broad behavior, expected patch fields, and expected memory/topology changes.
3. Add assertion helpers that check causal patch fields without brittle exact neural scores where the implementation is still evolving.
4. Implement food scenario: hunger increases food salience, eat action reduces hunger/increases reward when target edible exists.
5. Implement pain/poison scenario: harmful object creates pain/fear/cortisol and memory expectancy/danger bias on repeat.
6. Implement sleep scenario: fatigue triggers sleep, consolidation changes lifetime/plastic state without mutating genetic baseline.
7. Implement curiosity scenario: expected reward mismatch creates unresolved gap and raises curiosity salience.
8. Implement word/social scenarios at minimal level using sensory/language/social contexts; do not require SLM.

## Required tests and validation

- One integration test per scenario plus a combined smoke test.
- Golden patch shape assertions: pre-action, decision, outcome, memory/topology update.
- Determinism tests across repeated runs with same seed.

## Acceptance criteria

- Behavior harness proves the core architecture works on meaningful stimuli.
- Scenarios cover the learning loop, not just isolated structs.
- Teacher/word/social inputs remain perception/modulatory only.

## Failure handling

- If exact behavior is unstable, assert invariants and directionality rather than exact action every tick. Add golden traces later in P19.
- If a scenario needs missing mechanics, create a small explicit TODO with owning future plan instead of expanding scope wildly.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P18 - Scenario suite: food, pain, sleep, curiosity, word, social grounding
Branch: codex/P18-scenario-suite
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P19, P20, P23
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
