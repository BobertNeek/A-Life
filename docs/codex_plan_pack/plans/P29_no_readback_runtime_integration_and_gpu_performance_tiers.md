# P29 - No-readback runtime integration and GPU performance tiers

Group: Group 3 - GPU final integration

Branch: `codex/P29-gpu-runtime-performance`

Prerequisites: P28, P20

Concurrency: No. Final GPU integration gate.

Next plan(s): P34, P35, P36

## Purpose

Integrate GPU runtime, performance tiers, fallback behavior, and the no-readback rule into a usable backend.

## Owned scope

- GPU runtime integration, benchmarks, diagnostics, docs.

## Required implementation steps

1. Integrate GPU backend into runtime behind feature/config so CPU reference remains available and authoritative for tests.
2. Implement backend selection: CPU reference, GPU static, GPU plastic, GPU full where available. Fallback to CPU on unsupported hardware or validation failure.
3. Enforce no synchronous device-to-host readback in active gameplay path. Add API/design tests or instrumentation that make forbidden readback hard to introduce.
4. Run performance tiers from P20 using GPU where available. Record tier results, bottlenecks, and whether 60 FPS target is met for each tier on the test machine.
5. Implement throttling based on GPU timing budget: if neural GPU time exceeds threshold, non-essential association lobes drop update frequency while sensory/motor priority stays higher.
6. Add diagnostics export after frame/sleep boundaries only.
7. Document hardware requirements, feature flags, known GPU limitations, and parity tolerances.
8. Update traceability for final GPU performance and no-readback guardrail.

## Required tests and validation

- Backend selection tests, fallback tests, no-readback guard tests where possible, benchmark smoke tests, CPU/GPU parity regression tests, and throttling policy tests.
- Manual performance reports for tiers if hardware-dependent.

## Acceptance criteria

- GPU backend is usable without replacing the CPU oracle.
- Performance is measured against stated tiers.
- Forbidden readbacks are documented and guarded.

## Failure handling

- If performance misses target, report honestly and preserve correctness. Do not remove parity tests to improve numbers.
- If no GPU is available, complete integration gates that can run CPU-side and leave manual hardware validation instructions.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P29 - No-readback runtime integration and GPU performance tiers
Branch: codex/P29-gpu-runtime-performance
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P34, P35, P36
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
