# Task 5B Report

## RED

Focused Python contract tests were added before implementation and failed for
the intended missing production contracts:

- `test_outer_v2_contracts_remain_stable_with_anatomy_authoring_v1`: every
  production part asset raised `KeyError: 'anatomy_authoring'`.
- `test_all_lods_have_unique_confined_anatomy_outputs`: every production LOD
  raised `KeyError: 'anatomy_mask_sha256'`.
- `test_anatomy_rasterizer_rejects_conflicting_equal_priority_overlap`: the
  importer raised `AttributeError` because `anatomy_mask` did not exist.
- `test_anatomy_rasterizer_is_deterministic_and_preserves_occupancy`: the
  selected Norn head had no `anatomy_authoring` profile.

Command:

```powershell
python -m unittest scripts.test_geneforge_creature_recipes.GeneForgeRecipeContractTests.test_outer_v2_contracts_remain_stable_with_anatomy_authoring_v1 scripts.test_geneforge_creature_recipes.GeneForgeRecipeContractTests.test_all_lods_have_unique_confined_anatomy_outputs scripts.test_geneforge_creature_recipes.GeneForgeRecipeContractTests.test_anatomy_rasterizer_rejects_conflicting_equal_priority_overlap scripts.test_geneforge_creature_recipes.GeneForgeRecipeContractTests.test_anatomy_rasterizer_is_deterministic_and_preserves_occupancy
```

Result: `FAILED (errors=4)`.

## Implementation

- Preserved catalog/schema/importer/receipt/socket outer v2 contracts.
- Added nested `alife.geneforge_anatomy_authoring.v1` profiles to all 14 shared
  assets and anatomy path/digest records to all 42 LODs.
- Added one deterministic semantic-PNG-to-anatomy-PNG rasterizer used by both
  normal Blender fixture builds and `augment-anatomy`.
- Added exact seven-channel RGB ownership, coverage, shape, path, occupancy,
  transparent-pixel, receipt-set, digest, and budget validation in Python and
  Rust.
- Added atomic no-Blender augmentation with rollback. The production staging
  update generated 42 anatomy maps and 168 receipt-bound outputs while proving
  all 42 OBJ and 42 semantic hashes unchanged.
- Added separate semantic/anatomy mask counts to the Rust CLI receipt.

Production augmentation receipt:

```text
anatomy_masks=42
outputs=168
unchanged_obj_semantic=84
recipe_sha256=85b3a060ac11529d3d57db816de3eb41c773ac825d8cda9ab0bbcb909cf25b74
staging_files_including_receipt=169
staging_bytes_including_receipt=4315418
```

No real GeneForge Blender rebuild was run. Blender was used only by the
disposable fixture suite under ignored `target/artifacts/`.

## GREEN

Environment for Rust gates:

```powershell
$env:CARGO_TARGET_DIR='D:\A life\target'
$env:CARGO_BUILD_JOBS='1'
$env:RUST_TEST_THREADS='1'
```

Results:

- `python scripts/test_geneforge_creature_recipes.py`: PASS, 49 tests in
  110.596s.
- `cargo fmt --all -- --check`: PASS.
- `cargo test -p alife_game_app creature_part_catalog --lib -j 1 --offline`:
  PASS, 22 tests.
- `cargo test -p alife_tools creature_part_builder --lib -j 1 --offline`:
  PASS, 14 tests.
- `cargo test -p alife_tools --test creature_part_visual_contract -j 1 --offline`:
  PASS, 7 tests.
- `git diff --check`: PASS (line-ending conversion warnings only).

## Tracked Files

- `crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json`
- `scripts/build_geneforge_creature_parts.py`
- `scripts/test_geneforge_creature_recipes.py`
- `crates/alife_game_app/src/creature_part_catalog.rs`
- `crates/alife_tools/src/creature_part_builder.rs`
- `crates/alife_tools/src/bin/creature_part_builder.rs`
- `crates/alife_tools/tests/creature_part_visual_contract.rs`
- `.superpowers/sdd/task-5b-report.md`
