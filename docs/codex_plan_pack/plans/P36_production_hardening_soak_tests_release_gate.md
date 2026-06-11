# P36 - Production hardening, soak tests, release gate

Group: Group 5 - Release final

Branch: `codex/P36-production-hardening`

Prerequisites: P35

Concurrency: No. Final gate.

Next plan(s): None - this is the final release gate.

## Purpose

Run the final hardening gate: soak tests, audits, release checklist, performance reporting, and known limitation documentation.

## Owned scope

- CI/release docs, soak tests, hardening fixes across repo.

## Required implementation steps

1. Create a release checklist covering format, check, tests, clippy, boundary checks, golden traces, scenario suite, save/load round-trip, benchmark smoke, GPU parity/manual if available, docs, examples, and artifact generation.
2. Add long-run soak tests for headless CPU: many ticks, repeated sleep/wake, memory/topology bounded growth, no NaN, no invalid IDs, no unsealed learning, and deterministic replay where configured. Mark extended tests ignored/manual if too slow for CI.
3. Add GPU soak/performance test plan and scripts for machines with supported hardware. Include how to record tier results and parity summaries.
4. Audit public APIs for versioning, docs, and accidental TODO/panic/unimplemented in non-test runtime paths.
5. Audit feature flags: default build should be stable; optional features should compile independently where possible.
6. Audit logs/saves/assets for schema versions and migration/rejection behavior.
7. Write final architecture status report: implemented, partial, optional, known limitations, performance results, and next research directions.
8. Tag or prepare release candidate only after validations pass.

## Required tests and validation

- Full validation suite from `VALIDATION_PROTOCOL.md`.
- Extended soak tests manual/ignored if too slow.
- Docs/examples compile checks.
- Boundary/dependency audit.
- Performance tier report where hardware available.

## Acceptance criteria

- Repository has a defensible release gate.
- Long-run behavior does not corrupt learning state.
- Known limitations are explicit rather than hidden.
- No next plan is required; future work can be tracked as issues.

## Failure handling

- If a release gate fails, do not downgrade the gate. Fix the issue or document as a known blocker with exact reproduction.
- If performance target is not met, preserve correctness and record optimization backlog.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P36 - Production hardening, soak tests, release gate
Branch: codex/P36-production-hardening
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): None - this is the final release gate.
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
