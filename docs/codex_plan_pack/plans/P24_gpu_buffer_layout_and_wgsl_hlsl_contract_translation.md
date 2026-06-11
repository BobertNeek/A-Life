# P24 - GPU buffer layout and WGSL/HLSL contract translation

Group: Group 3 - GPU serial

Branch: `codex/P24-gpu-buffer-contracts`

Prerequisites: P14, P15

Concurrency: No for GPU path. Start after CPU reference and sparse schema.

Next plan(s): P25

## Purpose

Freeze the GPU data contract before writing serious shaders. The GPU backend must consume the CPU schema, not invent a parallel architecture.

## Owned scope

- `alife_gpu_backend` buffer/schema modules, shader contract docs, tests.

## Required implementation steps

1. Translate the CPU sparse projection schema into GPU buffer layouts: tile metadata, supertile masks, packed indices, fixed/lifetime/alpha/H buffers, activation ping-pong buffers, atomic accumulators, routing descriptors, and diagnostics buffers.
2. Define host-side buffer structs with explicit alignment/size rules. Avoid unsafe bytemuck/Pod unless every field is safe and tested.
3. Define WGSL contract docs for pass 0 clear, pass 1 SpMV projection, pass 2 activation finalization, pass 3 plasticity update, and later culling/recompaction.
4. Add schema conversion from CPU fixtures to GPU upload buffers without creating a device yet if that keeps tests portable.
5. Define fixed-point scale policy, activation clamp, overflow flag schema, and acceptable CPU/GPU tolerance policy.
6. Define no-readback gameplay rule and allowed diagnostic/export staging points.
7. Add shader files as stubs or compile-checked modules if existing infrastructure supports it. The implementation of actual compute passes comes in P25/P26.
8. Update traceability for GPU contracts.

## Required tests and validation

- Tests for buffer layout sizes/alignment, CPU fixture to GPU buffer conversion, tile metadata encoding, mask encoding, fixed-point scale validation, and schema version compatibility.
- Compile/check GPU crate; no actual GPU required if possible.
- Core boundary script.

## Acceptance criteria

- GPU backend has a clear data contract matching CPU sparse schema.
- No shader work can invent incompatible buffers later.
- No-readback rule is documented and testable at API level.

## Failure handling

- If WGSL compilation requires unavailable tooling, add text/schema tests and document manual validation. Do not claim shader runtime works yet.
- If layout portability is uncertain, prefer explicit serialization/packing functions over transmuting structs.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P24 - GPU buffer layout and WGSL/HLSL contract translation
Branch: codex/P24-gpu-buffer-contracts
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P25
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
