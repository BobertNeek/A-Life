# alife_tools

Developer asset builders, offline report readers, benchmarks, and persistence
validators. Normal gameplay does not depend on this crate.

## Asset and content tools

- [`creature_part_builder`](src/bin/creature_part_builder.rs) analyzes, validates,
  previews, and builds creature part packs.
- [`terrain_atlas_builder`](src/bin/terrain_atlas_builder.rs) builds albedo, normal,
  and ORM atlases from a source sheet and generation configuration.
- [`g16_content_authoring`](src/bin/g16_content_authoring.rs) validates content
  packs, world presets, lessons, and creature presets.

```text
cargo run -p alife_tools --bin terrain_atlas_builder -- SOURCE.png CONFIG.json OUTPUT_DIRECTORY
cargo run -p alife_tools --bin g16_content_authoring -- validate-pack content/fixtures/g16/content_pack_manifest.json
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

## Benchmarks and retained evidence

[`benchmark_tiers`](src/bin/benchmark_tiers.rs) reports declared population/class
budgets and can explicitly measure the real GPU runtime with `--gpu-runtime`.
Hardware execution is separate from deterministic fixtures and planning estimates.

The `ei0_exit_gate`, `era1_evolution`, and `era1_promotion` libraries read and validate
retained source-bound reports. `p33_evaluation` evaluates packed world records;
`p33_selection` plans managed pairings of real `CreatureGenome` parents using core
reproduction and phenotype compilation. Historical heuristic fixture results do
not demonstrate GPU execution or learned intelligence.

See [Development](../../docs/DEVELOPMENT.md) for current launch and validation
commands and [Evidence](../../docs/EVIDENCE.md) for retained measurements and limits.
