# Task 5C Cross-Torso Receipt

## Progress Checkpoint

- Baseline Python gate: `python scripts/test_geneforge_creature_recipes.py` timed out after `124028 ms` with no test output and no `target/artifacts/geneforge-import-tests/staging-a` produced. The fixture setup invokes Blender; after timeout no Python or Blender process remained. An unrelated Cargo process for `alife_gpu_backend` was observed and was not killed.
- Baseline app gate: `cargo test -p alife_game_app creature_part_catalog --lib -j 1 --offline -- --nocapture` initially failed to compile because the partial edit declared a 53-element `UNUSED_K` array with 49 values. The array was removed, exposing a second partial-edit defect: the inline SHA-256 round-key array had 32 values but was indexed for 64 rounds. That implementation is being repaired before the contract tests can run.
- Baseline tools gate: `cargo test -p alife_tools --test creature_part_visual_contract -j 1 --offline -- --nocapture` reached the same app compile failure and therefore did not execute.

Next: finish the SHA-256/app contract repair, add the independent `alife_tools` v2 staged parser and exact 252/432/684 accounting, then run the focused gates again before the deterministic no-Blender receipt verification.

## Completion Receipt (2026-07-18)

### Fixes closed

- `cargo fmt --all -- --check` first reproduced formatting drift in the three edited Rust files. `cargo fmt --all` repaired it and the final check passed.
- The normal build postprocess had accidentally generated 288 cross-torso slot records and then required zero. It now emits canonical-only v2 metadata (`180` slot records, `252` group keys); only `augment-cross-torso` creates alternate-target records.
- Canonical torso-frame records are excluded from the all-three-target and matrix-alias rules. Each non-torso source group still requires three target torso IDs and three distinct matrices.
- Repeated augmentation now validates and returns a deterministic byte-identical no-op when all 42 socket manifests and the recipe/receipt metadata already equal the complete result.
- Failed preflight now removes a provisionally created output recipe. The sentinel atomicity test proves staging, input recipe, sentinel bytes, and temporary/rollback sibling state remain unchanged.
- App and staged validators now reject source family/asset mismatch, invalid target/LOD/group/socket/space/schema, non-finite/non-affine matrices, residual drift, duplicate keys, matrix aliasing, count drift, and stable-hash drift.

### Exact focused gates

Environment for Cargo gates: `CARGO_TARGET_DIR=D:\A life\target`, `CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`, `-j 1`, `--offline`.

- `cargo fmt --all -- --check`: PASS, exit `0`.
- `cargo test -p alife_game_app creature_part_catalog --lib -j 1 --offline -- --nocapture`: PASS, `25 passed; 0 failed; 112 filtered out`, tests `1.03s`, command `12.7s`.
- `cargo test -p alife_tools --test creature_part_visual_contract -j 1 --offline -- --nocapture`: PASS, `13 passed; 0 failed`, tests `106.07s`, command `107.5s`.
- `python scripts/test_geneforge_creature_recipes.py`: PASS, `81 tests`, `271.894s`, command `272.6s`.
- The Python suite includes direct mocks that make Blender discovery and subprocess execution fatal inside `command_augment_cross_torso`; the command passed without invoking either.
- The Python suite includes a pre-existing sentinel failure case and verifies no output recipe, augmentation temporary, or rollback sibling remains.

### Canonical real staging refresh

- Normal build only (Blender allowed): `python scripts/build_geneforge_creature_parts.py build --source-root "E:\Creatures Reborn\resources for gpt\Geneforge4" --recipes crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json --staging target/artifacts/creature_parts/geneforge-staging`.
- Result: PASS in `250.1s`, `outputs=168`.
- Canonical digest binding: `python scripts/build_geneforge_creature_parts.py bind-output-digests --staging target/artifacts/creature_parts/geneforge-staging --recipes crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json --output crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json`.
- Result: PASS in `9.9s`, `bound_outputs=168`, canonical recipe digest `316d0a54faee2e10ca6df244c6bcc1a1f072bb7f4f8f5b45135500ef2cef6df2`.

### Real no-Blender deterministic augmentation

Two clean copies of `target/artifacts/creature_parts/geneforge-staging` were created at:

- `target/artifacts/creature_parts/task-5c-cross-torso-final-a/staging`
- `target/artifacts/creature_parts/task-5c-cross-torso-final-b/staging`

Each copy was augmented with `augment-cross-torso` using no `--source-root`, no Blender argument, and a separate `geneforge_recipes.cross-torso.json` output. Both commands passed (`8.3s` and `9.0s`) and reported:

- canonical slot records: `180`
- cross-torso slot records: `288`
- canonical group keys: `252`
- cross-torso group keys: `432`
- total unique group keys: `684`
- unchanged OBJ/semantic/anatomy outputs: `126`
- changed socket manifests: `42`
- receipt outputs: `168`
- donor count: `3`
- asset count: `14`
- augmented recipe digest: `86193a7f5613e0ab35478e4a9d8bdec936f5ee845bc4bae9bf4f0991a4187445`

Independent disk hashing proved:

- complete staging trees: byte-identical, `169` files each;
- emitted recipe bytes: identical, file SHA-256 `29725ef33286fe59aa8b5dbe81f6462d9d0373b06257fe7d4342a9f9fad6e6e8`;
- build receipts: identical, SHA-256 `061d4e847c61065dfc970ed18a52cda8895ae633eaf82965073a99212a9521fd`;
- receipt output digest maps: identical;
- OBJ hashes: unchanged `42/42` from canonical and identical A/B;
- semantic hashes: unchanged `42/42` from canonical and identical A/B;
- anatomy hashes: unchanged `42/42` from canonical and identical A/B;
- socket manifests: changed `42/42` from canonical and identical A/B;
- augmentation transaction leftovers: `0`.

### Scope and status

- Modified implementation remains confined to the six Task 5C files. This report is the required SDD receipt.
- No Task 6 runtime/render file, appearance genome, save schema, lineage behavior, production asset geometry, semantic pixels, or anatomy pixels was changed.
- Task 5C status: `DONE`. Task 6 remains separate and incomplete by design.

## Rejected Review Fix Receipt (2026-07-18)

### Fixes

- `_promote_augmented_pair` now invokes verified transaction recovery immediately when either the staged-tree replacement or recipe replacement raises an ordinary exception. Both fault positions restore the original staging tree, receipt, recipe, and binary sentinel byte-for-byte and remove the temporary tree, rollback tree, recipe temporary, recipe rollback, and transaction marker.
- Python preparation validation now builds a fresh oracle from recipe data plus source/target same-LOD socket manifests and binds slot matrices, group matrices, source/target/transformed anchors, bridge matrices, residuals, and prepared-geometry evidence to that oracle.
- The Rust staged validator now accepts exactly two complete states: canonical-only `180/0` slots and `252/0/252` groups, or augmented `180/288` slots and `252/432/684` groups. Receipt, every manifest population label, and observed records must agree; partial and mixed states are rejected. The all-three-target and matrix-distinctness rules remain mandatory for augmented state.

### RED

- `python scripts/test_geneforge_creature_recipes.py GeneForgeRecipeContractTests.test_augmentation_promotion_failures_restore_original_pair_without_leftovers`
  - RED, exit `1`, `1` test with both `staging` and `recipe` subtests failing. The staging failure left no live staging directory; the recipe failure left the promoted staging generation active.
- `python scripts/test_geneforge_creature_recipes.py GeneForgeImporterSubprocessTests.test_augmentation_promotion_failures_restore_original_pair_without_leftovers GeneForgeImporterSubprocessTests.test_preparation_validation_rejects_independently_wrong_matrix_and_anchor_evidence`
  - RED, exit `1`, `29.8s`. The first selector was a class-name typo; the matrix/evidence test executed and all six mutations were accepted: `prepared-matrix`, `slot-source-anchor`, `group-target-anchor`, `group-transformed-anchor`, `bridge-matrix`, and `residual`.
- `cargo test -p alife_tools --test creature_part_visual_contract staged_validator_accepts_only_complete_canonical_or_augmented_populations -j 1 --offline -- --nocapture`
  - RED, exit `1`, `0 passed; 1 failed`; the valid canonical-only staging failed at the augmented-only receipt metadata gate.

### Focused GREEN

Cargo environment: `CARGO_TARGET_DIR=D:\A life\target`, `CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`, `-j 1`, `--offline`.

- `python scripts/test_geneforge_creature_recipes.py GeneForgeRecipeContractTests.test_augmentation_promotion_failures_restore_original_pair_without_leftovers GeneForgeImporterSubprocessTests.test_preparation_validation_rejects_independently_wrong_matrix_and_anchor_evidence`
  - PASS, exit `0`, `2 tests`, `26.426s`.
- `cargo test -p alife_tools --test creature_part_visual_contract staged_validator_accepts_only_complete_canonical_or_augmented_populations -j 1 --offline -- --nocapture`
  - PASS, exit `0`, `1 passed; 0 failed; 13 filtered out`, test `30.64s`, command `37.8s`.
- `python scripts/test_geneforge_creature_recipes.py GeneForgeImporterSubprocessTests.test_augment_cross_torso_is_deterministic_no_blender_and_preserves_stable_bytes`
  - PASS, exit `0`, `1 test`, `31.862s` after widening the existing alias-mutation assertion to accept rejection by the new independent matrix oracle.
- `cargo test -p alife_tools --test creature_part_visual_contract staged_validator_rejects_task_5c_identity_count_and_stable_hash_mutations -j 1 --offline -- --nocapture`
  - PASS, exit `0`, `1 passed; 0 failed; 13 filtered out`, test `48.14s`, command `56.9s`.

### Requested Gates

- First full Python attempt: `python scripts/test_geneforge_creature_recipes.py`
  - Exit `1`, `83 tests`, `297.847s`, one expected-message mismatch: the alias mutation was rejected by `prepared_matrix drift` before the older `aliases target torso` branch. The assertion was corrected and its focused test passed.
- Final full Python gate: `python scripts/test_geneforge_creature_recipes.py`
  - PASS, exit `0`, `83 tests`, `320.372s`.
- `cargo test -p alife_game_app creature_part_catalog --lib -j 1 --offline -- --nocapture`
  - PASS, exit `0`, `25 passed; 0 failed; 112 filtered out`, test `1.49s`, command `3.1s`.
- `cargo test -p alife_tools --test creature_part_visual_contract -j 1 --offline -- --nocapture`
  - Exit `1`, `13 passed; 1 failed`, test `120.40s`. The implementation passed; the sole failure expected generic `metadata drift` after mutating receipt total `684` to `683`, while the new classifier correctly returned `partial assembly preparation population`. The assertion was corrected and the exact focused test passed. Per the user's time constraint, the full tools gate was not started again.
- `cargo fmt --all -- --check`: PASS after applying `cargo fmt --all` for one Rust line wrap.
- `git diff --check`: PASS; only Git's existing LF-to-CRLF working-copy warnings were emitted for the two Python files.
- Workspace-local `scripts/__pycache__` was removed. No Blender build was started during this fix pass. No remote ref was pushed.

### Self-review status

- Scope is four implementation/test files plus this required report.
- No production asset, recipe, geometry, mask, Task 6 runtime, save, lineage, or GUI surface changed.
- Status: `DONE` with one recorded verification concern: the complete tools test command was not rerun after its assertion-only message correction because the user explicitly ended long-running validation. The corrected focused mutation test is GREEN.
