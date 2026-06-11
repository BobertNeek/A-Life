# P03 - Core IDs, math primitives, units, stable adapter boundary

Group: Group 0 - Baseline serial

Branch: `codex/P03-core-ids-math`

Prerequisites: P02

Concurrency: No. This is the foundation for all core contracts.

Next plan(s): P04

## Purpose

Create the engine-independent identity and math layer that every contract will use. This is the hard boundary that prevents Bevy, Avian, wgpu, or renderer types from leaking into core cognition.

## Owned scope

- `alife_core` ID/math modules and tests; adapter conversion stubs only if needed.

## Required implementation steps

1. Define stable IDs in core: `CreatureId`, `WorldEntityId`, `GaussianClusterId`, `ConceptCellId`, `MemoryId`, `GenomeId`, `BrainClassId`, `ActionId`, and any missing typed index wrappers. Use transparent newtypes with derives for Debug, Clone/Copy where safe, Eq/Hash where safe, and serde if the crate already supports it.
2. Define engine-independent math primitives or wrappers: `Vec2f`, `Vec3f`, `Quatf`, `Aabb`, `Pose`, `Velocity`, bounded scalar helpers, and conversion traits. If using `glam`, document why it is acceptable and ensure Bevy types are not imported.
3. Define simulation units: ticks, seconds, duration ticks, normalized scalar, signed valence, confidence, intensity, and fixed-point scale wrappers where useful.
4. Add validation helpers for finite floats, normalized ranges, nonzero/valid IDs, monotonic ticks, and safe optional target handling.
5. Create adapter-boundary traits or conversion modules that future Bevy/Gaussian adapters can implement without pulling adapters into core.
6. Replace existing core structs that use raw engine IDs/math with these stable types, but only within the small contract surface already present.
7. Document the rule: Bevy Entity maps to `WorldEntityId` outside core.
8. Update traceability rows for engine-independent IDs and math.

## Required tests and validation

- Unit tests for ID equality, hashing, serialization if enabled, default invalid IDs if applicable, math conversion, finite-value validation, and range rejection.
- Boundary script proving no Bevy/wgpu/Avian imports in core.
- `cargo test -p alife_core ids math` or equivalent module-specific tests plus workspace tests.

## Acceptance criteria

- Every core contract can refer to stable IDs and math without Bevy/wgpu.
- Invalid/NaN values can be rejected consistently.
- Adapter crates have a clear conversion path but no reverse dependency from core.

## Failure handling

- If existing code depends on Bevy math in core, replace with core math and move conversion later to P21.
- If serde/bytemuck derives conflict with safe invariants, prefer explicit conversion over unsafe layout promises.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P03 - Core IDs, math primitives, units, stable adapter boundary
Branch: codex/P03-core-ids-math
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P04
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
