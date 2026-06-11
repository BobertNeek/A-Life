# P14 - Neural state, sparse projection schema, CPU math kernels

Group: Group 1 - Core neural substrate

Branch: `codex/P14-neural-state-projection-schema`

Prerequisites: P05, P06, P07, P08

Concurrency: Can start after P05-P08 merge; independent of P12/P13.

Next plan(s): P15, P24

## Purpose

Create the CPU sparse neural substrate and projection schema that will become the oracle for GPU parity. This is correctness before acceleration.

## Owned scope

- `alife_core` neural state/projection/sparse tile modules and tests.

## Required implementation steps

1. Define CPU reference neural state for one creature: activation buffers, previous activation buffers, lobe views, projection descriptors, weight split views, plasticity traces, and update metadata. Keep storage small/deterministic for tests.
2. Define sparse projection schema shared with GPU later: microtile 16x16, supertile 8x8 microtiles/128x128 macro grid, tile metadata, tile type enum, COO/dense/row/column variants if needed, supertile masks, routing projection references, and alignment rules.
3. Implement CPU reference SpMV over the sparse schema using `W_effective = W_genetic_fixed + W_lifetime_consolidated + alpha * H_operational`.
4. Implement CPU reference activation finalization with clamps and configurable activation function.
5. Implement CPU reference Oja/H-shadow update using safe floats first. Low-precision GPU approximations come later.
6. Implement overflow/range diagnostic hooks matching the spec concepts even if CPU float path cannot overflow the same way.
7. Add conversion from lobe/routing config to projection schema for small deterministic fixtures.
8. Update traceability for sparse projection, weight split, and CPU math.

## Required tests and validation

- Tests for microtile/supertile indexing, mask culling behavior, dense and COO tile decoding, CPU SpMV correctness on tiny matrices, effective weight formula, activation clamp, Oja update bounds, and lobe/routing projection validation.
- Tests for 16/128 alignment constraints from P05.
- Workspace tests and boundary script.

## Acceptance criteria

- CPU has a deterministic sparse neural math substrate.
- GPU backend can later reuse the same schema instead of inventing one.
- Weight split semantics are enforced by tests.

## Failure handling

- If implementing all tile formats is too much, implement dense and COO first, but define enum variants and mark unimplemented formats with explicit errors/tests.
- Do not optimize before correctness. CPU reference is the oracle.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P14 - Neural state, sparse projection schema, CPU math kernels
Branch: codex/P14-neural-state-projection-schema
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P15, P24
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
