# P22 - Semantic/Gaussian adapter

Group: Group 2 - Adapter optional parallel

Branch: `codex/P22-semantic-gaussian-adapter`

Prerequisites: P08, P10

Concurrency: Yes. Can run after sensory/experience contracts; avoid core edits.

Next plan(s): P35

## Purpose

Add optional semantic/Gaussian perception as an adapter, not as core world truth. This keeps advanced perception useful but non-mandatory.

## Owned scope

- `alife_semantic` or adapter crate; optional feature flags and tests.

## Required implementation steps

1. Implement adapter-side representations for semantic/Gaussian clusters, salience scoring, egocentric bin hashing, and compressed semantic codes. Keep renderer/3DGS internals out of core.
2. Convert adapter cluster observations into core optional Gaussian/semantic context snapshots.
3. Implement feature flags so the project builds and runs without semantic/Gaussian support.
4. Add confidence/bounds validation and graceful absence behavior.
5. Add simple fake semantic provider for tests and headless scenarios.
6. Do not implement full renderer or Gaussian splat system unless already present; this plan is the boundary/adapter.
7. Add docs explaining 3DGS/semantic context is optional perceptual input, not authoritative world state.
8. Update traceability for optional semantic/Gaussian boundary.

## Required tests and validation

- Tests for conversion to core context, optional absence, stable cluster IDs, salience sorting/capping, compressed code bounds, and build without feature.
- Core boundary script.

## Acceptance criteria

- Semantic/Gaussian data can enrich sensory context without coupling core to rendering.
- Missing semantic provider is not fatal.
- Future playground can enable the adapter cleanly.

## Failure handling

- If no real 3DGS source exists, provide a fake provider and interface only. Do not invent renderer-heavy code.
- If semantic codes are underspecified, use fixed `[i8; 32]` descriptors and versioned schema.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P22 - Semantic/Gaussian adapter
Branch: codex/P22-semantic-gaussian-adapter
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P35
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
