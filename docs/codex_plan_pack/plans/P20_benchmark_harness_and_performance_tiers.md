# P20 - Benchmark harness and performance tiers

Group: Group 2 - Validation/performance

Branch: `codex/P20-benchmark-tiers`

Prerequisites: P17, P18

Concurrency: Can run with P19 after P18.

Next plan(s): P21, P24, P29, P36

## Purpose

Start measuring the stated performance tiers early. Even if CPU-only tiers are slow, the harness defines the target and prevents vague performance claims.

## Owned scope

- Benchmarks, metrics, performance docs; no premature optimization.

## Required implementation steps

1. Create a benchmark harness for tiers 1, 10, 50, 100, 250, and 500 agents. Early CPU-only benchmarks may mark upper tiers as expected-slow; the key is consistent measurement.
2. Define metrics: tick time, memory usage estimate, patch throughput, memory/topology update time, neural projection time, sleep consolidation time, and scenario success metrics.
3. Add configurable update rates and biological compute budget fields: essential vs non-essential lobes, throttling thresholds, and fallback update frequency.
4. Add a benchmark report generator or simple Markdown output under `target/`/artifacts, not committed except baseline docs.
5. Integrate with CI as smoke benchmarks only; full performance tests can be manual/ignored.
6. Add performance budget docs for CPU reference vs expected GPU backend.
7. Ensure benchmark data does not require Bevy/GPU unless explicitly feature-gated later.
8. Update traceability for performance targets and tiers.

## Required tests and validation

- Smoke benchmark test that runs tier 1 and maybe tier 10 quickly.
- Unit tests for compute budget/throttling policy data.
- Manual/ignored benchmark command documented for larger tiers.

## Acceptance criteria

- The project can measure progress against 60 FPS/tier goals instead of guessing.
- Throttling policies exist as data before GPU integration.
- Benchmarks do not destabilize normal CI.

## Failure handling

- If Criterion or another dependency is added, keep it dev-only.
- If CPU cannot handle high tiers, document expected failure and leave GPU parity to P24-P29.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P20 - Benchmark harness and performance tiers
Branch: codex/P20-benchmark-tiers
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P21, P24, P29, P36
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
