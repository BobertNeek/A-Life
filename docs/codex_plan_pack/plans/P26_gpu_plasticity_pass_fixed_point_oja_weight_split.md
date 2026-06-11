# P26 - GPU plasticity pass, fixed-point/Oja, weight split

Group: Group 3 - GPU serial

Branch: `codex/P26-gpu-plasticity-oja`

Prerequisites: P25

Concurrency: No. Depends on static parity.

Next plan(s): P27

## Purpose

Add GPU plasticity safely after static parity exists. The pass must update plastic traces without corrupting genetic or lifetime baseline layers.

## Owned scope

- `alife_gpu_backend` pass 3 shader, fixed-point/stochastic tests, parity docs.

## Required implementation steps

1. Implement pass 3 plasticity update reading finalized pre/post activations and writing H_shadow only. One thread must own one synapse weight slot to avoid write conflicts.
2. Implement effective weight semantics consistent with core: genetic fixed + lifetime consolidated + alpha * H_operational. GPU may pack these as separate buffers but must not collapse lifetime learning into genetic weights.
3. Implement or stub behind feature the low-precision path: INT8/INT16 traces, 32-bit intermediate accumulation, fixed-point scale, stochastic rounding with deterministic LFSR seed for tests. If first implementation uses float H_shadow, keep contract ready for low precision and document.
4. Add overflow/saturation diagnostics and clamps. Validate scale factors before dispatch.
5. Compare GPU H_shadow updates against CPU reference on tiny fixtures with tolerance.
6. Ensure pass 3 uses static finalized activations from pass 2 and does not create read/write hazards.
7. Add long-run small fixture test for saturation/bias if feasible as ignored/manual extended test.
8. Update traceability for Oja/plasticity and weight split.

## Required tests and validation

- Tests for no update when alpha=0, update when alpha>0, genetic/lifetime buffers unchanged, H_shadow changed, clamp behavior, deterministic LFSR if implemented, and CPU/GPU parity on tiny fixtures.
- Shader compile and GPU parity tests as available.

## Acceptance criteria

- GPU can update plastic traces without corrupting inherited/lifetime baseline layers.
- Read/write hazards are avoided by pass separation.
- Fixed-point policy is explicit and test-covered.

## Failure handling

- If INT8 path is too risky for first pass, deliver float path plus exact low-precision contract/tests marked pending. Do not pretend INT8 is complete.
- If parity tolerance is large, investigate quantization; do not loosen tolerance silently.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P26 - GPU plasticity pass, fixed-point/Oja, weight split
Branch: codex/P26-gpu-plasticity-oja
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P27
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
