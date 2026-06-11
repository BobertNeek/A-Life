# P35 - Full playground, examples, UX docs

Group: Group 5 - Product integration

Branch: `codex/P35-playground-examples-docs`

Prerequisites: P21, P22, P23, P29, P34

Concurrency: Serial integration branch; can parallelize docs only after APIs stable.

Next plan(s): P36

## Purpose

Create the usable playground and developer-facing examples after the contracts, harnesses, adapters, GPU path, and persistence are stable.

## Owned scope

- Examples, playground app, docs, tutorials, developer UX.

## Required implementation steps

1. Create a full playground application that can run with CPU backend and optionally GPU backend. It should support at least one creature, simple food/hazard objects, sleep, patch logging, and visible drive/hormone/action debug state.
2. Integrate Bevy/Avian adapter if available, semantic adapter if feature enabled, school/teacher if feature enabled, and save/load/config system.
3. Add example configs for CPU-only, GPU-enabled, school lesson, benchmark tier, and offline log export.
4. Add user/developer docs: architecture overview, how to run, how to test, how to add an action, how to add a sensory channel, how to add a lesson, how to run benchmarks, how to read logs, and how to generate optional research assets.
5. Add troubleshooting docs for dependency leaks, GPU unavailable, schema mismatch, nondeterminism, and bad generated assets.
6. Add smoke examples that CI can at least compile. Runtime app smoke can be manual if graphics unavailable.
7. Ensure default path works without optional GPU/semantic/D2NWG/SLM features.
8. Update traceability for final integration.

## Required tests and validation

- Compile examples/default app.
- Run CPU-only playground smoke test or headless equivalent.
- Run docs link/path checks if available.
- Run full workspace tests/default and all-features as feasible.

## Acceptance criteria

- A developer can clone the repo and run a visible or headless playground with clear instructions.
- Optional features are documented and gated.
- Examples exercise the actual contracts, not mock-only paths.

## Failure handling

- If graphical app cannot run in CI, provide a headless app smoke test and manual graphics instructions.
- If optional adapters are incomplete, feature-gate them and document their status honestly.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P35 - Full playground, examples, UX docs
Branch: codex/P35-playground-examples-docs
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P36
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
