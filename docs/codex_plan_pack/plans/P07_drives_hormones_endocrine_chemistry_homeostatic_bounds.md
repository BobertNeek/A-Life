# P07 - Drives, hormones, endocrine chemistry, homeostatic bounds

Group: Group 1 - Parallel core contracts

Branch: `codex/P07-drives-hormones`

Prerequisites: P04

Concurrency: Yes. Can run with P05, P06, P08, P09 after P04.

Next plan(s): P10, P14, P15

## Purpose

Define the bounded homeostatic state that modulates cognition, learning, action confidence, sleep, and recovery. Without this, the brain loop will produce untestable raw floats.

## Owned scope

- `alife_core` drives, hormones, chemistry/homeostasis modules and tests.

## Required implementation steps

1. Define bounded drive vector types. Include hunger, fatigue, fear, pain, loneliness, curiosity, energy/BrainATP, temperature stress if present, and extension slots if the spec expects fixed-width arrays.
2. Define bounded hormone/endocrine vector types. Include adrenaline, cortisol, dopamine, oxytocin, serotonin, and learning/modulation fields needed by Oja/homeostasis.
3. Define deltas separately from snapshots. A snapshot is state at a time. A delta is an outcome/change. Do not mix them in the same struct without names.
4. Implement homeostatic update functions for simple deterministic baseline drift, clamping, decay, spike injection, and drive-threshold modulation. Keep first version simple and testable.
5. Define recovery triggers for seizure/hyperactivity, catatonia/energy hypoplasia, fatigue sleep entry, pain/frustration spikes, and safe idle fallback.
6. Expose modulation helpers that later neural code can use: threshold scale, learning-rate scale, salience weighting, and motor confidence adjustment.
7. Validate all finite/range constraints through P04 validation framework.
8. Update traceability for endocrine vectors, bounds, and recovery protocols.

## Required tests and validation

- Tests for clamping/rejection policy, finite value rejection, decay/spike behavior, fatigue sleep threshold, pain/frustration update, BrainATP lower/upper bounds, and recovery trigger detection.
- Property-style tests if feasible: random finite inputs never produce NaN/out-of-range outputs.
- Workspace tests and boundary script.

## Acceptance criteria

- Drives and hormones are explicit typed state, not raw anonymous arrays everywhere.
- All learning-relevant values can be validated before sealing experience or updating weights.
- Recovery triggers exist as data/contracts before runtime integration.

## Failure handling

- If exact biological equations are underspecified, implement conservative linear decay/spike functions and document them as reference defaults.
- Do not embed Bevy time/resources; use tick/delta inputs from core math.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P07 - Drives, hormones, endocrine chemistry, homeostatic bounds
Branch: codex/P07-drives-hormones
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P10, P14, P15
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
