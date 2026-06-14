# Schema and Versioning Conventions

Status: v0 scaffold convention.

A-Life public data formats must be explicitly versioned before they cross crate, process, GPU, save-file, or log boundaries. Version markers are part of the contract, not comments.

## Scope

Versioned schemas include:

- Runtime ABI markers such as sensory, action, and experience formats.
- Packed log/export formats.
- GPU buffer layouts and WGSL binding contracts.
- Save/load manifests and migrations.
- Lineage export manifests.
- Offline tool input and output files.

## Rules

- Central versions live in `crates/alife_core/src/version.rs` through
  `SchemaVersions::CURRENT`.
- Use small integer `abi_version` or `schema_version` fields for serialized and packed structs.
- Keep rich runtime cognition structs separate from packed logging/export structs.
- Reject unknown future major versions unless an explicit migration exists.
- Bump the relevant schema version for any breaking field layout, semantic meaning, unit, scale, or ordering change.
- Record breaking changes in `docs/codex_progress/DECISION_LOG.md` and update `docs/codex_progress/SPEC_TRACEABILITY.md`.
- Tests must cover rejection or conversion for breaking changes once the schema has executable readers.

## Naming

- `AbiVersion` names are for in-memory or CPU/GPU boundary contracts.
- `SchemaVersion` names are for persisted files, logs, manifests, and offline tool exchange.
- `CURRENT` constants identify the active version for scaffold code.

## Non-goals

This convention does not implement broad migrations yet. P34 adds executable
save/config/asset schema rejection plus a documented migration hook that
rejects until a future plan supplies a tested conversion path.
