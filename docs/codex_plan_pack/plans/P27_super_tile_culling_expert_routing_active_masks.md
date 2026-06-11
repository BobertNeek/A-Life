# P27 - Super-tile culling, expert routing, active masks

Group: Group 3 - GPU serial

Branch: `codex/P27-gpu-supertile-routing`

Prerequisites: P26

Concurrency: No. Depends on GPU forward/plasticity stability.

Next plan(s): P28

## Purpose

Add hierarchical culling and expert routing as performance optimizations that preserve CPU-equivalent behavior.

## Owned scope

- GPU routing/mask modules, shaders, CPU/GPU parity tests.

## Required implementation steps

1. Implement hierarchical culling: 16x16 microtiles, 8x8 microtile supertiles, packed 32-bit supertile mask words, and early exit in pass 1 where a supertile is inactive.
2. Integrate lobe routing masks from P05 so only active source-target lobe pathways dispatch or execute.
3. Add active tile execution masks derived from current lobe update frequency, sensory activity, and biological compute budget.
4. Implement CPU reference culling parity: culling must skip work but produce identical outputs to uncullable sparse reference for inactive-zero regions.
5. Add instrumentation counters for skipped supertiles/microtiles without requiring gameplay readback.
6. Document dispatch strategy and mask word indexing.
7. Add tests for boundary indices and 32-word packing.
8. Update traceability for supertile culling and expert routing.

## Required tests and validation

- Unit tests for supertile index math, mask packing/unpacking, all-zero cull, active tile pass-through, lobe routing integration, and CPU parity.
- GPU parity tests for masked vs unmasked outputs where available.

## Acceptance criteria

- Culling changes performance, not behavior.
- Routing is driven by core lobe data, not duplicated GPU-only constants.
- Mask encoding is tested at edge boundaries.

## Failure handling

- If dispatch-level culling is hard, implement shader early-exit first and document dispatch optimization for later.
- If instrumentation needs readback, keep it outside active gameplay or behind diagnostics.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P27 - Super-tile culling, expert routing, active masks
Branch: codex/P27-gpu-supertile-routing
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P28
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
