# P17 - Headless world and behavior harness

Group: Group 2 - Harness parallel

Branch: `codex/P17-headless-world-harness`

Prerequisites: P15

Concurrency: Yes. Can run with P21, P23, P24 after dependencies.

Next plan(s): P18, P19, P20

## Purpose

Build a deterministic headless world so the brain can be tested on actual behavior before Bevy/GPU integration.

## Owned scope

- `alife_world` or equivalent headless simulation crate; test fixtures.

## Required implementation steps

1. Create a pure/headless world harness independent of Bevy rendering. It should provide simple entities, positions, affordances, food, hazards, obstacles, agents, tokens/words, and time/tick progression.
2. Implement stable mapping from world objects to `WorldEntityId` and sensory snapshots through core adapter traits.
3. Implement action execution for core `ActionCommand`: move, inspect, eat, rest/sleep, approach, flee, grab, vocalize, idle, and failure for missing affordance/invalid target.
4. Implement outcome measurement: collisions, success/failure, drive/hormone deltas, reward/valence, pain/energy/frustration, touched entities, and simple social outcomes.
5. Provide deterministic scenario builder APIs with seed control.
6. Connect the harness to CPU reference brain stepping so tests can run multi-tick creature loops without Bevy/GPU.
7. Add simple telemetry collection of sealed patches for tests and later offline tools.
8. Update traceability for behavior harness.

## Required tests and validation

- Tests for sensory gathering, stable ID mapping, action execution, missing affordance failure, food reward, pain penalty, rest/sleep trigger, deterministic seed replay, and sealed patch collection.
- Workspace tests and core boundary script.

## Acceptance criteria

- A headless deterministic world can drive the CPU reference brain.
- Core contracts are exercised in realistic tick loops.
- No Bevy/GPU is required for behavior tests.

## Failure handling

- If action taxonomy is incomplete, implement minimal behaviors needed for scenarios and document pending verbs.
- Do not introduce engine-specific components into core to make the harness easier.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P17 - Headless world and behavior harness
Branch: codex/P17-headless-world-harness
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P18, P19, P20
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
