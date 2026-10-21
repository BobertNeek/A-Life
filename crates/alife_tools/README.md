# alife_tools

Developer asset builders, GPU diagnostics, and persistence
validators. Normal gameplay does not depend on this crate.

## Asset tools

- [`creature_part_builder`](src/bin/creature_part_builder.rs) analyzes, validates,
  previews, and builds creature part packs.
- [`terrain_atlas_builder`](src/bin/terrain_atlas_builder.rs) builds albedo, normal,
  and ORM atlases from a source sheet and generation configuration.

```text
cargo run -p alife_tools --bin terrain_atlas_builder -- SOURCE.png CONFIG.json OUTPUT_DIRECTORY
```

Keep useful editable inputs alongside exported runtime assets and refresh manifest
digests when registered outputs change.

## Persistence validation

[`p34_persistence`](src/bin/p34_persistence.rs) validates portable save, runtime
configuration, and asset-manifest files. It checks data contracts; it does not
exercise the player's save/load UI.

```text
cargo run -p alife_tools --bin p34_persistence -- validate-fixtures crates/alife_world/tests/fixtures/p34
cargo run -p alife_tools --bin p34_persistence -- validate-save crates/alife_world/tests/fixtures/p34/tiny_save.json crates/alife_world/tests/fixtures/p34
```

## GPU diagnostics

[`benchmark_tiers`](src/bin/benchmark_tiers.rs) measures actual closed-loop GPU work
with `--backend gpu-closed-loop`, selected classes, sensor profiles, population
rows, seed, target file, and output path. It can also write a canonical target file
or validate an existing report. Custom target sets and selected row subsets are
supported. Targets are comparison inputs; a planning estimate or CPU fixture is
not a hardware measurement.

The retired EI report/lab tools and their reports remain in Git history. Current
world reproduction, creature state, and learned checkpoints stay in their owning
runtime/data paths.

See [Development](../../docs/DEVELOPMENT.md) for current launch and validation
commands and [Evidence](../../docs/EVIDENCE.md) for retained measurements and limits.
