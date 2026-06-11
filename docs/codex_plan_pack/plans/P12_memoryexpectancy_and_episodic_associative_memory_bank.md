# P12 - MemoryExpectancy and episodic associative memory bank

Group: Group 1 - Parallel after P10/P11

Branch: `codex/P12-memory-expectancy`

Prerequisites: P10

Concurrency: Yes. Can run with P13 after P10; P11 useful but not strictly required unless logs are touched.

Next plan(s): P15, P16, P18

## Purpose

Implement memory as interpretation bias, not action replay. This preserves flexible behavior and prevents brittle copying of old motor commands.

## Owned scope

- `alife_core` memory module and tests; may add hooks to experience but not action replay.

## Required implementation steps

1. Define `MemoryRecord`, `MemoryQuery`, `MemoryMatch`, `MemoryBankConfig`, bounded ring storage, and `MemoryExpectancy` output.
2. `MemoryExpectancy` must include expected valence, predicted drive deltas, predicted sensory/outcome summary, affordance bias, danger/safety bias, social trust/fear bias, novelty/curiosity bias, confidence, source memory IDs, and no selected action replay field.
3. Implement deterministic in-memory ring insertion from sealed `ExperiencePatch` and query from current pre-action context.
4. Implement a simple cosine-like or normalized dot-product matching over bounded feature vectors. Keep it CPU-reference simple; optimize later only if tests preserve behavior.
5. Implement empty/no-match expectancy with neutral values and low confidence.
6. Implement caps, eviction policy, and validation to avoid unbounded memory growth.
7. Add interfaces for sleep consolidation to compress/merge memories later in P16.
8. Update traceability for memory expectancy and no replay.

## Required tests and validation

- Tests for empty recall, top-k matching, deterministic eviction, bounded values, insertion from sealed patch only, no selected action in expectancy, predicted drive delta behavior, and confidence thresholds.
- Regression test: a memory of an action must bias affordance/valence but cannot directly output that action as command.
- Workspace tests and boundary script.

## Acceptance criteria

- Memory recall biases interpretation rather than motor output.
- Memory storage is bounded and deterministic.
- CPU reference loop can query expectancy before decision.

## Failure handling

- If feature-vector design is uncertain, implement a minimal documented vector projection with extension points.
- If memory needs action IDs for learning analysis, store them in `MemoryRecord` but do not expose them through `MemoryExpectancy` as replay commands.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P12 - MemoryExpectancy and episodic associative memory bank
Branch: codex/P12-memory-expectancy
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P15, P16, P18
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
