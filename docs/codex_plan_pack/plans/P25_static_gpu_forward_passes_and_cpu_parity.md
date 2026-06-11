# P25 - Static GPU forward passes and CPU parity

Group: Group 3 - GPU serial

Branch: `codex/P25-gpu-static-forward-parity`

Prerequisites: P24

Concurrency: No. GPU parity work should remain serial.

Next plan(s): P26

## Purpose

Implement the first real GPU acceleration path with CPU parity: static forward projection and activation finalization only.

## Owned scope

- `alife_gpu_backend` WGSL passes 0-2, device test harness, parity fixtures.

## Required implementation steps

1. Implement pass 0 clear accumulators, pass 1 static SpMV projection using read-only effective weights or precomputed effective weights for first parity, and pass 2 activation finalize with clamps.
2. Use tiny deterministic fixtures first: one agent, small brain/lobe ranges, dense and COO tiles, known activations, known outputs.
3. Add optional GPU test harness using wgpu. Tests that require an actual adapter should be feature-gated or ignored in CI if no GPU is available; CPU-side packing tests stay in CI.
4. Compare GPU outputs against CPU reference from P14 within documented tolerance.
5. Verify no synchronous readback is needed inside active tick API. Parity tests may read back after dispatch for validation only.
6. Add shader compile validation where possible.
7. Document dispatch dimensions, workgroup sizes, and limitations.
8. Update traceability for static GPU parity.

## Required tests and validation

- CPU-only tests for fixture packing.
- GPU/manual/ignored parity tests for pass 0-2 outputs.
- Tests for activation clamp and mask skip behavior if implemented here.
- Workspace checks with GPU feature on/off.

## Acceptance criteria

- Static GPU forward path matches CPU reference on deterministic fixtures.
- GPU tests are reproducible and do not block non-GPU CI unless intentionally configured.
- No plasticity or structural recompaction is mixed into this milestone.

## Failure handling

- If GPU environment is unavailable, implement shader code plus compile/packing tests and mark runtime parity manual with exact command.
- If floating-point tolerance fails, diagnose scale/clamp/layout before changing CPU reference.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P25 - Static GPU forward passes and CPU parity
Branch: codex/P25-gpu-static-forward-parity
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P26
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
