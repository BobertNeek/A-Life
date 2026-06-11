# P10 - Three-phase ExperiencePatch runtime contract

Group: Group 1 - Core integration serial

Branch: `codex/P10-experience-three-phase`

Prerequisites: P05, P06, P07, P08, P09

Concurrency: No. Integrates P05-P09; run after their branches merge.

Next plan(s): P11, P12, P13, P15, P21, P23

## Purpose

Create the causal event spine of the project. Every later learning, memory, topology, logging, and teacher verifier component depends on sealed three-phase experience.

## Owned scope

- `alife_core` experience module and tests; may touch action/sensory/drives docs for integration only.

## Required implementation steps

1. Replace thin/header-only ExperiencePatch with the full causal model: `PreActionSnapshot`, `DecisionSnapshot`, `PostActionOutcome`, and sealed `ExperiencePatch`.
2. `PreActionSnapshot` must include creature ID, tick, body/pose, drives, hormones, sensory snapshot, memory expectancy placeholder or actual type if P12 stub exists, optional Gaussian/semantic context, social/language context, and relevant brain/genome/ABI IDs.
3. `DecisionSnapshot` must include proposals, selected action command, rejected top proposal, arbitration trace, action ABI version, decision timestamp/tick, and confidence/validation status.
4. `PostActionOutcome` must include success/failure, physical/collision result, drive deltas, hormone deltas, reward/valence, frustration/pain/energy deltas, prediction error, contradiction flags, concept/memory hints, and optional teacher feedback observed through perception.
5. Implement an `ExperiencePatchBuilder` or equivalent that enforces pre -> decision -> outcome order and refuses missing phases, non-monotonic ticks, mismatched creature IDs, incompatible ABI versions, or invalid bounded values.
6. Define sealed/validated patch semantics. Downstream memory/topology/learning/logging must accept only sealed patches.
7. Preserve runtime richness. Do not add `repr(C)`/zero-copy promises to this runtime struct.
8. Update traceability rows for causal patch assembly and validation.

## Required tests and validation

- Tests for successful deterministic assembly, missing phase rejection, out-of-order rejection, mismatched creature ID rejection, ABI mismatch rejection, invalid drive/hormone rejection, optional context absence, and no packed-layout assumptions.
- Tests that selected action belongs to the decision phase and outcome values are post-action only.
- Workspace tests and boundary script.

## Acceptance criteria

- A full patch can be assembled deterministically from three phases.
- No downstream code can accidentally learn from partial/unsealed experience.
- Runtime patch remains separate from packed logs.
- Core remains engine-independent.

## Failure handling

- If P12 MemoryExpectancy is not implemented yet, define a minimal placeholder type or trait boundary but do not implement memory recall here.
- If this causes broad compile failures, add transitional constructors only when they preserve causal order.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P10 - Three-phase ExperiencePatch runtime contract
Branch: codex/P10-experience-three-phase
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P11, P12, P13, P15, P21, P23
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
