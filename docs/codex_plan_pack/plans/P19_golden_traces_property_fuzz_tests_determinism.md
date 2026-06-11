# P19 - Golden traces, property/fuzz tests, determinism

Group: Group 2 - Validation serial after scenarios

Branch: `codex/P19-golden-determinism`

Prerequisites: P18

Concurrency: Can run with P20 after P18 if careful.

Next plan(s): P20, P21, P24

## Purpose

Add regression and property coverage so future GPU/adapters/research work can prove they preserve core behavior.

## Owned scope

- Test infrastructure, fixtures, deterministic trace files.

## Required implementation steps

1. Define a golden trace format for sealed ExperiencePatch summaries and key world/neural state summaries. Do not serialize full huge matrices unless needed.
2. Capture golden traces for the P18 scenarios and store them in fixtures with schema versions.
3. Add deterministic replay tests: same seed and config produce same trace summary; different seed changes only expected stochastic fields.
4. Add property/fuzz-style tests for bounded drives/hormones, no NaN, monotonic ticks, valid IDs, sealed-only memory/topology/logging, and action validation.
5. Add snapshot update workflow documentation so future intentional changes are reviewed, not blindly overwritten.
6. Add minimal shrink/failure diagnostics for trace mismatches.
7. Ensure tests run without GPU/Bevy unless feature flags explicitly request them.
8. Update traceability for determinism and golden traces.

## Required tests and validation

- Golden replay tests for all scenario traces.
- Property tests or randomized loops over validation functions.
- Workspace tests; if proptest/quickcheck dependency is added, document and justify.

## Acceptance criteria

- Core behavior has regression coverage.
- Future GPU/adapters can compare against golden CPU behavior.
- Validation catches state corruption before learning/logging.

## Failure handling

- If golden traces are too brittle, store semantic summaries and tolerances rather than raw floats.
- If property testing slows CI too much, bound case count and add a separate extended test command.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P19 - Golden traces, property/fuzz tests, determinism
Branch: codex/P19-golden-determinism
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P20, P21, P24
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
