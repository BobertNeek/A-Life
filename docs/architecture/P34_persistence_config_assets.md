# P34 Persistence, Config, and Asset Contracts

Status: v1 portable persistence contract.

P34 defines versioned save files, runtime config files, and asset manifests for
deterministic small-world persistence. The implementation lives in
`alife_world::persistence` with a tiny `alife_tools` validator. It does not add
tooling, GPU, Bevy, Avian, renderer, or OS-window dependencies to `alife_core`.

## Portable Save Boundary

P34 save files use schema `alife.p34.save_file.v1` and
`SchemaVersions::CURRENT.save`. They store:

- deterministic seed and config references
- stable world entity IDs, object summaries, positions, and tick
- creature IDs, genome IDs, brain class, homeostatic snapshot, and summaries of
  memory/topology/sleep diagnostics
- lifetime/H-trace counts and generated-weight asset references
- school state only as portable public state
- adapter remap tables that may mention stable slots, not engine-local handles

Main save files intentionally do not store bulk neural matrices or generated
weight tensors. Those are asset references with digests.

## Schema and Migration Policy

Current schemas load directly. Unknown future schemas, incompatible old schemas,
and mismatched config/manifest schemas reject before partial loading. P34
includes a migration hook type that deliberately reports unsupported migration
until a future plan adds a tested migration.

## Stable ID Policy

Portable saves serialize `WorldEntityId`, `OrganismId`, `GenomeId`, and related
core IDs. They do not serialize Bevy `Entity`, Avian handles, wgpu handles,
renderer handles, OS-window handles, or similar engine-local values. Adapter
remap entries are validation-scoped tables for future adapters and reject common
engine-local token shapes such as `Entity(...)`.

## Config Policy

Runtime configs use schema `alife.p34.runtime_config.v1`. Defaults are CPU
reference, deterministic, no active gameplay readback, and safe for headless
validation. GPU selections require the GPU feature flag and CPU fallback. School
teacher options require school to be enabled. Semantic providers are optional
unless a config explicitly names them as required.

## Asset Manifest Policy

Asset manifests use schema `alife.p34.asset_manifest.v1`. Entries record asset
ID, kind, relative path, digest, required/optional presence, schema version,
size metadata when useful, and provenance. Required missing assets and digest
mismatches reject. Optional missing assets are tolerated.

The tiny committed P34 fixtures are JSON references for tests. They are not
large generated tensors, large logs, or binary runtime assets.

## Handoff to P35

P35 can consume P34 config/save/manifest files to choose a backend, locate
runtime assets, and load tiny deterministic worlds. P35 should keep the same
stable-ID and schema-rejection policy rather than adding product-specific
engine handles to portable saves.
