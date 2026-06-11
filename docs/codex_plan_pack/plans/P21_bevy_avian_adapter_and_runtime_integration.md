# P21 - Bevy/Avian adapter and runtime integration

Group: Group 2 - Adapter parallel

Branch: `codex/P21-bevy-avian-adapter`

Prerequisites: P10, P17

Concurrency: Yes. Can run with P23/P24 after P15/P17; keep branch isolated.

Next plan(s): P35

## Purpose

Connect the pure core to Bevy/Avian through adapters only. This gives the project a game-engine path without letting engine types leak backward.

## Owned scope

- `alife_bevy_adapter` crate only, plus adapter tests/examples.

## Required implementation steps

1. Implement `BevyEntity <-> WorldEntityId` mapping table in the adapter crate. Do not put Bevy Entity in core.
2. Implement Bevy components/resources that mirror core state through stable IDs: creature body, affordance tags, sensory emitter, action sink, patch telemetry, and sleep/drive debug data.
3. Implement sensory adapter from Bevy/Avian world state to core `SensorySnapshot` and context streams. Keep conversion explicit and testable.
4. Implement action adapter from core `ActionCommand` to Bevy/Avian movement/kinematic/animation commands. Include failure feedback for missing targets/affordances.
5. Implement plugin scheduling that respects causal order: gather sensory, CPU brain tick, execute action, measure outcome, seal patch/update brain. If Bevy scheduling makes exact sequence hard, use explicit system sets and tests.
6. Add a small Bevy example or app feature-gated if rendering dependencies are heavy.
7. Add adapter tests for ID mapping and pure conversion functions. Full app tests can be smoke/ignored if CI lacks graphics.
8. Update traceability for Bevy adapter.

## Required tests and validation

- Unit tests for entity map, conversion to/from core math, sensory conversion, action conversion, and missing entity handling.
- Compile check for adapter crate with Bevy feature.
- Core boundary script proving no reverse dependency.

## Acceptance criteria

- Bevy integration consumes core contracts without contaminating them.
- World execution returns outcomes that can seal ExperiencePatch records.
- Adapter can be disabled without breaking core/headless tests.

## Failure handling

- If Avian API details are unstable, isolate behind a small adapter trait/module and compile-gate it.
- If CI cannot run Bevy app tests, keep pure conversion tests in CI and mark app smoke tests ignored/manual.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P21 - Bevy/Avian adapter and runtime integration
Branch: codex/P21-bevy-avian-adapter
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P35
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
