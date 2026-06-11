# P31 - ETF/Neural Collapse metrics and sensory prototype tooling

Group: Group 4 - Research optional parallel

Branch: `codex/P31-etf-neural-collapse-metrics`

Prerequisites: P30, P08, P14

Concurrency: Yes. Can run with P32/P33 after P30/core.

Next plan(s): P35, P36

## Purpose

Implement optional ETF/neural-collapse analysis and prototype generation for representation geometry without burdening gameplay.

## Owned scope

- Offline tools/research modules; optional sensory prototype assets.

## Required implementation steps

1. Implement Simplex ETF prototype generation for fixed sensory classes/affordances. Keep it as offline/tooling or static asset generation unless runtime already expects the table.
2. Implement metrics for representation geometry: class mean alignment, within-class variance, between-class simplex angle, NC-style summary statistics, and drift over time from packed logs or exported activations.
3. Add sensory-lobe initialization/export helper that writes versioned prototype tables for P08/P14 to consume as static assets if configured.
4. Add tests for ETF geometry: unit norm, centered prototypes, equiangular dot products within tolerance, stable deterministic generation, and schema version.
5. Document that ETF/neural collapse tools regularize/analyze representations; they are not required for normal runtime.
6. Add optional command to analyze P18/P19 traces if activations are available.
7. Update traceability for ETF/neural collapse optional tools.
8. Do not add heavy ML training dependencies to the core runtime.

## Required tests and validation

- Unit tests for ETF math and metrics.
- Tool smoke test on tiny synthetic embeddings.
- Build without research feature confirms runtime unaffected.

## Acceptance criteria

- ETF prototype generation and NC metrics exist as optional offline tools.
- Generated assets are versioned and deterministic.
- No research dependency is required by core gameplay.

## Failure handling

- If full neural collapse metrics need activation exports not yet present, implement synthetic/tool-level metrics and add export TODO for P35/P36.
- If Python ML stack is unavailable, implement core geometry in Rust.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P31 - ETF/Neural Collapse metrics and sensory prototype tooling
Branch: codex/P31-etf-neural-collapse-metrics
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P35, P36
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
