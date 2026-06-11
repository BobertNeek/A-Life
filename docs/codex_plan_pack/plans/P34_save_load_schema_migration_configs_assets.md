# P34 - Save/load, schema migration, configs, assets

Group: Group 5 - Product integration

Branch: `codex/P34-save-load-config-assets`

Prerequisites: P29, P30, P33

Concurrency: Mostly serial; can start partial config earlier but final after GPU/tools.

Next plan(s): P35

## Purpose

Make the project persistent and configurable. Saves, configs, and assets must be versioned and validated before product integration.

## Owned scope

- Save/load modules, config files, asset manifests, migration tests.

## Required implementation steps

1. Define save-state boundaries: genome, development state, creature mind summary, drives/hormones, memory bank, topology map, lifetime consolidated weights, H traces if needed, world state, school state, and backend config.
2. Do not serialize engine-local IDs directly. Save stable IDs plus adapter remap tables where needed.
3. Define schema versions and migration/rejection strategy for saves, packed logs, generated weight assets, and config files.
4. Implement config system for backend selection, brain class, benchmark tier, feature flags, school/teacher, semantic adapter, GPU limits, logging, and deterministic seed.
5. Implement asset manifest for generated weights, ETF prototypes, scenario configs, and example worlds.
6. Add load validation: reject incompatible schemas, invalid IDs, NaN/out-of-bound values, and missing required assets with clear diagnostics.
7. Add small fixture saves/assets only. Do not commit huge generated tensors.
8. Update traceability for persistence and configs.

## Required tests and validation

- Tests for save/load round-trip of tiny world/creature, schema rejection, migration if any, stable ID remapping, config parsing, asset manifest validation, and feature-flag defaults.
- Workspace tests with default features and all-features if feasible.

## Acceptance criteria

- The project can persist and restore deterministic small worlds.
- Schema/version failures are explicit, not silent corruption.
- Runtime assets are discoverable and validated.

## Failure handling

- If migration support is premature, implement rejection plus documented migration hook.
- If save files become too large, split bulk tensors into asset blobs and save references/digests.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P34 - Save/load, schema migration, configs, assets
Branch: codex/P34-save-load-config-assets
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
