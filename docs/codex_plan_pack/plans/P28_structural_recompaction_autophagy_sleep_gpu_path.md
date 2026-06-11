# P28 - Structural recompaction, autophagy, sleep GPU path

Group: Group 3 - GPU serial

Branch: `codex/P28-gpu-recompaction-autophagy`

Prerequisites: P27, P16

Concurrency: No. High conflict/risk; run serially.

Next plan(s): P29

## Purpose

Add structural recompaction/autophagy at safe boundaries. Active simulation must remain allocation-free and must not mutate active buffers mid-dispatch.

## Owned scope

- GPU backend structural edit buffers, sleep/recompaction pipeline, tests.

## Required implementation steps

1. Implement double-buffered matrix metadata concept: active buffer A, scratch buffer B, staged structural edits, frame/sleep boundary swap. Do not mutate active buffer mid-dispatch.
2. Compile structural edit batches from P16 CPU sleep consolidation into GPU scratch buffers. Keep active gameplay loops allocation-free.
3. Implement or stub with tests autophagic pruning kernel/path: wear/byproduct counters, threshold/quorum trigger, low-salience pruning, BrainATP recovery signal, byproduct decay.
4. Implement sleep/offscreen compaction path: clear activations, shadow registry drain/decay, concept/memory compaction hooks as CPU-owned operations, and GPU buffer update scheduling.
5. Add validation that structural edits preserve lobe/routing constraints, tile alignment, and buffer capacity.
6. Add safe swap API with explicit state machine: idle, uploading, ready_to_swap, active, failed.
7. Add recovery behavior for upload failure or invalid structural edit: keep active buffer, reject scratch buffer, record diagnostic.
8. Update traceability for recompaction/autophagy/sleep GPU.

## Required tests and validation

- Tests for structural edit validation, scratch/active swap state machine, invalid edit rejection, capacity bounds, no active mutation, byproduct decay/pruning if implemented, and CPU/GPU schema consistency.
- Manual/ignored GPU tests for actual buffer swap if hardware needed.

## Acceptance criteria

- Structural changes happen safely at sleep/frame boundaries.
- GPU backend can receive consolidation outputs without corrupting active simulation.
- Autophagy/recovery concepts are represented and tested.

## Failure handling

- If actual GPU recompaction is too much, deliver host-side validated buffer rebuild and swap API first. Do not claim dynamic shader-side allocation.
- If autophagy math is underspecified, implement deterministic reference thresholds and document as v1 policy.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P28 - Structural recompaction, autophagy, sleep GPU path
Branch: codex/P28-gpu-recompaction-autophagy
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P29
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
