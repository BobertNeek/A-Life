# P02 - Spec traceability matrix, decision/progress logs, invariant test harness

Group: Group 0 - Baseline serial

Branch: `codex/P02-traceability-invariants`

Prerequisites: P01

Concurrency: No. Run before contract branches.

Next plan(s): P03

## Purpose

Turn the specs into enforceable gates. Codex needs a traceability matrix and invariant tests so future work can prove it preserved the architecture rather than merely claiming it did.

## Owned scope

- Docs under `docs/codex_progress/`, test utilities, invariant test skeletons.

## Required implementation steps

1. Expand `SPEC_TRACEABILITY.md` into a real matrix covering all major spec obligations: core purity, three-phase experience, action command richness, memory expectancy, topology, genome/weight split, CPU reference, behavior harness, GPU parity, no readback, packed logging, teacher boundary, offline tools, save/load, and release gates.
2. Create a lightweight invariant test module or integration test location that future plans can extend. Do not overfit to current placeholder names; make it easy to add checks.
3. Add a dependency-boundary test or script that fails if forbidden dependencies appear in `alife_core` Cargo tree or manifests.
4. Add a schema/versioning convention document under `docs/architecture/` or equivalent.
5. Add a PR/completion receipt template if none exists.
6. Make the progress log machine-readable enough for Codex to identify completed plans. A simple Markdown table is acceptable; JSON can be added if useful.
7. Update the decision log with any path/convention choices made in P00-P02.
8. Do not implement domain contracts yet. This plan creates the rails.

## Required tests and validation

- Run docs/scripts validation added in P01.
- Run `cargo test --workspace --all-targets` if code tests were added.
- Run the boundary script and confirm it fails on a temporary forbidden-pattern fixture only if such a fixture is implemented safely.

## Acceptance criteria

- Traceability exists for every major spec area and names an owning plan.
- Future plans have a place to add invariant tests.
- Boundary checks are automated, not just written in prose.
- Progress logs can identify P00-P02 as complete.

## Failure handling

- If dependency-tree parsing is brittle, use both Cargo metadata and manifest grep. Prefer false positives that force review over silent leaks.
- If docs paths differ, keep a clear redirect file so future agents find the logs.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P02 - Spec traceability matrix, decision/progress logs, invariant test harness
Branch: codex/P02-traceability-invariants
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P03
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
