# P11 - Packed experience logging and side buffers

Group: Group 1 - Core logging serial

Branch: `codex/P11-packed-logging`

Prerequisites: P10

Concurrency: Mostly no for core; P30 can later branch from this.

Next plan(s): P12, P13, P30

## Purpose

Build stable export/logging without damaging runtime cognition structs. Packed frames are for tools and replay; runtime ExperiencePatch remains rich and validated.

## Owned scope

- `alife_core` packed log module, schema constants, tests; optional `alife_tools` reader stubs only if tiny.

## Required implementation steps

1. Define `PackedExperienceFrame` as a fixed-size, versioned, intentionally lossy export struct. Use `repr(C)` only if layout tests justify it. Include schema version, flags, creature ID, tick, position/heading, drive/hormone arrays, selected action ID, success flag, target IDs, valence/reward, pain/energy deltas, total salience, and side-buffer offsets/counts.
2. Define side buffer contracts for variable-length visible entities, touched entities, heard tokens, salience clusters, memory links, concept links, and optional semantic codes.
3. Define `ExperiencePacker` that consumes sealed `ExperiencePatch` only. It must not accept partial phases.
4. Define schema version and compatibility checks. Add rejection behavior for unsupported schema.
5. Add byte-size/layout tests if bytemuck or zerocopy is used. Avoid unsafe derives unless every field is layout-safe.
6. Add a minimal reader/writer trait or in-memory writer. File IO can wait for P30/P34 unless already simple.
7. Document which runtime fields are intentionally lossy in packed form.
8. Update traceability for runtime/log split and packed schema.

## Required tests and validation

- Tests for fixed frame size, schema version, packing from sealed patch, rejection of unsealed/invalid patch, side-buffer indexing, variable-length data not stored inline, and round-trip of minimal frame metadata.
- Workspace tests and boundary script.

## Acceptance criteria

- Packed logging cannot contaminate runtime `ExperiencePatch` layout.
- Variable-length data lives in side buffers.
- Offline tools have a stable schema to target.

## Failure handling

- If safe zero-copy is not achievable, prefer explicit binary serialization with tests over unsafe layout shortcuts.
- If existing logs use a different schema, add migration/rejection docs rather than silently changing interpretation.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P11 - Packed experience logging and side buffers
Branch: codex/P11-packed-logging
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P12, P13, P30
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
