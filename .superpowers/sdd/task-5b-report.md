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

## Independent Review RED

The reviewer-started polygon and rectangular-mask tests were preserved and
expanded before implementation. Focused failures were observed for all five
Important findings:

- Python focused command (four methods): `FAILED (failures=4, errors=6)`.
  Both collinear/repeated polygons and the bow-tie polygon were accepted;
  `_validate_augmented_tree` accepted `64x63`; receipt mutations reached a
  `TypeError` because `_verify_receipt_outputs` had no recipe-bound source
  contract; and `canonical_path_is_within` did not exist.
- `cargo test -p alife_game_app
  geneforge_v2_rejects_repeated_collinear_and_self_intersecting_polygons --lib
  -j 1 --offline -- --nocapture`: FAIL, because catalog validation returned
  success for an invalid polygon.
- `cargo test -p alife_tools --test creature_part_visual_contract
  staged_validator_binds_each_receipt_source_to_its_exact_donor_outputs -j 1
  --offline -- --nocapture`: FAIL, because every corrupted `receipt.sources`
  case still returned `Ok(GeneForgeStagingValidation { ... })`.
- `cargo test -p alife_tools --test creature_part_visual_contract
  staged_validator_requires_native_rgba8_and_filter_zero_png_rows -j 1
  --offline -- --nocapture`: FAIL. An RGB PNG reached semantic-color checking
  and produced `occupied semantic colors do not match`, proving native PNG
  encoding was not enforced.
- Rust confinement no-run command: compile RED `E0432`, unresolved import
  `canonical_path_is_within`, proving the testable pure containment contract
  was absent.

## Independent Review GREEN

Implementation receipts:

- Python validates both mask dimensions as exactly `64x64`, rejects rejected
  augmentation before replacement, and leaves staging plus recipe bytes
  unchanged.
- Python and Rust reject lexical escapes plus symlink/junction/reparse output
  components before staged reads, hashes, size checks, or rewrites. The Windows
  symlink regression ran successfully; the pure canonical containment helper
  also covers hosts where link creation is unavailable.
- Python and Rust reject repeated vertices, degenerate edges, normalized UV
  polygon area at or below the documented `1e-6` epsilon, collinear polygons,
  and self-intersections.
- Receipt v2 now deserializes and validates exact donor-owned source accounting:
  donor set, asset/output counts, per-source uniqueness, cross-source
  uniqueness, ownership, and top-level union. Python applies the same contract
  to both 126-output legacy and 168-output augmented receipts.
- Rust validates native `64x64` RGBA8 PNG headers and exact decoded lengths,
  then uses a bounded dependency-free zlib/DEFLATE reader to require filter-zero
  rows before semantic interpretation. RGB, indexed, grayscale, and nonzero
  filter regressions all rebind receipt digests and are rejected at encoding.
- Successful augmentation preserves established receipt list ordering. Once
  both final replacements commit, rollback-directory cleanup is best-effort and
  cannot report a false failure with final state already installed.

Final commands and results:

- `python scripts/test_geneforge_creature_recipes.py`: PASS, 54 tests in
  118.055s.
- `cargo fmt --all -- --check`: PASS.
- `cargo test -p alife_game_app creature_part_catalog --lib -j 1 --offline`:
  PASS, 23 tests.
- `cargo test -p alife_tools creature_part_builder --lib -j 1 --offline`:
  PASS, 14 tests.
- `cargo test -p alife_tools --test creature_part_visual_contract -j 1
  --offline`: PASS, 10 tests; test execution 41.65s.
- `cargo run -p alife_tools --bin creature_part_builder -j 1 --offline --
  validate-geneforge-staging --staging
  target/artifacts/creature_parts/geneforge-staging --recipes
  crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json`:
  PASS, 3 donors, 14 assets, 42 LODs, 42 OBJ, 42 semantic, 42 anatomy,
  4,315,418 bytes including receipt.

Read-only real-staging audit:

```text
recipe_sha256=85b3a060ac11529d3d57db816de3eb41c773ac825d8cda9ab0bbcb909cf25b74
receipt_outputs=168
validated_outputs=168
obj_semantic_hashes_unchanged=84
staging_files_including_receipt=169
staging_bytes_including_receipt=4315418
reparse_entries=0
```

No real Blender rebuild or reaugmentation was required. Outer catalog schema
v2, `schema_version=2`, importer v2, receipt v2, socket v2, recipe digest, and
all ignored production staging bytes remain unchanged.

Independent review changed files:

- `scripts/build_geneforge_creature_parts.py`
- `scripts/test_geneforge_creature_recipes.py`
- `crates/alife_game_app/src/creature_part_catalog.rs`
- `crates/alife_tools/src/creature_part_builder.rs`
- `crates/alife_tools/tests/creature_part_visual_contract.rs`
- `.superpowers/sdd/task-5b-report.md`

## Tracked Files

- `crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json`
- `scripts/build_geneforge_creature_parts.py`
- `scripts/test_geneforge_creature_recipes.py`
- `crates/alife_game_app/src/creature_part_catalog.rs`
- `crates/alife_tools/src/creature_part_builder.rs`
- `crates/alife_tools/src/bin/creature_part_builder.rs`
- `crates/alife_tools/tests/creature_part_visual_contract.rs`
- `.superpowers/sdd/task-5b-report.md`
