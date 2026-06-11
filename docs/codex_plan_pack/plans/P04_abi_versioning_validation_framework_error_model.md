# P04 - ABI versioning, validation framework, error model

Group: Group 0 - Baseline serial

Branch: `codex/P04-abi-validation-errors`

Prerequisites: P03

Concurrency: No. Required before parallel domain contracts.

Next plan(s): P05, P06, P07, P08, P09

## Purpose

Create shared versioning, validation, and error handling before parallel branches define domain contracts. This prevents every branch from inventing incompatible error/version patterns.

## Owned scope

- `alife_core` ABI/version modules, validation traits, error types, test helpers.

## Required implementation steps

1. Create a central schema/version module with constants for sensory ABI, action ABI, experience schema, packed log schema, genome schema, neural projection schema, save schema, and teacher/school schema.
2. Define `Validate`/`Validated` patterns or functions used by all contracts. Validation should return typed errors, not strings only.
3. Define error enums for contract validation, missing phase data, invalid ID, non-finite scalar, out-of-range drive/hormone, incompatible ABI, packed-log schema mismatch, and backend parity errors.
4. Add a compact diagnostic type suitable for logs and test assertions.
5. Add helper macros/functions for version checks without hiding errors.
6. Update existing placeholder ABI uses to point at the central module.
7. Add documentation stating when to bump a version and when to add a migration.
8. Update decision/progress/traceability logs.

## Required tests and validation

- Unit tests for version compatibility, rejection of incompatible ABI, validation error display/debug, and conversion from validation errors where used.
- `cargo test -p alife_core abi validation error` or equivalent.
- Full workspace check after replacing placeholder ABI references.

## Acceptance criteria

- All future public contracts can share validation and versioning conventions.
- Errors are typed and testable.
- Breaking schema changes have an explicit process.

## Failure handling

- If this plan exposes too many public error variants, keep the top-level enum stable and use private/internal detail enums.
- If existing tests assume old ABI constants, update tests and document the compatibility decision.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P04 - ABI versioning, validation framework, error model
Branch: codex/P04-abi-validation-errors
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P05, P06, P07, P08, P09
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
